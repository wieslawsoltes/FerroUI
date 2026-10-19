//! `SortExtensions` (`Cache/Internal/SortExtensions.cs`): the positions of items in sorted lists,
//! and the binary searches of .NET that they use (`List<T>.BinarySearch` and
//! `ListEx.BinarySearch`).
//!
//! The searches return .NET's result: the index of an item that compares as equal, or the bitwise
//! complement of the index where the item would be inserted.

use crate::list::SortException;
use std::cmp::Ordering;

/// A binary search of `value` in `list` (`ListEx.BinarySearch(IList<TItem>, TItem, IComparer)`):
/// the index of an item that compares as equal, or the complement of the insert position.
pub(crate) fn binary_search<I>(list: &[I], value: &I, comparer: &dyn Fn(&I, &I) -> Ordering) -> isize {
    let mut lower: isize = 0;
    let mut upper: isize = list.len() as isize - 1;
    while lower <= upper {
        let middle = lower + ((upper - lower) / 2);
        match comparer(value, &list[middle as usize]) {
            Ordering::Less => upper = middle - 1,
            Ordering::Greater => lower = middle + 1,
            Ordering::Equal => return middle,
        }
    }
    !lower
}

/// A binary search of `value` in the `count` items of `list` from `index`
/// (`List<T>.BinarySearch(int index, int count, T item, IComparer<T>)`): the index in `list` of an
/// item that compares as equal, or the complement of the insert position.
pub(crate) fn binary_search_range<I>(list: &[I], index: usize, count: usize, value: &I, comparer: &dyn Fn(&I, &I) -> Ordering) -> isize {
    let mut lower = index as isize;
    let mut upper = index as isize + count as isize - 1;
    while lower <= upper {
        let middle = lower + ((upper - lower) >> 1);
        match comparer(&list[middle as usize], value) {
            Ordering::Equal => return middle,
            Ordering::Less => lower = middle + 1,
            Ordering::Greater => upper = middle - 1,
        }
    }
    !lower
}

/// The index of `item` in `source` (`GetCurrentPosition`): by a binary search with `comparer`
/// when `use_binary_search` is set, else by `equals`.
///
/// Deviation (DEVIATIONS.md, DynamicData): the equality of `IndexOf` is a parameter, as items
/// need no `PartialEq`, and the message of the error names the type of the item but not the item.
pub(crate) fn get_current_position<I>(
    source: &[I],
    item: &I,
    comparer: &dyn Fn(&I, &I) -> Ordering,
    use_binary_search: bool,
    equals: &dyn Fn(&I, &I) -> bool,
) -> Result<usize, SortException> {
    let index = if use_binary_search {
        binary_search(source, item, comparer)
    } else {
        source.iter().position(|candidate| equals(item, candidate)).map_or(-1, |index| index as isize)
    };
    if index < 0 {
        return Err(SortException::new(format!("Cannot find item: {} from {} items", std::any::type_name::<I>(), source.len())));
    }
    Ok(index as usize)
}

/// The position at which `item` is inserted in `source` (`GetInsertPosition`): by a binary search
/// when `use_binary_search` is set, else by a linear one.
pub(crate) fn get_insert_position<I>(source: &[I], item: &I, comparer: &dyn Fn(&I, &I) -> Ordering, use_binary_search: bool) -> usize {
    if use_binary_search {
        get_insert_position_binary(source, item, comparer)
    } else {
        get_insert_position_linear(source, item, comparer)
    }
}

/// The insert position of `item` by a binary search (`GetInsertPositionBinary`). When the list
/// holds an item that compares as equal, which a sort without uniqueness allows, the linear search
/// decides (upstream's `SortException` after it cannot happen, as the linear search always finds
/// a position).
pub(crate) fn get_insert_position_binary<I>(list: &[I], item: &I, comparer: &dyn Fn(&I, &I) -> Ordering) -> usize {
    let index = binary_search(list, item, comparer);
    let insert_index = !index;
    if insert_index < 0 {
        return get_insert_position_linear(list, item, comparer);
    }
    insert_index as usize
}

/// The insert position of `item` by a linear search (`GetInsertPositionLinear`): before the first
/// item that is greater, else at the end.
pub(crate) fn get_insert_position_linear<I>(list: &[I], item: &I, comparer: &dyn Fn(&I, &I) -> Ordering) -> usize {
    list.iter().position(|candidate| comparer(item, candidate) == Ordering::Less).unwrap_or(list.len())
}

