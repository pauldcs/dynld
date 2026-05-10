//! `mmap`/`munmap`/`mprotect` bindings.

use core::ptr::NonNull;

use crate::syscalls;

pub type c_int = i32;

/// Extract the result of a BSD syscall
#[inline(always)]
fn br<T>(r: Result<usize, usize>, ok: impl FnOnce(usize) -> T) -> Result<T, c_int> {
    match r {
        Ok(v) => Ok(ok(v)),
        Err(e) => Err(e as c_int),
    }
}

/// `mmap(addr, len, prot, flags, fd, offset)`
///
/// On success returns the chosen address, on failure returns `errno`
#[inline]
unsafe fn mmap_internal(
    addr: *mut u8,
    len: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: i64,
) -> Result<*mut u8, c_int> {
    br(
        unsafe {
            syscalls::syscall6(
                syscalls::SYS_MMAP,
                addr as usize,
                len,
                prot as usize,
                flags as usize,
                fd as usize,
                offset as usize,
            )
        },
        |v| v as *mut u8,
    )
}

/// `munmap(addr, len)`.
#[inline]
unsafe fn munmap_internal(addr: *mut u8, len: usize) -> Result<(), c_int> {
    br(
        unsafe { syscalls::syscall2(syscalls::SYS_MUNMAP, addr as usize, len) },
        |_| (),
    )
}

/// `mprotect(addr, len, prot)`.
#[inline]
unsafe fn mprotect_internal(addr: *mut u8, len: usize, prot: c_int) -> Result<(), c_int> {
    br(
        unsafe {
            syscalls::syscall3(
                syscalls::SYS_MPROTECT,
                addr as usize,
                len,
                prot as u32 as usize,
            )
        },
        |_| (),
    )
}

/// A page-protection bitmask
pub type prot = c_int;

/// `mmap` flags bitmask
pub type map_flags = c_int;

/// File descriptor.
pub type fd_t = c_int;

/// Byte offset within a mapped file.
pub type off_t = i64;

pub const PROT_NONE: prot = 0x0;
pub const PROT_READ: prot = 0x1;
pub const PROT_WRITE: prot = 0x2;
pub const PROT_EXEC: prot = 0x4;

pub const MAP_SHARED: map_flags = 0x0001;
pub const MAP_PRIVATE: map_flags = 0x0002;
pub const MAP_FIXED: map_flags = 0x0010;
pub const MAP_ANON: map_flags = 0x1000;

pub const MAP_FAILED: usize = !0;

/// Map `len` anonymous, zero-filled bytes into the current process and
/// return a non-null pointer to the new region.
#[inline]
pub fn mmap_anon(len: usize, prot: prot) -> Result<NonNull<u8>, c_int> {
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
    if p as usize == MAP_FAILED {
        return Err(-1);
    }
    NonNull::new(p).ok_or(-1)
}

/// Map `len` bytes of `fd` starting at `offset` into the current process.
///
/// `flags` must include exactly one of [`MAP_SHARED`] or [`MAP_PRIVATE`].
#[inline]
pub fn mmap_file(
    len: usize,
    prot: prot,
    flags: map_flags,
    fd: fd_t,
    offset: off_t,
) -> Result<NonNull<u8>, c_int> {
    let p = unsafe { mmap_internal(core::ptr::null_mut(), len, prot, flags, fd, offset)? };
    if p as usize == MAP_FAILED {
        return Err(-1);
    }
    NonNull::new(p).ok_or(-1)
}

/// Unmap a region previously returned by [`mmap_anon`] or [`mmap_file`].
#[inline]
pub fn munmap(addr: NonNull<u8>, len: usize) -> Result<(), c_int> {
    unsafe { munmap_internal(addr.as_ptr(), len) }
}

/// Change the protection on a mapped range.
#[inline]
pub fn mprotect(addr: NonNull<u8>, len: usize, prot: prot) -> Result<(), c_int> {
    unsafe { mprotect_internal(addr.as_ptr(), len, prot) }
}

/// Copy `count` bytes from `src` to `dst`, both within the current process.
///
/// # Safety
/// Both ranges must be valid for `count` bytes and properly aligned for
/// byte access. Overlap is permitted.
#[inline]
pub unsafe fn mmap_copy(src: *const u8, dst: *mut u8, count: usize) {
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, count);
    }
}
