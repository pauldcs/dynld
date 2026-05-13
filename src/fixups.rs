//! A Mach-O Chained Fixup parser

extern crate alloc;
use core::mem::MaybeUninit;

use alloc::vec::Vec;

use crate::{
    SYMBOL_NAME_LEN,
    array::{ArrayString, FixedString},
    bindings_macho::{
        BIND_SPECIAL_DYLIB_FLAT_LOOKUP, BIND_SPECIAL_DYLIB_MAIN_EXECUTABLE,
        BIND_SPECIAL_DYLIB_SELF, BIND_SPECIAL_DYLIB_WEAK_LOOKUP, dyld_chained_fixups_header,
        dyld_chained_import, dyld_chained_import_addend, dyld_chained_import_addend64,
        dyld_chained_import_format_variants, dyld_chained_ptr_64_bind, dyld_chained_ptr_64_rebase,
        dyld_chained_ptr_arm64e_auth_bind, dyld_chained_ptr_arm64e_auth_bind_24,
        dyld_chained_ptr_arm64e_auth_rebase, dyld_chained_ptr_arm64e_bind,
        dyld_chained_ptr_arm64e_bind_24, dyld_chained_ptr_arm64e_rebase,
        dyld_chained_ptr_format_variants, dyld_chained_starts_in_image,
        dyld_chained_starts_in_segment, linkedit_data_command, load_command, load_command_variants,
        mach_header_64,
    },
    dyld_shared_cache::DyldSharedCache,
    dylib::{DylibSymbol, dylib_symbol_resolve},
    image::{Image, LCIterator, Symbol},
    loader::Library,
};

const PAGE_STARTS_OFFSET: u32 = 22;

/// A Mach-O Chained Fixup
///
/// Chained fixups is a new way to store information that will be used by the
/// dynamic linker.
///
/// They replace LC_DYLD_INFO(_ONLY) since OSX 12 ans iOS 15.
/// Their main purpose is to reduce launch time and make the binary overall
/// more compact
#[derive(Debug, Clone)]
pub struct Fixup {
    /// The offset of the fixup (within the image)
    pub offset: usize,
    /// The kind of the fixups
    pub kind: FixupKind,
}

/// The possible kinds for a Chained Fixup
#[derive(Debug, Clone)]
pub enum FixupKind {
    /// An offset that must be turned into a pointer.
    ///
    /// Usually handled by simply adding the address where the image is mapped
    /// to the target offset.
    Rebase { target: u64 },
    /// Usually handled by resolving the address of the symbol referenced by
    /// this.
    Bind {
        /// the name of the symbol
        symbol_name: ArrayString<SYMBOL_NAME_LEN>,
        /// the library ordinal of the symbol
        ordinal: u32,
        /// tells if the resolved pointer should be signed
        #[allow(unused)]
        is_auth: bool,

        #[allow(unused)]
        addend: i64,
    },
}
impl Fixup {
    /// Created a new Chained Fixup of kind `Rebase`
    pub fn rebase(offset: usize, target: u64) -> Self {
        Fixup {
            offset,
            kind: FixupKind::Rebase { target },
        }
    }

    /// Created a new Chained Fixup of kind `Bind`
    pub fn bind(
        offset: usize,
        symbol_name: ArrayString<SYMBOL_NAME_LEN>,
        ordinal: u32,
        addend: i64,
        is_auth: bool,
    ) -> Self {
        Fixup {
            offset,
            kind: FixupKind::Bind {
                symbol_name,
                ordinal,
                is_auth,
                addend,
            },
        }
    }
}

impl<'bytes> Image<'bytes> {
    /// Finds the `LC_DYLD_CHAINED_FIXUPS` load command
    ///
    /// This function iterates over the load commands and tries to find
    /// the chained fixups. It returns `(linkedit_data_offset, dyld_chained_fixups_header)` if found.
    fn chained_fixups_header_and_offset_find(
        &self,
    ) -> Result<Option<(u32, dyld_chained_fixups_header)>, &'static str> {
        let mut next_load_command_offset = size_of::<mach_header_64>();
        let mut contains_fixups_header = false;

        let mut load_command_iterator = LCIterator::try_with_container(self.container())?;
        while let Some(load_command { cmd, cmdsize }) = load_command_iterator.try_next()? {
            if matches!(cmd, load_command_variants::LC_DYLD_CHAINED_FIXUPS) {
                contains_fixups_header = true;
                break;
            }
            next_load_command_offset += cmdsize as usize;
        }

        if contains_fixups_header {
            // a LC_DYLD_CHAINED_FIXUPS is a linkedit_data_command. We need to extract `dataoff` from
            // it. This will tell us where the `dyld_chained_fixups_header` is located at
            let linkedit_data_command { dataoff, .. } = self
                .container()
                .deserialize_type_at_offset::<linkedit_data_command>(next_load_command_offset)
                .map_err(|_| "error: could not find `linkedit_data_command`")?;

            Ok(Some((
                dataoff,
                self.container()
                    .deserialize_type_at_offset::<dyld_chained_fixups_header>(dataoff as usize)
                    .map_err(|_| "error: could not find `dyld_chained_fixups_header`")?,
            )))
        } else {
            Ok(None)
        }
    }

    /// Applies all fixups to the image
    ///
    /// IMPORTANT:
    /// This function cannot use any static DATA pointers as they are not rebased yet
    ///
    /// This function parses `LC_DYLD_CHAINED_FIXUPS`. Note that they are parsed on the stack,
    /// there is a limit of 4096 fixups.
    pub fn chained_fixups_parse_all(&mut self) -> Result<Vec<Fixup>, &'static str> {
        let mut fixups = Vec::new();
        let Some((
            fixup_data_offset,
            dyld_chained_fixups_header {
                imports_offset,
                starts_offset,
                symbols_offset,
                imports_format,
                ..
            },
            // find LC_DYLD_CHAINED_FIXUPS header
        )) = self.chained_fixups_header_and_offset_find()?
        else {
            // returning the empty fixups when no chained fixups
            // are found, this isn't an error
            return Ok(fixups);
        };

        let starts_offset = fixup_data_offset as u32 + starts_offset;

        let dyld_chained_starts_in_image { seg_count, .. } = self
            .container()
            .deserialize_type_at_offset::<dyld_chained_starts_in_image>(starts_offset as usize)
            .map_err(|_| "error: could not find `dyld_chained_starts_in_image`")?;

        let imports_offset = fixup_data_offset + imports_offset;
        let symbols_offset = fixup_data_offset + symbols_offset;

        for seg_offset_index in 0..seg_count {
            let seg_offset_address = starts_offset as usize
                + size_of::<dyld_chained_starts_in_image>()
                + seg_offset_index as usize * size_of::<u32>();

            let seg_offset = self
                .container()
                .deserialize_type_at_offset::<u32>(seg_offset_address)
                .map_err(|_| "error: could not find `dyld_chained_starts_in_image`")?;

            // copied from https://blog.xpnsec.com/building-a-mach-o-memory-loader-part-1/
            //
            // Sometimes segment_offset is 0… no idea why… and it looks like dyld identifies
            // this too and just ignores them, so we’ll do the same
            if seg_offset == 0 {
                continue;
            }

            let seg_starts_offset = starts_offset + seg_offset;

            // now we can extract the actual fixups from this segment
            let dyld_chained_starts_in_segment {
                page_count,
                segment_offset,
                page_size,
                pointer_format,
                ..
            } = self
                .container()
                .deserialize_type_at_offset(seg_starts_offset as usize)
                .map_err(|_| "error: could not read `dyld_chained_starts_in_segment` from a segment offset")?;

            for page_index in 0..page_count {
                let page_start = self
                    .container()
                    .deserialize_type_at_offset::<u16>(
                        (seg_starts_offset + PAGE_STARTS_OFFSET) as usize
                            + page_index as usize * size_of::<u16>(),
                    )
                    .map_err(|_| "error: could not read the page starts array")?;

                let mut chain_offset = segment_offset as usize
                    + (page_index as usize * page_size as usize)
                    + page_start as usize;

                loop {
                    let fixup_value = self
                        .container()
                        .deserialize_type_at_offset::<u64>(chain_offset)
                        .map_err(|_| "error: could read the fixup value from a chain_offset")?;

                    let (next_offset, fixup) = match pointer_format {
                        dyld_chained_ptr_format_variants::DYLD_CHAINED_PTR_64
                        | dyld_chained_ptr_format_variants::DYLD_CHAINED_PTR_64_OFFSET => self
                            .fixup_64_parse_one(
                                fixup_value,
                                chain_offset,
                                imports_format,
                                imports_offset as u64,
                                symbols_offset as u64,
                            )?,
                        dyld_chained_ptr_format_variants::DYLD_CHAINED_PTR_ARM64E
                        | dyld_chained_ptr_format_variants::DYLD_CHAINED_PTR_ARM64E_USERLAND => {
                            self.fixup_arm64e_parse_one(
                                fixup_value,
                                chain_offset,
                                imports_format,
                                imports_offset as u64,
                                symbols_offset as u64,
                            )?
                        }
                        dyld_chained_ptr_format_variants::DYLD_CHAINED_PTR_ARM64E_USERLAND24 => {
                            self.fixup_arm64e_userland24_parse_one(
                                fixup_value,
                                chain_offset,
                                imports_format,
                                imports_offset as u64,
                                symbols_offset as u64,
                            )?
                        }
                        _ => {
                            unimplemented!("pointer format: {pointer_format:?}")
                        }
                    };

                    fixups.push(fixup);

                    if next_offset == 0 {
                        break;
                    }

                    chain_offset += next_offset as usize;
                }
            }
        }
        Ok(fixups)
    }

    fn fixup_64_parse_one(
        &self,
        value: u64,
        offset: usize,
        imports_format: dyld_chained_import_format_variants,
        imports_offset: u64,
        symbols_offset: u64,
    ) -> Result<(u64, Fixup), &'static str> {
        let bind = (value >> 63) & 1;

        if bind == 1 {
            let bind_ptr = dyld_chained_ptr_64_bind(value);
            let (lib_ordinal, symbol_name) = self.import_symbol_parse(
                bind_ptr.ordinal(),
                imports_format,
                imports_offset,
                symbols_offset,
            )?;

            Ok((
                bind_ptr.next() as u64 * 4,
                Fixup {
                    offset,
                    kind: FixupKind::Bind {
                        symbol_name,
                        is_auth: false,
                        ordinal: lib_ordinal as u32,
                        addend: bind_ptr.addend() as i64,
                    },
                },
            ))
        } else {
            let rebase_ptr = dyld_chained_ptr_64_rebase(value);
            let target = rebase_ptr.target();
            let high8 = (rebase_ptr.high8() as u64) << 56;
            let next = rebase_ptr.next() as u64 * 4;

            Ok((
                next,
                Fixup {
                    offset,
                    kind: FixupKind::Rebase {
                        target: target | high8,
                    },
                },
            ))
        }
    }

    fn fixup_arm64e_parse_one(
        &self,
        value: u64,
        offset: usize,
        imports_format: dyld_chained_import_format_variants,
        imports_offset: u64,
        symbols_offset: u64,
    ) -> Result<(u64, Fixup), &'static str> {
        let bind = (value >> 62) & 1;
        let auth = (value >> 63) & 1;

        if bind == 1 {
            if auth == 1 {
                let auth_bind = dyld_chained_ptr_arm64e_auth_bind(value);
                let (lib_ordinal, symbol_name) = self.import_symbol_parse(
                    auth_bind.ordinal() as u32,
                    imports_format,
                    imports_offset,
                    symbols_offset,
                )?;

                Ok((
                    auth_bind.next() as u64 * 8,
                    Fixup::bind(offset, symbol_name, lib_ordinal as u32, 0, true),
                ))
            } else {
                let bind_ptr = dyld_chained_ptr_arm64e_bind(value);
                let (lib_ordinal, symbol_name) = self.import_symbol_parse(
                    bind_ptr.ordinal() as u32,
                    imports_format,
                    imports_offset,
                    symbols_offset,
                )?;

                Ok((
                    bind_ptr.next() as u64 * 8,
                    Fixup::bind(
                        offset,
                        symbol_name,
                        lib_ordinal as u32,
                        bind_ptr.addend() as i64,
                        false,
                    ),
                ))
            }
        } else if auth == 1 {
            let auth_rebase = dyld_chained_ptr_arm64e_auth_rebase(value);
            Ok((
                auth_rebase.next() as u64 * 8,
                Fixup::rebase(offset, auth_rebase.target() as u64),
            ))
        } else {
            let rebase = dyld_chained_ptr_arm64e_rebase(value);
            let high8 = (rebase.high8() as u64) << 56;
            Ok((
                rebase.next() as u64 * 8,
                Fixup::rebase(offset, rebase.target() | high8),
            ))
        }
    }

    fn fixup_arm64e_userland24_parse_one(
        &self,
        value: u64,
        offset: usize,
        imports_format: dyld_chained_import_format_variants,
        imports_offset: u64,
        symbols_offset: u64,
    ) -> Result<(u64, Fixup), &'static str> {
        let bind = (value >> 62) & 1;
        let auth = (value >> 63) & 1;

        if bind == 1 {
            if auth == 1 {
                let auth_bind = dyld_chained_ptr_arm64e_auth_bind_24(value);
                let (lib_ordinal, symbol_name) = self.import_symbol_parse(
                    auth_bind.ordinal(),
                    imports_format,
                    imports_offset,
                    symbols_offset,
                )?;
                Ok((
                    auth_bind.next() as u64 * 8,
                    Fixup::bind(offset, symbol_name, lib_ordinal as u32, 0, true),
                ))
            } else {
                let bind = dyld_chained_ptr_arm64e_bind_24(value);
                let (lib_ordinal, symbol_name) = self.import_symbol_parse(
                    bind.ordinal(),
                    imports_format,
                    imports_offset,
                    symbols_offset,
                )?;

                Ok((
                    bind.next() as u64 * 8,
                    Fixup::bind(
                        offset,
                        symbol_name,
                        lib_ordinal as u32,
                        bind.addend() as i64,
                        false,
                    ),
                ))
            }
        } else {
            self.fixup_arm64e_parse_one(
                value,
                offset,
                imports_format,
                imports_offset,
                symbols_offset,
            )
        }
    }

    fn import_symbol_parse(
        &self,
        ordinal: u32,
        imports_format: dyld_chained_import_format_variants,
        imports_offset: u64,
        symbols_offset: u64,
    ) -> Result<(u8, ArrayString<SYMBOL_NAME_LEN>), &'static str> {
        let (lib_ordinal, name_offset) = match imports_format {
            dyld_chained_import_format_variants::DYLD_CHAINED_IMPORT => {
                let import = self
                    .container()
                    .deserialize_type_at_offset::<dyld_chained_import>(
                        imports_offset as usize + (ordinal * 4) as usize,
                    )
                    .map_err(|_| "error: could parse a dyld_chained_import_format_variants::DYLD_CHAINED_IMPORT from imports_offset")?;

                (import.lib_ordinal(), import.name_offset())
            }
            dyld_chained_import_format_variants::DYLD_CHAINED_IMPORT_ADDEND => {
                let import = self.container().deserialize_type_at_offset::<dyld_chained_import_addend>(
                    imports_offset as usize + (ordinal * 8) as usize,
                ).map_err(|_| "error: could parse a dyld_chained_import_format_variants::DYLD_CHAINED_IMPORT_ADDEND from imports_offset")?;

                (import.header.lib_ordinal(), import.addend as u32)
            }
            dyld_chained_import_format_variants::DYLD_CHAINED_IMPORT_ADDEND64 => {
                let import = self
                    .container()
                    .deserialize_type_at_offset::<dyld_chained_import_addend64>(
                        imports_offset as usize + (ordinal * 16) as usize,
                    ).map_err(|_| "error: could parse a dyld_chained_import_format_variants::DYLD_CHAINED_IMPORT_ADDEND64 from imports_offset")?;

                (import.header.lib_ordinal(), import.addend as u32)
            }
        };
        Ok((
            lib_ordinal,
            ArrayString::try_from_str(
                self.container()
                    .deserialize_string_at_offset((symbols_offset + name_offset as u64) as usize),
            )
            .unwrap(),
        ))
    }
}

pub unsafe fn fixup_all_chained_fixups(
    dst_ptr: *mut u8,
    page_zero_size: usize,
    fixups: &Vec<Fixup>,
    symbols: &Vec<Symbol>,
    dylibs: &Vec<Library>,
    dyld_shared_cache: Option<&DyldSharedCache<'_>>,
    ignore_binds: bool,
) -> Result<(), &'static str> {
    for Fixup {
        offset,
        kind: fixup_type,
    } in fixups
    {
        match &fixup_type {
            FixupKind::Rebase { target } => {
                unsafe {
                    *(dst_ptr.add(*offset).add(page_zero_size).addr() as *mut u64) =
                        dst_ptr.add(*target as usize).add(page_zero_size).addr() as u64;
                };
            }
            FixupKind::Bind {
                symbol_name,
                ordinal,
                ..
            } => {
                // this is because this code is used for the rebasing of dynld itself, as well as the fixing
                // up of the actual executable. When we rebase ourselves, we cannot yet handle this binds.
                // this is due to the fact that we don't yet have access to the dyld_shared_cache.
                if ignore_binds {
                    continue;
                }

                match *ordinal {
                    BIND_SPECIAL_DYLIB_SELF => {
                        return Err("BIND_SPECIAL_DYLIB_SELF is not yet supported");
                    }
                    BIND_SPECIAL_DYLIB_MAIN_EXECUTABLE => {
                        return Err("BIND_SPECIAL_DYLIB_MAIN_EXECUTABLE is not yet supported");
                    }
                    BIND_SPECIAL_DYLIB_FLAT_LOOKUP => {
                        return Err("BIND_SPECIAL_DYLIB_FLAT_LOOKUP is not yet supported");
                    }
                    BIND_SPECIAL_DYLIB_WEAK_LOOKUP => unsafe {
                        fixup_dylib_weak_symbol(
                            dst_ptr,
                            *offset,
                            page_zero_size,
                            symbols,
                            symbol_name,
                        )?;
                    },
                    _ => unsafe {
                        fixup_regular_symbol(
                            dst_ptr,
                            *offset,
                            page_zero_size,
                            symbol_name,
                            &dylibs[*ordinal as usize - 1],
                            dyld_shared_cache,
                        )?;
                    },
                };
            }
        }
    }
    Ok(())
}

unsafe fn fixup_dylib_weak_symbol(
    vm_start_ptr: *mut u8,
    fixup_offset: usize,
    page_zero_size: usize,
    symbols: &Vec<Symbol>,
    symbol_name: &FixedString<[MaybeUninit<u8>; SYMBOL_NAME_LEN]>,
) -> Result<(), &'static str> {
    unsafe {
        let dst_addr = vm_start_ptr.add(fixup_offset).add(page_zero_size) as *mut u64;
        let symbol = symbols
            .iter()
            .find(|sym| sym.array_string_cmp(symbol_name))
            .ok_or(
                "could not find a matching symbol while fixing up a BIND_SPECIAL_DYLIB_WEAK_LOOKUP",
            )?;

        //println!(
        //    "0x{:x} ({:x}): {} -> 0x{:x}",
        //    dst_addr.addr(),
        //    *dst_addr,
        //    symbol_name.as_str(),
        //    symbol.impl_offset
        //);

        *dst_addr = (vm_start_ptr.add(symbol.impl_offset).addr()) as u64;
    }
    Ok(())
}

unsafe fn fixup_regular_symbol(
    vm_start_ptr: *mut u8,
    fixup_offset: usize,
    page_zero_size: usize,
    symbol_name: &FixedString<[MaybeUninit<u8>; SYMBOL_NAME_LEN]>,
    library: &Library,
    dyld_shared_cache: Option<&DyldSharedCache<'_>>,
) -> Result<(), &'static str> {
    match library {
        Library::System(library_name) if let Some(shared_cache) = dyld_shared_cache => unsafe {
            let dst_addr = vm_start_ptr.add(page_zero_size).add(fixup_offset) as *mut u64;
            let resolved_symbol = shared_cache
                .symbol_resolve(library_name.as_bytes(), symbol_name.as_bytes())?
                .ok_or_else(|| "could not find a symbol in the dyld shared cache")?;

            //println!(
            //    "0x{:x} ({:x}): {} -> 0x{:x}",
            //    dst_addr.addr(),
            //    *dst_addr,
            //    symbol_name.as_str(),
            //    resolved_symbol
            //);

            *dst_addr = resolved_symbol;
        },
        Library::User(lib_container) => {
            let sym = dylib_symbol_resolve(lib_container, symbol_name.as_bytes())?
                .ok_or_else(|| "failed to find symbol in user dylib")?;
            match sym {
                DylibSymbol::Defined {
                    offset_from_mach_header,
                } => unsafe {
                    let dst_addr = vm_start_ptr.add(page_zero_size).add(fixup_offset) as *mut u64;
                    let a = lib_container
                        .as_bytes()
                        .as_ptr()
                        .add(offset_from_mach_header as usize);

                    *dst_addr = a.addr() as u64;
                },
                DylibSymbol::Reexported { .. } => unimplemented!("DylibSymbol::Reexported"),
            }
        }
        _ => return Err("failed to link library"),
    }

    Ok(())
}
