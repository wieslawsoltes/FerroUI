// RichTextKit
// Copyright © 2019-2020 Topten Software. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may
// not use this product except in compliance with the License. You may obtain
// a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
// Copied from: https://github.com/toptensoftware/RichTextKit

use std::cmp::Ordering;

/// Binary searching of a list with a comparer.
///
/// Unlike `slice::binary_search_by`, which may return any of several equal
/// items, the search visits the items in a fixed order. The result is the
/// index of the found item or the bitwise complement of the index of the
/// next larger item.
pub struct BinarySearchExtension;

impl BinarySearchExtension {
    fn get_median(low: i32, hi: i32) -> i32 {
        debug_assert!(low <= hi);
        debug_assert!(hi - low >= 0, "Length overflow!");
        low + ((hi - low) >> 1)
    }

    /// Performs a binary search on the entire contents of a list.
    ///
    /// Returns the index of the found item; otherwise the bitwise complement
    /// of the index of the next larger item.
    pub fn binary_search<T>(list: &[T], value: &T, comparer: impl Fn(&T, &T) -> Ordering) -> i32 {
        Self::binary_search_range(list, 0, list.len() as i32, value, comparer)
    }

    /// Performs a binary search on a subset of a list: `length` items
    /// starting at `index`.
    ///
    /// Returns the index of the found item; otherwise the bitwise complement
    /// of the index of the next larger item.
    pub fn binary_search_range<T>(
        list: &[T],
        index: i32,
        length: i32,
        value: &T,
        comparer: impl Fn(&T, &T) -> Ordering,
    ) -> i32 {
        // Based on this: https://referencesource.microsoft.com/#mscorlib/system/array.cs,957
        let mut lo = index;
        let mut hi = index + length - 1;
        while lo <= hi {
            let i = Self::get_median(lo, hi);
            match comparer(&list[i as usize], value) {
                Ordering::Equal => return i,
                Ordering::Less => lo = i + 1,
                Ordering::Greater => hi = i - 1,
            }
        }
        !lo
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    fn compare(a: &i32, b: &i32) -> Ordering {
        a.cmp(b)
    }

    #[test]
    fn finds_an_item_or_the_complement_of_the_next_larger_one() {
        let list = [1, 3, 5, 7];
        let empty: [i32; 0] = [];

        assert_eq!(0, BinarySearchExtension::binary_search(&list, &1, compare));
        assert_eq!(3, BinarySearchExtension::binary_search(&list, &7, compare));
        assert_eq!(!0, BinarySearchExtension::binary_search(&list, &0, compare));
        assert_eq!(!2, BinarySearchExtension::binary_search(&list, &4, compare));
        assert_eq!(!4, BinarySearchExtension::binary_search(&list, &8, compare));
        assert_eq!(!0, BinarySearchExtension::binary_search(&empty, &8, compare));
    }

    #[test]
    fn searches_a_range() {
        let list = [9, 1, 3, 5, 0];

        assert_eq!(2, BinarySearchExtension::binary_search_range(&list, 1, 3, &3, compare));
        assert_eq!(!1, BinarySearchExtension::binary_search_range(&list, 1, 3, &0, compare));
        assert_eq!(!4, BinarySearchExtension::binary_search_range(&list, 1, 3, &6, compare));
    }

    #[test]
    fn the_first_item_visited_wins_among_equal_items() {
        let list = [2, 2, 2, 2, 2];
        assert_eq!(2, BinarySearchExtension::binary_search(&list, &2, compare));
    }
}
