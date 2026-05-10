use core::ptr::NonNull;

use crate::{
    bindings_macho::mach_header_64, dsc, dyld_shared_cache::DyldSharedCache,
    dynld::dynamically_link, jump, libc::exit_error, println_err,
};

/// The main entrypoint of the dynld
///
/// This function is meant to take a pointer to the [`mach_header_64`] with
/// the size of the executable. It dynamically links it and executes it.
///
/// This function never returns and if an internal error happens, it will loop
/// infinitly.
pub fn dynld_entrypoint(
    executable_header: *mut mach_header_64,
    executable_size: usize,
    argc: usize,
    argv: *const *const u8,
    envp: *const *const u8,
    _apple: *const *const u8,
) -> ! {
    let mut dyld_shared_cache_baddr_in_vm = 0;
    if unsafe { dsc::dyld_shared_cache_base_address_get(&mut dyld_shared_cache_baddr_in_vm) } == 0 {
        println_err!("failed to find the dyld_shared_cache base address");
        exit_error()
    }

    let mut dyld_shared_cache = DyldSharedCache::new();
    if let Err(err) = unsafe {
        dyld_shared_cache.from_live_mapping_initialize(
            dyld_shared_cache_baddr_in_vm as *const u8,
            dyld_shared_cache_baddr_in_vm - 0x180000000,
        )
    } {
        println_err!("failed initializing dyld shared cache: '{err}'");
        exit_error()
    }

    //println!("{:#?}", dyld_shared_cache.libraries);
    let entry_point = dynamically_link(
        &dyld_shared_cache,
        executable_header as *mut u8,
        executable_size,
    )
    .unwrap_or_else(|err| {
        println_err!("dylinking error: '{err}', giving up...");
        exit_error()
    });

    // we only check that the entrypoint is not NULL, if
    // it is something else, we expect it to be valid at this point
    if entry_point == 0 {
        println_err!("the entry_point of the program is NULL, giving up...");
        exit_error()
    }

    //unsafe {
    //    tlv_initialize_descriptors_export(executable_header as *const mach_header_64);
    //}

    // jump to the entrypoint. We should make sure that this is OK to do
    // before doing this, we fully expect everything to work
    unsafe {
        jump::entry_and_ret(
            NonNull::new_unchecked(entry_point as *mut u8),
            argc,
            argv,
            envp,
        )
    };

    // THIS FUNCTION CANNOT RETURN! the caller nulled the link register
    // and the frame pointer, we try to exit but trigger and inifite loop
    // if that does not work

    exit_error()
}
