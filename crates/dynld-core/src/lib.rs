#![no_std]

#[allow(non_camel_case_types)]
pub mod bindings;
pub mod dsc;
pub mod dylib;
pub mod dynld;
pub mod entrypoint;
pub mod fixups;
pub mod image;
pub mod tlv;

pub use container::Container;
pub use dynld::dynamic_link;
pub use fixups::fixup_all_chained_fixups;
pub use image::Image;

pub(crate) const PAGE_ZERO_SIZE: usize = 0x100000000;
pub(crate) const DYLD_SHARED_CACHE_PATH: &str =
    "/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld/dyld_shared_cache_arm64e";
