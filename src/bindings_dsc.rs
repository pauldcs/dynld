//! On-disk layout of the dyld shared cache.
//!
//! Direct translation of `<mach-o/dyld_cache_format.h>`. Field ordering and
//! sizes match the C definitions exactly so these can be deserialised
//! straight from a memory-mapped cache file.

use core::ffi::c_char;
use scroll_derive::Pread;

/// Header at offset 0 of every dyld shared cache file.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pread)]
pub struct dyld_cache_header {
    /// e.g. "dyld_v0    i386"
    pub magic: [c_char; 16],
    /// File offset to first [`dyld_cache_mapping_info`].
    pub mapping_offset: u32,
    /// Number of [`dyld_cache_mapping_info`] entries.
    pub mapping_count: u32,
    /// UNUSED: moved to `images_offset` to prevent older `dsc_extractor`s from crashing.
    pub images_offset_old: u32,
    /// UNUSED: moved to `images_count` to prevent older `dsc_extractor`s from crashing.
    pub images_count_old: u32,
    /// Base address of dyld when cache was built.
    pub dyld_base_address: u64,
    /// File offset of code signature blob.
    pub code_signature_offset: u64,
    /// Size of code signature blob (zero means to end of file).
    pub code_signature_size: u64,
    /// Unused. Used to be file offset of kernel slid info.
    pub slide_info_offset_unused: u64,
    /// Unused. Used to be size of kernel slid info.
    pub slide_info_size_unused: u64,
    /// File offset of where local symbols are stored.
    pub local_symbols_offset: u64,
    /// Size of local symbols information.
    pub local_symbols_size: u64,
    /// Unique value for each shared cache file.
    pub uuid: [u8; 16],
    /// 0 for development, 1 for production, 2 for multi-cache.
    pub cache_type: u64,
    /// File offset to table of `u64` pool addresses.
    pub branch_pools_offset: u32,
    /// Number of `u64` entries.
    pub branch_pools_count: u32,
    /// (Unslid) address of mach_header of dyld in cache.
    pub dyld_in_cache_mh: u64,
    /// (Unslid) address of entry point (`_dyld_start`) of dyld in cache.
    pub dyld_in_cache_entry: u64,
    /// File offset to first [`dyld_cache_image_text_info`].
    pub images_text_offset: u64,
    /// Number of [`dyld_cache_image_text_info`] entries.
    pub images_text_count: u64,
    /// (Unslid) address of `dyld_cache_patch_info`.
    pub patch_info_addr: u64,
    /// Size of all of the patch information pointed to via `patch_info_addr`.
    pub patch_info_size: u64,
    /// Unused.
    pub other_image_group_addr_unused: u64,
    /// Unused.
    pub other_image_group_size_unused: u64,
    /// (Unslid) address of list of program launch closures.
    pub prog_closures_addr: u64,
    /// Size of list of program launch closures.
    pub prog_closures_size: u64,
    /// (Unslid) address of trie of indexes into program launch closures.
    pub prog_closures_trie_addr: u64,
    /// Size of trie of indexes into program launch closures.
    pub prog_closures_trie_size: u64,
    /// Platform number (macOS=1, etc).
    pub platform: u32,
    /// Bitfield: `format_version:8, dylibs_expected_on_disk:1, simulator:1,
    /// locally_built_cache:1, built_from_chained_fixups:1, new_format_tlvs:1,
    /// padding:19`. Use the accessors on this struct rather than the raw bits.
    pub flags: u32,
    /// Base load address of cache if not slid.
    pub shared_region_start: u64,
    /// Overall size required to map the cache and all subcaches, if any.
    pub shared_region_size: u64,
    /// Runtime slide of cache can be between zero and this value.
    pub max_slide: u64,
    /// (Unslid) address of `ImageArray` for dylibs in this cache.
    pub dylibs_image_array_addr: u64,
    /// Size of `ImageArray` for dylibs in this cache.
    pub dylibs_image_array_size: u64,
    /// (Unslid) address of trie of indexes of all cached dylibs.
    pub dylibs_trie_addr: u64,
    /// Size of trie of cached dylib paths.
    pub dylibs_trie_size: u64,
    /// (Unslid) address of `ImageArray` for dylibs and bundles with dlopen closures.
    pub other_image_array_addr: u64,
    /// Size of `ImageArray` for dylibs and bundles with dlopen closures.
    pub other_image_array_size: u64,
    /// (Unslid) address of trie of indexes of all dylibs and bundles with dlopen closures.
    pub other_trie_addr: u64,
    /// Size of trie of dylibs and bundles with dlopen closures.
    pub other_trie_size: u64,
    /// File offset to first [`dyld_cache_mapping_and_slide_info`].
    pub mapping_with_slide_offset: u32,
    /// Number of [`dyld_cache_mapping_and_slide_info`] entries.
    pub mapping_with_slide_count: u32,
    /// Unused.
    pub dylibs_pbl_state_array_addr_unused: u64,
    /// (Unslid) address of `PrebuiltLoaderSet` of all cached dylibs.
    pub dylibs_pbl_set_addr: u64,
    /// (Unslid) address of pool of `PrebuiltLoaderSet` for each program.
    pub programs_pbl_set_pool_addr: u64,
    /// Size of pool of `PrebuiltLoaderSet` for each program.
    pub programs_pbl_set_pool_size: u64,
    /// (Unslid) address of trie mapping program path to `PrebuiltLoaderSet`.
    pub program_trie_addr: u64,
    /// Size of program trie.
    pub program_trie_size: u32,
    /// OS version of dylibs in this cache for the main platform.
    pub os_version: u32,
    /// Alternate platform, e.g. iOSMac on macOS.
    pub alt_platform: u32,
    /// Alternate OS version, e.g. 14.0 for iOSMac.
    pub alt_os_version: u32,
    /// VM offset from cache_header to Swift optimizations header.
    pub swift_opts_offset: u64,
    /// Size of Swift optimizations header.
    pub swift_opts_size: u64,
    /// File offset to first [`dyld_subcache_entry`].
    pub sub_cache_array_offset: u32,
    /// Number of subcache entries.
    pub sub_cache_array_count: u32,
    /// Unique value for the shared cache file containing unmapped local symbols.
    pub symbol_file_uuid: [u8; 16],
    /// (Unslid) address of the start of where Rosetta can add read-only/executable data.
    pub rosetta_read_only_addr: u64,
    /// Maximum size of the Rosetta read-only/executable region.
    pub rosetta_read_only_size: u64,
    /// (Unslid) address of the start of where Rosetta can add read-write data.
    pub rosetta_read_write_addr: u64,
    /// Maximum size of the Rosetta read-write region.
    pub rosetta_read_write_size: u64,
    /// File offset to first [`dyld_cache_image_info`].
    pub images_offset: u32,
    /// Number of [`dyld_cache_image_info`] entries.
    pub images_count: u32,
    /// 0 for development, 1 for production, when `cache_type` is multi-cache (2).
    pub cache_sub_type: u32,
    /// VM offset from cache_header to ObjC optimizations header.
    pub objc_opts_offset: u64,
    /// Size of ObjC optimizations header.
    pub objc_opts_size: u64,
    /// VM offset from cache_header to embedded cache atlas for process introspection.
    pub cache_atlas_offset: u64,
    /// Size of embedded cache atlas.
    pub cache_atlas_size: u64,
    /// VM offset from cache_header to the location of `dyld_cache_dynamic_data_header`.
    pub dynamic_data_offset: u64,
    /// Maximum size of space reserved for dynamic data.
    pub dynamic_data_max_size: u64,
    /// File offset to first `dyld_cache_tpro_mapping_info`.
    pub tpro_mappings_offset: u32,
    /// Number of `dyld_cache_tpro_mapping_info` entries.
    pub tpro_mappings_count: u32,
    /// (Unslid) address of `dyld_cache_function_variant_info`.
    pub function_variant_info_addr: u64,
    /// Size of all of the variant information pointed to via `function_variant_info_addr`.
    pub function_variant_info_size: u64,
    /// File offset to `dyld_prewarming_header`.
    pub prewarming_data_offset: u64,
    /// Byte size of prewarming data.
    pub prewarming_data_size: u64,
}

/// One entry in the cache's mapping table. Defines the VM-address ↔
/// file-offset relationship for a contiguous region of the cache.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pread)]
pub struct dyld_cache_mapping_info {
    /// Unslid VM address where this region is mapped.
    pub address: u64,
    /// Byte size of the region.
    pub size: u64,
    /// Offset of the region within the cache file.
    pub file_offset: u64,
    /// Maximum permitted protection (`VM_PROT_*` bits).
    pub max_prot: u32,
    /// Initial protection (`VM_PROT_*` bits).
    pub init_prot: u32,
}

/// One entry per dylib embedded in the cache.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pread)]
pub struct dyld_cache_image_info {
    /// Unslid VM address of the image's `mach_header`.
    pub address: u64,
    /// Modification time of the source dylib on disk at cache-build time.
    pub mod_time: u64,
    /// Inode of the source dylib on disk at cache-build time.
    pub inode: u64,
    /// File offset (within the cache) of the NUL-terminated install path.
    pub path_file_offset: u32,
    /// Padding.
    pub pad: u32,
}

/// Subcache descriptor. Modern caches are split across a main file plus
/// numbered/suffixed sibling files; this entry lives in the main cache and
/// describes one of those siblings.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pread)]
pub struct dyld_subcache_entry {
    /// LC_UUID of the subcache file.
    pub uuid: [u8; 16],
    /// Offset of this subcache from the main cache's base address (in unslid VM space).
    pub cache_vm_offset: u64,
    /// Filename suffix appended to the main cache name, e.g. `".25.data"` or `".03.development"`.
    pub file_suffix: [c_char; 32],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pread)]
pub struct dyld_info_command {
    pub cmd: u32,
    pub cmdsize: u32,
    pub rebase_off: u32,
    pub rebase_size: u32,
    pub bind_off: u32,
    pub bind_size: u32,
    pub weak_bind_off: u32,
    pub weak_bind_size: u32,
    pub lazy_bind_off: u32,
    pub lazy_bind_size: u32,
    pub export_off: u32,
    pub export_size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pread)]
pub struct dylib_command_header {
    pub cmd: u32,
    pub cmdsize: u32,
    /// Offset (from start of load command) of the install-name string.
    pub name_offset: u32,
    pub timestamp: u32,
    pub current_version: u32,
    pub compatibility_version: u32,
}

/// Slide info version 2 (standard arm64 and x86_64).
#[derive(Debug, Clone, Copy, Pread)]
#[repr(C)]
pub struct dyld_cache_slide_info_2 {
    /// Version (should be 2)
    pub version: u32,
    /// Page size (typically 4KB)
    pub page_size: u32,
    /// Offset to page starts array (from start of this struct)
    pub page_starts_offset: u32,
    /// Number of page starts entries
    pub page_starts_count: u32,
    /// Offset to page extras array (from start of this struct)
    pub page_extras_offset: u32,
    /// Number of page extras entries
    pub page_extras_count: u32,
    /// Mask for delta field in pointer
    pub delta_mask: u64,
    /// Value to add to rebased pointers
    pub value_add: u64,
}

impl dyld_cache_slide_info_2 {
    /// Returns the mask for the value portion of a pointer.
    pub fn value_mask(&self) -> u64 {
        !self.delta_mask
    }

    /// Returns the shift amount for the delta field.
    pub fn delta_shift(&self) -> u32 {
        self.delta_mask.trailing_zeros()
    }
}

/// Slide info version 3 (arm64e with PAC).
#[derive(Debug, Clone, Copy, Pread)]
#[repr(C)]
pub struct dyld_cache_slide_info_3 {
    /// Version (should be 3)
    pub version: u32,
    /// Page size (typically 16KB)
    pub page_size: u32,
    /// Number of page starts entries
    pub page_starts_count: u32,
    /// Value to add for authenticated pointers
    pub auth_value_add: u64,
    // Followed by page_starts array of u16
}

/// Slide info version 5 (arm64e iOS 18+).
#[derive(Debug, Clone, Copy, Pread)]
#[repr(C)]
pub struct dyld_cache_slide_info_5 {
    /// Version (should be 5)
    pub version: u32,
    /// Page size
    pub page_size: u32,
    /// Number of page starts entries
    pub page_starts_count: u32,
    /// Value to add to pointers
    pub value_add: u64,
    // Followed by page_starts array of u16
}

/// Encoded pointer for slide info v3.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct slide_pointer_3(pub u64);

impl slide_pointer_3 {
    /// Returns true if this is an authenticated pointer.
    #[inline]
    pub fn is_auth(&self) -> bool {
        (self.0 >> 63) & 1 != 0
    }

    /// Returns the offset to the next rebase location (in 8-byte units).
    #[inline]
    pub fn offset_to_next(&self) -> u64 {
        (self.0 >> 51) & 0x7FF
    }

    /// For authenticated pointers: returns the offset from the auth base.
    #[inline]
    pub fn auth_offset(&self) -> u32 {
        (self.0 & 0xFFFFFFFF) as u32
    }

    /// For plain pointers: returns the decoded value.
    #[inline]
    pub fn plain_value(&self) -> u64 {
        // Sign extend from 51 bits
        let value = self.0 & 0x0007_FFFF_FFFF_FFFF;
        let top8 = ((self.0 >> 43) & 0xFF) as u8;
        ((top8 as u64) << 56) | value
    }
}

/// Encoded pointer for slide info v5.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct slide_pointer_5(pub u64);

impl slide_pointer_5 {
    /// Returns true if this is an authenticated pointer.
    #[inline]
    pub fn is_auth(&self) -> bool {
        (self.0 >> 63) & 1 != 0
    }

    /// Returns the offset to the next rebase (in 8-byte units).
    #[inline]
    pub fn next(&self) -> u64 {
        (self.0 >> 51) & 0x7FF
    }

    /// Returns the runtime offset (for both auth and non-auth).
    #[inline]
    pub fn runtime_offset(&self) -> u64 {
        self.0 & 0x0007_FFFF_FFFF_FFFF
    }

    /// For non-auth pointers: returns the high 8 bits.
    #[inline]
    pub fn high8(&self) -> u8 {
        ((self.0 >> 43) & 0xFF) as u8
    }
}

#[derive(Debug, Clone, Copy, Pread)]
#[repr(C)]
pub struct dyld_cache_mapping_and_slide_info {
    /// Virtual memory address
    pub address: u64,
    /// Size in bytes
    pub size: u64,
    /// File offset
    pub file_offset: u64,
    /// Slide info file offset
    pub slide_info_file_offset: u64,
    /// Slide info file size
    pub slide_info_file_size: u64,
    /// Flags
    pub flags: u64,
    /// Maximum memory protection
    pub max_prot: u32,
    /// Initial memory protection
    pub init_prot: u32,
}
