// Copyright (c) Six Labors.
// Licensed under the Apache License, Version 2.0.
// Ported from: https://github.com/SixLabors/Fonts/

use super::ArraySlice;

/// Provides a mapped view of an underlying slice, selecting arbitrary indices
/// from the source array.
pub struct MappedArraySlice<T> {
    data: ArraySlice<T>,
    map: ArraySlice<i32>,
}

impl<T> MappedArraySlice<T> {
    /// Creates a view of the data slice through the map slice.
    ///
    /// In a debug build, panics when the map is longer than the data.
    pub fn new(data: &ArraySlice<T>, map: &ArraySlice<i32>) -> Self {
        if cfg!(debug_assertions) && map.length() > data.length() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'map')");
        }

        Self { data: data.clone(), map: map.clone() }
    }

    /// Gets the number of items in the map.
    pub fn length(&self) -> usize {
        self.map.length()
    }

    /// Writes the specified element of the slice.
    ///
    /// Panics when the index is not less than [`length`](Self::length).
    pub fn set(&self, index: usize, value: T) {
        let mapped = self.map.get(index);
        self.data.set(mapped as usize, value);
    }
}

impl<T: Copy> MappedArraySlice<T> {
    /// Reads the specified element of the slice.
    ///
    /// Panics when the index is not less than [`length`](Self::length).
    pub fn get(&self, index: usize) -> T {
        self.data.get(self.map.get(index) as usize)
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    #[test]
    fn reads_and_writes_through_the_map() {
        let data = ArraySlice::new(vec![10, 20, 30, 40]);
        let map = ArraySlice::new(vec![3, 1]);
        let target = MappedArraySlice::new(&data, &map);

        assert_eq!(2, target.length());
        assert_eq!(40, target.get(0));
        assert_eq!(20, target.get(1));

        target.set(1, 21);
        assert_eq!(21, data.get(1));
    }
}
