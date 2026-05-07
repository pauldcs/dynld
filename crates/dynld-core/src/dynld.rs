use core::ptr::{self};

extern crate alloc;
use alloc::vec::Vec;

use array::{array_string::ArrayString, array_vec::ArrayVec};
use container::{Container, Endian};
use dynld_std::println;
use sys::mach::{vm_alloc_task_self, vm_copy_into_task_self, vm_protect};

use crate::{
    PAGE_ZERO_SIZE,
    bindings::macho::{
        CPU_TYPE_ARM64, LC_REQ_DYLD, N_ALT_ENTRY, N_ARM_THUMB_DEF, N_COLD_FUNC, N_EXT, N_PEXT,
        N_SECT, N_STAB, N_SYMBOL_RESOLVER, N_TYPE, N_UNDF, N_WEAK_DEF, N_WEAK_REF,
        S_INIT_FUNC_OFFSETS, S_THREAD_LOCAL_VARIABLES, SECTION_TYPE, SEG_DATA, SEG_TEXT,
        dylib_command, entry_point_command, fat_arch, fat_header, get_library_ordinal,
        load_command, load_command_variants, mach_header_64, mach_magic, macho_filetype_variants,
        nlist_64, section_64, segment_command_64, symtab_command,
    },
    dsc::DyldSharedCache,
    fixups::fixup_all_chained_fixups,
    image::{Image, Segment, Symbol},
    tlv::tlv_initialize_descriptors_export,
};

pub fn macho_endian_from_magic(magic: mach_magic) -> Endian {
    match magic {
        mach_magic::MH_MAGIC | mach_magic::MH_MAGIC_64 => Endian::Little,
        mach_magic::MH_CIGAM
        | mach_magic::MH_CIGAM_64
        | mach_magic::FAT_MAGIC
        | mach_magic::FAT_MAGIC_64
        | mach_magic::FAT_CIGAM
        | mach_magic::FAT_CIGAM_64 => Endian::Big,
    }
}

/// Dynamically link the program
///
/// This function performs the full dynamic linking of the program.
/// Once this returns, we can safely jump to its entrypoint without
/// troubles
pub fn dynamic_link(
    dyld_shared_cache: &DyldSharedCache<'_>,
    program_header: *mut u8,
    program_size: usize,
) -> Result<u64, &'static str> {
    let mut segments = ArrayVec::<Segment, 64>::new_array();
    let mut fixups = Vec::new();
    let mut symbols = Vec::new();
    let mut init_functions = ArrayVec::<usize, 64>::new_array();
    let mut libraries = ArrayVec::<(&str, usize), 64>::new_array();
    let mut has_thread_locals = false;

    let mut is_dylib = false;
    let mut offset = size_of::<mach_header_64>();
    let mut entry_point = 0;

    let magic = unsafe { ptr::read_unaligned(program_header as *const mach_magic) };
    let image_container = arm64_container_create(&Container::with_bytes(
        unsafe { core::slice::from_raw_parts(program_header, program_size) },
        macho_endian_from_magic(magic),
    ))?;

    let mach_header_64 {
        filetype, ncmds, ..
    } = image_container
        .deserialize_type_at_offset::<mach_header_64>(0)
        .map_err(|_| "could not read `mach_header_64` from arm64 image")?;

    if !matches!(filetype, macho_filetype_variants::MH_EXECUTE) {
        if matches!(filetype, macho_filetype_variants::MH_DYLIB) {
            is_dylib = true;
        } else {
            return Err("the arm64 image filetype must be MH_EXECUTE");
        }
    }
    for _ in 0..ncmds {
        let load_command { cmd, cmdsize } = image_container
            .deserialize_type_at_offset(offset)
            .map_err(|_| "could not parse a load command in the inner arm64 image")?;

        match cmd {
            // DYLIB COMMANDS
            load_command_variants::LC_LOAD_DYLIB
            | load_command_variants::LC_LOAD_WEAK_DYLIB
            | load_command_variants::LC_LAZY_LOAD_DYLIB => {
                let dylib_command { dylib, .. } = image_container
                    .deserialize_type_at_offset(offset)
                    .map_err(|_| "could not parse a dylib_command in the inner arm64 image")?;

                libraries.push((
                    image_container
                        .deserialize_string_at_offset(offset + dylib.name.offset as usize),
                    dylib.name.offset as usize,
                ));
            }
            load_command_variants::LC_MAIN => {
                if is_dylib {
                    return Err("a dynamic library should not contain a `LC_MAIN` load command");
                }

                let entry_point_command { entryoff, .. } =
                    image_container.deserialize_type_at_offset(offset).map_err(
                        |_| "could not parse `entry_point_command` in the inner arm64 image",
                    )?;

                entry_point = entryoff as usize;
            }
            // SYMTAB COMMAND
            load_command_variants::LC_SYMTAB => {
                let symtab_command {
                    symoff,
                    nsyms,
                    stroff,
                    ..
                } = image_container
                    .deserialize_type_at_offset(offset)
                    .map_err(|_| "could not parse `symtab_command` in the inner arm64 image")?;

                let mut nlist_offset = symoff as usize;

                for _ in 0..nsyms {
                    let nlist_64 {
                        n_un,
                        n_type,
                        n_value,
                        n_desc,
                        n_sect,
                    } = image_container
                        .deserialize_type_at_offset(nlist_offset)
                        .map_err(|_| "could not parse `nlist_64` in the inner arm64 image")?;

                    if n_type & N_STAB != 0 {
                        // don't care about debug symbols
                        continue;
                    }

                    match n_type & N_TYPE {
                        N_SECT => {
                            if (n_type & N_EXT) == 0 {
                                if n_desc & N_ALT_ENTRY != 0 {
                                    if n_type & N_PEXT != 0 {
                                        return Err(
                                            "unsupported makeAltEntry with Scope::wasLinkageUnit",
                                        );
                                    } else {
                                        return Err("makeAltEntry with Scope::translationUnit");
                                    }
                                } else if n_type & N_PEXT != 0 {
                                    if n_desc & N_WEAK_DEF != 0 {
                                        return Err("makeWeakDefWasPrivateExtern");
                                    } else {
                                        symbols.push(Symbol::make_regular_export(
                                            ArrayString::try_from_str(
                                                image_container.deserialize_string_at_offset(
                                                    (stroff + n_un.n_strx) as usize,
                                                ),
                                            )
                                            .ok_or_else(|| "could not make_regular_export")?,
                                            n_value as usize,
                                            n_sect,
                                            (n_desc & N_COLD_FUNC) != 0,
                                            (n_desc & N_ARM_THUMB_DEF) != 0,
                                        ))
                                    }
                                } else {
                                    symbols.push(Symbol::make_regular_local(
                                        ArrayString::try_from_str(
                                            image_container.deserialize_string_at_offset(
                                                (stroff + n_un.n_strx) as usize,
                                            ),
                                        )
                                        .ok_or_else(|| "could not make_regular_local")?,
                                        n_value as usize,
                                        n_sect,
                                        (n_desc & N_COLD_FUNC) != 0,
                                        (n_desc & N_ARM_THUMB_DEF) != 0,
                                    ))
                                }
                            } else if n_type & N_PEXT != 0 {
                                if n_desc & N_ALT_ENTRY != 0 {
                                    return Err("makeAltEntry with Scope::linkageUnit");
                                } else if n_desc & N_WEAK_DEF != 0 {
                                    return Err("makeWeakDefHidden");
                                } else if n_desc & N_SYMBOL_RESOLVER != 0 {
                                    return Err("makeDynamicResolver with Scope::linkageUnit");
                                } else {
                                    return Err("makeRegularHidden");
                                }
                            } else if n_desc & N_ALT_ENTRY != 0 {
                                return Err("makeAltEntry with Scope::global");
                            } else if (n_desc & (N_WEAK_DEF | N_WEAK_REF))
                                == (N_WEAK_DEF | N_WEAK_REF)
                            {
                                return Err("makeWeakDefAutoHide");
                            } else if n_desc & N_WEAK_DEF != 0 {
                                symbols.push(Symbol::make_weak_def_export(
                                    ArrayString::try_from_str(
                                        image_container.deserialize_string_at_offset(
                                            (stroff + n_un.n_strx) as usize,
                                        ),
                                    )
                                    .ok_or_else(|| "could not make_weak_def_export")?,
                                    n_value as usize,
                                    n_sect,
                                    (n_desc & N_COLD_FUNC) != 0,
                                    (n_desc & N_ARM_THUMB_DEF) != 0,
                                ))
                            } else if n_desc & N_SYMBOL_RESOLVER != 0 {
                                return Err("makeDynamicResolver");
                            } else {
                                symbols.push(Symbol::make_regular_export(
                                    ArrayString::try_from_str(
                                        image_container.deserialize_string_at_offset(
                                            (stroff + n_un.n_strx) as usize,
                                        ),
                                    )
                                    .ok_or_else(|| "could not make_regular_export")?,
                                    n_value as usize,
                                    n_sect,
                                    (n_desc & N_COLD_FUNC) != 0,
                                    (n_desc & N_ARM_THUMB_DEF) != 0,
                                ))
                            }
                        }
                        N_UNDF => {
                            if n_value == 0 {
                                symbols.push(Symbol::make_undefined(
                                    ArrayString::try_from_str(
                                        image_container.deserialize_string_at_offset(
                                            (stroff + n_un.n_strx) as usize,
                                        ),
                                    )
                                    .ok_or_else(|| "could not make_regular_export")?,
                                    get_library_ordinal(n_desc),
                                    (n_desc & N_WEAK_REF) != 0,
                                ))
                            } else if n_type & N_PEXT != 0 {
                                unimplemented!("N_PEXT: n_type: {}", n_type & N_TYPE)
                            } else {
                                unimplemented!("n_type: {}", n_type & N_TYPE)
                            }
                        }
                        _ =>
                        /*println!("[warning]: ignoring n_type: {}", n_type & N_TYPE)*/
                        {
                            ()
                        }
                    }
                    nlist_offset += size_of::<nlist_64>();
                }
            }
            // SEGMENT COMMAND
            load_command_variants::LC_SEGMENT_64 => {
                let segment_command_64 {
                    initprot,
                    vmaddr: vm_addr,
                    vmsize: vm_size,
                    fileoff: file_off,
                    segname,
                    nsects,
                    ..
                } = image_container
                    .deserialize_type_at_offset(offset)
                    .map_err(|_| "could not parse `segment_command_64` in the inner arm64 image")?;

                if segname.as_ref().starts_with(SEG_TEXT.as_ref()) {
                    for sect_index in 0..nsects {
                        let section_64 {
                            flags,
                            mut offset,
                            size: sect_size,
                            ..
                        } = image_container
                            .deserialize_type_at_offset(
                                offset
                                    + size_of::<segment_command_64>()
                                    + sect_index as usize * size_of::<section_64>(),
                            )
                            .map_err(
                                |_| "could not parse `segment_command_64` in the inner arm64 image",
                            )?;

                        if (flags & SECTION_TYPE) == S_INIT_FUNC_OFFSETS {
                            let func_end_offset = offset + sect_size as u32;
                            while offset < func_end_offset {
                                init_functions.push(
                                    image_container
                                        .deserialize_type_at_offset::<u32>(offset as usize)
                                        .map_err(|_| "could not parse S_INIT_FUNC_OFFSETS")?
                                        as usize,
                                );

                                offset += size_of::<u32>() as u32;
                            }
                        } else {
                            //println_err!("[warning] ignoring {} section", section_type_name(flags));
                        }
                    }
                } else if segname.as_ref().starts_with(SEG_DATA.as_ref()) {
                    for sect_index in 0..nsects {
                        let section_64 {
                            flags,
                            offset: mut start_off,
                            size: sect_size,
                            ..
                        } = image_container
                            .deserialize_type_at_offset(
                                offset
                                    + size_of::<segment_command_64>()
                                    + sect_index as usize * size_of::<section_64>(),
                            )
                            .map_err(
                                |_| "could not parse `segment_command_64` in the inner arm64 image",
                            )?;

                        match flags & SECTION_TYPE {
                            S_THREAD_LOCAL_VARIABLES => {
                                has_thread_locals = true;
                            }
                            S_INIT_FUNC_OFFSETS => {
                                let func_end_offset = start_off + sect_size as u32;
                                while start_off < func_end_offset {
                                    init_functions.push(
                                        image_container
                                            .deserialize_type_at_offset::<u32>(offset as usize)
                                            .map_err(|_| "could not parse S_INIT_FUNC_OFFSETS")?
                                            as usize,
                                    );

                                    start_off += size_of::<u32>() as u32;
                                }
                            }
                            _ => {
                                //println_err!(
                                //    "[warning] ignoring {} section",
                                //    section_type_name(flags)
                                //);
                            }
                        }
                    }
                }

                segments.push(Segment {
                    prot: initprot,
                    vm_addr,
                    file_offset: file_off as usize,
                    vm_size: vm_size as usize,
                });
            }
            // CHAINED FIXUPS COMMAND
            load_command_variants::LC_DYLD_CHAINED_FIXUPS => {
                fixups = Image::with_container(&image_container).chained_fixups_parse_all()?;
            }

            command if command & LC_REQ_DYLD == 0 => {
                //println_err!("[warning] ignoring LC_REQ_DYLD load command: {command:?}");
            }
            _ => {}
        }
        offset += cmdsize as usize;
    }

    if has_thread_locals {
        unsafe {
            tlv_initialize_descriptors_export(program_header as *const mach_header_64);
        }
    }

    let (min_addr, max_addr) = segments
        .iter()
        .map(
            |Segment {
                 vm_addr, vm_size, ..
             }| { (vm_addr, *vm_addr + *vm_size as u64) },
        )
        .fold((u64::MAX, 0), |(min, max), (start, end)| {
            (min.min(*start), max.max(end))
        });

    let vm_size = (max_addr - min_addr) as usize;
    let vm = vm_alloc_task_self(vm_size).unwrap_or_else(|kr| {
        println!("could not allocate memory {kr:?}");
        panic!("cannot allocate memory")
    });

    for Segment {
        vm_addr,
        vm_size,
        file_offset,
        ..
    } in &segments
    {
        if *vm_addr == 0 {
            continue;
        }

        unsafe {
            vm_copy_into_task_self(
                image_container.as_bytes().as_ptr().add(*file_offset).addr() as u64,
                vm.add(*vm_addr as usize).as_ptr().addr() as u64,
                *vm_size,
            );
        }
    }

    let mut loaded_dylibs = ArrayVec::<&str, 32>::new_array();
    for (dylib_name, _) in libraries {
        loaded_dylibs.push(dylib_name);
    }

    fixup_all_chained_fixups(
        vm.as_ptr() as *mut u8,
        PAGE_ZERO_SIZE,
        &fixups,
        &symbols,
        &loaded_dylibs,
        Some(dyld_shared_cache),
        false,
    )?;

    unsafe {
        for Segment {
            vm_addr,
            vm_size,
            prot,
            ..
        } in &segments
        {
            [false, true].into_iter().for_each(|max| {
                vm_protect(
                    vm.add(*vm_addr as usize).as_ptr().addr() as u64,
                    *vm_size,
                    max as u32,
                    *prot as i32,
                )
                .map_err(|_| "could not apply vm protections to a memory region")
                // fix ts
                .unwrap()
            });
        }
    }

    //dyld_shared_cache.vm_protect_executable().unwrap();

    Ok(unsafe { vm.add(entry_point).add(PAGE_ZERO_SIZE).as_ptr().addr() as u64 })
}

/// Tries to find an image with a `CPU_TYPE_ARM64` cputype
///
/// If the container is a fat_magic image, we try to find
/// the correct architecture within it
fn arm64_container_create<'bytes>(
    image_container: &Container<'bytes>,
) -> Result<Container<'bytes>, &'static str> {
    let header = image_container
        .deserialize_type_at_offset::<mach_header_64>(0)
        .unwrap();

    // if the image already has a CPU_TYPE_ARM64 cputype,
    // we can return it
    if header.magic == mach_magic::MH_MAGIC_64 && header.cputype == CPU_TYPE_ARM64 {
        return Ok(*image_container);
    }

    // if this is not a fat binary and is not a CPU_TYPE_ARM64,
    // we return an error
    if header.magic != mach_magic::FAT_MAGIC {
        return Err("Not a valid ARM64 Mach-O or fat binary");
    }

    let fat_header = image_container
        .deserialize_type_at_offset::<fat_header>(0)
        .unwrap();
    (0..fat_header.nfat_arch)
        .filter_map(|index| {
            let offset = size_of::<fat_header>() + index as usize * size_of::<fat_arch>();

            let fat_arch { offset, size, .. } = image_container
                .deserialize_type_at_offset::<fat_arch>(offset)
                .ok()?;

            image_container
                .slice(offset as usize, size as usize)
                .map(|bytes| {
                    Container::with_bytes(
                        bytes,
                        macho_endian_from_magic(unsafe {
                            ptr::read_unaligned(bytes.as_ptr() as *const mach_magic)
                        }),
                    )
                })
        })
        .find(|container| {
            container
                .deserialize_type_at_offset::<mach_header_64>(0)
                .map(|h| h.magic == mach_magic::MH_MAGIC_64 && h.cputype == CPU_TYPE_ARM64)
                .unwrap_or(false)
        })
        .map(|container| arm64_container_create(&container))
        .ok_or_else(|| "error: could not read `mach_header_64` from inner mach-o")?
}
