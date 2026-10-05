//! `ArrayBuilder<T>`: a growable buffer that can be cleared and reused without
//! giving its storage back (the pooled scratch buffer of the text formatter).

/// A simple growable buffer.
///
/// Upstream hands out `ArraySlice`s aliasing the builder's array; here the
/// builder owns a `Vec<T>` and ranges are borrowed as plain slices.
#[derive(Debug, Default)]
pub struct ArrayBuilder<T> {
    data: Vec<T>,
}

impl<T> ArrayBuilder<T> {
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// The number of items.
    #[inline]
    pub fn length(&self) -> usize {
        self.data.len()
    }

    /// The allocated capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Appends one item.
    #[inline]
    pub fn add_item(&mut self, value: T) {
        self.data.push(value);
    }

    /// Removes every item, keeping the storage.
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Drops the storage (C# `arrayBuilder = default`).
    pub fn reset(&mut self) {
        self.data = Vec::new();
    }

    /// The items.
    #[inline]
    pub fn as_span(&self) -> &[T] {
        &self.data
    }

    /// The items, mutably.
    #[inline]
    pub fn as_span_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// `length` items starting at `start`.
    #[inline]
    pub fn as_slice(&self, start: usize, length: usize) -> &[T] {
        &self.data[start..start + length]
    }

    /// `length` items starting at `start`, mutably.
    #[inline]
    pub fn as_slice_mut(&mut self, start: usize, length: usize) -> &mut [T] {
        &mut self.data[start..start + length]
    }
}

impl<T: Clone + Default> ArrayBuilder<T> {
    /// Sets the number of items; new items are default values.
    pub fn set_length(&mut self, value: usize) {
        self.data.resize(value, T::default());
    }

    /// Appends `length` default items and returns them.
    pub fn add(&mut self, length: usize) -> &mut [T] {
        let position = self.data.len();
        self.data.resize(position + length, T::default());
        &mut self.data[position..]
    }

    /// Appends a copy of `value` and returns the appended range.
    pub fn add_slice(&mut self, value: &[T]) -> &mut [T] {
        let position = self.data.len();
        self.data.extend_from_slice(value);
        &mut self.data[position..]
    }
}

impl<T> std::ops::Index<usize> for ArrayBuilder<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &T {
        &self.data[index]
    }
}

impl<T> std::ops::IndexMut<usize> for ArrayBuilder<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.data[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_reuse() {
        let mut builder = ArrayBuilder::<i32>::new();
        builder.add_item(1);
        builder.add(2).fill(7);
        builder.add_slice(&[9]);
        assert_eq!(builder.as_span(), &[1, 7, 7, 9]);
        let capacity = builder.capacity();
        builder.clear();
        assert_eq!(builder.length(), 0);
        assert_eq!(builder.capacity(), capacity);
        builder.set_length(2);
        builder[1] = 5;
        assert_eq!(builder.as_slice(1, 1), &[5]);
    }
}
