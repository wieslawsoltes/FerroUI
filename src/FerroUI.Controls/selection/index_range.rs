// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

use std::fmt;
use std::rc::Rc;

/// An inclusive range of indexes.
///
/// The arithmetic wraps on overflow, as the managed implementation's does:
/// "select all" without a source selects the range `0..=i32::MAX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexRange {
    begin: i32,
    end: i32,
}

const INVALID: IndexRange = IndexRange { begin: i32::MIN, end: i32::MIN };

impl IndexRange {
    /// A range holding a single index.
    pub fn from_index(index: i32) -> Self {
        Self { begin: index, end: index }
    }

    pub fn new(begin: i32, end: i32) -> Self {
        // Accept out of order begin/end pairs, just swap them.
        if begin > end {
            Self { begin: end, end: begin }
        } else {
            Self { begin, end }
        }
    }

    #[inline]
    pub fn begin(&self) -> i32 {
        self.begin
    }

    #[inline]
    pub fn end(&self) -> i32 {
        self.end
    }

    #[inline]
    pub fn count(&self) -> i32 {
        self.end.wrapping_sub(self.begin).wrapping_add(1)
    }

    #[inline]
    pub fn contains(&self, index: i32) -> bool {
        index >= self.begin && index <= self.end
    }

    /// Splits the range after `split_index`: the part up to and including
    /// the index, and the part after it if there is one.
    pub fn split(&self, split_index: i32) -> (IndexRange, Option<IndexRange>) {
        let before = IndexRange::new(self.begin, split_index);

        if split_index < self.end {
            (before, Some(IndexRange::new(split_index + 1, self.end)))
        } else {
            (before, None)
        }
    }

    pub fn intersects(&self, other: IndexRange) -> bool {
        self.begin <= other.end && self.end >= other.begin
    }

    pub fn adjacent(&self, other: IndexRange) -> bool {
        self.begin == other.end.wrapping_add(1) || self.end == other.begin.wrapping_sub(1)
    }

    /// Whether any of the ranges contains the index.
    pub fn contains_in(ranges: Option<&[IndexRange]>, index: i32) -> bool {
        let Some(ranges) = ranges else {
            return false;
        };

        if index < 0 {
            return false;
        }

        ranges.iter().any(|range| range.contains(index))
    }

    /// The `index`th index of the ranges.
    pub fn get_at(ranges: &[IndexRange], index: i32) -> i32 {
        let mut current_index = 0i32;

        for range in ranges {
            let current_count = range.count();

            if index >= current_index && index < current_index.wrapping_add(current_count) {
                return range.begin + (index - current_index);
            }

            current_index = current_index.wrapping_add(current_count);
        }

        panic!("The index was out of range.");
    }

    /// Adds a range to the ranges. Returns the number of indexes added,
    /// which are recorded in `added`.
    pub fn add(ranges: &mut Vec<IndexRange>, range: IndexRange, mut added: Option<&mut Vec<IndexRange>>) -> i32 {
        let mut range = range;
        let mut result = 0i32;
        let mut i = 0;

        while i < ranges.len() && range != INVALID {
            let existing = ranges[i];

            if range.intersects(existing) || range.adjacent(existing) {
                if range.begin < existing.begin {
                    let add = IndexRange::new(range.begin, existing.begin - 1);
                    ranges[i] = IndexRange::new(range.begin, existing.end);
                    if let Some(added) = added.as_deref_mut() {
                        added.push(add);
                    }
                    result = result.wrapping_add(add.count());
                }

                range = if range.end <= existing.end { INVALID } else { IndexRange::new(existing.end + 1, range.end) };
            } else if range.end < existing.begin {
                ranges.insert(i, range);
                if let Some(added) = added.as_deref_mut() {
                    added.push(range);
                }
                result = result.wrapping_add(range.count());
                range = INVALID;
            }

            i += 1;
        }

        if range != INVALID {
            ranges.push(range);
            if let Some(added) = added.as_deref_mut() {
                added.push(range);
            }
            result = result.wrapping_add(range.count());
        }

        Self::merge_ranges(ranges);
        result
    }

    /// Adds each of the `source` ranges to `destination`.
    pub fn add_ranges(
        destination: &mut Vec<IndexRange>,
        source: &[IndexRange],
        mut added: Option<&mut Vec<IndexRange>>,
    ) -> i32 {
        let mut result = 0i32;

        for range in source {
            result = result.wrapping_add(Self::add(destination, *range, added.as_deref_mut()));
        }

        result
    }

    /// Restricts the ranges to `range`. Returns the number of indexes
    /// removed, which are recorded in `removed`.
    pub fn intersect(ranges: &mut Vec<IndexRange>, range: IndexRange, mut removed: Option<&mut Vec<IndexRange>>) -> i32 {
        let mut result = 0i32;
        let mut i = 0;

        while i < ranges.len() && range != INVALID {
            let mut existing = ranges[i];

            if existing.end < range.begin || existing.begin > range.end {
                if let Some(removed) = removed.as_deref_mut() {
                    removed.push(existing);
                }
                ranges.remove(i);
                result = result.wrapping_add(existing.count());
                continue;
            }

            if existing.begin < range.begin {
                let except = IndexRange::new(existing.begin, range.begin - 1);
                if let Some(removed) = removed.as_deref_mut() {
                    removed.push(except);
                }
                existing = IndexRange::new(range.begin, existing.end);
                ranges[i] = existing;
                result = result.wrapping_add(except.count());
            }

            if existing.end > range.end {
                let except = IndexRange::new(range.end + 1, existing.end);
                if let Some(removed) = removed.as_deref_mut() {
                    removed.push(except);
                }
                ranges[i] = IndexRange::new(existing.begin, range.end);
                result = result.wrapping_add(except.count());
            }

            i += 1;
        }

        Self::merge_ranges(ranges);

        if let Some(removed) = removed {
            Self::merge_ranges(removed);
        }

        result
    }

    /// Removes a range from the ranges. Returns the number of indexes
    /// removed, which are recorded in `removed`.
    pub fn remove(
        ranges: Option<&mut Vec<IndexRange>>,
        range: IndexRange,
        mut removed: Option<&mut Vec<IndexRange>>,
    ) -> i32 {
        let Some(ranges) = ranges else {
            return 0;
        };

        let mut result = 0i32;
        let mut i = 0;

        while i < ranges.len() {
            let existing = ranges[i];

            if range.intersects(existing) {
                if range.begin <= existing.begin && range.end >= existing.end {
                    ranges.remove(i);
                    if let Some(removed) = removed.as_deref_mut() {
                        removed.push(existing);
                    }
                    result = result.wrapping_add(existing.count());
                    continue;
                } else if range.begin > existing.begin && range.end >= existing.end {
                    ranges[i] = IndexRange::new(existing.begin, range.begin - 1);
                    if let Some(removed) = removed.as_deref_mut() {
                        removed.push(IndexRange::new(range.begin, existing.end));
                    }
                    result = result.wrapping_add(existing.end - (range.begin - 1));
                } else if range.begin > existing.begin && range.end < existing.end {
                    ranges[i] = IndexRange::new(existing.begin, range.begin - 1);
                    i += 1;
                    ranges.insert(i, IndexRange::new(range.end + 1, existing.end));
                    if let Some(removed) = removed.as_deref_mut() {
                        removed.push(range);
                    }
                    result = result.wrapping_add(range.count());
                } else if range.end <= existing.end {
                    let remove = IndexRange::new(existing.begin, range.end);
                    ranges[i] = IndexRange::new(range.end + 1, existing.end);
                    if let Some(removed) = removed.as_deref_mut() {
                        removed.push(remove);
                    }
                    result = result.wrapping_add(remove.count());
                }
            }

            i += 1;
        }

        result
    }

    /// Removes each of the `source` ranges from `destination`.
    pub fn remove_ranges(
        destination: &mut Vec<IndexRange>,
        source: &[IndexRange],
        mut added: Option<&mut Vec<IndexRange>>,
    ) -> i32 {
        let mut result = 0i32;

        for range in source {
            result = result.wrapping_add(Self::remove(Some(destination), *range, added.as_deref_mut()));
        }

        result
    }

    /// Enumerates the indexes of the ranges.
    pub fn enumerate_indices(ranges: Rc<Vec<IndexRange>>) -> IndexRangeIndices {
        IndexRangeIndices { ranges, range: 0, next: None }
    }

    /// The number of indexes in the ranges.
    pub fn get_count(ranges: &[IndexRange]) -> i32 {
        let mut result = 0i32;

        for range in ranges {
            result = result.wrapping_add(range.count());
        }

        result
    }

    fn merge_ranges(ranges: &mut Vec<IndexRange>) {
        if ranges.len() < 2 {
            return;
        }

        let mut i = ranges.len() - 2;

        loop {
            let r = ranges[i];
            let r1 = ranges[i + 1];

            if r.intersects(r1) || r.end == r1.begin.wrapping_sub(1) {
                ranges[i] = IndexRange::new(r.begin, r1.end);
                ranges.remove(i + 1);
            }

            if i == 0 {
                break;
            }

            i -= 1;
        }
    }
}

impl fmt::Display for IndexRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{}]", self.begin, self.end)
    }
}

/// The indexes of a list of ranges, in order. Works on a snapshot of the
/// ranges, so the selection may change while it is enumerated.
pub(crate) struct IndexRangeIndices {
    ranges: Rc<Vec<IndexRange>>,
    range: usize,
    next: Option<i32>,
}

impl Iterator for IndexRangeIndices {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        let range = *self.ranges.get(self.range)?;
        let index = self.next.unwrap_or(range.begin);

        if index >= range.end {
            self.range += 1;
            self.next = None;
        } else {
            self.next = Some(index + 1);
        }

        Some(index)
    }
}
