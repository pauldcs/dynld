#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

use array::array_vec::ArrayVec;
use core::{
    arch::global_asm,
    ptr::{self},
};
use dynld_alloc::{Allocator, VM};
use dynld_core::{
    Container, Image,
    bindings::macho::{mach_header_64, mach_magic},
    dynld::macho_endian_from_magic,
    entrypoint::dynld_entrypoint,
    fixup_all_chained_fixups,
};
use dynld_std::{self, EXIT_FAILURE, STDERR_FILENO};
use sys::unistd::exit_error;

extern crate alloc;
use alloc::vec::Vec;

global_asm!(include_str!("dyld_start.s"));

/// Currently, we only want to support ARM64
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
compile_error!("cuurently dynld only targets aarch64-apple-darwin");

/// The panic handler
///
/// note that you cannot use println or anything that uses data pointers in
/// this handler. This handler might be called before we were able to rebase ourselves
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // println_err!("panic: {info}");
    let _ = sys::write(STDERR_FILENO, b"panic\n");
    let _ = sys::exit(EXIT_FAILURE);
    #[allow(deref_nullptr)]
    let _ = unsafe { *(0 as *const u8) };
    loop {}
}

/// Our allocator
#[global_allocator]
static ALLOC: Allocator = Allocator::new(VM);

/// The information passed by the kernel
///
/// This struct gets passed as the first argument to the
/// start function, called by the kernel.
#[repr(C)]
pub struct Stack {
    /// A pointer to the dynamic linkes own mach header
    pub dylinker_header: *const mach_header_64,
    /// A pointer to the executables mach header
    pub executable_header: *const mach_header_64,
    /// The size of the dynamic linker image
    pub dylinker_size: usize,
    /// The size of the executable image
    pub executable_size: usize,
    /// `argc` as passed to the executable
    pub argc: usize,
    /// `argv` as passed to the executable
    pub argv: *const *const u8,
    /// `envp` as passed to the executable
    pub envp: *const *const u8,
    /// the apple strings of the executable
    pub apple: *const *const u8,
}

/// Rebases the dynamic linker itself
///
/// This program cannot use any DATA pointers before this function is called as they need
/// to be fixed-up first. Note that the segments of the dynamic linker image must have
/// been protected with `vm_protect` with `VM_PROT_WRITE` before for this to work.
fn rebase_self(self_header: *mut u8, self_size: usize) {
    // wrap the binary blob in a Container in order to be able to parse it
    let magic = unsafe { ptr::read_unaligned(self_header as *const mach_magic) };
    let dylinker_container = Container::with_bytes(
        unsafe { core::slice::from_raw_parts(self_header, self_size) },
        macho_endian_from_magic(magic),
    );

    // try to parse the fixups, we need them to be able to repabase the
    // data pointers for things to work further down the road
    match Image::with_container(&dylinker_container).chained_fixups_parse_all() {
        Ok(fixups) => {
            if !fixups.is_empty() {
                // unwrap if this does not work, we might want to return
                // a proper error instead though
                fixup_all_chained_fixups(
                    self_header,
                    0,
                    &fixups,
                    &Vec::new(),
                    &ArrayVec::new_array(),
                    None,
                    true,
                )
                .map_err(|_| {
                    let _ = sys::write(STDERR_FILENO, b"self rebase error, exiting ...\n");
                    exit_error()
                });
            }
        }
        Err(error) => {
            // we cannot use `println_err` here as we failed to rebase ourselves so we
            // cannot trust any of our DATA pointers, we do this bare write
            // to inform the user and just exit
            let _ = sys::write(STDERR_FILENO, error.as_bytes());
            let _ = sys::write(STDERR_FILENO, b"\nexiting ...\n");

            // bye
            exit_error()
        }
    };
}

/// The main entrypoint of the dynamic linker
///
/// This expects the segments of the image pointed to by `dylinker_header`
/// to have been protected according to its segments. The caller needs to
/// parse it and apply the protections manually to it before calling this
#[unsafe(no_mangle)]
pub unsafe extern "C" fn start(info: *const Stack) {
    let Stack {
        dylinker_header,
        dylinker_size,
        executable_header,
        executable_size,
        argc,
        argv,
        envp,
        apple,
    } = unsafe { &*info };

    // none of the data pointers in the current binary are valid before
    // this function has been executed.
    rebase_self(*dylinker_header as *mut u8, *dylinker_size);

    dynld_entrypoint(
        *executable_header as *mut mach_header_64,
        *executable_size,
        *argc,
        *argv,
        *envp,
        *apple,
    );

    // unreachable
}
