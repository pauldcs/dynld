#![no_std]

pub mod print;

/// Indicates successful execution of a program
pub const EXIT_SUCCESS: usize = 0;
/// Indicates unsuccessful execution of a program
pub const EXIT_FAILURE: usize = 0;

/// Standard input file descriptor
pub const STDIN_FILENO: usize = 0;
/// Standard output file descriptor
pub const STDOUT_FILENO: usize = 1;
/// Standard error file descriptor
pub const STDERR_FILENO: usize = 2;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bzero(dst: *mut core::ffi::c_void, n: usize) {
    unsafe {
        core::ptr::write_bytes(dst as *mut u8, 0, n);
    }
}
