//! `QueryWhenChanged<TObject, TKey, TValue>` (`Cache/Internal/QueryWhenChanged.cs`).

use super::{merge_many_values, AnonymousQuery, Cache};
use crate::cache::{ChangeSet, ICache, IQuery};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// A query of the items after each change set (`QueryWhenChanged.Run()` without a trigger).
pub(crate) fn query_when_changed<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(source: Rc<dyn IObservable<ChangeSet<T, K>>>) -> Rc<dyn IObservable<Rc<dyn IQuery<T, K>>>> {
    Observable::defer(move || {
        let cache: RefCell<Option<Cache<T, K>>> = RefCell::new(None);
        source.select(move |changes: ChangeSet<T, K>| {
            let mut cache = cache.borrow_mut();
            let cache = cache.get_or_insert_with(|| Cache::with_capacity(changes.count()));
            cache.clone_changes(&changes);
            Rc::new(AnonymousQuery::new(cache)) as Rc<dyn IQuery<T, K>>
        })
    })
}

/// A query of the items after each change set and whenever the observable that
/// `item_changed_trigger` gives for an item has a value (`QueryWhenChanged.Run()` with a trigger).
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream serializes both through a
/// `SharedDeliveryQueue`; on one thread they are merged as they come.
pub(crate) fn query_when_changed_with_trigger<T: Clone + 'static, K: Eq + Hash + Clone + 'static, V: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    item_changed_trigger: Rc<dyn Fn(&T) -> Rc<dyn IObservable<V>>>,
) -> Rc<dyn IObservable<Rc<dyn IQuery<T, K>>>> {
    Observable::create(move |observer: Rc<dyn IObserver<Rc<dyn IQuery<T, K>>>>| {
        let state = Rc::new(RefCell::new(Cache::<T, K>::new()));
        let shared = source.publish();
        let shared_source: Rc<dyn IObservable<ChangeSet<T, K>>> = shared.clone();

        let inline_change = {
            let state = state.clone();
            merge_many_values(shared_source.clone(), item_changed_trigger.clone()).select(move |_| Rc::new(AnonymousQuery::new(&state.borrow())) as Rc<dyn IQuery<T, K>>)
        };
        let source_changed = {
            let state = state.clone();
            shared_source.select(move |changes: ChangeSet<T, K>| {
                state.borrow_mut().clone_changes(&changes);
                Rc::new(AnonymousQuery::new(&state.borrow())) as Rc<dyn IQuery<T, K>>
            })
        };

        let subscription = source_changed.merge(inline_change).subscribe_safe(observer);
        let connection = shared.connect();
        Disposable::create(move || {
            subscription.dispose();
            connection.dispose();
        })
    })
}
