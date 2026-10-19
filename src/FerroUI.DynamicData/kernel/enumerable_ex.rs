//! `EnumerableEx` of the kernel (`Kernel/EnumerableEx.cs`): helpers of sequences.
//!
//! `AsArray` and `AsList` are `Vec` and `collect` in Rust; the internal `EmptyIfNull`,
//! `EnumerateOne`, `ForEach` and `ToHashSet` are `unwrap_or_default`, `std::iter::once`,
//! `for_each` and `collect::<HashSet<_>>`.

use super::ItemWithIndex;
use std::collections::HashMap;
use std::hash::Hash;

/// The items whose value by `value_selector` is that of another item, group by group in the order
/// of the first item of each group (`Duplicates`).
pub fn duplicates<T, V: Eq + Hash>(source: impl IntoIterator<Item = T>, value_selector: impl Fn(&T) -> V) -> Vec<T> {
    let mut groups: Vec<Vec<T>> = Vec::new();
    let mut group_of_value: HashMap<V, usize> = HashMap::new();
    for item in source {
        let value = value_selector(&item);
        match group_of_value.get(&value) {
            Some(&group) => groups[group].push(item),
            None => {
                group_of_value.insert(value, groups.len());
                groups.push(vec![item]);
            }
        }
    }
    groups.into_iter().filter(|group| group.len() > 1).flatten().collect()
}

/// The items of `items_to_find` that are in `source`, with their indexes there, in the order of
/// `items_to_find` (`IndexOfMany(IEnumerable<T>, IEnumerable<T>)`).
pub fn index_of_many<T: PartialEq + Clone>(source: &[T], items_to_find: &[T]) -> Vec<ItemWithIndex<T>> {
    index_of_many_with(source, items_to_find, |item, index| ItemWithIndex::new(item.clone(), index))
}

/// [`index_of_many`] with the result that `result_selector` makes of each item and its index
/// (`IndexOfMany(.., Func<TObject, int, TResult>)`).
pub fn index_of_many_with<T: PartialEq, R>(source: &[T], items_to_find: &[T], result_selector: impl Fn(&T, usize) -> R) -> Vec<R> {
    let mut result = Vec::new();
    for item in items_to_find {
        for (index, element) in source.iter().enumerate() {
            if element == item {
                result.push(result_selector(element, index));
            }
        }
    }
    result
}

/// The items with their indexes (`WithIndex`).
pub fn with_index<T>(source: impl IntoIterator<Item = T>) -> Vec<ItemWithIndex<T>> {
    source.into_iter().enumerate().map(|(index, item)| ItemWithIndex::new(item, index)).collect()
}
