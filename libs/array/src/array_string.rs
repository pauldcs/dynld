use core::mem::MaybeUninit;
use core::ops::Deref;
use core::str;

use crate::array_vec::FixedVec;
use crate::uninit_array::UninitArray;

/// A string of fixed size. It cannot grow.
pub type ArrayString<const N: usize> = FixedString<[MaybeUninit<u8>; N]>;

/// A string of fixed size, backed by a [`FixedVec`] of bytes.
///
/// The contents are always guaranteed to be valid UTF-8.
pub struct FixedString<A: ?Sized + UninitArray<Item = u8>> {
    vec: FixedVec<A>,
}

impl<const N: usize> FixedString<[MaybeUninit<u8>; N]> {
    /// Creates a new empty [`ArrayString`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            vec: FixedVec::new_array(),
        }
    }

    /// Creates a new [`ArrayString`] containing the contents of `s`.
    ///
    /// Returns `None` if `s.len() > N`.
    #[inline]
    pub fn try_from_str(s: &str) -> Option<Self> {
        let mut this = Self::new();
        this.try_push_str(s).then_some(this)
    }
}

impl<A: ?Sized + UninitArray<Item = u8>> FixedString<A> {
    /// Returns the length of the string in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns the capacity of the string in bytes.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// Returns the remaining capacity of the string in bytes.
    #[inline]
    pub fn remaining_capacity(&self) -> usize {
        self.vec.remaining_capacity()
    }

    /// Returns whether the string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Returns the string contents as a `&str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: the bytes in `vec` are always valid UTF-8.
        unsafe { str::from_utf8_unchecked(&self.vec) }
    }

    /// Tries to push a string slice onto the end. Returns `false` if there
    /// is not enough remaining capacity; in that case the string is unchanged.
    pub fn try_push_str(&mut self, s: &str) -> bool {
        if s.len() > self.remaining_capacity() {
            return false;
        }
        // SAFETY: capacity verified above. Bytes come from a `&str`, so
        // appending them preserves the UTF-8 invariant.
        for &b in s.as_bytes() {
            let _ = self.vec.try_push(b);
        }
        true
    }

    /// Clears the string.
    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear();
    }
}

impl<A: ?Sized + UninitArray<Item = u8>> Deref for FixedString<A> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}
