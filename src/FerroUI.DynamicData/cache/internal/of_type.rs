//! `OfType<TObject, TKey, TDestination>` (`Cache/Internal/OfType.cs`).

use crate::cache::{Change, ChangeReason, ChangeSet};
use ferroui_reactive::linq::AnyObject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::rc::Rc;

/// The item as a `U`, if it is one.
fn as_destination<T: AnyObject + Clone, U: 'static>(item: &T) -> Option<Rc<U>> {
    item.clone().downcast_object::<U>().ok()
}

/// `OfType.Run`: the items that are of `U`; an update that changes whether the item is one becomes
/// an addition or a removal, and moves are left out.
///
/// Deviation (DEVIATIONS.md, DynamicData): an item is a `Rc<dyn Any>` (`AnyObject`), which is of
/// the one type it was made of; upstream's object is also of its base classes and interfaces.
pub(crate) fn of_type<T: AnyObject + Clone, K: Clone + 'static, U: 'static>(source: Rc<dyn IObservable<ChangeSet<T, K>>>, suppress_empty_change_sets: bool) -> Rc<dyn IObservable<ChangeSet<Rc<U>, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<U>, K>>>| {
        let (on_next, on_error, on_completed) = (observer.clone(), observer.clone(), observer);
        source.subscribe_all(
            move |upstream_changes: ChangeSet<T, K>| {
                let mut downstream_changes = Vec::with_capacity(upstream_changes.count());
                for change in &upstream_changes {
                    if change.reason() == ChangeReason::Moved {
                        continue;
                    }
                    let current = as_destination::<T, U>(change.current());
                    let previous = change.previous().and_then(as_destination::<T, U>);
                    let transformed = match (change.reason(), current) {
                        (ChangeReason::Update, Some(add_destination)) if previous.is_none() => Some(Change::new(ChangeReason::Add, change.key().clone(), add_destination)),
                        (ChangeReason::Update, None) => previous.map(|remove_destination| Change::new(ChangeReason::Remove, change.key().clone(), remove_destination)),
                        (reason, Some(other_destination)) => Some(Change::with_previous(reason, change.key().clone(), other_destination, previous, None, None)),
                        (_, None) => None,
                    };
                    if let Some(transformed) = transformed {
                        downstream_changes.push(transformed);
                    }
                }
                if !suppress_empty_change_sets || !downstream_changes.is_empty() {
                    on_next.on_next(ChangeSet::from(downstream_changes));
                }
            },
            move |error| on_error.on_error(error),
            move || on_completed.on_completed(),
        )
    })
}
