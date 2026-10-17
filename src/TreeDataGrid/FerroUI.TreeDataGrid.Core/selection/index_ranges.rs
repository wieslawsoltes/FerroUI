//! `Selection/IndexRanges.cs`: index paths as ranges of indexes per parent path.

use super::IndexRange;
use crate::IndexPath;
use std::collections::BTreeMap;

/// A set of index paths, kept as sorted ranges of child indexes per parent path.
#[derive(Default)]
pub struct IndexRanges {
    ranges: BTreeMap<IndexPath, Vec<IndexRange>>,
    count: i32,
}

impl IndexRanges {
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of paths.
    pub fn count(&self) -> i32 {
        self.count
    }

    /// The ranges per parent path.
    pub fn ranges(&self) -> &BTreeMap<IndexPath, Vec<IndexRange>> {
        &self.ranges
    }

    /// The path at `index`, in order of parent paths.
    ///
    /// # Panics
    /// When `index` is not below [`count`](Self::count) (`IndexOutOfRangeException`).
    pub fn get(&self, mut index: i32) -> IndexPath {
        for (parent, ranges) in &self.ranges {
            let count = IndexRange::get_count(ranges);
            if index < count {
                return parent.append(IndexRange::get_at(ranges, index));
            }
            index -= count;
        }
        panic!("Index was outside the bounds of the array.");
    }

    /// Adds a path.
    pub fn add(&mut self, index: &IndexPath) {
        let parent = index.parent();
        let ranges = self.ranges.entry(parent).or_default();
        self.count += IndexRange::add(ranges, IndexRange::from_index(index.last()), None);
    }

    /// Adds a range of children of `parent`.
    pub fn add_range(&mut self, parent: &IndexPath, range: IndexRange) {
        let ranges = self.ranges.entry(parent.clone()).or_default();
        self.count += IndexRange::add(ranges, range, None);
    }

    /// Adds ranges of children of `parent`. As the reference: a parent that has no ranges takes the
    /// list; one that has ranges adds them to the given list, which is not stored.
    pub fn add_ranges(&mut self, parent: &IndexPath, mut ranges: Vec<IndexRange>) {
        match self.ranges.get(parent) {
            None => {
                self.count += IndexRange::get_count(&ranges);
                self.ranges.insert(parent.clone(), ranges);
            }
            Some(existing) => {
                self.count += IndexRange::add_ranges(&mut ranges, existing, None);
            }
        }
    }

    /// Removes a path; false when it is not in the set.
    pub fn remove(&mut self, index: &IndexPath) -> bool {
        let parent = index.parent();
        if let Some(ranges) = self.ranges.get_mut(&parent) {
            if IndexRange::remove(Some(ranges), IndexRange::from_index(index.last()), None) > 0 {
                self.count -= 1;
                return true;
            }
        }
        false
    }

    /// Removes a range of children of `parent`.
    pub fn remove_range(&mut self, parent: &IndexPath, range: IndexRange) -> bool {
        if let Some(existing) = self.ranges.get_mut(parent) {
            let removed = IndexRange::remove(Some(existing), range, None);
            self.count -= removed;
            return removed > 0;
        }
        false
    }

    /// Removes ranges of children of `parent`.
    pub fn remove_ranges(&mut self, parent: &IndexPath, ranges: &[IndexRange]) -> bool {
        if let Some(existing) = self.ranges.get_mut(parent) {
            let removed = IndexRange::remove_ranges(existing, ranges, None);
            self.count -= removed;
            return removed > 0;
        }
        false
    }

    pub fn contains(&self, index: &IndexPath) -> bool {
        let parent = index.parent();
        match self.ranges.get(&parent) {
            Some(ranges) => IndexRange::contains_in(Some(ranges), index.last()),
            None => false,
        }
    }

    /// Whether a parent path of the set is `index` or a descendant of it.
    pub fn contains_descendents(&self, index: &IndexPath) -> bool {
        self.ranges.keys().any(|i| index == i || index.is_ancestor_of(i))
    }

    /// The paths, in order of parent paths.
    pub fn iter(&self) -> impl Iterator<Item = IndexPath> + '_ {
        self.ranges.iter().flat_map(|(parent, ranges)| {
            let count = IndexRange::get_count(ranges);
            (0..count).map(move |i| parent.append(IndexRange::get_at(ranges, i)))
        })
    }

    pub fn to_vec(&self) -> Vec<IndexPath> {
        self.iter().collect()
    }
}
