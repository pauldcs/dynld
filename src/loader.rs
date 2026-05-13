use core::ptr::NonNull;

extern crate alloc;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    PAGE_ZERO_SIZE, SYMBOL_NAME_LEN,
    array::{ArrayString, ArrayVec},
    bindings_macho::{
        LC_REQ_DYLD, N_ALT_ENTRY, N_ARM_THUMB_DEF, N_COLD_FUNC, N_EXT, N_PEXT, N_SECT, N_STAB,
        N_SYMBOL_RESOLVER, N_TYPE, N_UNDF, N_WEAK_DEF, N_WEAK_REF, S_INIT_FUNC_OFFSETS,
        S_THREAD_LOCAL_VARIABLES, SECTION_TYPE, SEG_DATA, SEG_TEXT, dylib_command,
        entry_point_command, get_library_ordinal, load_command, load_command_variants,
        mach_header_64, mach_magic, macho_filetype_variants, nlist_64, rpath_command, section_64,
        segment_command_64, symtab_command,
    },
    container::{self, Container},
    dyld_shared_cache::DyldSharedCache,
    fixups::{Fixup, fixup_all_chained_fixups},
    format,
    image::{Image, Segment, Symbol},
    jump, mach, tlv,
    user_dylibs::disk_path_mmap,
};

pub fn macho_endian_from_magic(magic: mach_magic) -> container::Endian {
    match magic {
        mach_magic::MH_MAGIC | mach_magic::MH_MAGIC_64 => container::Endian::Little,
        mach_magic::MH_CIGAM
        | mach_magic::MH_CIGAM_64
        | mach_magic::FAT_MAGIC
        | mach_magic::FAT_MAGIC_64
        | mach_magic::FAT_CIGAM
        | mach_magic::FAT_CIGAM_64 => container::Endian::Big,
    }
}

/// This is a placeholder at the root of a path to let you know
/// how to look for the rest of the path. eg:
///     @rpath/libfoo.dylib
pub enum Rpath {
    /// Resolves to the directory containing the binary that has this load command
    LoaderPath,
    /// Resolves to the directory containing the main executable
    ExecutablePath,
    /// A placeholder that gets resolved against the list of run-path search paths
    RPath,
}

/// Parsed contents of a Mach-O image, ready to be mapped and linked.
pub struct ParsedImage {
    segments: ArrayVec<Segment, 64>,
    fixups: Vec<Fixup>,
    symbols: Vec<Symbol>,
    init_functions: Vec<usize>,
    libraries: ArrayVec<(String, usize), 64>,
    pub entry_point: usize,
    has_thread_locals: bool,
    is_dylib: bool,
    pub rpath: Option<Rpath>,
}

pub enum Library<'image> {
    System(String),
    User(Container<'image>),
}

/// Dynamically link the program.
///
/// On return the image has been mapped, fixed up, and its initializers
/// have run, jumping to the returned address enters the program safely.
///
/// IMPORTANT: if this is a dylib , it returns the address at which it is loaded and its size,
/// if it is an executable it returns the address at which it is loaded and its entrypoint
pub fn macho_loader<'image>(
    dyld_shared_cache: &DyldSharedCache<'_>,
    image: Container<'image>,
    exec_dir: &str,
    dylib_dir: Option<&str>,
) -> Result<(NonNull<u8>, usize), &'static str> {
    let is_executable = dylib_dir.is_none();
    let parsed_image = macho_parser(&image)?;

    let dependencies = dependencies_collect(&parsed_image, dyld_shared_cache)?;
    let dylib_images = user_dylibs_load_recursively(
        &dependencies,
        dyld_shared_cache,
        exec_dir,
        &parsed_image.rpath,
    )?;

    let (vm, vm_size) =
        macho_segments_map_to_vm(dylib_dir.is_none(), &image, &parsed_image.segments)?;

    image_fixups_apply(
        vm,
        &parsed_image,
        &dylib_images,
        dyld_shared_cache,
        is_executable,
    )?;

    image_finalize(vm, &parsed_image, is_executable)?;

    match dylib_dir {
        Some(_) => Ok((vm, vm_size)),
        None => Ok((vm, unsafe {
            vm.add(parsed_image.entry_point)
                .add(PAGE_ZERO_SIZE)
                .addr()
                .into()
        })),
    }
}

/// Walks the parsed image's `libraries` and returns each path and
/// wether is is a system library or a user library
fn dependencies_collect(
    parsed_image: &ParsedImage,
    dyld_shared_cache: &DyldSharedCache<'_>,
) -> Result<ArrayVec<(String, bool), 32>, &'static str> {
    let mut dependencies: ArrayVec<(String, bool), 32> = ArrayVec::new_array();
    for (lib_name, _) in &parsed_image.libraries {
        if dependencies.len() == 32 {
            return Err("exceeded maximum amount of dylibs");
        }
        let is_system_dylib = dyld_shared_cache.is_library_cached(lib_name.as_bytes());
        dependencies.push((lib_name.to_string(), is_system_dylib));
    }
    Ok(dependencies)
}

/// For every non system libs: resolve its install name to a disk
/// path, mmap it, and recursively load it. Returns each loaded image
fn user_dylibs_load_recursively<'a>(
    dependencies: &ArrayVec<(String, bool), 32>,
    dyld_shared_cache: &DyldSharedCache<'_>,
    exec_dir: &str,
    rpath: &Option<Rpath>,
) -> Result<Vec<Library<'a>>, &'static str> {
    let mut loaded = Vec::new();
    for (install_name, is_system_dylib) in dependencies.iter() {
        if *is_system_dylib {
            loaded.push(Library::System(install_name.clone()));
            continue;
        }
        let dylib_path = user_dylib_disk_path_resolve(install_name, rpath, exec_dir);

        let (vm, vm_size) = macho_loader(
            dyld_shared_cache,
            disk_path_mmap(dylib_path.as_bytes())?,
            exec_dir,
            Some(&dylib_path),
        )?;

        loaded.push(Library::User(Container::with_bytes(
            unsafe { core::slice::from_raw_parts(vm.as_ptr() as *const u8, vm_size) },
            container::Endian::Little,
        )));
    }

    Ok(loaded)
}

/// Turns a dylib install name into a NUL terminated disk path
fn user_dylib_disk_path_resolve(
    install_name: &str,
    rpath: &Option<Rpath>,
    exec_dir: &str,
) -> String {
    let resolved = if let Some(rest) = install_name.strip_prefix("@rpath/") {
        match rpath {
            Some(Rpath::LoaderPath) => format!("{exec_dir}/{rest}"),
            Some(Rpath::ExecutablePath) => format!("{exec_dir}/{rest}"),
            Some(Rpath::RPath) => unimplemented!("@rpath search paths"),
            None => rest.to_string(),
        }
    } else if let Some(rest) = install_name.strip_prefix("@loader_path/") {
        format!("{exec_dir}/{rest}")
    } else if let Some(rest) = install_name.strip_prefix("@executable_path/") {
        format!("{exec_dir}/{rest}")
    } else {
        install_name.to_string()
    };

    format!("{resolved}\0")
}

/// Applies chained fixups to the newly mapped image.
fn image_fixups_apply(
    vm: NonNull<u8>,
    parsed_image: &ParsedImage,
    dependencies: &Vec<Library>,
    dyld_shared_cache: &DyldSharedCache<'_>,
    is_executable: bool,
) -> Result<(), &'static str> {
    let fixup_base_offset = if is_executable { PAGE_ZERO_SIZE } else { 0 };
    unsafe {
        fixup_all_chained_fixups(
            vm.as_ptr() as *mut u8,
            fixup_base_offset,
            &parsed_image.fixups,
            &parsed_image.symbols,
            dependencies,
            Some(dyld_shared_cache),
            false,
        )
    }
}

/// Initializes TLVs (executable only)
fn image_finalize(
    vm: NonNull<u8>,
    parsed_image: &ParsedImage,
    is_executable: bool,
) -> Result<(), &'static str> {
    if is_executable && parsed_image.has_thread_locals {
        macho_tlv_initialize(vm);
    }
    macho_segments_vm_protect(vm, &parsed_image.segments)?;
    macho_init_functions_exec(vm, &parsed_image.init_functions);
    Ok(())
}

/// Walk every load command once and gather everything we need to link
fn macho_parser<'a>(image: &Container<'a>) -> Result<ParsedImage, &'static str> {
    let mut parsed = ParsedImage {
        segments: ArrayVec::new_array(),
        fixups: Vec::new(),
        symbols: Vec::new(),
        init_functions: Vec::new(),
        libraries: ArrayVec::new_array(),
        entry_point: 0,
        has_thread_locals: false,
        is_dylib: false,
        rpath: None,
    };

    let mach_header_64 {
        filetype, ncmds, ..
    } = image
        .deserialize_type_at_offset::<mach_header_64>(0)
        .map_err(|_| "could not read `mach_header_64` from arm64 image")?;

    parsed.is_dylib = match filetype {
        macho_filetype_variants::MH_EXECUTE => false,
        macho_filetype_variants::MH_DYLIB => true,
        _ => return Err("the arm64 image filetype must be MH_EXECUTE"),
    };

    let mut offset = size_of::<mach_header_64>();
    for _ in 0..ncmds {
        let load_command { cmd, cmdsize } = image
            .deserialize_type_at_offset(offset)
            .map_err(|_| "could not parse a load command in the inner arm64 image")?;

        match cmd {
            load_command_variants::LC_LOAD_DYLIB
            | load_command_variants::LC_LOAD_WEAK_DYLIB
            | load_command_variants::LC_LAZY_LOAD_DYLIB => {
                parse_dylib_command(&image, offset, &mut parsed.libraries)?;
            }
            load_command_variants::LC_MAIN => {
                parsed.entry_point = parse_main_command(&image, offset, parsed.is_dylib)?;
            }
            load_command_variants::LC_SYMTAB => {
                parse_symtab(&image, offset, &mut parsed.symbols)?;
            }
            load_command_variants::LC_SEGMENT_64 => {
                parse_segment(
                    &image,
                    offset,
                    &mut parsed.segments,
                    &mut parsed.init_functions,
                    &mut parsed.has_thread_locals,
                )?;
            }
            load_command_variants::LC_DYLD_CHAINED_FIXUPS => {
                parsed.fixups = Image::with_container(&image).chained_fixups_parse_all()?;
            }
            load_command_variants::LC_RPATH => {
                parse_rpath(&image, offset, &mut parsed.rpath)?;
            }
            // this should maybe trigger a warning, we encountered a load command
            // that should require us to do something
            command if command & LC_REQ_DYLD == 0 => {}
            _ => {}
        }

        offset += cmdsize as usize;
    }

    Ok(parsed)
}

fn parse_dylib_command<'a>(
    image: &'a Container<'a>,
    offset: usize,
    libraries: &mut ArrayVec<(String, usize), 64>,
) -> Result<(), &'static str> {
    let dylib_command { dylib, .. } = image
        .deserialize_type_at_offset(offset)
        .map_err(|_| "could not parse a dylib_command in the inner arm64 image")?;

    libraries.push((
        image
            .deserialize_string_at_offset(offset + dylib.name.offset as usize)
            .to_string(),
        dylib.name.offset as usize,
    ));
    Ok(())
}

fn parse_main_command(
    image: &Container<'_>,
    offset: usize,
    is_dylib: bool,
) -> Result<usize, &'static str> {
    if is_dylib {
        return Err("a dynamic library should not contain a `LC_MAIN` load command");
    }
    let entry_point_command { entryoff, .. } = image
        .deserialize_type_at_offset(offset)
        .map_err(|_| "could not parse `entry_point_command` in the inner arm64 image")?;
    Ok(entryoff as usize)
}

fn parse_rpath(
    image: &Container<'_>,
    offset: usize,
    rpath: &mut Option<Rpath>,
) -> Result<(), &'static str> {
    let rpath_command { path, .. } = image
        .deserialize_type_at_offset(offset)
        .map_err(|_| "could not parse `rpath_command` in the inner arm64 image")?;

    match image.deserialize_string_at_offset(offset + path.offset as usize) {
        "@rpath" => *rpath = Some(Rpath::RPath),
        "@loader_path" => *rpath = Some(Rpath::LoaderPath),
        "@executable_path" => *rpath = Some(Rpath::ExecutablePath),
        _ => return Err("coulnt not parse rpath: unknown placeholder"),
    }

    Ok(())
}

/// Parse the symbol table, classifying each entry into a Symbol variant
/// or rejecting those that we dont support
fn parse_symtab(
    image: &Container<'_>,
    offset: usize,
    symbols: &mut Vec<Symbol>,
) -> Result<(), &'static str> {
    let symtab_command {
        symoff,
        nsyms,
        stroff,
        ..
    } = image
        .deserialize_type_at_offset(offset)
        .map_err(|_| "could not parse `symtab_command` in the inner arm64 image")?;

    let mut nlist_offset = symoff as usize;
    for _ in 0..nsyms {
        let entry: nlist_64 = image
            .deserialize_type_at_offset(nlist_offset)
            .map_err(|_| "could not parse `nlist_64` in the inner arm64 image")?;
        nlist_offset += size_of::<nlist_64>();

        // Debug symbols carry no linkage information.
        if entry.n_type & N_STAB != 0 {
            continue;
        }

        match entry.n_type & N_TYPE {
            N_SECT => classify_defined_symbol(image, stroff, &entry, symbols)?,
            N_UNDF => classify_undefined_symbol(image, stroff, &entry, symbols)?,
            _ => {} // ignore N_ABS, N_PBUD, N_INDR
        }
    }
    Ok(())
}

/// Decode an N_SECT entry
fn classify_defined_symbol(
    image: &Container<'_>,
    stroff: u32,
    entry: &nlist_64,
    symbols: &mut Vec<Symbol>,
) -> Result<(), &'static str> {
    let name = match read_symbol_name(
        image,
        stroff,
        entry.n_un.n_strx,
        "could not read symbol name",
    ) {
        Ok(name) => name,
        Err(_) => {
            // maybe emit warning?
            return Ok(());
        }
    };

    let value = entry.n_value as usize;
    let sect = entry.n_sect;
    let cold = (entry.n_desc & N_COLD_FUNC) != 0;
    let thumb = (entry.n_desc & N_ARM_THUMB_DEF) != 0;

    let is_external = entry.n_type & N_EXT != 0;
    let is_private_extern = entry.n_type & N_PEXT != 0;
    let is_weak_def = entry.n_desc & N_WEAK_DEF != 0;
    let is_alt_entry = entry.n_desc & N_ALT_ENTRY != 0;
    let is_resolver = entry.n_desc & N_SYMBOL_RESOLVER != 0;

    match (
        is_external,
        is_private_extern,
        is_weak_def,
        is_alt_entry,
        is_resolver,
    ) {
        // translation unit local
        (false, false, false, false, _) => {
            symbols.push(Symbol::make_regular_local(name, value, sect, cold, thumb));
        }
        // private extern: linkage unit scoped export
        (false, true, false, false, _) => {
            symbols.push(Symbol::make_regular_export(name, value, sect, cold, thumb));
        }
        // global weak def
        (true, false, true, false, _) if entry.n_desc & N_WEAK_REF == 0 => {
            symbols.push(Symbol::make_weak_def_export(name, value, sect, cold, thumb));
        }
        // global regular export
        (true, false, false, false, false) => {
            symbols.push(Symbol::make_regular_export(name, value, sect, cold, thumb));
        }

        // these are things we recognize but not yet implement
        (false, false, _, true, _) => Err("makeAltEntry with Scope::translationUnit")?,
        (false, true, true, _, _) => Err("makeWeakDefWasPrivateExtern")?,
        (false, true, false, true, _) => {
            Err("unsupported makeAltEntry with Scope::wasLinkageUnit")?
        }
        (true, true, _, true, _) => Err("makeAltEntry with Scope::linkageUnit")?,
        (true, true, true, _, _) => Err("makeWeakDefHidden")?,
        (true, true, _, _, true) => Err("makeDynamicResolver with Scope::linkageUnit")?,
        (true, true, _, _, _) => Err("makeRegularHidden")?,
        (true, false, _, true, _) => Err("makeAltEntry with Scope::global")?,
        (true, false, true, _, _) => Err("makeWeakDefAutoHide")?, // N_WEAK_DEF | N_WEAK_REF
        (true, false, _, _, true) => Err("makeDynamicResolver")?,
        _ => Err("unknown symbol kind")?,
    }
    Ok(())
}

fn classify_undefined_symbol(
    image: &Container<'_>,
    stroff: u32,
    entry: &nlist_64,
    symbols: &mut Vec<Symbol>,
) -> Result<(), &'static str> {
    if entry.n_value != 0 {
        if entry.n_type & N_PEXT != 0 {
            unimplemented!("N_PEXT: n_type: {}", entry.n_type & N_TYPE);
        }
        unimplemented!("n_type: {}", entry.n_type & N_TYPE);
    }

    let name = match read_symbol_name(
        image,
        stroff,
        entry.n_un.n_strx,
        "could not read symbol name",
    ) {
        Ok(name) => name,
        Err(_) => {
            // maybe emit warning?
            return Ok(());
        }
    };

    symbols.push(Symbol::make_undefined(
        name,
        get_library_ordinal(entry.n_desc),
        (entry.n_desc & N_WEAK_REF) != 0,
    ));
    Ok(())
}

fn read_symbol_name(
    image: &Container<'_>,
    stroff: u32,
    n_strx: u32,
    err: &'static str,
) -> Result<ArrayString<SYMBOL_NAME_LEN>, &'static str> {
    ArrayString::try_from_str(image.deserialize_string_at_offset((stroff + n_strx) as usize))
        .ok_or(err)
}

/// Parse one LC_SEGMENT_64
fn parse_segment(
    image: &Container<'_>,
    offset: usize,
    segments: &mut ArrayVec<Segment, 64>,
    init_functions: &mut Vec<usize>,
    has_thread_locals: &mut bool,
) -> Result<(), &'static str> {
    let segment_command_64 {
        initprot,
        vmaddr,
        vmsize,
        fileoff,
        segname,
        nsects,
        ..
    } = image
        .deserialize_type_at_offset(offset)
        .map_err(|_| "could not parse `segment_command_64` in the inner arm64 image")?;

    let sections_off = offset + size_of::<segment_command_64>();

    if segname.as_ref().starts_with(SEG_TEXT.as_ref())
        || segname.as_ref().starts_with(SEG_DATA.as_ref())
    {
        for i in 0..nsects {
            let section_off = sections_off + i as usize * size_of::<section_64>();
            scan_section(image, section_off, init_functions, has_thread_locals)?;
        }
    }

    segments.push(Segment {
        prot: initprot,
        vm_addr: vmaddr,
        file_offset: fileoff as usize,
        vm_size: vmsize as usize,
    });
    Ok(())
}

/// Read one section header and pick out the bits the linker cares about
fn scan_section(
    image: &Container<'_>,
    section_off: usize,
    init_functions: &mut Vec<usize>,
    has_thread_locals: &mut bool,
) -> Result<(), &'static str> {
    let section_64 {
        flags,
        offset,
        size,
        ..
    } = image
        .deserialize_type_at_offset(section_off)
        .map_err(|_| "could not parse `section_64` in the inner arm64 image")?;

    match flags & SECTION_TYPE {
        S_INIT_FUNC_OFFSETS => read_init_func_offsets(image, offset, size as u32, init_functions)?,
        S_THREAD_LOCAL_VARIABLES => *has_thread_locals = true,
        _ => {} // other section types are not relevant
    }
    Ok(())
}

fn read_init_func_offsets(
    image: &Container<'_>,
    mut cursor: u32,
    size: u32,
    init_functions: &mut Vec<usize>,
) -> Result<(), &'static str> {
    let end = cursor + size;
    while cursor < end {
        let func: u32 = image
            .deserialize_type_at_offset(cursor as usize)
            .map_err(|_| "could not parse S_INIT_FUNC_OFFSETS")?;
        init_functions.push(func as usize);
        cursor += size_of::<u32>() as u32;
    }
    Ok(())
}

/// Allocate a single VM region covering every segment, then copy each
/// segment's file bytes into them
fn macho_segments_map_to_vm(
    skip_page_zero: bool,
    image: &Container<'_>,
    segments: &[Segment],
) -> Result<(NonNull<u8>, usize), &'static str> {
    let (min_addr, max_addr) = segments
        .iter()
        .map(|s| (s.vm_addr, s.vm_addr + s.vm_size as u64))
        .fold((u64::MAX, 0u64), |(lo, hi), (s, e)| (lo.min(s), hi.max(e)));

    let vm_size = (max_addr - min_addr) as usize;
    let vm = mach::vm_alloc_task_self(vm_size).map_err(|_| "failed to vm_alloc_task_self")?;

    for seg in segments {
        // __PAGEZERO has vm_addr == 0
        if skip_page_zero && seg.vm_addr == 0 {
            continue;
        }

        //println!(
        //    "{:x}, {:x}",
        //    image.as_bytes().as_ptr().addr(),
        //    seg.file_offset
        //);

        unsafe {
            mach::vm_copy_into_task_self(
                image.as_bytes().as_ptr().add(seg.file_offset).addr() as u64,
                vm.add(seg.vm_addr as usize).as_ptr().addr() as u64,
                seg.vm_size,
            );
        }
    }

    Ok((vm, vm_size))
}

/// Apply each segment's permissions. We set max-protection first, then the
/// current protection
fn macho_segments_vm_protect(vm: NonNull<u8>, segments: &[Segment]) -> Result<(), &'static str> {
    for seg in segments {
        for set_max in [true, false] {
            unsafe {
                mach::vm_protect(
                    vm.add(seg.vm_addr as usize).as_ptr().addr() as u64,
                    seg.vm_size,
                    set_max as u32,
                    seg.prot as i32,
                )
                .map_err(|_| "could not apply vm protections to a memory region")?;
            }
        }
    }
    Ok(())
}

/// Call every initializer collected from S_INIT_FUNC_OFFSETS, in order.
fn macho_init_functions_exec(vm: NonNull<u8>, init_functions: &[usize]) {
    for &func in init_functions {
        unsafe {
            jump::entry_and_ret(
                vm.add(func).add(PAGE_ZERO_SIZE),
                0,
                [core::ptr::null()].as_ptr(),
                [core::ptr::null()].as_ptr(),
                [core::ptr::null()].as_ptr(),
            );
        }
    }
}

fn macho_tlv_initialize(vm: NonNull<u8>) {
    unsafe {
        tlv::tlv_initialize_descriptors_export(
            vm.add(PAGE_ZERO_SIZE).as_ptr() as *const mach_header_64
        );
    }
}
