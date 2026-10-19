//! `DisposeMany<T>` (`List/Internal/DisposeMany.cs`): disposes each item when it is removed or
//! replaced, and every item left when the sequence ends or the subscription ends.

use crate::list::{clone_changes, ChangeSet, ListChangeReason};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, Observer};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// The disposal of an item, a no-op for one that is not a disposable (`(item as IDisposable)?.Dispose()`).
pub(crate) type DisposeItem<T> = Rc<dyn Fn(&T)>;

/// `DisposeMany.Run`.
pub(crate) fn dispose_many<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>, dispose: DisposeItem<T>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let dispose = dispose.clone();
        let cached_items = Rc::new(RefCell::new(Vec::<T>::new()));
        let (next_items, error_items, completed_items) = (cached_items.clone(), cached_items.clone(), cached_items.clone());
        let (next_dispose, error_dispose, completed_dispose) = (dispose.clone(), dispose.clone(), dispose.clone());
        let (next_observer, error_observer, completed_observer) = (observer.clone(), observer.clone(), observer);
        let source_subscription = source.subscribe(Observer::create_all(
            move |change_set: ChangeSet<T>| {
                next_observer.on_next(change_set.clone());
                for change in &change_set {
                    match change.reason() {
                        ListChangeReason::Clear => {
                            let items = next_items.borrow().clone();
                            for item in &items {
                                next_dispose(item);
                            }
                        }
                        ListChangeReason::Remove => next_dispose(change.item().current()),
                        ListChangeReason::RemoveRange => {
                            for item in change.range() {
                                next_dispose(item);
                            }
                        }
                        ListChangeReason::Replace => {
                            if let Some(previous) = change.item().previous() {
                                next_dispose(previous);
                            }
                        }
                        _ => {}
                    }
                }
                clone_changes(&mut *next_items.borrow_mut(), &change_set);
            },
            move |error| {
                error_observer.on_error(error);
                process_finalization(&error_items, &*error_dispose);
            },
            move || {
                completed_observer.on_completed();
                process_finalization(&completed_items, &*completed_dispose);
            },
        ));
        Disposable::create(move || {
            source_subscription.dispose();
            process_finalization(&cached_items, &*dispose);
        })
    })
}

/// `ProcessFinalization`: disposes every item left, and forgets them.
fn process_finalization<T: Clone>(cached_items: &RefCell<Vec<T>>, dispose: &dyn Fn(&T)) {
    let items = std::mem::take(&mut *cached_items.borrow_mut());
    for item in &items {
        dispose(item);
    }
}
