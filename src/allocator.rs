//! A simple global allocator backed by our mach vm wrappers. This was prompted from
//! Claude and i did not test it properly at all. We should replace this with a better
//! allocator as soon as possible. This is only used for things such as holding symbols,
//! which are not expected to change (not a lot of alloc/dealloc)

#![deny(unsafe_op_in_unsafe_fn)]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::mem;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::mach;

pub struct VM;

impl VM {
    fn alloc(&self, size: usize) -> Result<NonNull<u8>, i32> {
        mach::vm_alloc_task_self(size)
    }
    unsafe fn dealloc(&self, ptr: NonNull<u8>, size: usize) -> Result<(), i32> {
        mach::vm_dealloc_task_self(ptr.as_ptr() as u64, size)
    }
}

pub const PAGE_SIZE: usize = 4096;
pub const CHUNK_SIZE: usize = 64 * PAGE_SIZE; // 256 KiB
pub const LARGE_THRESHOLD: usize = CHUNK_SIZE / 2;

/// Frees smaller than this are leaked due to the shitty design of this whole thing
pub const MIN_REUSE: usize = mem::size_of::<FreeNode>();

/// A free-list node, written into the freed memory itself.
#[repr(C)]
pub struct FreeNode {
    next: Option<NonNull<FreeNode>>,
    size: usize,
}

// SAFETY: nodes are only dereferenced while the allocator lock is held.
unsafe impl Send for FreeNode {}

struct State {
    /// Current bump chunk: pointer to start, total size, and offset.
    chunk: Option<Chunk>,
    /// Free list head (first-fit search, simple and small).
    free: Option<NonNull<FreeNode>>,
}

#[derive(Copy, Clone)]
struct Chunk {
    base: NonNull<u8>,
    size: usize,
    offset: usize,
}

impl State {
    const fn new() -> Self {
        Self {
            chunk: None,
            free: None,
        }
    }
}

/// The allocator.
pub struct Allocator {
    backend: VM,
    locked: AtomicBool,
    state: UnsafeCell<State>,
}

// SAFETY: all access to `state` is protected by the `locked` spinlock.
unsafe impl Sync for Allocator {}

impl Allocator {
    pub const fn new(backend: VM) -> Self {
        Self {
            backend,
            locked: AtomicBool::new(false),
            state: UnsafeCell::new(State::new()),
        }
    }

    /// Acquire the spinlock and run `f` with exclusive access to `State`.
    fn with_state<R>(&self, f: impl FnOnce(&mut State, &VM) -> R) -> R {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        let result = f(unsafe { &mut *self.state.get() }, &self.backend);
        self.locked.store(false, Ordering::Release);
        result
    }

    fn alloc_impl(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(1);
        let align = layout.align().max(1);

        if size < MIN_REUSE {
            // this is shameful but one day i will fix this and write a proper
            // allocator
            panic!("attempted to alloc something smaller that a FreeNode")
        }

        if size >= LARGE_THRESHOLD {
            return match self.backend.alloc(size) {
                Ok(p) => p.as_ptr(),
                Err(_) => core::ptr::null_mut(),
            };
        }

        self.with_state(|state, backend| {
            if let Some(p) = take_from_free_list(state, size, align) {
                return p.as_ptr();
            }

            if let Some(p) = bump_alloc(state, size, align) {
                return p.as_ptr();
            }

            let chunk_size = CHUNK_SIZE.max(size + align);
            let base = match backend.alloc(chunk_size) {
                Ok(p) => p,
                Err(_) => return core::ptr::null_mut(),
            };
            state.chunk = Some(Chunk {
                base,
                size: chunk_size,
                offset: 0,
            });

            bump_alloc(state, size, align)
                .map(|p| p.as_ptr())
                .unwrap_or(core::ptr::null_mut())
        })
    }

    unsafe fn dealloc_impl(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(1);
        let Some(nn) = NonNull::new(ptr) else { return };

        if size >= LARGE_THRESHOLD {
            let _ = unsafe { self.backend.dealloc(nn, size) };
            return;
        }

        if size < MIN_REUSE {
            // Too small to track, leak
            return;
        }

        self.with_state(|state, _| {
            // SAFETY: ptr is a valid, unique allocation of at least `size` bytes,
            // and `size >= MIN_REUSE` so a FreeNode fits.
            let node = nn.cast::<FreeNode>();
            unsafe {
                node.as_ptr().write(FreeNode {
                    next: state.free,
                    size,
                });
            }
            state.free = Some(node);
        });
    }
}

// SAFETY: alloc_impl returns either null or a unique, properly-aligned pointer
// to at least `layout.size()` bytes; dealloc_impl preserves those invariants.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_impl(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.dealloc_impl(ptr, layout) }
    }
}

#[inline]
fn align_up(addr: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (addr + align - 1) & !(align - 1)
}

/// Try to satisfy a request from the current bump chunk. Returns `None` if
/// there's no chunk or it's exhausted.
fn bump_alloc(state: &mut State, size: usize, align: usize) -> Option<NonNull<u8>> {
    let chunk = state.chunk.as_mut()?;
    let base = chunk.base.as_ptr() as usize;
    let cur = base + chunk.offset;
    let aligned = align_up(cur, align);
    let new_offset = aligned.checked_add(size)?.checked_sub(base)?;
    if new_offset > chunk.size {
        return None;
    }
    chunk.offset = new_offset;
    NonNull::new(aligned as *mut u8)
}

/// Walk the free list, return the first node that fits with the right alignment.
fn take_from_free_list(state: &mut State, size: usize, align: usize) -> Option<NonNull<u8>> {
    let mut cursor: *mut Option<NonNull<FreeNode>> = &mut state.free;
    unsafe {
        while let Some(node) = *cursor {
            let node_ref = node.as_ref();
            let addr = node.as_ptr() as usize;
            if node_ref.size >= size && addr == align_up(addr, align) {
                *cursor = node_ref.next;
                return Some(node.cast::<u8>());
            }
            cursor = &mut (*node.as_ptr()).next;
        }
    }
    None
}
