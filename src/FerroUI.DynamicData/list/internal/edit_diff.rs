//! `EditDiff<T>` (`List/Internal/EditDiff.cs`): the edit that makes a source list hold a new set
//! of items, with the fewest changes.

use crate::list::{IExtendedList, IList, ISourceList};

/// The distinct items of `first` that are not in `second`, by `equal` (`Enumerable.Except` with a
/// comparer).
fn except<T: Clone>(first: &[T], second: &[T], equal: &dyn Fn(&T, &T) -> bool) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    for item in first {
        if second.iter().any(|other| equal(item, other)) || result.iter().any(|other| equal(item, other)) {
            continue;
        }
        result.push(item.clone());
    }
    result
}

/// `EditDiff.Edit`: removes the items that are not among `items` and adds those that are new.
pub(crate) fn edit_diff<T: Clone + PartialEq + 'static, S: ISourceList<T> + ?Sized>(source: &S, items: Vec<T>, equality_comparer: &dyn Fn(&T, &T) -> bool) {
    let mut items = Some(items);
    source.edit_dyn(&mut |inner_list: &mut dyn IExtendedList<T>| {
        let Some(new_items) = items.take() else { return };
        let original_items: Vec<T> = (0..inner_list.count()).map(|index| inner_list.get(index)).collect();
        let removes = except(&original_items, &new_items, equality_comparer);
        let adds = except(&new_items, &original_items, equality_comparer);
        // `ListEx.Remove(IEnumerable<T>)`: each item removed by a search.
        for item in &removes {
            IList::remove(inner_list, item);
        }
        inner_list.add_range(adds);
    });
}
