//! `ReadOnlyMemory<T>`: a cheaply cloneable, sliceable, immutable view over a
//! shared buffer (the counterpart of .NET's `ReadOnlyMemory<T>`).
//!
//! Text in the text formatting subsystem is `ReadOnlyMemory<u16>` (UTF-16 code
//! units, see `media/text_formatting/mod.rs` for the index-unit decision) and
//! font tables are `ReadOnlyMemory<u8>`.

use std::fmt;
use std::rc::Rc;

/// An immutable, reference counted slice of a shared buffer.
///
/// Cloning and slicing never copy the elements; they bump one reference count.
pub struct ReadOnlyMemory<T> {
    owner: Option<Rc<[T]>>,
    start: usize,
    length: usize,
}

impl<T> ReadOnlyMemory<T> {
    /// The empty memory (C# `default` / `ReadOnlyMemory<T>.Empty`).
    pub const fn empty() -> Self {
        Self { owner: None, start: 0, length: 0 }
    }

    /// Wraps a whole shared buffer.
    pub fn new(owner: Rc<[T]>) -> Self {
        let length = owner.len();
        Self { owner: Some(owner), start: 0, length }
    }

    /// Wraps `length` elements of a shared buffer starting at `start`.
    ///
    /// Panics when the range is outside the buffer.
    pub fn with_range(owner: Rc<[T]>, start: usize, length: usize) -> Self {
        assert!(start <= owner.len() && length <= owner.len() - start, "range outside of the buffer");
        Self { owner: Some(owner), start, length }
    }

    /// Takes ownership of a vector.
    pub fn from_vec(values: Vec<T>) -> Self {
        Self::new(Rc::from(values))
    }

    /// The number of elements (C# `Length`).
    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Whether the memory has no elements (C# `IsEmpty`).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// The elements (C# `Span`).
    #[inline]
    pub fn span(&self) -> &[T] {
        match &self.owner {
            Some(owner) => &owner[self.start..self.start + self.length],
            None => &[],
        }
    }

    /// The elements from `start` to the end (C# `Slice(start)`).
    ///
    /// Panics when `start` is outside the memory.
    pub fn slice_from(&self, start: usize) -> Self {
        assert!(start <= self.length, "start outside of the memory");
        self.slice(start, self.length - start)
    }

    /// `length` elements starting at `start` (C# `Slice(start, length)`).
    ///
    /// Panics when the range is outside the memory.
    pub fn slice(&self, start: usize, length: usize) -> Self {
        assert!(start <= self.length && length <= self.length - start, "range outside of the memory");
        Self { owner: self.owner.clone(), start: self.start + start, length }
    }

    /// The offset of this memory inside its underlying buffer.
    #[inline]
    pub fn offset_in_owner(&self) -> usize {
        self.start
    }

    /// The underlying buffer, when there is one.
    #[inline]
    pub fn owner(&self) -> Option<&Rc<[T]>> {
        self.owner.as_ref()
    }

    /// Whether both memories view the same buffer (regardless of range).
    pub fn shares_owner_with(&self, other: &Self) -> bool {
        match (&self.owner, &other.owner) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T: Clone> ReadOnlyMemory<T> {
    /// Copies a slice into a new shared buffer.
    pub fn from_slice(values: &[T]) -> Self {
        Self::new(Rc::from(values))
    }

    /// Copies the elements out (C# `ToArray`).
    pub fn to_vec(&self) -> Vec<T> {
        self.span().to_vec()
    }
}

impl ReadOnlyMemory<u16> {
    /// Encodes a string as UTF-16 code units (C# `string.AsMemory()`).
    pub fn from_str(text: &str) -> Self {
        Self::from_vec(text.encode_utf16().collect())
    }

    /// Decodes the UTF-16 code units, replacing lone surrogates with U+FFFD.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(self.span())
    }
}

impl<T> Clone for ReadOnlyMemory<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { owner: self.owner.clone(), start: self.start, length: self.length }
    }
}

impl<T> Default for ReadOnlyMemory<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: fmt::Debug> fmt::Debug for ReadOnlyMemory<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.span()).finish()
    }
}

/// Element-wise equality (unlike C#, where `ReadOnlyMemory<T>.Equals` compares
/// the underlying object and range; use [`ReadOnlyMemory::shares_owner_with`]
/// for that).
impl<T: PartialEq> PartialEq for ReadOnlyMemory<T> {
    fn eq(&self, other: &Self) -> bool {
        self.span() == other.span()
    }
}

impl<T> From<Vec<T>> for ReadOnlyMemory<T> {
    fn from(values: Vec<T>) -> Self {
        Self::from_vec(values)
    }
}

impl<T> From<Rc<[T]>> for ReadOnlyMemory<T> {
    fn from(values: Rc<[T]>) -> Self {
        Self::new(values)
    }
}

impl From<&str> for ReadOnlyMemory<u16> {
    fn from(text: &str) -> Self {
        Self::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_share_the_buffer() {
        let memory = ReadOnlyMemory::from_vec(vec![1, 2, 3, 4, 5]);
        let slice = memory.slice(1, 3);
        assert_eq!(slice.span(), &[2, 3, 4]);
        assert_eq!(slice.slice_from(1).span(), &[3, 4]);
        assert_eq!(slice.offset_in_owner(), 1);
        assert!(slice.shares_owner_with(&memory));
        assert!(ReadOnlyMemory::<i32>::empty().is_empty());
    }

    #[test]
    fn utf16_round_trip() {
        let memory = ReadOnlyMemory::<u16>::from_str("a\u{1F600}b");
        assert_eq!(memory.len(), 4);
        assert_eq!(memory.to_string_lossy(), "a\u{1F600}b");
    }
}
