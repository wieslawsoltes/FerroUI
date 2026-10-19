//! `MaxEx` (`Aggregation/MaxEx.cs`): the running maximum and minimum of the values of the items.
//!
//! The operators are methods of [`AggregationCacheEx`](super::AggregationCacheEx) and
//! [`AggregationListEx`](super::AggregationListEx) (`maximum`, `maximum_with`, `minimum`,
//! `minimum_with`). Upstream's `TResult : struct, IComparable<TResult>` is `R: Copy + PartialOrd`,
//! compared as `Comparer<TResult>.Default` compares (a NaN below every other value, equal to a
//! NaN); `emptyValue`, which defaults to `default(TResult)`, is the argument of the `_with` forms.

use super::{AggregateChangeSets, AggregateType};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::cmp::Ordering;
use std::rc::Rc;

/// Which extreme is computed (`MaxOrMin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MaxOrMin {
    Max,
    Min,
}

/// The aggregate change set of each change set with the items of the collection after it
/// (`ChangesAndCollection<T>`).
pub(crate) type ChangesAndCollection<T> = (Rc<dyn super::IAggregateChangeSet<T>>, Vec<T>);

/// `CompareTo` of `IComparable<TResult>`: the order of the values, a value unordered with itself
/// (a NaN) below every other value and equal to another such value.
fn compare<R: PartialOrd>(left: &R, right: &R) -> Ordering {
    left.partial_cmp(right).unwrap_or_else(|| {
        let left_unordered = left.partial_cmp(left).is_none();
        let right_unordered = right.partial_cmp(right).is_none();
        match (left_unordered, right_unordered) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    })
}

/// `Enumerable.Max` or `Enumerable.Min` of the values of `collection` (not empty) with the default
/// comparer: the first value is kept on ties.
fn extreme<T, R: Copy + PartialOrd>(collection: &[T], value_selector: &dyn Fn(&T) -> R, max_or_min: MaxOrMin) -> Option<R> {
    let mut values = collection.iter().map(value_selector);
    let first = values.next()?;
    Some(values.fold(first, |current, value| {
        let replaces = match max_or_min {
            MaxOrMin::Max => compare(&value, &current) == Ordering::Greater,
            MaxOrMin::Min => compare(&value, &current) == Ordering::Less,
        };
        if replaces {
            value
        } else {
            current
        }
    }))
}

/// `Calculate`: the extreme kept from the added values; when a removed value is the extreme, it is
/// looked up again in the collection. Each change set gives the extreme, `empty_value` when there
/// are no items, without repeating a value.
pub(crate) fn calculate<T: 'static, R: Copy + PartialOrd + 'static>(
    source: Rc<dyn IObservable<ChangesAndCollection<T>>>,
    value_selector: Rc<dyn Fn(&T) -> R>,
    max_or_min: MaxOrMin,
    empty_value: R,
) -> Rc<dyn IObservable<R>> {
    source
        .scan_with(None::<R>, move |state: Option<R>, (changes, collection): ChangesAndCollection<T>| {
            let mut current = state;
            let mut requires_reset = false;

            for change in changes.items() {
                let value = value_selector(change.item());
                let existing = *current.get_or_insert(value);

                if change.aggregate_type() == AggregateType::Add {
                    let replaces = match max_or_min {
                        MaxOrMin::Max => compare(&value, &existing) == Ordering::Greater,
                        MaxOrMin::Min => compare(&value, &existing) == Ordering::Less,
                    };
                    if replaces {
                        current = Some(value);
                    }
                } else {
                    // Whether the extreme has been removed; if so, it is looked up again in the
                    // collection.
                    if compare(&value, &existing) != Ordering::Equal {
                        continue;
                    }

                    requires_reset = true;
                    break;
                }
            }

            if requires_reset {
                current = extreme(&collection, &*value_selector, max_or_min);
            }

            current
        })
        .select(move |extreme: Option<R>| extreme.unwrap_or(empty_value))
        .distinct_until_changed()
}

/// `ToChangesAndCollection` of a cache: the aggregate change set and the items of each change set
/// of one shared subscription.
pub(crate) fn cache_changes_and_collection<T: Clone + 'static, K: Eq + std::hash::Hash + Clone + 'static>(
    source: Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>,
) -> Rc<dyn IObservable<ChangesAndCollection<T>>> {
    source.publish_with(|shared: Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>| {
        let changes: AggregateChangeSets<T> = shared.select(super::cache_aggregate::<T, K>);
        let data = crate::cache::ObservableCacheEx::to_collection(&shared);
        data.zip(changes, |data: Vec<T>, changes| (changes, data))
    })
}

/// `ToChangesAndCollection` of a list.
pub(crate) fn list_changes_and_collection<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<crate::list::ChangeSet<T>>>,
) -> Rc<dyn IObservable<ChangesAndCollection<T>>> {
    source.publish_with(|shared: Rc<dyn IObservable<crate::list::ChangeSet<T>>>| {
        let changes: AggregateChangeSets<T> = shared.select(super::list_aggregate::<T>);
        let data = crate::list::ObservableListEx::to_collection(&shared);
        data.zip(changes, |data: Vec<T>, changes| (changes, data))
    })
}
