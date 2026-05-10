//! Direct Mach trap taken from XNU.

use core::ptr::NonNull;

use crate::syscalls;

/// `kern_return_t`. `KERN_SUCCESS` is `0`; everything else is a failure.
pub type kern_return_t = i32;

/// `mach_port_name_t`. A 32-bit port-name in the caller's IPC space.
pub type port = u32;

pub const KERN_SUCCESS: kern_return_t = 0;

/// A 64-bit virtual address in the current task's address space.
pub type vm_addr = u64;

/// A size in bytes. Mach uses 64-bit sizes here even on 32-bit systems.
pub type vm_size = u64;

/// VM protection bitmask (matches `<mach/vm_prot.h>`).
pub type vm_prot = i32;

/// Mach `boolean_t`. Use [`FALSE`] and [`TRUE`] when calling [`vm_protect`].
pub type boolean_t = u32;

// Mach trap numbers (negative, per `osfmk/mach/syscall_sw.h`).
const TRAP_KERNELRPC_MACH_VM_ALLOCATE: usize = (-10_isize) as usize;
const TRAP_KERNELRPC_MACH_VM_DEALLOCATE: usize = (-12_isize) as usize;
const TRAP_KERNELRPC_MACH_VM_PROTECT: usize = (-14_isize) as usize;
const TRAP_KERNELRPC_MACH_PORT_DEALLOCATE: usize = (-18_isize) as usize;
const TRAP_TASK_SELF: usize = (-28_isize) as usize;

/// Extract the raw `kern_return_t` from a Mach trap result.
#[inline(always)]
fn kr(r: Result<usize, usize>) -> kern_return_t {
    let v = match r {
        Ok(v) | Err(v) => v,
    };
    v as i32
}

/// `task_self_trap()` — return the caller's task port name.
///
/// Cannot fail; the kernel returns the port directly in `x0`.
#[inline]
fn task_self_internal() -> port {
    let r = unsafe { syscalls::syscall0(TRAP_TASK_SELF) };
    match r {
        Ok(v) | Err(v) => v as u32,
    }
}

/// `_kernelrpc_mach_port_deallocate_trap(target, name)`.
#[inline]
#[allow(dead_code)]
unsafe fn mach_port_deallocate_internal(target: port, name: port) -> kern_return_t {
    kr(unsafe {
        syscalls::syscall3(
            TRAP_KERNELRPC_MACH_PORT_DEALLOCATE,
            target as usize,
            name as usize,
            0,
        )
    })
}

/// `_kernelrpc_mach_vm_allocate_trap(target, &mut addr, size, flags)`.
///
/// On success the chosen address is written back through `addr`.
#[inline]
unsafe fn vm_allocate_internal(
    target: port,
    addr: *mut u64,
    size: u64,
    flags: i32,
) -> kern_return_t {
    kr(unsafe {
        syscalls::syscall4(
            TRAP_KERNELRPC_MACH_VM_ALLOCATE,
            target as usize,
            addr as usize,
            size as usize,
            flags as u32 as usize,
        )
    })
}

/// `_kernelrpc_mach_vm_deallocate_trap(target, address, size)`.
#[inline]
unsafe fn vm_deallocate_internal(target: port, address: u64, size: u64) -> kern_return_t {
    kr(unsafe {
        syscalls::syscall3(
            TRAP_KERNELRPC_MACH_VM_DEALLOCATE,
            target as usize,
            address as usize,
            size as usize,
        )
    })
}

/// `_kernelrpc_mach_vm_protect_trap(target, address, size, set_maximum, new_protection)`.
#[inline]
unsafe fn vm_protect_internal(
    target: port,
    address: u64,
    size: u64,
    set_maximum: u32,
    new_protection: i32,
) -> kern_return_t {
    kr(unsafe {
        syscalls::syscall5(
            TRAP_KERNELRPC_MACH_VM_PROTECT,
            target as usize,
            address as usize,
            size as usize,
            set_maximum as usize,
            new_protection as u32 as usize,
        )
    })
}

/// `VM_FLAGS_ANYWHERE` — let the kernel pick an address.
const VM_FLAGS_ANYWHERE: i32 = 0x0000_0001;

/// Allocate `size` bytes in the current task's address space and return a
/// non-null pointer to the new region. Pages are zero-filled and initially
/// readable, writable, and executable (subject to platform restrictions).
///
/// Panics with the kernel's `kern_return_t` if allocation fails.
#[inline]
pub fn vm_alloc_task_self(size: usize) -> Result<NonNull<u8>, i32> {
    let mut addr: vm_addr = 0;
    let kr = unsafe {
        vm_allocate_internal(
            task_self_internal(),
            &mut addr,
            size as vm_size,
            VM_FLAGS_ANYWHERE,
        )
    };
    if kr != KERN_SUCCESS {
        return Err(kr);
    }
    NonNull::new(addr as *mut u8).ok_or(-1)
}

/// Deallocate a region previously returned by [`vm_alloc_task_self`].
#[inline]
pub fn vm_dealloc_task_self(address: vm_addr, size: usize) -> Result<(), i32> {
    let kr = unsafe { vm_deallocate_internal(task_self_internal(), address, size as vm_size) };
    if kr != KERN_SUCCESS {
        return Err(kr);
    }
    Ok(())
}

/// Change the protection on a range in the current task.
///
/// `set_maximum` is [`TRUE`] to update the maximum permitted protection,
/// [`FALSE`] to update the current protection.
#[inline]
pub fn vm_protect(
    address: vm_addr,
    size: usize,
    set_maximum: boolean_t,
    protection: vm_prot,
) -> Result<(), i32> {
    let kr = unsafe {
        vm_protect_internal(
            task_self_internal(),
            address,
            size as vm_size,
            set_maximum,
            protection,
        )
    };
    if kr != KERN_SUCCESS {
        return Err(kr);
    }
    Ok(())
}

/// Copy `count` bytes from `src` to `dst`, both within the current task.
#[inline]
pub unsafe fn vm_copy_into_task_self(src: vm_addr, dst: vm_addr, count: usize) {
    unsafe {
        core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, count);
    }
}
