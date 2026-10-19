//! `OnBeingRemoved<TObject, TKey>` (`Cache/Internal/OnBeingRemoved.cs`): runs an action for each
//! item removed, and for every item left when the subscription ends.

use super::Cache;
use crate::cache::{ChangeReason, ChangeSet, ICache, IQuery};
use crate::internal::synchronize_safe;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::ObservableExt;
use ferroui_reactive::Observable;
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// `OnBeingRemoved.Run`.
pub(crate) fn on_being_removed<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    remove_action: Rc<dyn Fn(&T, &K)>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let cache = Rc::new(RefCell::new(Cache::<T, K>::new()));
        let (target, action) = (cache.clone(), remove_action.clone());
        let subscriber = synchronize_safe(source.clone())
            .do_(move |changes: &ChangeSet<T, K>| register_for_removal(changes, &target, &*action))
            .subscribe_safe(observer);
        let remove_action = remove_action.clone();
        Disposable::create(move || {
            subscriber.dispose();
            let key_values = cache.borrow().key_values();
            for (key, value) in &key_values {
                remove_action(value, key);
            }
            cache.borrow_mut().clear();
        })
    })
}

/// `RegisterForRemoval`: the action for each removal, then the change set into the cache.
fn register_for_removal<T: Clone, K: Eq + Hash + Clone>(changes: &ChangeSet<T, K>, cache: &RefCell<Cache<T, K>>, remove_action: &dyn Fn(&T, &K)) {
    for change in changes {
        if change.reason() == ChangeReason::Remove {
            remove_action(change.current(), change.key());
        }
    }
    cache.borrow_mut().clone_changes(changes);
}
