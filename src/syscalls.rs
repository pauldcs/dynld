//! Darwin/XNU syscall primitives for aarch64.

use core::arch::asm;

pub const SYS_EXIT: usize = 1;
pub const SYS_WRITE: usize = 4;

#[inline(always)]
fn decode_ret(val: usize, err_bit: u64) -> Result<usize, usize> {
    if err_bit == 0 { Ok(val) } else { Err(val) }
}

#[inline(always)]
pub unsafe fn syscall0(num: usize) -> Result<usize, usize> {
    let val: usize;
    let err: u64;
    unsafe {
        asm!(
            "svc #0x80",
            "cset {err}, cs",
            in("x16") num,
            lateout("x0") val,
            err = lateout(reg) err,
            lateout("x1") _,
            lateout("x16") _,
            lateout("x17") _,
            options(nostack),
        );
    }
    decode_ret(val, err)
}

#[inline(always)]
pub unsafe fn syscall1(num: usize, a0: usize) -> Result<usize, usize> {
    let val: usize;
    let err: u64;
    unsafe {
        asm!(
            "svc #0x80",
            "cset {err}, cs",
            in("x16") num,
            inlateout("x0") a0 => val,
            err = lateout(reg) err,
            lateout("x1") _,
            lateout("x16") _,
            lateout("x17") _,
            options(nostack),
        );
    }
    decode_ret(val, err)
}

#[inline(always)]
pub unsafe fn syscall3(num: usize, a0: usize, a1: usize, a2: usize) -> Result<usize, usize> {
    let val: usize;
    let err: u64;
    unsafe {
        asm!(
            "svc #0x80",
            "cset {err}, cs",
            in("x16") num,
            inlateout("x0") a0 => val,
            in("x1") a1,
            in("x2") a2,
            err = lateout(reg) err,
            lateout("x1") _,
            lateout("x16") _,
            lateout("x17") _,
            options(nostack),
        );
    }
    decode_ret(val, err)
}

#[inline(always)]
pub unsafe fn syscall4(
    num: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
) -> Result<usize, usize> {
    let val: usize;
    let err: u64;
    unsafe {
        asm!(
            "svc #0x80",
            "cset {err}, cs",
            in("x16") num,
            inlateout("x0") a0 => val,
            in("x1") a1,
            in("x2") a2,
            in("x3") a3,
            err = lateout(reg) err,
            lateout("x1") _,
            lateout("x16") _,
            lateout("x17") _,
            options(nostack),
        );
    }
    decode_ret(val, err)
}

#[inline(always)]
pub unsafe fn syscall5(
    num: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
) -> Result<usize, usize> {
    let val: usize;
    let err: u64;
    unsafe {
        asm!(
            "svc #0x80",
            "cset {err}, cs",
            in("x16") num,
            inlateout("x0") a0 => val,
            in("x1") a1,
            in("x2") a2,
            in("x3") a3,
            in("x4") a4,
            err = lateout(reg) err,
            lateout("x1") _,
            lateout("x16") _,
            lateout("x17") _,
            options(nostack),
        );
    }
    decode_ret(val, err)
}
