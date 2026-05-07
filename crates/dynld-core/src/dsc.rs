use core::slice;

use container::{Container, read_uleb128};
use sys::unistd::O_RDONLY;

extern crate alloc;

use alloc::vec::Vec;

use crate::{
    DYLD_SHARED_CACHE_PATH,
    bindings::{
        dyld_shared_cache::{
            dyld_cache_header, dyld_cache_image_info, dyld_cache_mapping_info, dyld_info_command,
            dyld_subcache_entry, dylib_command_header,
        },
        macho::{
            EXPORT_SYMBOL_FLAGS_REEXPORT, linkedit_data_command, load_command,
            load_command_variants, mach_header_64,
        },
    },
    dylib::DynamicLib,
};

/// One mapped cache file (main cache or a subcache) and the location of its
/// own mapping table.
struct DyldSharedSubCache<'image> {
    pub container: Container<'image>,
    mappings_offset: u32,
    mappings_count: u32,
}

impl<'image> DyldSharedSubCache<'image> {
    /// Iterate this subcache's mapping table.
    fn mappings(&self) -> impl Iterator<Item = dyld_cache_mapping_info> + '_ {
        let stride = size_of::<dyld_cache_mapping_info>();
        let base = self.mappings_offset as usize;
        (0..self.mappings_count as usize).map(move |i| {
            self.container
                .deserialize_type_at_offset::<dyld_cache_mapping_info>(base + i * stride)
                .expect("malformed cache mapping table")
        })
    }
}

#[derive(Debug)]
pub struct SharedCacheLibrary<'image> {
    sub_cache: u8,
    offset: usize,
    lib: DynamicLib<'image>,
}

pub struct DyldSharedCache<'image> {
    /// Main cache at index 0, subcaches at 1..N.
    sub_caches: Vec<DyldSharedSubCache<'image>>,
    libraries: Vec<SharedCacheLibrary<'image>>,
}

impl<'image> DyldSharedCache<'image> {
    pub fn new() -> Self {
        Self {
            sub_caches: Vec::new(),
            libraries: Vec::new(),
        }
    }

    /// Maps a dyld shared cache file into memory.
    ///
    /// This functions takes a path to a dyld shared cache on disk
    /// and `mmap`s it into memory
    fn file_map_into_memory(path: &[u8]) -> Result<Container<'image>, &'static str> {
        // this fd will leak
        let fd = unsafe { sys::open(path.as_ptr(), O_RDONLY, 0) }
            .map_err(|_| "could not open cache file")? as u32;

        let mut stat = sys::file::File::default();
        sys::fstat(fd as usize, &mut stat).map_err(|_| "could not stat cache file")?;
        let size = stat.st_size as usize;

        // this map will leak
        let bytes = unsafe {
            let ptr = sys::mmap::mmap_file(
                size,
                sys::mmap::PROT_READ | sys::mmap::PROT_EXEC,
                sys::mmap::MAP_PRIVATE,
                fd as sys::mmap::fd_t,
                0,
            )
            .map_err(|_| "could not mmap cache file")?;
            slice::from_raw_parts(ptr.as_ptr(), size)
        };

        //vm_protect(bytes.as_ptr().addr() as u64, size, 0, PROT_READ | PROT_EXEC).unwrap();

        // fix: the endianness should be read from the header
        Ok(Container::with_bytes(bytes, container::Endian::Little))
    }

    /// Build a shared cache path
    ///
    /// This function takes a path buffer and a suffix, it concatenates the
    /// dyld shared cache path (hardcoded) with the suffix.
    /// eg. dyld_shared_cache_arm64e.01
    fn path_with_suffix_build(
        path_buffer: &mut [u8; 256],
        suffix: &[u8],
    ) -> Result<usize, &'static str> {
        let base = DYLD_SHARED_CACHE_PATH.as_bytes();
        let suffix_len = suffix.iter().position(|&b| b == 0).unwrap_or(suffix.len());
        let total = base.len() + suffix_len + 1;
        if total > path_buffer.len() {
            return Err("cache path too long");
        }

        path_buffer[..base.len()].copy_from_slice(base);
        path_buffer[base.len()..base.len() + suffix_len].copy_from_slice(&suffix[..suffix_len]);
        path_buffer[base.len() + suffix_len] = 0;

        Ok(total)
    }

    /// Opens a dyld shared subcache
    ///
    /// this function opens a dyld shared subcache and pushes it into
    /// the `sub_caches` array of this struct
    fn subcache_open(&mut self, suffix: &[u8]) -> Result<(), &'static str> {
        let mut path = [0u8; 256];
        let path_len = Self::path_with_suffix_build(&mut path, suffix)?;
        let container = Self::file_map_into_memory(&path[..path_len])?;

        let dyld_cache_header {
            mapping_offset,
            mapping_count,
            ..
        } = container
            .deserialize_type_at_offset(0)
            .map_err(|_| "could not deserialize dyld_cache_header")?;

        self.sub_caches.push(DyldSharedSubCache {
            mappings_offset: mapping_offset,
            mappings_count: mapping_count,
            container,
        });

        Ok(())
    }

    /// Loads the dyld shared cache
    ///
    /// this function loads the main shared cache (dyld_shared_cache_arm64e) and all
    /// of it's subcaches into memory.
    fn load_dsc_with_subcaches(&mut self) -> Result<(), &'static str> {
        self.subcache_open(&[])?;

        let (sub_cache_array_offset, sub_cache_array_count) = {
            let dyld_cache_header {
                sub_cache_array_offset,
                sub_cache_array_count,
                ..
            } = self.sub_caches[0]
                .container
                .deserialize_type_at_offset::<dyld_cache_header>(0)
                .map_err(|_| "could not deserialize dyld_cache_header")?;

            (sub_cache_array_offset, sub_cache_array_count)
        };

        for sub_cache_array_index in 0..sub_cache_array_count as usize {
            let dyld_subcache_entry { file_suffix, .. } = self.sub_caches[0]
                .container
                .deserialize_type_at_offset(
                    sub_cache_array_offset as usize
                        + sub_cache_array_index * size_of::<dyld_subcache_entry>(),
                )
                .map_err(|_| "could not deserialize dyld_subcache_entry")?;

            unsafe {
                self.subcache_open(core::slice::from_raw_parts(
                    file_suffix.as_ptr() as *const u8,
                    file_suffix.len(),
                ))?;
            }
        }

        Ok(())
    }

    /// Find the subcache and its file offset for a vmaddr
    fn file_off_get_from_vmaddr(&self, vmaddr: u64) -> Result<(u8, usize), &'static str> {
        for (sub_cache_index, sub_cache) in self.sub_caches.iter().enumerate() {
            for dyld_cache_mapping_info {
                address,
                size,
                file_offset,
                ..
            } in sub_cache.mappings()
            {
                if (address..address + size).contains(&vmaddr) {
                    return Ok((
                        sub_cache_index as u8,
                        (file_offset + (vmaddr - address)) as usize,
                    ));
                }
            }
        }
        Err("vm address not covered by any cache mapping")
    }

    /// Initializes the dyld shared cache.
    ///
    /// This function searches for the dyld shared cache on disk (an
    /// it's subcaches) and loads it into memory with `mmap`.
    ///
    /// This is required to do before any symbol addresses can be resolved.
    pub fn init_from_disk(&mut self) -> Result<(), &'static str> {
        self.load_dsc_with_subcaches()?;

        let main_cache = &self.sub_caches[0].container;
        let dyld_cache_header {
            images_count,
            images_offset,
            ..
        } = main_cache
            .deserialize_type_at_offset(0)
            .map_err(|_| "could not deserialize dyld_cache_header")?;

        for image_index in 0..images_count as usize {
            let dyld_cache_image_info {
                address,
                path_file_offset,
                ..
            } = main_cache
                .deserialize_type_at_offset(
                    images_offset as usize + image_index * size_of::<dyld_cache_image_info>(),
                )
                .map_err(|_| "could not deserialize dyld_cache_image_info")?;

            match self.file_off_get_from_vmaddr(address) {
                Ok((sub_idx, macho_off)) => self.libraries.push(SharedCacheLibrary {
                    sub_cache: sub_idx,
                    offset: macho_off,
                    lib: DynamicLib::Cached(
                        main_cache
                            .deserialize_cstr_at_offset(path_file_offset as usize)
                            .to_str()
                            .unwrap(),
                    ),
                }),
                Err(_) => (), // miss
            }

            // todo:
            // parse the mach-o at `self.sub_caches[sub_idx].container`
            // starting at `macho_off` and push a `SharedCacheLibrary`.
        }
        Ok(())
    }
}

/// A span within a dynamic library's exports-trie
#[derive(Debug, Clone, Copy)]
struct ExportsSpan {
    /// The sub cache that contains this export
    sub_cache: u8,
    /// The offset of the export within the sub cache
    file_offset: u32,
    /// The size of the export
    size: u32,
}

/// What an exports-trie terminal node tells us about a symbol
enum ExportKind<'a> {
    /// Symbol lives here, at `mach_header_address + offset`
    Regular { offset: u64 },
    /// Symbol is re-exported. `ordinal` indexes the importing dylib's
    /// `LC_LOAD_DYLIB` or `LC_REEXPORT_DYLIB` list. `name` is the
    /// symbol's name in the target library (empty means that its the same name)
    Reexport { ordinal: u64, name: &'a [u8] },
}

impl<'image> DyldSharedCache<'image> {
    /// Tries to find a dynamic library within the cache
    ///
    /// this function looks for the library path within the dyld shared
    /// cache.
    ///
    /// This should be done for every library and search for the path
    /// on disk only once it has not been found in the cache.
    fn library_find_in_cache(&self, path: &[u8]) -> Option<&SharedCacheLibrary<'image>> {
        self.libraries.iter().find(|lib| match &lib.lib {
            DynamicLib::Cached(p) => p.as_bytes() == path,
            _ => false,
        })
    }

    /// Resolved the address of a symbol
    ///
    /// This function takes a library path, as declared in the Mach-O and a
    /// symbol name (eg. _printf), and tries to find it in the cache or in
    /// a subcache. This function essecially does the same as `dlsym`
    pub fn symbol_resolve(
        &self,
        library_path: &[u8],
        symbol: &[u8],
    ) -> Result<Option<u64>, &'static str> {
        // libSystem can re-export about 10 libraries, dfs of those
        const MAX_DEPTH: usize = 16;
        self.resolve_in(library_path, symbol, MAX_DEPTH)
    }

    fn resolve_in(
        &self,
        library_path: &[u8],
        symbol: &[u8],
        depth_left: usize,
    ) -> Result<Option<u64>, &'static str> {
        if depth_left == 0 {
            return Ok(None);
        }

        let Some(lib) = self.library_find_in_cache(library_path) else {
            return Ok(None);
        };

        if let Some(exports) = self.find_exports_span(lib)? {
            let sub_cache_container = self.sub_caches[exports.sub_cache as usize].container;
            let trie_bytes =
                sub_cache_container.slice(exports.file_offset as usize, exports.size as usize);

            if let Some(trie) = trie_bytes {
                match walk_trie(trie, symbol) {
                    Some(ExportKind::Regular { offset }) => {
                        let Some(lib_unslid) = self.library_load_address(lib)? else {
                            return Ok(None);
                        };
                        let symbol_unslid = lib_unslid + offset;
                        let (sub_idx, file_off) = self.file_off_get_from_vmaddr(symbol_unslid)?;
                        let mmap_base = self.sub_caches[sub_idx as usize]
                            .container
                            .as_bytes()
                            .as_ptr()
                            .addr() as u64;

                        return Ok(Some(mmap_base + file_off as u64));
                    }
                    Some(ExportKind::Reexport { ordinal, name }) => {
                        let Some(target_path) = self.dylib_at_ordinal(lib, ordinal)? else {
                            return Ok(None);
                        };

                        let mut path_buf = [0u8; 256];
                        let n = target_path.len().min(path_buf.len());
                        path_buf[..n].copy_from_slice(&target_path[..n]);
                        return self.resolve_in(
                            &path_buf[..n],
                            if name.is_empty() { symbol } else { name },
                            depth_left - 1,
                        );
                    }
                    None => { /* fall through to umbrella search */ }
                }
            }
        }

        for reexport_path in self.iter_reexport_paths(lib) {
            let mut path_buf = [0u8; 256];
            let n = reexport_path.len().min(path_buf.len());
            path_buf[..n].copy_from_slice(&reexport_path[..n]);
            if let Some(addr) = self.resolve_in(&path_buf[..n], symbol, depth_left - 1)? {
                return Ok(Some(addr));
            }
        }

        Ok(None)
    }

    /// Creates an iterator over a libraries reexport paths
    fn iter_reexport_paths<'a>(
        &'a self,
        lib: &'a SharedCacheLibrary<'image>,
    ) -> impl Iterator<Item = &'a [u8]> + 'a {
        let container = &self.sub_caches[lib.sub_cache as usize].container;
        let mh = container
            .deserialize_type_at_offset::<mach_header_64>(lib.offset)
            .ok();

        // Collect into a small fixed buffer up front so we don't have to
        // thread the load-command iterator state through.
        let mut paths = Vec::new();
        if let Some(mach_header_64 { ncmds, .. }) = mh {
            let mut cmd_off = lib.offset + size_of::<mach_header_64>();
            for _ in 0..ncmds {
                let load_command { cmd, cmdsize, .. } =
                    match container.deserialize_type_at_offset(cmd_off) {
                        Ok(c) => c,
                        Err(_) => break,
                    };

                if matches!(cmd, load_command_variants::LC_REEXPORT_DYLIB) {
                    if let Ok(dylib_command_header { name_offset, .. }) =
                        container.deserialize_type_at_offset(cmd_off)
                    {
                        paths.push(
                            container
                                .deserialize_cstr_at_offset(cmd_off + name_offset as usize)
                                .to_bytes(),
                        );
                    }
                }
                cmd_off += cmdsize as usize;
            }
        }
        paths.into_iter()
    }

    /// The unslid VM address of the library's mach-o header.
    fn library_load_address(
        &self,
        lib: &SharedCacheLibrary<'image>,
    ) -> Result<Option<u64>, &'static str> {
        let main = &self.sub_caches[0].container;
        let dyld_cache_header {
            images_offset,
            images_count,
            ..
        } = main.deserialize_type_at_offset(0).map_err(
            |_| "could not read dyld_cache_header while loading dynamic library address",
        )?;

        for image_index in 0..images_count as usize {
            let dyld_cache_image_info {
                path_file_offset,
                address,
                ..
            } = main.deserialize_type_at_offset(images_offset as usize + image_index * size_of::<dyld_cache_image_info>()).map_err(
                |_| "could not read dyld_cache_image_info while loading dynamic library address",
            )?;

            let path_bytes = main
                .deserialize_cstr_at_offset(path_file_offset as usize)
                .to_bytes();

            if path_bytes
                == match &lib.lib {
                    DynamicLib::Cached(p) => p.as_bytes(),
                    _ => return Ok(None),
                }
            {
                return Ok(Some(address));
            }
        }

        Ok(None)
    }

    /// Locate the library's exports-trie span
    ///
    /// This function walks all load commands to search for the export trie span
    fn find_exports_span(
        &self,
        lib: &SharedCacheLibrary<'image>,
    ) -> Result<Option<ExportsSpan>, &'static str> {
        let container = &self.sub_caches[lib.sub_cache as usize].container;
        let mach_header_64 {ncmds, .. } = container.deserialize_type_at_offset(lib.offset).map_err(
            |_| "could not read mach_header_64 while checking export spans for a dynamic library",
        )?;

        let mut load_command_offset = lib.offset + size_of::<mach_header_64>();
        for _ in 0..ncmds {
            let load_command { cmd, cmdsize, .. } = container
                .deserialize_type_at_offset(load_command_offset)
                .map_err(
                |_| "could not read load_command while checking export spans for a dynamic library",
            )?;
            match cmd {
                load_command_variants::LC_DYLD_EXPORTS_TRIE => {
                    let linkedit_data_command { dataoff, datasize, ..} = container
                        .deserialize_type_at_offset(load_command_offset).map_err(
                            |_| "could not read linkedit_data_command while checking export spans for a dynamic library",
                        )?;
                    return Ok(Some(ExportsSpan {
                        sub_cache: lib.sub_cache,
                        file_offset: dataoff,
                        size: datasize,
                    }));
                }
                load_command_variants::LC_DYLD_INFO_ONLY => {
                    let dyld_info_command {export_off, export_size, ..} = container
                        .deserialize_type_at_offset(load_command_offset)
                        .map_err(
                |_| "could not read dyld_info_command while checking export spans for a dynamic library",
            )?;
                    return Ok(Some(ExportsSpan {
                        sub_cache: lib.sub_cache,
                        file_offset: export_off,
                        size: export_size,
                    }));
                }
                _ => {}
            }
            load_command_offset += cmdsize as usize;
        }
        Ok(None)
    }

    /// Resolve a dylib ordinal to its install path by walking the
    /// `LC_LOAD_DYLIB` / `LC_REEXPORT_DYLIB` commands of `lib`.
    fn dylib_at_ordinal(
        &self,
        lib: &SharedCacheLibrary<'image>,
        ordinal: u64,
    ) -> Result<Option<&[u8]>, &'static str> {
        let container = &self.sub_caches[lib.sub_cache as usize].container;
        let mach_header_64 { ncmds, .. } =
            container.deserialize_type_at_offset(lib.offset).map_err(
                |_| "could not read mach_header_64 when searching for dynamic library at ordinal",
            )?;

        let mut load_command_offset = lib.offset + size_of::<mach_header_64>();
        let mut seen: u64 = 0;

        for _ in 0..ncmds {
            let load_command { cmd, cmdsize, .. } = container
                .deserialize_type_at_offset(load_command_offset)
                .map_err(
                    |_| "could not read load_command when searching for dynamic library at ordinal",
                )?;

            if matches!(
                cmd,
                load_command_variants::LC_LOAD_DYLIB | load_command_variants::LC_REEXPORT_DYLIB
            ) {
                seen += 1;
                if seen == ordinal {
                    let dylib_command_header {name_offset, ..} = container
                        .deserialize_type_at_offset(load_command_offset)
                        .map_err(
                |_| "could not read dylib_command_header when searching for dynamic library at ordinal",
            )?;
                    return Ok(Some(
                        container
                            .deserialize_cstr_at_offset(load_command_offset + name_offset as usize)
                            .to_bytes(),
                    ));
                }
            }
            load_command_offset += cmdsize as usize;
        }
        Ok(None)
    }

    pub fn vm_protect_executable(&self) -> Result<(), &'static str> {
        for sub in &self.sub_caches {
            let bytes = sub.container.as_bytes();
            for max in [false, true] {
                sys::mach::vm_protect(
                    bytes.as_ptr() as sys::mach::vm_addr,
                    bytes.len(),
                    max as u32,
                    sys::mach::VM_PROT_READ | sys::mach::VM_PROT_EXECUTE,
                )
                .map_err(|_| "could not apply vm protections to a memory region")?
            }
        }
        Ok(())
    }
}

/// Walk an exports trie looking for `symbol`.
///
/// This function returns the terminal info on match and
/// `None` on miss or malformed trie.
fn walk_trie<'a>(trie: &'a [u8], symbol: &[u8]) -> Option<ExportKind<'a>> {
    let mut node_offset: usize = 0;
    let mut remaining: &[u8] = symbol;

    loop {
        let mut position = node_offset;
        if position >= trie.len() {
            return None;
        }

        let (terminal_size, consumed) = read_uleb128(&trie[position..])?;
        position += consumed;
        let terminal_end = position + terminal_size as usize;

        if remaining.is_empty() && terminal_size > 0 {
            let (flags, bytes_consumed) = read_uleb128(&trie[position..])?;

            position += bytes_consumed;
            if flags & EXPORT_SYMBOL_FLAGS_REEXPORT != 0 {
                let (ordinal, bytes_consumed) = read_uleb128(&trie[position..])?;
                position += bytes_consumed;

                let name_end = trie[position..]
                    .iter()
                    .position(|&byte| byte == 0)
                    .map(|index| position + index)?;

                return Some(ExportKind::Reexport {
                    ordinal,
                    name: &trie[position..name_end],
                });
            } else {
                let (offset, _) = read_uleb128(&trie[position..])?;
                //let _ = flags & EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER;
                //let _ = flags & EXPORT_SYMBOL_FLAGS_KIND_MASK;
                return Some(ExportKind::Regular { offset });
            }
        }

        position = terminal_end;
        if position >= trie.len() {
            return None;
        }
        let child_count = trie[position] as usize;
        position += 1;

        let mut matched_child: Option<usize> = None;
        for _ in 0..child_count {
            let label_start = position;
            let label_end = trie[label_start..].iter().position(|&b| b == 0)?;
            let label = &trie[label_start..label_start + label_end];

            position = label_start + label_end + 1;

            let (child_off, bytes_consumed) = read_uleb128(&trie[position..])?;
            position += bytes_consumed;

            if remaining.starts_with(label) {
                matched_child = Some(child_off as usize);
                remaining = &remaining[label.len()..];
                break;
            }
        }

        node_offset = matched_child?;
    }
}
