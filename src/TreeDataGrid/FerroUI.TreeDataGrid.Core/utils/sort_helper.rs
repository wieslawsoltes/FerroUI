//! `Utils/SortHelper.cs`: the introspective sort and the binary search of the .NET runtime, so that
//! rows are ordered as the reference orders them (the sort is not stable; the stable sort of
//! [`super::stable_sort`] breaks ties by index).
//!
//! Adapted, as the reference, from the .NET runtime (<https://github.com/dotnet/runtime>), MIT License,
//! Copyright (c) .NET Foundation and Contributors.

use std::cmp::Ordering;

/// Sorts `keys` with `comparer` (introspective sort: quicksort with a median-of-three pivot,
/// insertion sort for 16 items or fewer, heapsort past the depth limit).
pub fn sort<T: Clone>(keys: &mut [T], comparer: &mut dyn FnMut(&T, &T) -> Ordering) {
    if keys.len() > 1 {
        let depth_limit = 2 * ((keys.len() as u32).ilog2() as i32 + 1);
        intro_sort(keys, depth_limit, comparer);
    }
}

/// The index of `value` in the sorted `items`, or the bitwise complement of the index of the
/// first larger item.
pub fn binary_search<T>(items: &[T], value: &T, compare: &mut dyn FnMut(&T, &T) -> Ordering) -> i32 {
    binary_search_range(items, 0, items.len(), value, compare)
}

/// [`binary_search`] within `length` items from `index`.
pub fn binary_search_range<T>(
    items: &[T],
    index: usize,
    length: usize,
    value: &T,
    compare: &mut dyn FnMut(&T, &T) -> Ordering,
) -> i32 {
    let mut lo = index as i32;
    let mut hi = (index + length) as i32 - 1;
    while lo <= hi {
        let i = lo + ((hi - lo) >> 1);
        match compare(&items[i as usize], value) {
            Ordering::Equal => return i,
            Ordering::Less => lo = i + 1,
            Ordering::Greater => hi = i - 1,
        }
    }
    !lo
}

fn swap_if_greater<T>(keys: &mut [T], comparer: &mut dyn FnMut(&T, &T) -> Ordering, i: usize, j: usize) {
    debug_assert!(i != j);
    if comparer(&keys[i], &keys[j]) == Ordering::Greater {
        keys.swap(i, j);
    }
}

fn intro_sort<T: Clone>(keys: &mut [T], mut depth_limit: i32, comparer: &mut dyn FnMut(&T, &T) -> Ordering) {
    let mut partition_size = keys.len();
    while partition_size > 1 {
        if partition_size <= 16 {
            if partition_size == 2 {
                swap_if_greater(keys, comparer, 0, 1);
                return;
            }
            if partition_size == 3 {
                swap_if_greater(keys, comparer, 0, 1);
                swap_if_greater(keys, comparer, 0, 2);
                swap_if_greater(keys, comparer, 1, 2);
                return;
            }
            insertion_sort(&mut keys[..partition_size], comparer);
            return;
        }
        if depth_limit == 0 {
            heap_sort(&mut keys[..partition_size], comparer);
            return;
        }
        depth_limit -= 1;
        let p = pick_pivot_and_partition(&mut keys[..partition_size], comparer);
        // The pivot is in place: the partitions are sorted without it.
        intro_sort(&mut keys[p + 1..partition_size], depth_limit, comparer);
        partition_size = p;
    }
}

fn pick_pivot_and_partition<T: Clone>(keys: &mut [T], comparer: &mut dyn FnMut(&T, &T) -> Ordering) -> usize {
    debug_assert!(keys.len() >= 16);
    let hi = keys.len() - 1;
    // Median of three, which also partitions the three.
    let middle = hi >> 1;
    swap_if_greater(keys, comparer, 0, middle);
    swap_if_greater(keys, comparer, 0, hi);
    swap_if_greater(keys, comparer, middle, hi);

    let pivot = keys[middle].clone();
    keys.swap(middle, hi - 1);
    let (mut left, mut right) = (0usize, hi - 1);
    while left < right {
        loop {
            left += 1;
            if comparer(&keys[left], &pivot) != Ordering::Less {
                break;
            }
        }
        loop {
            right -= 1;
            if comparer(&pivot, &keys[right]) != Ordering::Less {
                break;
            }
        }
        if left >= right {
            break;
        }
        keys.swap(left, right);
    }
    if left != hi - 1 {
        keys.swap(left, hi - 1);
    }
    left
}

fn heap_sort<T: Clone>(keys: &mut [T], comparer: &mut dyn FnMut(&T, &T) -> Ordering) {
    let n = keys.len();
    let mut i = n >> 1;
    while i >= 1 {
        down_heap(keys, i, n, comparer);
        i -= 1;
    }
    let mut i = n;
    while i > 1 {
        keys.swap(0, i - 1);
        down_heap(keys, 1, i - 1, comparer);
        i -= 1;
    }
}

fn down_heap<T: Clone>(keys: &mut [T], mut i: usize, n: usize, comparer: &mut dyn FnMut(&T, &T) -> Ordering) {
    let d = keys[i - 1].clone();
    while i <= n >> 1 {
        let mut child = 2 * i;
        if child < n && comparer(&keys[child - 1], &keys[child]) == Ordering::Less {
            child += 1;
        }
        if comparer(&d, &keys[child - 1]) != Ordering::Less {
            break;
        }
        keys[i - 1] = keys[child - 1].clone();
        i = child;
    }
    keys[i - 1] = d;
}

fn insertion_sort<T: Clone>(keys: &mut [T], comparer: &mut dyn FnMut(&T, &T) -> Ordering) {
    for i in 0..keys.len() - 1 {
        let t = keys[i + 1].clone();
        let mut j = i as isize;
        while j >= 0 && comparer(&t, &keys[j as usize]) == Ordering::Less {
            keys[(j + 1) as usize] = keys[j as usize].clone();
            j -= 1;
        }
        keys[(j + 1) as usize] = t;
    }
}
