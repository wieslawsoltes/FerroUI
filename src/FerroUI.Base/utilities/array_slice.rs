//! `ArraySlice<T>`: a view over a shared, mutable array.
//!
//! Upstream's `ArraySlice<T>` is a `(T[] data, int start, int length)` triple
//! with reference semantics: several slices alias one array and writes through
//! one are visible through the others. The Rust counterpart shares the array
//! through `Rc<RefCell<..>>`; reads borrow the whole array once (`span()`), so
//! loops over glyphs pay one borrow, not one per element.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

/// A view over `length` elements of a shared array starting at `start`.
pub struct ArraySlice<T> {
    data: Rc<RefCell<Vec<T>>>,
    start: usize,
    length: usize,
}

impl<T> ArraySlice<T> {
    /// An empty slice (C# `ArraySlice<T>.Empty`).
    pub fn empty() -> Self {
        Self { data: Rc::new(RefCell::new(Vec::new())), start: 0, length: 0 }
    }

    /// A slice over a whole new array.
    pub fn new(data: Vec<T>) -> Self {
        let length = data.len();
        Self { data: Rc::new(RefCell::new(data)), start: 0, length }
    }

    /// A slice over part of a shared array.
    ///
    /// Panics when the range is outside the array.
    pub fn from_shared(data: Rc<RefCell<Vec<T>>>, start: usize, length: usize) -> Self {
        {
            let array = data.borrow();
            assert!(start <= array.len() && length <= array.len() - start, "range outside of the array");
        }
        Self { data, start, length }
    }

    /// The shared array this slice views.
    #[inline]
    pub fn data(&self) -> &Rc<RefCell<Vec<T>>> {
        &self.data
    }

    /// Whether the slice has no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// The offset of the slice inside the shared array.
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// The number of elements.
    #[inline]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Borrows the elements (C# `Span`).
    ///
    /// Panics when the array is mutably borrowed.
    #[inline]
    pub fn span(&self) -> Ref<'_, [T]> {
        Ref::map(self.data.borrow(), |array| &array[self.start..self.start + self.length])
    }

    /// Mutably borrows the elements (C# `Span`).
    ///
    /// Panics when the array is borrowed.
    #[inline]
    pub fn span_mut(&self) -> RefMut<'_, [T]> {
        RefMut::map(self.data.borrow_mut(), |array| &mut array[self.start..self.start + self.length])
    }

    /// Writes one element.
    #[inline]
    pub fn set(&self, index: usize, value: T) {
        self.span_mut()[index] = value;
    }

    /// A slice of the same array. As upstream, `start` is relative to the
    /// underlying array, not to this slice.
    pub fn slice(&self, start: usize, length: usize) -> Self {
        Self::from_shared(self.data.clone(), start, length)
    }

    /// The first `length` elements.
    ///
    /// Panics when `length` is larger than the slice.
    pub fn take(&self, length: usize) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        assert!(length <= self.length, "length out of range");
        Self { data: self.data.clone(), start: self.start, length }
    }

    /// Everything after the first `length` elements.
    ///
    /// Panics when `length` is larger than the slice.
    pub fn skip(&self, length: usize) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        assert!(length <= self.length, "length out of range");
        Self { data: self.data.clone(), start: self.start + length, length: self.length - length }
    }
}

impl<T: Copy> ArraySlice<T> {
    /// Reads one element.
    #[inline]
    pub fn get(&self, index: usize) -> T {
        self.span()[index]
    }

    /// Sets every element to `value`.
    pub fn fill(&self, value: T) {
        self.span_mut().fill(value);
    }
}

impl<T> Clone for ArraySlice<T> {
    fn clone(&self) -> Self {
        Self { data: self.data.clone(), start: self.start, length: self.length }
    }
}

impl<T> From<Vec<T>> for ArraySlice<T> {
    fn from(data: Vec<T>) -> Self {
        Self::new(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_alias_the_array() {
        let slice = ArraySlice::new(vec![1, 2, 3, 4, 5]);
        let tail = slice.skip(2);
        tail.set(0, 30);
        assert_eq!(&*slice.span(), &[1, 2, 30, 4, 5]);
        assert_eq!(tail.start(), 2);
        assert_eq!(tail.take(2).length(), 2);
        // `slice` takes an absolute start, as upstream.
        assert_eq!(&*tail.slice(1, 2).span(), &[2, 30]);
        slice.take(2).fill(0);
        assert_eq!(slice.get(1), 0);
    }
}
