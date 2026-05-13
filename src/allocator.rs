//! this was prompted from claude i will one day make a proper one

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

/// Minimum reusable allocation size: a freed block must be at least large
/// enough to hold a `FreeNode` written into it. All small allocations are
/// rounded up to this size so every dealloc can be tracked.
pub const MIN_REUSE: usize = mem::size_of::<FreeNode>();

/// Minimum alignment for any allocation we manage. Freed blocks have a
/// `FreeNode` written into them, so every block must be aligned for one.
pub const MIN_ALIGN: usize = mem::align_of::<FreeNode>();

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
    /// Free list head (first-fit search, with splitting on take).
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

    /// Normalize a `Layout` to the (size, align) pair we actually allocate.
    ///
    /// - Size is rounded up to at least `MIN_REUSE` *and* to a multiple of
    ///   `MIN_ALIGN`. The first guarantees freed blocks can hold a `FreeNode`;
    ///   the second means every block boundary is `FreeNode`-aligned, which
    ///   makes the splitting logic in `take_from_free_list` trivially sound.
    /// - Align is bumped to at least `MIN_ALIGN` so freed pointers are always
    ///   `FreeNode`-aligned when we write a node into them on dealloc.
    fn normalize(layout: Layout) -> (usize, usize) {
        let raw = layout.size().max(MIN_REUSE);
        let size = align_up(raw, MIN_ALIGN);
        let align = layout.align().max(MIN_ALIGN);
        (size, align)
    }

    fn alloc_impl(&self, layout: Layout) -> *mut u8 {
        let (size, align) = Self::normalize(layout);

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

            // Current chunk (if any) can't satisfy the request. Grab a new one.
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
        let Some(nn) = NonNull::new(ptr) else { return };
        let (size, _align) = Self::normalize(layout);

        if size >= LARGE_THRESHOLD {
            let _ = unsafe { self.backend.dealloc(nn, size) };
            return;
        }

        self.with_state(|state, _| {
            // SAFETY: every allocation we hand out is at least `MIN_REUSE`
            // bytes and aligned to at least `MIN_ALIGN`, so a `FreeNode`
            // fits and is properly aligned here.
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

/// Walk the free list and return the first node that fits with the right
/// alignment. If the node has enough leftover room to hold another `FreeNode`,
/// split it: hand back the head, return the tail to the list.
fn take_from_free_list(state: &mut State, size: usize, align: usize) -> Option<NonNull<u8>> {
    let mut cursor: *mut Option<NonNull<FreeNode>> = &mut state.free;
    unsafe {
        while let Some(node) = *cursor {
            let node_size = node.as_ref().size;
            let node_next = node.as_ref().next;
            let addr = node.as_ptr() as usize;

            // Aligned, since every block we hand out is `MIN_ALIGN`-aligned
            // and any user-requested alignment is also bumped to a power of
            // two ≥ MIN_ALIGN. But double-check rather than assume.
            if node_size >= size && addr % align == 0 {
                // Unlink this node from the list.
                *cursor = node_next;

                // If the tail has room for another FreeNode, split it.
                //
                // Alignment is automatic: `addr` is MIN_ALIGN-aligned because
                // every pointer we hand out is, and `size` is a multiple of
                // MIN_ALIGN because `normalize` rounds it up. So `addr + size`
                // is MIN_ALIGN-aligned and fit for a FreeNode write.
                let leftover = node_size - size;
                if leftover >= MIN_REUSE {
                    let tail_addr = addr + size;
                    // SAFETY: tail_addr is within the original block, is
                    // MIN_ALIGN-aligned (see above), and has `leftover` bytes
                    // available with leftover >= MIN_REUSE = size_of::<FreeNode>().
                    let tail = tail_addr as *mut FreeNode;
                    tail.write(FreeNode {
                        next: state.free,
                        size: leftover,
                    });
                    state.free = Some(NonNull::new_unchecked(tail));
                }

                return Some(node.cast::<u8>());
            }

            cursor = &mut (*node.as_ptr()).next;
        }
    }
    None
}
