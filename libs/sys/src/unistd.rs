use crate::{
    SYS_CLOSE, SYS_EXIT, SYS_FSTAT, SYS_OPEN, SYS_READ, SYS_WRITE,
    file::File,
    syscalls::{syscall1, syscall2, syscall3},
};

pub const O_RDONLY: usize = 0x0000;
pub const O_WRONLY: usize = 0x0001;
pub const O_RDWR: usize = 0x0002;
pub const O_NONBLOCK: usize = 0x0004;
pub const O_APPEND: usize = 0x0008;
pub const O_CREAT: usize = 0x0200;
pub const O_TRUNC: usize = 0x0400;
pub const O_EXCL: usize = 0x0800;

pub const MODE_644: usize = 0o644;
pub const MODE_755: usize = 0o755;

/// Attempts to write data to the object referenced by the descriptor `fildes`
/// from the buffer `buf`
pub fn write(fildes: usize, buf: &[u8]) -> Result<usize, usize> {
    unsafe { syscall3(SYS_WRITE, fildes, buf.as_ptr() as usize, buf.len()) }
}

/// Terminates the current process
pub fn exit(status: usize) -> Result<usize, usize> {
    unsafe { syscall1(SYS_EXIT, status) }
}

/// Exits with EXIT_FAILURE
pub fn exit_error() -> ! {
    let _ = unsafe { syscall1(SYS_EXIT, 1) };

    unreachable!("exit should have exited")
}

/// Exits with EXIT_SUCCESS
pub fn exit_success() -> ! {
    let _ = unsafe { syscall1(SYS_EXIT, 0) };

    unreachable!("exit should have exited")
}

/// Opens the file at `path` (which MUST be a NUL-terminated C string) with
/// the given `flags` and `mode`. Returns the file descriptor on success.
///
/// # Safety
/// `path` must point to a valid NUL-terminated byte sequence.
pub unsafe fn open(path: *const u8, flags: usize, mode: usize) -> Result<usize, usize> {
    unsafe { syscall3(SYS_OPEN, path as usize, flags, mode) }
}

/// Reads up to `buf.len()` bytes from `fildes` into `buf`. Returns the number
/// of bytes actually read (0 means EOF).
pub fn read(fildes: usize, buf: usize, count: usize) -> Result<usize, usize> {
    unsafe { syscall3(SYS_READ, fildes, buf, count) }
}

/// Closes the file descriptor `fildes`.
pub fn close(fildes: usize) -> Result<usize, usize> {
    unsafe { syscall1(SYS_CLOSE, fildes) }
}

/// Fills `out` with metadata for the open file `fildes`.
pub fn fstat(fildes: usize, out: &mut File) -> Result<usize, usize> {
    unsafe { syscall2(SYS_FSTAT, fildes, out as *mut File as usize) }
}

/// Result of a successful read.
pub struct FileRead<'a> {
    pub bytes_read: usize,
    pub buf: &'a [u8],
}
