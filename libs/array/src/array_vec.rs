use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

use crate::uninit_array::UninitArray;

/// A vector of fixed size. It cannot grow.
pub type ArrayVec<T, const N: usize> = FixedVec<[MaybeUninit<T>; N]>;

/// A vector of fixed size. It cannot grow.
pub struct FixedVec<A: ?Sized + UninitArray> {
    len: u32,
    array: A,
}

impl<A: UninitArray> FixedVec<A> {
    /// Creates a new [`ArrayVec`] with the given array.
    #[inline]
    pub const fn new(array: A) -> Self {
        Self { len: 0, array }
    }

    /// Returns the inner storage of the vector.
    #[inline]
    pub fn into_inner(self) -> A {
        let array = unsafe { core::ptr::read(&self.array) };
        core::mem::forget(self);
        array
    }
}

impl<'a, T> FixedVec<&'a mut [MaybeUninit<T>]> {
    /// Returns the inner storage of the vector.
    #[inline]
    pub fn into_inner_slice(self) -> &'a mut [T] {
        let slice = self.into_inner();
        unsafe { core::slice::from_raw_parts_mut(slice.as_mut_ptr() as *mut T, slice.len()) }
    }
}

impl<const N: usize, T> FixedVec<[MaybeUninit<T>; N]> {
    /// Creates a new [`ArrayVec`] with an fixed-size array.
    #[inline]
    pub const fn new_array() -> Self {
        Self::new(unsafe { MaybeUninit::uninit().assume_init() })
    }
}

impl<A: ?Sized + UninitArray> FixedVec<A> {
    /// Returns the length of the vector.
    ///
    /// This is the number of elements that have been initialized in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns the capacity of the vector.
    ///
    /// This is the maximum number of elements that can be initialized in the vector.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.array.len()
    }

    /// Returns the remaining capacity of the vector.
    #[inline]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity() - self.len()
    }

    /// Returns whether the vector is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.remaining_capacity() == 0
    }

    /// Returns whether the vector is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<A: ?Sized + UninitArray> FixedVec<A> {
    /// Pushes an item to the end of the vector without checking whether
    /// the vector is full already.
    ///
    /// # Panics
    ///
    /// This function panics if the vector is already full.
    #[inline]
    #[track_caller]
    pub fn push(&mut self, item: A::Item) {
        match self.try_push(item) {
            Ok(()) => {}
            Err(_) => panic!("ArrayVec is full"),
        }
    }

    /// Pushes an item to the end of the vector.
    #[inline]
    pub fn try_push(&mut self, item: A::Item) -> Result<(), A::Item> {
        if self.is_full() {
            return Err(item);
        }

        unsafe {
            let ptr = self.array.as_mut_ptr().add(self.len as usize);
            ptr.write(item);
            self.len += 1;
        }

        Ok(())
    }

    /// Pops the last item in the vector, if it is not empty.
    #[inline]
    pub fn pop(&mut self) -> Option<A::Item> {
        if self.is_empty() {
            return None;
        }

        unsafe {
            self.len -= 1;
            Some(self.array.as_mut_ptr().add(self.len as usize).read())
        }
    }

    /// Clears the vector.
    #[inline]
    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

impl<A: ?Sized + UninitArray> Deref for FixedVec<A> {
    type Target = [A::Item];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.array.as_ptr(), self.len as usize) }
    }
}

impl<A: ?Sized + UninitArray> DerefMut for FixedVec<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.array.as_mut_ptr(), self.len as usize) }
    }
}

impl<A: UninitArray> Extend<A::Item> for FixedVec<A> {
    #[inline]
    fn extend<I: IntoIterator<Item = A::Item>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<A: ?Sized + UninitArray> Drop for FixedVec<A> {
    #[inline]
    fn drop(&mut self) {
        let slice: &mut [A::Item] = self;
        unsafe { core::ptr::drop_in_place(slice) }
    }
}

impl<A: UninitArray> IntoIterator for FixedVec<A> {
    type IntoIter = IntoIter<A>;
    type Item = A::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            len: self.len as usize,
            index: 0,
            array: self.into_inner(),
        }
    }
}

impl<'a, A: ?Sized + UninitArray> IntoIterator for &'a FixedVec<A> {
    type Item = &'a A::Item;
    type IntoIter = core::slice::Iter<'a, A::Item>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        <[_]>::iter(self)
    }
}

impl<'a, A: ?Sized + UninitArray> IntoIterator for &'a mut FixedVec<A> {
    type Item = &'a mut A::Item;
    type IntoIter = core::slice::IterMut<'a, A::Item>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        <[_]>::iter_mut(self)
    }
}

/// An iterator over the elements of an [`ArrayVec`].
pub struct IntoIter<A: ?Sized + UninitArray> {
    index: usize,
    len: usize,
    array: A,
}

impl<A: ?Sized + UninitArray> Iterator for IntoIter<A> {
    type Item = A::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }

        unsafe {
            let ptr = self.array.as_mut_ptr().add(self.index);
            self.index += 1;
            Some(ptr.read())
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<A: UninitArray> ExactSizeIterator for IntoIter<A> {
    #[inline]
    fn len(&self) -> usize {
        self.len - self.index
    }
}

impl<A: ?Sized + UninitArray> DoubleEndedIterator for IntoIter<A> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }

        unsafe {
            self.len -= 1;
            let ptr = self.array.as_mut_ptr().add(self.len);
            Some(ptr.read())
        }
    }
}

impl<A: ?Sized + UninitArray> Drop for IntoIter<A> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let p = self.array.as_mut_ptr().add(self.index);
            let len = self.len - self.index;
            let slice = core::slice::from_raw_parts_mut(p, len);
            core::ptr::drop_in_place(slice);
        }
    }
}
