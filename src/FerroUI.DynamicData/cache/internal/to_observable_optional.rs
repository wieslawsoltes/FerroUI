//! `ToObservableOptional<TObject, TKey>` (`Cache/Internal/ToObservableOptional.cs`).

use crate::cache::{ChangeReason, ChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// `ToObservableOptional.Run`: the item of `key` as an option each time it changes; equal
/// successive values (by `equality_comparer`) are left out.
pub(crate) fn to_observable_optional<T: Clone + 'static, K: Eq + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    key: K,
    equality_comparer: Rc<dyn Fn(&T, &T) -> bool>,
) -> Rc<dyn IObservable<Option<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<Option<T>>>| {
        let last_value: RefCell<Option<T>> = RefCell::new(None);
        let (key, equality_comparer) = (key.clone(), equality_comparer.clone());
        let (on_next, on_error, on_completed) = (observer.clone(), observer.clone(), observer);
        source.subscribe_all(
            move |changes: ChangeSet<T, K>| {
                for change in &changes {
                    if *change.key() != key {
                        continue;
                    }
                    let emit_value = match change.reason() {
                        ChangeReason::Remove => None,
                        _ => Some(change.current().clone()),
                    };
                    let should_emit = {
                        let last_value = last_value.borrow();
                        match (&emit_value, &*last_value) {
                            (Some(emit), Some(last)) => !equality_comparer(last, emit),
                            (None, None) => false,
                            _ => true,
                        }
                    };
                    if should_emit {
                        *last_value.borrow_mut() = emit_value.clone();
                        on_next.on_next(emit_value);
                    }
                }
            },
            move |error| on_error.on_error(error),
            move || on_completed.on_completed(),
        )
    })
}

/// `ToObservableOptional(.., bool initialOptionalWhenMissing, ..)` with `true`: a `None` first when
/// the first change set has no item of the key.
pub(crate) fn to_observable_optional_with_initial<T: Clone + 'static, K: Eq + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    key: K,
    equality_comparer: Rc<dyn Fn(&T, &T) -> bool>,
) -> Rc<dyn IObservable<Option<T>>> {
    Observable::defer(move || {
        let is_first_notification = std::cell::Cell::new(true);
        to_observable_optional(source.clone(), key.clone(), equality_comparer.clone())
            .select(|value: Option<T>| (value, false))
            .merge(Observable::return_((None, true)))
            .where_(move |notification: &(Option<T>, bool)| {
                // Decided after the merge serializes both inputs; updating before the delivery also
                // handles a notification that comes back in.
                let should_emit = !notification.1 || is_first_notification.get();
                is_first_notification.set(false);
                should_emit
            })
            .select(|notification: (Option<T>, bool)| notification.0)
    })
}
