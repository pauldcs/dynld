use core::slice;

use container::{Container, read_uleb128};

extern crate alloc;

use alloc::vec::Vec;

use crate::{
    bindings_dsc::{
        dyld_cache_header, dyld_cache_image_info, dyld_cache_mapping_info, dyld_info_command,
        dyld_subcache_entry, dylib_command_header,
    },
    bindings_macho::{
        EXPORT_SYMBOL_FLAGS_REEXPORT, linkedit_data_command, load_command, load_command_variants,
        mach_header_64,
    },
    container,
};

// The dyld shared cache is a single, very large file (actually several files
// since it was split into subcaches around iOS 13 / Big Sur) into
// which Apple stashes all the system libraries that ship with the OS. At
// boot time, dyld maps it at a known base address so that the cost of resolving
// common library symbols is paid once for the whole machine instead of once per process.
// The cache contains a pre-linked, pre-rebased copy of every library the system needs, with
// inter-library references handled. That's what makes it fast
// to use and confusing to parse.
//
// This module reads symbols out of a cache that has already been mapped
// into our address space by dyld. We never open files, never call mmap,
// and never decide where the cache should live etc. We just locate symbols
// inside what's already there. The caller hands us a pointer to the
// `dyld_cache_header` of the main cache and the ASLR slide that was
// applied to it, and we walk it from there.

/// Re-export chains can in principle be arbitrarily long. They aren't, in
/// practice, but the trie format doesn't bound them and a malformed cache
/// could send us in circles. 16 levels is far more than any real library
/// uses and small enough to fail fast on a bad input.
const MAX_REEXPORT_RESOLUTION_DEPTH: usize = 16;

/// One mapped subcache. The main cache is just the first such subcache as
/// far as this code is concerned, sitting at index 0. The rest are listed
/// in the main cache's `sub_cache_array`. Each subcache carries its own
/// header, its own mapping table, and its own slice of bytes, but they
/// are not independent on disk. File offsets stored inside one subcache's
/// load commands are sometimes meant to index into another subcache,
/// and that has consequences throughout this code.
struct DyldSharedSubCache<'image> {
    pub container: Container<'image>,
    /// The unslid VM address that file offset 0 of this subcache
    /// corresponds to. It's the `address` field of the subcache's first
    /// mapping_info entry, and it's what we add to a file offset to get a
    /// VM address (or subtract from a VM address to get an offset into our
    /// `Container`'s slice, see `runtime_offset_for_vm_address` below.
    unslid_base_vm_address: u64,
    mapping_table_offset: u32,
    mapping_table_entry_count: u32,
}

/// A subcache within our dyld shared cache
impl<'image> DyldSharedSubCache<'image> {
    /// Wrap an already mapped subcache in a `Container`. We do not own the
    /// underlying memory, dyld put it there and is keeping it alive.
    ///
    /// SAFETY: `runtime_pointer` must point to at least `length` valid,
    /// readable bytes for the lifetime `'image`. In practice that means
    /// the caller has confirmed that what dyld mapped is still mapped, and
    /// has computed `length` correctly, too large and we'll cheerfully read past
    /// the mapping into whatever happens to be there.
    unsafe fn from_runtime_mapping(
        runtime_pointer: *const u8,
        length: usize,
        unslid_base_vm_address: u64,
    ) -> Result<Self, &'static str> {
        let bytes = unsafe { slice::from_raw_parts(runtime_pointer, length) };
        let container = Container::with_bytes(bytes, container::Endian::Little);

        let dyld_cache_header {
            mapping_offset,
            mapping_count,
            ..
        } = dsc_header_read(&container)?;

        Ok(Self {
            container,
            unslid_base_vm_address,
            mapping_table_offset: mapping_offset,
            mapping_table_entry_count: mapping_count,
        })
    }

    /// Walks this subcache's mapping table. There are usually only three
    /// or four entries (__TEXT, __DATA, __LINKEDIT, sometimes __AUTH),
    /// so iteration cost is negligible and we don't bother caching them.
    fn mappings_iter(&self) -> impl Iterator<Item = dyld_cache_mapping_info> + '_ {
        let entry_size = size_of::<dyld_cache_mapping_info>();
        let table_base = self.mapping_table_offset as usize;
        (0..self.mapping_table_entry_count as usize).map(move |entry_index| {
            self.container
                .deserialize_type_at_offset::<dyld_cache_mapping_info>(
                    table_base + entry_index * entry_size,
                )
                .expect("malformed cache mapping table")
        })
    }

    /// VM-address-to-runtime-offset translation for this subcache. The
    /// distinction is important because the bytes we have were laid out by dyld
    /// at their VM addresses (with all the gaps the cache designers chose
    /// to leave between mappings), not packed contiguously the way the
    /// file was. So an address that sits inside this subcache's range
    /// translates to `address - subcache_base`, which is the offset we can
    /// actually feed into the `Container`. Returns `None` if the address
    /// isn't covered by any of this subcache's mappings.
    fn runtime_offset_for_vm_address(&self, unslid_vm_address: u64) -> Option<usize> {
        self.mappings_iter().find_map(|mapping| {
            let mapping_range = mapping.address..mapping.address + mapping.size;
            mapping_range
                .contains(&unslid_vm_address)
                .then(|| (unslid_vm_address - self.unslid_base_vm_address) as usize)
        })
    }

    /// Same idea as the function above, but starting from a file offset
    /// instead of a VM address. The file packs mappings tightly and memory
    /// doesnt. So a file offset of like 0x10000000 might correspond
    /// to a offset of 0x40000000
    fn runtime_offset_for_file_offset(&self, file_offset: u64) -> Option<usize> {
        self.mappings_iter().find_map(|mapping| {
            let file_range = mapping.file_offset..mapping.file_offset + mapping.size;
            file_range.contains(&file_offset).then(|| {
                let offset_within_mapping = file_offset - mapping.file_offset;
                ((mapping.address - self.unslid_base_vm_address) + offset_within_mapping) as usize
            })
        })
    }

    /// We need this when a file offset stored in this subcache actually points
    /// at bytes that physically live in some other subcache.
    fn vm_address_for_file_offset(&self, file_offset: u64) -> Option<u64> {
        self.mappings_iter().find_map(|mapping| {
            let file_range = mapping.file_offset..mapping.file_offset + mapping.size;
            file_range
                .contains(&file_offset)
                .then(|| mapping.address + (file_offset - mapping.file_offset))
        })
    }
}

fn dsc_header_read(container: &Container) -> Result<dyld_cache_header, &'static str> {
    container
        .deserialize_type_at_offset(0)
        .map_err(|_| "could not deserialize dyld_cache_header")
}

#[derive(Debug)]
pub struct SharedCacheLibrary<'image> {
    /// Index into `DyldSharedCache::sub_caches`
    sub_cache_index: u8,
    /// Where this library's `mach_header_64` is inside the subcache's
    /// runtime memory.
    mach_header_runtime_offset: usize,
    /// Where the library came from.
    install_path: &'image str,
    /// The unslid VM address dyld would use as this image's load address.
    unslid_load_address: u64,
}

/// A view bundling a library together with the subcache it
/// lives in. Used as the entry point for "walk this image's load
/// commands"
struct MachOImageView<'a, 'image> {
    container: &'a Container<'image>,
    /// The hosting subcache.
    _subcache: &'a DyldSharedSubCache<'image>,
    mach_header_runtime_offset: usize,
}

impl<'a, 'image> MachOImageView<'a, 'image> {
    fn for_library(cache: &'a DyldSharedCache<'image>, lib: &SharedCacheLibrary<'image>) -> Self {
        let subcache = &cache.sub_caches[lib.sub_cache_index as usize];
        Self {
            container: &subcache.container,
            _subcache: subcache,
            mach_header_runtime_offset: lib.mach_header_runtime_offset,
        }
    }

    fn read_mach_header(&self) -> Result<mach_header_64, &'static str> {
        self.container
            .deserialize_type_at_offset(self.mach_header_runtime_offset)
            .map_err(|_| "could not read mach_header_64")
    }

    /// Walks every load command in declaration order, calling `visit` for
    /// each one.
    fn load_commands_find_in<T>(
        &self,
        mut visit: impl FnMut(load_command_variants, usize) -> Result<Option<T>, &'static str>,
    ) -> Result<Option<T>, &'static str> {
        let header = self.read_mach_header()?;
        let mut current_command_offset =
            self.mach_header_runtime_offset + size_of::<mach_header_64>();

        for _ in 0..header.ncmds {
            let load_command { cmd, cmdsize, .. } = self
                .container
                .deserialize_type_at_offset(current_command_offset)
                .map_err(|_| "could not read load_command header")?;

            if let Some(found) = visit(cmd, current_command_offset)? {
                return Ok(Some(found));
            }

            current_command_offset += cmdsize as usize;
        }
        Ok(None)
    }

    /// Reads the install-name string from a dylib-shaped load command
    /// (`LC_LOAD_DYLIB`, `LC_REEXPORT_DYLIB`, etc.)
    fn dylib_install_name_read_at(
        &self,
        command_offset: usize,
    ) -> Result<&'image [u8], &'static str> {
        let dylib_command_header { name_offset, .. } = self
            .container
            .deserialize_type_at_offset(command_offset)
            .map_err(|_| "could not read dylib_command_header")?;

        Ok(self
            .container
            .deserialize_cstr_at_offset(command_offset + name_offset as usize)
            .to_bytes())
    }

    /// Locates the exports trie span
    fn exports_trie_span_find(
        &self,
        sub_cache_index: u8,
    ) -> Result<Option<ExportsTrieSpan>, &'static str> {
        self.load_commands_find_in(|cmd, command_offset| match cmd {
            load_command_variants::LC_DYLD_EXPORTS_TRIE => {
                let linkedit_data_command {
                    dataoff, datasize, ..
                } = self
                    .container
                    .deserialize_type_at_offset(command_offset)
                    .map_err(|_| "could not read linkedit_data_command")?;

                Ok(Some(ExportsTrieSpan {
                    sub_cache_index,
                    file_offset: dataoff,
                    size_in_bytes: datasize,
                }))
            }
            load_command_variants::LC_DYLD_INFO_ONLY => {
                let dyld_info_command {
                    export_off,
                    export_size,
                    ..
                } = self
                    .container
                    .deserialize_type_at_offset(command_offset)
                    .map_err(|_| "could not read dyld_info_command")?;
                Ok(Some(ExportsTrieSpan {
                    sub_cache_index,
                    file_offset: export_off,
                    size_in_bytes: export_size,
                }))
            }
            _ => Ok(None),
        })
    }

    /// Resolves a 1-based dylib ordinal
    fn install_path_get_from_dylib_ordinal(
        &self,
        target_ordinal: u64,
    ) -> Result<Option<&'image [u8]>, &'static str> {
        let mut dylib_dependencies_seen_so_far: u64 = 0;

        self.load_commands_find_in(|cmd, command_offset| {
            if !is_dylib_dependency_command(cmd) {
                return Ok(None);
            }

            dylib_dependencies_seen_so_far += 1;
            if dylib_dependencies_seen_so_far == target_ordinal {
                Ok(Some(self.dylib_install_name_read_at(command_offset)?))
            } else {
                Ok(None)
            }
        })
    }

    /// Collects the install paths of every `LC_REEXPORT_DYLIB` in this
    /// image.
    fn dylib_paths_collect_reexported(&self) -> Result<Vec<&'image [u8]>, &'static str> {
        let mut reexported_paths = Vec::new();

        self.load_commands_find_in::<()>(|cmd, command_offset| {
            if matches!(cmd, load_command_variants::LC_REEXPORT_DYLIB) {
                if let Ok(install_path) = self.dylib_install_name_read_at(command_offset) {
                    reexported_paths.push(install_path);
                }
            }
            Ok(None)
        })?;

        Ok(reexported_paths)
    }
}

fn is_dylib_dependency_command(cmd: load_command_variants) -> bool {
    matches!(
        cmd,
        load_command_variants::LC_LOAD_DYLIB | load_command_variants::LC_REEXPORT_DYLIB
    )
}

/// Where to find an image's exports trie. The `sub_cache_index` here is
/// the home subcache basically
#[derive(Debug, Clone, Copy)]
struct ExportsTrieSpan {
    sub_cache_index: u8,
    file_offset: u32,
    size_in_bytes: u32,
}

/// What an exports-trie terminal node ends up telling us about a symbol.
enum ExportTerminal<'a> {
    /// The symbol is defined in this image, at `mach_header + offset`.
    DefinedHere { offset_from_mach_header: u64 },
    /// The symbol is re-exported from another library. The dependency
    /// ordinal points at one of this image's `LC_LOAD_DYLIB` /
    /// `LC_REEXPORT_DYLIB` commands; `aliased_name` lets the symbol be
    /// renamed across the boundary, with an empty slice meaning "same
    /// name as on this side".
    ReexportedFromDependency {
        dependency_ordinal: u64,
        aliased_name: &'a [u8],
    },
}

/// The thing a caller actually holds onto. Owns the list of subcaches
/// (well, the views over them) and the index of libraries by install
/// path, and is the entry point for symbol resolution.
pub struct DyldSharedCache<'image> {
    /// Main cache at index 0, subcaches at indexes 1..N. The order is
    /// important!
    sub_caches: Vec<DyldSharedSubCache<'image>>,
    libraries: Vec<SharedCacheLibrary<'image>>,
    /// The ASLR slide dyld applied to the whole cache.
    cache_slide: u64,
}

impl<'image> DyldSharedCache<'image> {
    pub fn new() -> Self {
        Self {
            sub_caches: Vec::new(),
            libraries: Vec::new(),
            cache_slide: 0,
        }
    }

    /// Initializes the view from a cache that's already mapped into our
    /// address space.
    ///
    /// `main_cache_header_pointer` is the runtime address of the main
    /// cache's `dyld_cache_header, wherever in the process's
    /// address space dyld put it. `cache_slide` is the offset from the
    /// cache's preferred unslid base to its actual runtime address; on
    /// arm64 Apple platforms the preferred base is `0x180000000`, so
    /// the slide is normally `runtime_pointer - 0x180000000`.
    ///
    /// SAFETY: the entire cache (main plus every subcache) must remain
    /// mapped
    pub unsafe fn from_live_mapping_initialize(
        &mut self,
        main_cache_header_pointer: *const u8,
        cache_slide: u64,
    ) -> Result<(), &'static str> {
        self.cache_slide = cache_slide;
        unsafe { self.dyld_shared_cache_attach_all(main_cache_header_pointer)? };
        self.main_cache_libraries_index_all()?;
        Ok(())
    }

    /// Discovers and attaches every subcache. The main cache's header
    /// contains a `sub_cache_array` listing each subcache with an offset
    /// (`cache_vm_offset`) from the main cache base. Since dyld
    /// laid the whole thing out contiguously in our address space, we
    /// can compute each subcache's runtime pointer just by adding that
    /// offset to the main cache's runtime pointer
    unsafe fn dyld_shared_cache_attach_all(
        &mut self,
        main_cache_header_pointer: *const u8,
    ) -> Result<(), &'static str> {
        // we need the main cache's base and total length before we can
        // construct a `DyldSharedSubCache` for it, but the way to read those is through a
        // `DyldSharedSubCache`. So we have these helpers
        let main_unslid_base =
            unsafe { read_main_cache_unslid_base_from(main_cache_header_pointer)? };

        let main_total_length =
            unsafe { read_main_cache_total_mapped_length_from(main_cache_header_pointer)? };

        let main_subcache = unsafe {
            DyldSharedSubCache::from_runtime_mapping(
                main_cache_header_pointer,
                main_total_length,
                main_unslid_base,
            )?
        };
        self.sub_caches.push(main_subcache);

        let dyld_cache_header {
            sub_cache_array_offset,
            sub_cache_array_count,
            ..
        } = dsc_header_read(&self.sub_caches[0].container)?;

        for subcache_index in 0..sub_cache_array_count as usize {
            let entry_offset =
                sub_cache_array_offset as usize + subcache_index * size_of::<dyld_subcache_entry>();
            let subcache_entry: dyld_subcache_entry = self.sub_caches[0]
                .container
                .deserialize_type_at_offset(entry_offset)
                .map_err(|_| "could not deserialize dyld_subcache_entry")?;

            let subcache_unslid_base = main_unslid_base + subcache_entry.cache_vm_offset;
            let subcache_runtime_pointer =
                unsafe { main_cache_header_pointer.add(subcache_entry.cache_vm_offset as usize) };

            let subcache_total_length =
                unsafe { read_main_cache_total_mapped_length_from(subcache_runtime_pointer)? };

            let subcache = unsafe {
                DyldSharedSubCache::from_runtime_mapping(
                    subcache_runtime_pointer,
                    subcache_total_length,
                    subcache_unslid_base,
                )?
            };
            self.sub_caches.push(subcache);
        }
        Ok(())
    }

    fn main_cache_libraries_index_all(&mut self) -> Result<(), &'static str> {
        let main_cache = &self.sub_caches[0].container;
        let dyld_cache_header {
            images_count,
            images_offset,
            ..
        } = dsc_header_read(main_cache)?;

        for image_index in 0..images_count as usize {
            let image_info_offset =
                images_offset as usize + image_index * size_of::<dyld_cache_image_info>();
            let dyld_cache_image_info {
                address: unslid_load_address,
                path_file_offset,
                ..
            } = main_cache
                .deserialize_type_at_offset(image_info_offset)
                .map_err(|_| "could not deserialize dyld_cache_image_info")?;

            let Some((sub_cache_index, mach_header_runtime_offset)) =
                self.subcache_and_offset_locate_from_addr(unslid_load_address)
            else {
                continue;
            };

            let path_runtime_offset = self.sub_caches[0]
                .runtime_offset_for_file_offset(path_file_offset as u64)
                .ok_or("image path offset not covered by any mapping")?;

            self.libraries.push(SharedCacheLibrary {
                sub_cache_index,
                mach_header_runtime_offset,
                install_path: main_cache
                    .deserialize_cstr_at_offset(path_runtime_offset)
                    .to_str()
                    .unwrap(),
                unslid_load_address,
            });
        }
        Ok(())
    }

    /// Scan over subcaches looking for the one whose mappings
    /// cover the given VM address.
    fn subcache_and_offset_locate_from_addr(&self, unslid_vm_address: u64) -> Option<(u8, usize)> {
        self.sub_caches
            .iter()
            .enumerate()
            .find_map(|(subcache_index, subcache)| {
                subcache
                    .runtime_offset_for_vm_address(unslid_vm_address)
                    .map(|runtime_offset| (subcache_index as u8, runtime_offset))
            })
    }

    /// Turns an unslid VM address into a runtime pointer.
    fn unslid_address_to_runtime_pointer_translate(
        &self,
        unslid_vm_address: u64,
    ) -> Result<u64, &'static str> {
        if self
            .subcache_and_offset_locate_from_addr(unslid_vm_address)
            .is_none()
        {
            return Err("vm address not covered by any cache mapping");
        }
        Ok(unslid_vm_address + self.cache_slide)
    }

    fn library_find_by_install_path(
        &self,
        install_path: &[u8],
    ) -> Option<&SharedCacheLibrary<'image>> {
        self.libraries
            .iter()
            .find(|lib| lib.install_path.as_bytes() == install_path)
    }

    /// The public entry point. Given a library's install path (the same
    /// string `dlopen` would take) IMPORTANT: the symbol must be passed as "_malloc",
    /// don't strip the "_"
    pub fn symbol_resolve(
        &self,
        library_install_path: &[u8],
        symbol: &[u8],
    ) -> Result<Option<u64>, &'static str> {
        self.symbol_resolve_with_max_depth(
            library_install_path,
            symbol,
            MAX_REEXPORT_RESOLUTION_DEPTH,
        )
    }

    fn symbol_resolve_with_max_depth(
        &self,
        library_install_path: &[u8],
        symbol: &[u8],
        depth_budget_remaining: usize,
    ) -> Result<Option<u64>, &'static str> {
        if depth_budget_remaining == 0 {
            return Ok(None);
        }

        let Some(library) = self.library_find_by_install_path(library_install_path) else {
            return Ok(None);
        };

        if let Some(resolution) = self.try_resolve_via_exports_trie(library, symbol)? {
            return match resolution {
                TrieResolution::Address(runtime_address) => Ok(Some(runtime_address)),
                TrieResolution::FollowReexport {
                    target_library_path,
                    target_symbol,
                } => self.symbol_resolve_with_max_depth(
                    target_library_path,
                    target_symbol,
                    depth_budget_remaining - 1,
                ),
            };
        }

        self.try_resolve_via_umbrella_reexports(library, symbol, depth_budget_remaining)
    }

    fn try_resolve_via_exports_trie<'r>(
        &'r self,
        library: &'r SharedCacheLibrary<'image>,
        symbol: &'r [u8],
    ) -> Result<Option<TrieResolution<'r>>, &'static str> {
        let image_view = MachOImageView::for_library(self, library);

        let Some(trie_span) = image_view.exports_trie_span_find(library.sub_cache_index)? else {
            return Ok(None);
        };

        let home_subcache = &self.sub_caches[trie_span.sub_cache_index as usize];
        let Some(trie_unslid_vm_address) =
            home_subcache.vm_address_for_file_offset(trie_span.file_offset as u64)
        else {
            return Ok(None);
        };

        let Some((hosting_subcache_index, trie_runtime_offset)) =
            self.subcache_and_offset_locate_from_addr(trie_unslid_vm_address)
        else {
            return Ok(None);
        };

        let Some(trie_bytes) = self.sub_caches[hosting_subcache_index as usize]
            .container
            .slice(trie_runtime_offset, trie_span.size_in_bytes as usize)
        else {
            return Ok(None);
        };

        let Some(terminal) = exports_trie_walk_for_symbol(trie_bytes, symbol) else {
            return Ok(None);
        };

        match terminal {
            ExportTerminal::DefinedHere {
                offset_from_mach_header,
            } => {
                let symbol_unslid_address = library.unslid_load_address + offset_from_mach_header;
                let runtime_address =
                    self.unslid_address_to_runtime_pointer_translate(symbol_unslid_address)?;
                Ok(Some(TrieResolution::Address(runtime_address)))
            }
            ExportTerminal::ReexportedFromDependency {
                dependency_ordinal,
                aliased_name,
            } => {
                let Some(target_library_path) =
                    image_view.install_path_get_from_dylib_ordinal(dependency_ordinal)?
                else {
                    return Ok(None);
                };
                let target_symbol = if aliased_name.is_empty() {
                    symbol
                } else {
                    aliased_name
                };
                Ok(Some(TrieResolution::FollowReexport {
                    target_library_path,
                    target_symbol,
                }))
            }
        }
    }

    fn try_resolve_via_umbrella_reexports(
        &self,
        library: &SharedCacheLibrary<'image>,
        symbol: &[u8],
        depth_budget_remaining: usize,
    ) -> Result<Option<u64>, &'static str> {
        let image_view = MachOImageView::for_library(self, library);
        let reexported_paths = image_view.dylib_paths_collect_reexported()?;

        for reexported_path in reexported_paths {
            if let Some(address) = self.symbol_resolve_with_max_depth(
                reexported_path,
                symbol,
                depth_budget_remaining - 1,
            )? {
                return Ok(Some(address));
            }
        }
        Ok(None)
    }
}

/// The intermediate result of looking up a symbol in one library.
enum TrieResolution<'a> {
    Address(u64),
    FollowReexport {
        target_library_path: &'a [u8],
        target_symbol: &'a [u8],
    },
}

unsafe fn read_main_cache_unslid_base_from(header_pointer: *const u8) -> Result<u64, &'static str> {
    let provisional_length =
        size_of::<dyld_cache_header>() + 32 * size_of::<dyld_cache_mapping_info>();
    let provisional_bytes = unsafe { slice::from_raw_parts(header_pointer, provisional_length) };
    let provisional_container = Container::with_bytes(provisional_bytes, container::Endian::Little);

    let dyld_cache_header { mapping_offset, .. } = dsc_header_read(&provisional_container)?;

    let first_mapping: dyld_cache_mapping_info = provisional_container
        .deserialize_type_at_offset(mapping_offset as usize)
        .map_err(|_| "could not read first mapping_info")?;

    Ok(first_mapping.address)
}

unsafe fn read_main_cache_total_mapped_length_from(
    header_pointer: *const u8,
) -> Result<usize, &'static str> {
    let provisional_length =
        size_of::<dyld_cache_header>() + 64 * size_of::<dyld_cache_mapping_info>();
    let provisional_bytes = unsafe { slice::from_raw_parts(header_pointer, provisional_length) };
    let provisional_container = Container::with_bytes(provisional_bytes, container::Endian::Little);

    let dyld_cache_header {
        mapping_offset,
        mapping_count,
        ..
    } = dsc_header_read(&provisional_container)?;

    let first_mapping: dyld_cache_mapping_info = provisional_container
        .deserialize_type_at_offset(mapping_offset as usize)
        .map_err(|_| "could not read first mapping_info")?;
    let base_address = first_mapping.address;

    let mut max_runtime_end: u64 = 0;
    for entry_index in 0..mapping_count as usize {
        let entry_offset =
            mapping_offset as usize + entry_index * size_of::<dyld_cache_mapping_info>();
        let mapping: dyld_cache_mapping_info = provisional_container
            .deserialize_type_at_offset(entry_offset)
            .map_err(|_| "could not read mapping_info")?;
        let runtime_end = (mapping.address - base_address) + mapping.size;
        if runtime_end > max_runtime_end {
            max_runtime_end = runtime_end;
        }
    }
    Ok(max_runtime_end as usize)
}

fn exports_trie_walk_for_symbol<'a>(
    trie_bytes: &'a [u8],
    symbol: &[u8],
) -> Option<ExportTerminal<'a>> {
    let mut current_node_offset: usize = 0;
    let mut unmatched_suffix_of_symbol: &[u8] = symbol;

    loop {
        let node = TrieNodeReader::open(trie_bytes, current_node_offset)?;

        // Reached a terminal that matches the whole symbol — done.
        if unmatched_suffix_of_symbol.is_empty() && node.has_terminal_payload() {
            return node.terminal_payload_read();
        }

        let (matched_label, child_node_offset) =
            node.child_edge_matching_prefix_find(unmatched_suffix_of_symbol)?;

        unmatched_suffix_of_symbol = &unmatched_suffix_of_symbol[matched_label.len()..];
        current_node_offset = child_node_offset;
    }
}

struct TrieNodeReader<'a> {
    trie_bytes: &'a [u8],
    child_edges_start_offset: usize,
    terminal_payload_start_offset: usize,
    terminal_payload_size_in_bytes: u64,
}

impl<'a> TrieNodeReader<'a> {
    fn open(trie_bytes: &'a [u8], node_offset: usize) -> Option<Self> {
        if node_offset >= trie_bytes.len() {
            return None;
        }
        let (terminal_payload_size_in_bytes, terminal_size_field_byte_count) =
            read_uleb128(&trie_bytes[node_offset..])?;
        let terminal_payload_start_offset = node_offset + terminal_size_field_byte_count;
        let child_edges_start_offset =
            terminal_payload_start_offset + terminal_payload_size_in_bytes as usize;

        if child_edges_start_offset >= trie_bytes.len() {
            return None;
        }

        Some(Self {
            trie_bytes,
            child_edges_start_offset,
            terminal_payload_start_offset,
            terminal_payload_size_in_bytes,
        })
    }

    fn has_terminal_payload(&self) -> bool {
        self.terminal_payload_size_in_bytes > 0
    }

    fn terminal_payload_read(&self) -> Option<ExportTerminal<'a>> {
        let mut cursor = self.terminal_payload_start_offset;

        let (export_flags, flags_field_byte_count) = read_uleb128(&self.trie_bytes[cursor..])?;
        cursor += flags_field_byte_count;

        if export_flags & EXPORT_SYMBOL_FLAGS_REEXPORT != 0 {
            let (dependency_ordinal, ordinal_field_byte_count) =
                read_uleb128(&self.trie_bytes[cursor..])?;
            cursor += ordinal_field_byte_count;

            let aliased_name_terminator = self.trie_bytes[cursor..].iter().position(|&b| b == 0)?;
            let aliased_name = &self.trie_bytes[cursor..cursor + aliased_name_terminator];

            Some(ExportTerminal::ReexportedFromDependency {
                dependency_ordinal,
                aliased_name,
            })
        } else {
            let (offset_from_mach_header, _) = read_uleb128(&self.trie_bytes[cursor..])?;
            Some(ExportTerminal::DefinedHere {
                offset_from_mach_header,
            })
        }
    }

    /// Scans this node's child edges and returns the first one whose
    /// label is a prefix of `remaining_symbol_suffix`
    fn child_edge_matching_prefix_find(
        &self,
        remaining_symbol_suffix: &[u8],
    ) -> Option<(&'a [u8], usize)> {
        let mut cursor = self.child_edges_start_offset;
        let child_edge_count = self.trie_bytes[cursor] as usize;
        cursor += 1;

        for _ in 0..child_edge_count {
            let label_start = cursor;
            let label_length = self.trie_bytes[label_start..]
                .iter()
                .position(|&b| b == 0)?;
            let label = &self.trie_bytes[label_start..label_start + label_length];
            cursor = label_start + label_length + 1;

            let (child_node_offset, offset_field_byte_count) =
                read_uleb128(&self.trie_bytes[cursor..])?;
            cursor += offset_field_byte_count;

            if remaining_symbol_suffix.starts_with(label) {
                return Some((label, child_node_offset as usize));
            }
        }
        None
    }
}
