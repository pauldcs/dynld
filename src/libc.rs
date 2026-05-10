use crate::syscalls;

/// Indicates unsuccessful execution of a program
pub const EXIT_FAILURE: usize = 0;

/// Standard output file descriptor
pub const STDOUT_FILENO: usize = 1;
/// Standard error file descriptor
pub const STDERR_FILENO: usize = 2;

/// Attempts to write data to the object referenced by the descriptor `fildes`
/// from the buffer `buf`
pub fn write(fildes: usize, buf: &[u8]) -> Result<usize, usize> {
    unsafe {
        syscalls::syscall3(
            syscalls::SYS_WRITE,
            fildes,
            buf.as_ptr() as usize,
            buf.len(),
        )
    }
}

/// Terminates the current process
pub fn exit(status: usize) -> Result<usize, usize> {
    unsafe { syscalls::syscall1(syscalls::SYS_EXIT, status) }
}

/// Exits with EXIT_FAILURE
pub fn exit_error() -> ! {
    let _ = unsafe { syscalls::syscall1(syscalls::SYS_EXIT, EXIT_FAILURE) };

    unreachable!("exit should have exited")
}

// while we use compiler mem intrinsics, this function is not included as it is
// basically part of libc. OSX needs it for some reason so we make it available.
// The compiler will complain that this is missing if we dont include it.
//
// i didn't find an answer why
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bzero(dst: *mut core::ffi::c_void, n: usize) {
    unsafe {
        core::ptr::write_bytes(dst as *mut u8, 0, n);
    }
}
