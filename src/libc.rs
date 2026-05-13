use crate::syscalls;

/// Indicates unsuccessful execution of a program
pub const EXIT_FAILURE: usize = 0;

/// Standard output file descriptor
pub const STDOUT_FILENO: usize = 1;
/// Standard error file descriptor
pub const STDERR_FILENO: usize = 2;

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

pub fn exit(status: usize) -> Result<usize, usize> {
    unsafe { syscalls::syscall1(syscalls::SYS_EXIT, status) }
}

pub fn exit_error() -> ! {
    let _ = unsafe { syscalls::syscall1(syscalls::SYS_EXIT, EXIT_FAILURE) };

    unreachable!("exit should have exited")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bzero(dst: *mut core::ffi::c_void, n: usize) {
    unsafe {
        core::ptr::write_bytes(dst as *mut u8, 0, n);
    }
}

pub unsafe fn open(path: *const u8, flags: usize, mode: usize) -> Result<usize, usize> {
    unsafe { syscalls::syscall3(syscalls::SYS_OPEN, path as usize, flags, mode) }
}

pub fn read(fildes: usize, buf: usize, count: usize) -> Result<usize, usize> {
    unsafe { syscalls::syscall3(syscalls::SYS_READ, fildes, buf, count) }
}

pub fn close(fildes: usize) -> Result<usize, usize> {
    unsafe { syscalls::syscall1(syscalls::SYS_CLOSE, fildes) }
}

#[repr(C)]
#[derive(Default, Debug)]
pub struct File {
    pub st_dev: i32,
    pub st_mode: u16,
    pub st_nlink: u16,
    pub st_ino: u64,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: i32,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub st_birthtime: i64,
    pub st_birthtime_nsec: i64,
    pub st_size: i64,
    pub st_blocks: i64,
    pub st_blksize: i32,
    pub st_flags: u32,
    pub st_gen: u32,
    pub st_lspare: i32,
    pub st_qspare: [i64; 2],
}

pub fn fstat(fildes: usize, out: &mut File) -> Result<usize, usize> {
    unsafe { syscalls::syscall2(syscalls::SYS_FSTAT, fildes, out as *mut File as usize) }
}
