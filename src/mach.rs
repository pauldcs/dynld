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

pub const FALSE: boolean_t = 0;
pub const TRUE: boolean_t = 1;

pub const VM_PROT_NONE: vm_prot = 0x0;
pub const VM_PROT_READ: vm_prot = 0x1;
pub const VM_PROT_WRITE: vm_prot = 0x2;
pub const VM_PROT_EXECUTE: vm_prot = 0x4;

/// `VM_FLAGS_ANYWHERE` — let the kernel pick an address.
const VM_FLAGS_ANYWHERE: i32 = 0x0000_0001;

// MIG msgh_id for `task_info` in the `task` subsystem.
//   subsystem `task` base = 3400, `task_info` is the 6th routine
//   (task_create, task_terminate, task_threads, mach_ports_register,
//    mach_ports_lookup, task_info) → 3400 + 5 = 3405.
pub const MIG_ID_TASK_INFO: i32 = 3405;

// `task_flavor_t` for TASK_DYLD_INFO.
pub const TASK_DYLD_INFO: i32 = 17;

// `TASK_DYLD_INFO_COUNT = sizeof(task_dyld_info) / sizeof(natural_t)`.
//   task_dyld_info layout (MIG-marshalled, 64-bit):
//     u64 all_image_info_addr
//     u64 all_image_info_size
//     i32 all_image_info_format
//   = 20 bytes = 5 natural_t (4-byte) units.
pub const TASK_DYLD_INFO_COUNT: u32 = 5;

// MACH_MSGH_BITS / MACH_MSG_TYPE constants.
//   MACH_MSG_TYPE_COPY_SEND      = 19
//   MACH_MSG_TYPE_MAKE_SEND_ONCE = 21
//   MACH_MSGH_BITS(remote, local) = (remote) | ((local) << 8)
pub const MACH_MSG_TYPE_COPY_SEND: u32 = 19;
pub const MACH_MSG_TYPE_MAKE_SEND_ONCE: u32 = 21;

pub const MACH_MSG_TIMEOUT_NONE: u32 = 0;
pub const MACH_PORT_NULL: port = 0;

pub const MACH_SEND_MSG_OPT: i32 = 0x0000_0001;
pub const MACH_RCV_MSG_OPT: i32 = 0x0000_0002;

pub const MACH64_SEND_MSG: u64 = 0x0000_0000_0000_0001;
pub const MACH64_RCV_MSG: u64 = 0x0000_0000_0000_0002;
pub const MACH64_MACH_MSG2: u64 = 0x0000_0002_0000_0000;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct mach_msg_header_t {
    pub msgh_bits: u32,
    pub msgh_size: u32,
    pub msgh_remote_port: u32,
    pub msgh_local_port: u32,
    pub msgh_voucher_port: u32,
    pub msgh_id: i32,
}

// The standard little-endian descriptor used by
// every MIG client on Apple platforms.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ndr_record_t {
    mig_vers: u8,
    if_vers: u8,
    reserved1: u8,
    mig_encoding: u8,
    int_rep: u8,
    char_rep: u8,
    float_rep: u8,
    reserved2: u8,
}

// `NDR_record` as exported by libsystem_kernel.dylib:
//   mig_vers = 0, if_vers = 0, reserved1 = 0,
//   mig_encoding = 0 (NDR_PROTOCOL_2_0),
//   int_rep = 0 (NDR_INT_LITTLE_ENDIAN),
//   char_rep = 0 (NDR_CHAR_ASCII),
//   float_rep = 0 (NDR_FLOAT_IEEE),
//   reserved2 = 0.
pub const NDR_RECORD: ndr_record_t = ndr_record_t {
    mig_vers: 0,
    if_vers: 0,
    reserved1: 0,
    mig_encoding: 0,
    int_rep: 0,
    char_rep: 0,
    float_rep: 0,
    reserved2: 0,
};

// Mach trap numbers (negative, per `osfmk/mach/syscall_sw.h`).
const TRAP_KERNELRPC_MACH_VM_ALLOCATE: usize = (-10_isize) as usize;
const TRAP_KERNELRPC_MACH_VM_DEALLOCATE: usize = (-12_isize) as usize;
const TRAP_KERNELRPC_MACH_VM_PROTECT: usize = (-14_isize) as usize;
const TRAP_KERNELRPC_MACH_PORT_DEALLOCATE: usize = (-18_isize) as usize;
const TRAP_KERNELRPC_MACH_PORT_MOD_REFS: usize = (-19_isize) as usize;
const TRAP_MACH_REPLY_PORT: usize = (-26_isize) as usize;
const TRAP_TASK_SELF: usize = (-28_isize) as usize;
const TRAP_MACH_MSG2: usize = (-47_isize) as usize;

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

/// `mach_reply_port()` — allocate a reply port (receive right) in the caller's
/// IPC space. Returns the port name, or `0` on failure.
#[inline]
pub fn mach_reply_port_internal() -> port {
    let r = unsafe { syscalls::syscall0(TRAP_MACH_REPLY_PORT) };
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

/// `_kernelrpc_mach_port_mod_refs_trap(target, name, right, delta)`.
///
/// `right` selects which right type the delta applies to:
///   0 = MACH_PORT_RIGHT_SEND, 1 = MACH_PORT_RIGHT_RECEIVE,
///   2 = MACH_PORT_RIGHT_SEND_ONCE, …
#[inline]
unsafe fn mach_port_mod_refs_internal(
    target: port,
    name: port,
    right: i32,
    delta: i32,
) -> kern_return_t {
    kr(unsafe {
        syscalls::syscall4(
            TRAP_KERNELRPC_MACH_PORT_MOD_REFS,
            target as usize,
            name as usize,
            right as u32 as usize,
            delta as u32 as usize,
        )
    })
}

#[inline]
#[allow(clippy::too_many_arguments)]
unsafe fn mach_msg2_trap_internal(
    data: *mut u8,
    options: u64,
    msgh_bits_and_send_size: u64,
    msgh_remote_and_local_port: u64,
    msgh_voucher_and_id: u64,
    desc_count_and_rcv_name: u64,
    rcv_size_and_priority: u64,
    timeout: u64,
) -> kern_return_t {
    kr(unsafe {
        syscalls::syscall8(
            TRAP_MACH_MSG2,
            data as usize,
            options as usize,
            msgh_bits_and_send_size as usize,
            msgh_remote_and_local_port as usize,
            msgh_voucher_and_id as usize,
            desc_count_and_rcv_name as usize,
            rcv_size_and_priority as usize,
            timeout as usize,
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

/// `mach_task_self()` — return the caller's task port name.
///
/// Cannot fail; the kernel returns the port directly in `x0`.
#[inline]
pub fn mach_task_self() -> port {
    task_self_internal()
}

/// `mach_reply_port()` — allocate a fresh reply port (a receive right) in
/// the caller's IPC space.
///
/// Returns `Err(-1)` if the kernel handed back `MACH_PORT_NULL`
#[inline]
pub fn mach_reply_port() -> Result<port, i32> {
    let name = mach_reply_port_internal();
    if name == MACH_PORT_NULL {
        Err(-1)
    } else {
        Ok(name)
    }
}

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

/// `mach_msg2_trap` — send and/or receive a Mach message.
#[inline]
#[allow(clippy::too_many_arguments)]
pub unsafe fn mach_msg2(
    data: *mut u8,
    options: u64,
    msgh_bits: u32,
    send_size: u32,
    remote_port: port,
    local_port: port,
    voucher_port: port,
    msgh_id: i32,
    desc_count: u32,
    rcv_name: port,
    rcv_size: u32,
    priority: u32,
    timeout: u64,
) -> Result<(), i32> {
    let msgh_bits_and_send_size: u64 = (msgh_bits as u64) | ((send_size as u64) << 32);
    let remote_and_local: u64 = (remote_port as u64) | ((local_port as u64) << 32);
    let voucher_and_id: u64 = (voucher_port as u64) | ((msgh_id as u32 as u64) << 32);
    let desc_and_rcv_name: u64 = (desc_count as u64) | ((rcv_name as u64) << 32);
    let rcv_and_priority: u64 = (rcv_size as u64) | ((priority as u64) << 32);

    let kr = unsafe {
        mach_msg2_trap_internal(
            data,
            options,
            msgh_bits_and_send_size,
            remote_and_local,
            voucher_and_id,
            desc_and_rcv_name,
            rcv_and_priority,
            timeout,
        )
    };
    if kr != KERN_SUCCESS {
        return Err(kr);
    }
    Ok(())
}

/// Adjust the user reference count on a port right by `delta`.
#[inline]
pub fn mach_port_mod_refs(name: port, right: i32, delta: i32) -> Result<(), i32> {
    let kr = unsafe { mach_port_mod_refs_internal(task_self_internal(), name, right, delta) };
    if kr != KERN_SUCCESS {
        return Err(kr);
    }
    Ok(())
}
