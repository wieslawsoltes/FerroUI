//! `MergeManyItems<TObject, TKey, TDestination>` (`Cache/Internal/MergeManyItems.cs`): as
//! `MergeMany`, with each value paired with the item whose sequence produced it.

use super::subscribe_many;
use super::ObservableSelector;
use crate::cache::ChangeSet;
use crate::kernel::ItemWithValue;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::hash::Hash;
use std::rc::Rc;

/// `MergeManyItems.Run`: the sequence of each item is subscribed while the item is in the cache,
/// and its values, errors and completion go to the observer as they come.
pub(crate) fn merge_many_items<T: Clone + 'static, K: Eq + Hash + Clone + 'static, D: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    observable_selector: ObservableSelector<T, K, D>,
) -> Rc<dyn IObservable<ItemWithValue<T, D>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ItemWithValue<T, D>>>| {
        let observable_selector = observable_selector.clone();
        subscribe_many(
            source.clone(),
            Rc::new(move |item: &T, key: &K| {
                let selected = observable_selector(item, key);
                let item = item.clone();
                selected.select(move |value| ItemWithValue::new(item.clone(), value)).subscribe_safe(observer.clone())
            }),
        )
        .subscribe_ignore()
    })
}
