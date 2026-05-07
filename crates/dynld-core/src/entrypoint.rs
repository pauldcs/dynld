use core::ptr::NonNull;

use dynld_std::{EXIT_FAILURE, println_err};
use sys::unistd::exit_error;

use crate::{bindings::macho::mach_header_64, dsc::DyldSharedCache, dynamic_link};

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
    let mut dyld_shared_cache = DyldSharedCache::new();

    if let Err(err) = dyld_shared_cache.init_from_disk() {
        println_err!("failed initializing dyld shared cache: '{err}'");
        exit_error()
    }

    let entry_point = dynamic_link(
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

    // jump to the entrypoint. We should make sure that this is OK to do
    // before doing this, we fully expect everything to work

    //println!("[+] jumping to entry @ 0x{entry_point:x} ...\n");

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

    let _ = sys::exit(EXIT_FAILURE);
    unreachable!();
    //loop {}
}
