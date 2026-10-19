//! `EditDiffChangeSetOptional<TObject, TKey>` (`Cache/Internal/EditDiffChangeSetOptional.cs`): the
//! changes of a cache that holds the value of the latest option of a sequence.

use crate::cache::{Change, ChangeReason, ChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// `ValueContainer`: the value and its key.
#[derive(Clone)]
struct ValueContainer<T, K> {
    object: T,
    key: K,
}

/// `EditDiffChangeSetOptional.Run`.
pub(crate) fn edit_diff_change_set_optional<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<Option<T>>>,
    key_selector: Rc<dyn Fn(&T) -> K>,
    equality_comparer: Rc<dyn Fn(&T, &T) -> bool>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let previous: Rc<RefCell<Option<ValueContainer<T, K>>>> = Rc::new(RefCell::new(None));
        let (key_selector, equality_comparer) = (key_selector.clone(), equality_comparer.clone());
        let (on_next, on_error, on_completed) = (observer.clone(), observer.clone(), observer);
        source.subscribe_all(
            move |next_value: Option<T>| {
                let current = next_value.map(|value| {
                    let key = key_selector(&value);
                    ValueContainer { object: value, key }
                });
                let changes: Vec<Change<T, K>> = match (previous.borrow().clone(), current.clone()) {
                    (Some(prev), Some(curr)) => create_update_changes(prev, curr, &*equality_comparer),
                    (None, Some(curr)) => vec![Change::new(ChangeReason::Add, curr.key, curr.object)],
                    (Some(prev), None) => vec![Change::new(ChangeReason::Remove, prev.key, prev.object)],
                    (None, None) => Vec::new(),
                };
                *previous.borrow_mut() = current;
                if !changes.is_empty() {
                    on_next.on_next(ChangeSet::from(changes));
                }
            },
            move |error| on_error.on_error(error),
            move || on_completed.on_completed(),
        )
    })
}

/// `CreateUpdateChanges`: an update for the same key unless the values are equal, else a removal
/// and an addition.
fn create_update_changes<T, K: PartialEq>(prev: ValueContainer<T, K>, curr: ValueContainer<T, K>, equality_comparer: &dyn Fn(&T, &T) -> bool) -> Vec<Change<T, K>> {
    if prev.key == curr.key {
        if !equality_comparer(&prev.object, &curr.object) {
            return vec![Change::update(curr.key, curr.object, prev.object)];
        }
        return Vec::new();
    }
    vec![Change::new(ChangeReason::Remove, prev.key, prev.object), Change::new(ChangeReason::Add, curr.key, curr.object)]
}
