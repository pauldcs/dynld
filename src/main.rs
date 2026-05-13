#![no_std]
#![no_main]

use crate::{
    allocator::{Allocator, VM},
    array::ArrayVec,
    bindings_macho::{mach_header_64, mach_magic},
    container::Container,
    entrypoint::dynld_entrypoint,
    fixups::{Fixup, FixupKind, fixup_all_chained_fixups},
    image::Image,
    libc::{EXIT_FAILURE, STDERR_FILENO, exit_error},
    loader::macho_endian_from_magic,
};
use core::arch::global_asm;

extern crate alloc;
use alloc::vec::Vec;

#[allow(non_camel_case_types)]
mod bindings_dsc;

#[allow(non_camel_case_types)]
#[allow(unused)] // some code is usused there but will become useful
mod bindings_macho;

mod user_dylibs;

#[allow(non_camel_case_types)]
mod mach;

#[allow(non_camel_case_types, non_snake_case)]
mod dsc;

mod allocator;
mod array;
mod container;
mod dyld_shared_cache;
mod entrypoint;
mod fixups;
mod image;
mod jump;
mod libc;
mod loader;
mod macho_image;
mod print;
//mod ptrauth;
mod dylib;
mod mmap;
mod syscalls;
mod tlv;

pub(crate) const PAGE_ZERO_SIZE: usize = 0x100000000;
pub(crate) const DYLD_LIKELY_UNSLID_VM_ADDR: u64 = 0x180000000;
pub(crate) const SYMBOL_NAME_LEN: usize = 128;
pub(crate) const LIBSYSTEM_PATH: &str = "/usr/lib/libSystem.B.dylib";

global_asm!(include_str!("__dyld_start.s"));

static mut HAS_REBASED_SELF: bool = false;

/// Currently, we only want to support ARM64
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
compile_error!("cuurently dynld only targets aarch64-apple-darwin");

/// The panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if unsafe { HAS_REBASED_SELF } {
        println_err!("panic: {info}");
    } else {
        let _ = libc::write(STDERR_FILENO, b"panic\n");
    }
    let _ = libc::exit(EXIT_FAILURE);

    // if exit failed, we segfault to kill the process
    #[allow(deref_nullptr, clippy::zero_ptr)]
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
#[derive(Copy, Clone)]
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
fn rebase_self_and_extract_bind_fixups(dylinker_container: &Container<'_>) -> Vec<Fixup> {
    // try to parse the fixups, we need them to be able to repabase the
    // data pointers for things to work further down the road
    match Image::with_container(&dylinker_container).chained_fixups_parse_all() {
        Ok(mut fixups) => {
            if !fixups.is_empty() {
                // unwrap if this does not work, we might want to return
                // a proper error instead though
                unsafe {
                    fixup_all_chained_fixups(
                        dylinker_container.as_bytes().as_ptr() as *mut u8,
                        0,
                        &fixups,
                        &Vec::new(),
                        &Vec::new(),
                        None,
                        true,
                    )
                    .map_err(|_| {
                        let _ = libc::write(STDERR_FILENO, b"self rebase error, exiting ...\n");
                        exit_error()
                    })
                };
            };
            // we return the bind fixups, as we cannot handle them right now. We first need to load
            // the shared cache first

            return fixups
                .extract_if(.., |fixup| matches!(fixup.kind, FixupKind::Bind { .. }))
                .collect();
        }
        Err(error) => {
            // we cannot use `println_err` here as we failed to rebase ourselves so we
            // cannot trust any of our DATA pointers, we do this bare write
            // to inform the user and just exit
            let _ = libc::write(STDERR_FILENO, error.as_bytes());
            let _ = libc::write(STDERR_FILENO, b"\nexiting ...\n");

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
        executable_header,
        dylinker_size,
        executable_size,
        argc,
        argv,
        envp,
        apple,
    } = unsafe { *info };

    // wrap the binary blob in a Container in order to be able to parse it
    let dylinker_mach_magic =
        unsafe { core::ptr::read_unaligned(dylinker_header as *const mach_magic) };

    let dylinker_container = Container::with_bytes(
        unsafe { core::slice::from_raw_parts(dylinker_header as *const u8, dylinker_size) },
        macho_endian_from_magic(dylinker_mach_magic),
    );

    // none of the data pointers in the current binary are valid before
    // this function has been executed.
    let self_bind_fixups = rebase_self_and_extract_bind_fixups(&dylinker_container);

    unsafe { HAS_REBASED_SELF = true };

    dynld_entrypoint(
        self_bind_fixups,
        dylinker_container,
        executable_header as *mut mach_header_64,
        executable_size,
        argc,
        argv,
        envp,
        apple,
    );

    // unreachable
}
