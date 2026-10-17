//! `Utils/StableSort.cs`.
//!
//! Adapted, as the reference, from the .NET runtime (<https://github.com/dotnet/runtime>), MIT License,
//! Copyright (c) .NET Foundation and Contributors.

use super::sort_helper;
use std::cmp::Ordering;

/// The indexes of `count` elements, sorted by `compare` (which compares two indexes; the callers
/// break ties by index, which makes the order stable).
pub fn sorted_map(count: usize, compare: &mut dyn FnMut(&i32, &i32) -> Ordering) -> Vec<i32> {
    let mut map: Vec<i32> = (0..count as i32).collect();
    sort_helper::sort(&mut map, compare);
    map
}
