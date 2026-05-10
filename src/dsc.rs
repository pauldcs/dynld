//! Rust FFI bindings for `dyld_all_image_infos` and related dyld helpers.
//!

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct user64_dyld_all_image_infos {
    pub version: u32,
    pub infoArrayCount: u32,
    pub infoArray: u64,
    pub notification: u64,
    pub processDetachedFromSharedRegion: u8,
    pub libSystemInitialized: u8,
    pub dyldImageLoadAddress: u64,
    pub jitInfo: u64,
    pub dyldVersion: u64,
    pub errorMessage: u64,
    pub terminationFlags: u64,
    pub coreSymbolicationShmPage: u64,
    pub systemOrderFlag: u64,
    pub uuidArrayCount: u64,
    pub uuidArray: u64,
    pub dyldAllImageInfosAddress: u64,
    pub initialImageCount: u64,
    pub errorKind: u64,
    pub errorClientOfDylibPath: u64,
    pub errorTargetDylibPath: u64,
    pub errorSymbol: u64,
    pub sharedCacheSlide: u64,
    pub sharedCacheUUID: [u8; 16],
    pub sharedCacheBaseAddress: u64,
    pub timestamp: u64,
    pub dyldPath: u64,
    pub notifyMachPorts: [u32; 8],
    pub reserved: [u64; 9],
    pub compact_dyld_image_info_addr: u64,
    pub compact_dyld_image_info_size: u64,
    pub platform: u32,
    pub aotInfoArrayCount: u32,
    pub aotInfoArray: u64,
    pub aotTimestamp: u64,
}

unsafe extern "C" {
    /// Fetch the `dyld_all_image_infos` of `task` into `image_infos`.
    ///
    /// Returns `true` on success.
    ///
    /// # Safety
    /// `image_infos` must be a valid, writable pointer to a
    /// `user64_dyld_all_image_infos`. `task` must be a valid task port
    /// the caller has the right to introspect.
    #[allow(unused)]
    pub fn dyld_all_image_infos_get(image_infos: *mut user64_dyld_all_image_infos) -> u8;

    /// Get the base address of the dyld shared cache for the current process.
    ///
    /// Returns `true` on success, on success `*start_address` is filled in.
    ///
    /// # Safety
    /// `start_address` must be a valid, writable pointer to a `u64`.
    pub fn dyld_shared_cache_base_address_get(start_address: *mut u64) -> u8;
}
