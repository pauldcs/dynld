use core::ptr::NonNull;

use crate::syscalls;

pub const PROT_NONE: i32 = 0x0;
pub const PROT_READ: i32 = 0x1;
pub const PROT_WRITE: i32 = 0x2;
pub const PROT_EXEC: i32 = 0x4;
pub const MAP_SHARED: i32 = 0x01;
pub const MAP_PRIVATE: i32 = 0x02;
pub const MAP_FIXED: i32 = 0x10;
pub const MAP_ANON: i32 = 0x20;

pub const MAP_FAILED: usize = !0;

#[inline]
unsafe fn mmap_internal(
    addr: *mut u8,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
) -> Result<*mut u8, i32> {
    let r = unsafe {
        syscalls::syscall6(
            syscalls::SYS_MMAP,
            addr as usize,
            len,
            prot as usize,
            flags as usize,
            fd as isize as usize,
            offset as usize,
        )
    };
    match r {
        Ok(v) if v == MAP_FAILED => Err(0),
        Ok(v) => Ok(v as *mut u8),
        Err(e) => Err(e as i32),
    }
}

#[inline]
unsafe fn munmap_internal(addr: *mut u8, len: usize) -> Result<(), i32> {
    let r = unsafe { syscalls::syscall2(syscalls::SYS_MUNMAP, addr as usize, len) };
    match r {
        Ok(_) => Ok(()),
        Err(e) => Err(e as i32),
    }
}

#[inline]
unsafe fn mprotect_internal(addr: *mut u8, len: usize, prot: i32) -> Result<(), i32> {
    let r =
        unsafe { syscalls::syscall3(syscalls::SYS_MPROTECT, addr as usize, len, prot as usize) };
    match r {
        Ok(_) => Ok(()),
        Err(e) => Err(e as i32),
    }
}

#[inline]
pub fn mmap_anon(len: usize, prot: i32) -> Result<NonNull<u8>, i32> {
    if len == 0 {
        return Err(22); // EINVAL
    }
    let p = unsafe {
        mmap_internal(
            core::ptr::null_mut(),
            len,
            prot,
            MAP_PRIVATE | MAP_ANON,
            -1,
            0,
        )?
    };
    NonNull::new(p).ok_or(22)
}

#[inline]
pub fn mmap_file(
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
) -> Result<NonNull<u8>, i32> {
    if len == 0 {
        return Err(22);
    }
    let sp = flags & (MAP_SHARED | MAP_PRIVATE);
    if sp != MAP_SHARED && sp != MAP_PRIVATE {
        return Err(22);
    }
    let p = unsafe { mmap_internal(core::ptr::null_mut(), len, prot, flags, fd, offset)? };
    NonNull::new(p).ok_or(22)
}

#[inline]
pub fn munmap(addr: NonNull<u8>, len: usize) -> Result<(), i32> {
    unsafe { munmap_internal(addr.as_ptr(), len) }
}

#[inline]
pub fn mprotect(addr: NonNull<u8>, len: usize, prot: i32) -> Result<(), i32> {
    unsafe { mprotect_internal(addr.as_ptr(), len, prot) }
}

#[inline]
pub unsafe fn mmap_copy(src: *const u8, dst: *mut u8, count: usize) {
    unsafe { core::ptr::copy_nonoverlapping(src, dst, count) };
}
