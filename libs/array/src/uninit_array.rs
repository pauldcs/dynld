use core::mem::MaybeUninit;

/// A trait for fixed-size arrays.
///
/// Note that, in the context of this trait, "fixed size" does not mean that the size must be
/// known at compile-time. Simply that the size cannot change after creation.
///
/// # Safety
///
/// The functions returned by this trait must coherent with one another.
///
/// If a function has a shared reference to the array, then accessing up to `len` elements of the
/// array must be safe.
///
/// If a function has a mutable reference to the array, then accessing up to `len` elements of the
/// array must be safe, and modifying up to `len` elements of the array must be safe.
pub unsafe trait UninitArray {
    /// The type of item stored in the array.
    type Item;

    /// Returns the length of the array.
    fn len(&self) -> usize;

    /// Returns a shared reference to the item at the given index.
    fn as_ptr(&self) -> *const Self::Item;

    /// Returns a mutable reference to the item at the given index.
    fn as_mut_ptr(&mut self) -> *mut Self::Item;
}

unsafe impl<const N: usize, T> UninitArray for [MaybeUninit<T>; N] {
    type Item = T;

    #[inline]
    fn len(&self) -> usize {
        N
    }

    #[inline]
    fn as_ptr(&self) -> *const Self::Item {
        <[_]>::as_ptr(self) as *mut Self::Item
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut Self::Item {
        <[_]>::as_mut_ptr(self) as *mut Self::Item
    }
}

unsafe impl<T> UninitArray for [MaybeUninit<T>] {
    type Item = T;

    #[inline]
    fn len(&self) -> usize {
        <[_]>::len(self)
    }

    #[inline]
    fn as_ptr(&self) -> *const Self::Item {
        <[_]>::as_ptr(self) as *mut Self::Item
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut Self::Item {
        <[_]>::as_mut_ptr(self) as *mut Self::Item
    }
}

unsafe impl<'a, A: ?Sized + UninitArray> UninitArray for &'a mut A {
    type Item = A::Item;

    #[inline]
    fn len(&self) -> usize {
        (**self).len()
    }

    #[inline]
    fn as_ptr(&self) -> *const Self::Item {
        (**self).as_ptr()
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut Self::Item {
        (**self).as_mut_ptr()
    }
}
