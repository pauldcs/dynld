use core::slice;

use container::Container;

extern crate alloc;

use alloc::vec::Vec;

use crate::{
    bindings_dsc::{
        dyld_cache_header, dyld_cache_image_info, dyld_cache_mapping_info, dyld_subcache_entry,
    },
    container,
    macho_image::{
        ExportTerminal, ExportsTrieSpan, MAX_REEXPORT_RESOLUTION_DEPTH, MachOImage,
        exports_trie_walk_for_symbol,
    },
};

// The dyld shared cache is a single, very large file (actually several files) into
// which Apple stashes all the system libraries that ship with the OS.

/// One mapped subcache.
struct DyldSharedSubCache<'image> {
    pub container: Container<'image>,
    /// The unslid VM address that file offset 0 of this subcache
    /// corresponds to.
    unslid_base_vm_address: u64,
    mapping_table_offset: u32,
    mapping_table_entry_count: u32,
}

impl<'image> DyldSharedSubCache<'image> {
    /// SAFETY: `runtime_pointer` must point to at least `length` valid,
    /// readable bytes for the lifetime `'image`.
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

    /// Usually three or four entries (__TEXT, __DATA, __LINKEDIT,
    /// sometimes __AUTH).
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

    /// VM-address-to-runtime-offset translation.
    fn runtime_offset_for_vm_address(&self, unslid_vm_address: u64) -> Option<usize> {
        self.mappings_iter().find_map(|mapping| {
            let mapping_range = mapping.address..mapping.address + mapping.size;
            mapping_range
                .contains(&unslid_vm_address)
                .then(|| (unslid_vm_address - self.unslid_base_vm_address) as usize)
        })
    }

    fn runtime_offset_for_file_offset(&self, file_offset: u64) -> Option<usize> {
        self.mappings_iter().find_map(|mapping| {
            let file_range = mapping.file_offset..mapping.file_offset + mapping.size;
            file_range.contains(&file_offset).then(|| {
                let offset_within_mapping = file_offset - mapping.file_offset;
                ((mapping.address - self.unslid_base_vm_address) + offset_within_mapping) as usize
            })
        })
    }

    /// Used when a file offset stored in this subcache actually points at
    /// bytes that physically live in some other subcache.
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
    sub_cache_index: u8,
    mach_header_runtime_offset: usize,
    install_path: &'image str,
    /// The unslid VM address dyld would use as this image's load address.
    unslid_load_address: u64,
}

pub struct DyldSharedCache<'image> {
    /// Main cache at index 0, subcaches at indexes 1..N.
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
    pub unsafe fn from_existing(
        &mut self,
        main_cache_header_pointer: *const u8,
        cache_slide: u64,
    ) -> Result<(), &'static str> {
        self.cache_slide = cache_slide;
        unsafe { self.dyld_shared_cache_attach_all(main_cache_header_pointer)? };
        self.main_cache_libraries_index_all()?;
        Ok(())
    }

    /// The main cache's header contains a `sub_cache_array` listing each
    /// subcache with an offset (`cache_vm_offset`) from the main cache
    /// base. Since dyld laid the whole thing out contiguously in our
    /// address space, we compute each subcache's runtime pointer just by
    /// adding that offset to the main cache's runtime pointer.
    unsafe fn dyld_shared_cache_attach_all(
        &mut self,
        main_cache_header_pointer: *const u8,
    ) -> Result<(), &'static str> {
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

    /// Find the subcache whose mappings cover `unslid_vm_address`.
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

    pub fn is_library_cached(&self, install_path: &[u8]) -> bool {
        self.library_find_by_install_path(install_path).is_some()
    }

    fn image_view_for_library<'a>(
        &'a self,
        library: &SharedCacheLibrary<'image>,
    ) -> MachOImage<'a, 'image> {
        let subcache = &self.sub_caches[library.sub_cache_index as usize];
        MachOImage::new(&subcache.container, library.mach_header_runtime_offset)
    }

    /// Resolves `symbol` (mangled, eg. `b"_malloc"`) within `library_install_path`.
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

    /// Walks a library's exports trie.
    fn try_resolve_via_exports_trie<'r>(
        &'r self,
        library: &'r SharedCacheLibrary<'image>,
        symbol: &'r [u8],
    ) -> Result<Option<TrieResolution<'r>>, &'static str> {
        let image = self.image_view_for_library(library);
        let Some(ExportsTrieSpan {
            file_offset,
            size_in_bytes,
        }) = image.exports_trie_span_find()?
        else {
            return Ok(None);
        };

        let Some(linkedit) = image.linkedit_segment_find()? else {
            return Ok(None);
        };
        let trie_unslid_vm = linkedit.vmaddr + (file_offset as u64 - linkedit.fileoff);

        let Some((hosting_subcache_index, trie_runtime_offset)) =
            self.subcache_and_offset_locate_from_addr(trie_unslid_vm)
        else {
            return Ok(None);
        };

        let Some(trie_bytes) = self.sub_caches[hosting_subcache_index as usize]
            .container
            .slice(trie_runtime_offset, size_in_bytes as usize)
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
                    image.dylib_install_path_for_ordinal(dependency_ordinal)?
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
        let image = self.image_view_for_library(library);
        let reexported_paths = image.reexported_dylib_paths_collect()?;

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
