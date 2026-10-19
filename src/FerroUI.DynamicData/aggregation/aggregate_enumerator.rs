//! `AggregateEnumerator<T>` and `AggregateEnumerator<TObject, TKey>`
//! (`Aggregation/AggregateEnumerator.cs`): the change sets of lists and caches read as aggregate
//! change sets.

use super::{AggregateItem, AggregateType, IAggregateChangeSet};
use crate::list::ListChangeReason;
use crate::ChangeReason;
use std::rc::Rc;

/// A change set of a list as an aggregate change set (`AggregateEnumerator<T>`): an `Add` and each
/// item of an `AddRange` are added; a `Remove`, each item of a `RemoveRange` and of a `Clear` are
/// removed; a `Replace` removes the previous item and adds the current one. `Refresh` and `Moved`
/// leave the aggregation as it is.
pub(crate) struct AggregateEnumerator<T> {
    source: crate::list::ChangeSet<T>,
}

impl<T> AggregateEnumerator<T> {
    pub(crate) fn new(source: crate::list::ChangeSet<T>) -> Self {
        Self { source }
    }
}

impl<T: Clone> IAggregateChangeSet<T> for AggregateEnumerator<T> {
    fn items(&self) -> Box<dyn Iterator<Item = AggregateItem<T>> + '_> {
        Box::new(self.source.changes().iter().flat_map(|change| -> Vec<AggregateItem<T>> {
            match change.reason() {
                ListChangeReason::Add => vec![AggregateItem::new(AggregateType::Add, change.item().current().clone())],
                ListChangeReason::AddRange => change.range().iter().map(|item| AggregateItem::new(AggregateType::Add, item.clone())).collect(),
                ListChangeReason::Replace => {
                    let previous = change.item().previous().expect("a Replace has the previous item").clone();
                    vec![AggregateItem::new(AggregateType::Remove, previous), AggregateItem::new(AggregateType::Add, change.item().current().clone())]
                }
                ListChangeReason::Remove => vec![AggregateItem::new(AggregateType::Remove, change.item().current().clone())],
                ListChangeReason::RemoveRange | ListChangeReason::Clear => {
                    change.range().iter().map(|item| AggregateItem::new(AggregateType::Remove, item.clone())).collect()
                }
                ListChangeReason::Refresh | ListChangeReason::Moved => Vec::new(),
            }
        }))
    }
}

/// A change set of a cache as an aggregate change set (`AggregateEnumerator<TObject, TKey>`): an
/// `Add` is added, a `Remove` removed, an `Update` removes the previous item and adds the current
/// one. `Refresh` and `Moved` leave the aggregation as it is.
pub(crate) struct CacheAggregateEnumerator<T, K> {
    source: crate::cache::ChangeSet<T, K>,
}

impl<T, K> CacheAggregateEnumerator<T, K> {
    pub(crate) fn new(source: crate::cache::ChangeSet<T, K>) -> Self {
        Self { source }
    }
}

impl<T: Clone, K> IAggregateChangeSet<T> for CacheAggregateEnumerator<T, K> {
    fn items(&self) -> Box<dyn Iterator<Item = AggregateItem<T>> + '_> {
        Box::new(self.source.changes().iter().flat_map(|change| -> Vec<AggregateItem<T>> {
            match change.reason() {
                ChangeReason::Add => vec![AggregateItem::new(AggregateType::Add, change.current().clone())],
                ChangeReason::Update => {
                    let previous = change.previous().expect("an Update has the previous item").clone();
                    vec![AggregateItem::new(AggregateType::Remove, previous), AggregateItem::new(AggregateType::Add, change.current().clone())]
                }
                ChangeReason::Remove => vec![AggregateItem::new(AggregateType::Remove, change.current().clone())],
                _ => Vec::new(),
            }
        }))
    }
}

/// A change set of a list as a shared aggregate change set.
pub(crate) fn list_aggregate<T: Clone + 'static>(changes: crate::list::ChangeSet<T>) -> Rc<dyn IAggregateChangeSet<T>> {
    Rc::new(AggregateEnumerator::new(changes))
}

/// A change set of a cache as a shared aggregate change set.
pub(crate) fn cache_aggregate<T: Clone + 'static, K: 'static>(changes: crate::cache::ChangeSet<T, K>) -> Rc<dyn IAggregateChangeSet<T>> {
    Rc::new(CacheAggregateEnumerator::new(changes))
}
