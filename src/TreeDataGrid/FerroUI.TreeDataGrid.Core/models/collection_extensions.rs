//! `Models/CollectionExtensions.cs`.

use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use std::cmp::Ordering;

/// Whether the indexes of a change are known (`HasKnownIndexes`): a change of a collection that
/// does not report where it happened is handled as a reset.
pub fn has_known_indexes<T>(e: &NotifyCollectionChangedEventArgs<'_, T>) -> bool {
    match e.action {
        NotifyCollectionChangedAction::Add => e.new_starting_index >= 0,
        NotifyCollectionChangedAction::Remove => e.old_starting_index >= 0,
        NotifyCollectionChangedAction::Replace | NotifyCollectionChangedAction::Move => {
            e.old_starting_index >= 0 && e.new_starting_index >= 0
        }
        NotifyCollectionChangedAction::Reset => true,
    }
}

/// The index of the row whose model compares equal to `model` within `from..=to` of the sorted
/// `items` (`to` -1: the last), or the bitwise complement of the index of the first larger one.
pub fn binary_search_rows<R, M>(
    items: &[R],
    model: &M,
    model_of: impl Fn(&R) -> M,
    comparison: &mut dyn FnMut(&M, &M) -> Ordering,
    from: i32,
    to: i32,
) -> i32 {
    let to = if to == -1 { items.len() as i32 - 1 } else { to };
    let mut lo = from;
    let mut hi = to;
    while lo <= hi {
        let i = ((hi as u32 + lo as u32) >> 1) as i32;
        match comparison(model, &model_of(&items[i as usize])) {
            Ordering::Equal => return i,
            Ordering::Greater => lo = i + 1,
            Ordering::Less => hi = i - 1,
        }
    }
    !lo
}

/// Inserts `count` copies of `item` at `index` (`InsertMany`).
pub fn insert_many<T: Clone>(list: &mut Vec<T>, index: usize, item: T, count: usize) {
    list.splice(index..index, std::iter::repeat_n(item, count));
}

/// A copy of `count` items from `index` (`Slice`).
pub fn slice<T: Clone>(list: &[T], index: usize, count: usize) -> Vec<T> {
    list[index..index + count].to_vec()
}
