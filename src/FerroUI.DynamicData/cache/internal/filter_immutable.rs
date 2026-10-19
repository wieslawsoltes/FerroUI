//! `FilterImmutable<TObject, TKey>` (`Cache/Internal/FilterImmutable.cs`).

use crate::cache::{Change, ChangeReason, ChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::hash::Hash;
use std::rc::Rc;

/// The changes of the items that `predicate` accepts, for items that never change: nothing is
/// cached, so an update is evaluated by its previous and its current item, a refresh is not
/// evaluated again, moves are dropped and indexes are not propagated (`FilterImmutable.Run`).
///
/// `predicate` is consumer code that may fail: a failure ends the sequence with its error, as
/// upstream catches the exception and forwards it.
pub(crate) fn filter_immutable<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    predicate: Rc<dyn Fn(&T) -> Result<bool, ObservableError>>,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let predicate = predicate.clone();
        let (next, error, completed) = (observer.clone(), observer.clone(), observer);
        source.subscribe_all(
            move |upstream_changes: ChangeSet<T, K>| match filter_changes(&upstream_changes, &*predicate) {
                Ok(downstream_changes) => {
                    if !suppress_empty_change_sets || downstream_changes.count() != 0 {
                        next.on_next(downstream_changes);
                    }
                }
                // We're invoking "untrusted" consumer code, (I.E. _predicate), so catch and
                // propagate any errors.
                Err(e) => next.on_error(e),
            },
            move |e| error.on_error(e),
            move || completed.on_completed(),
        )
    })
}

fn filter_changes<T: Clone, K: Clone>(upstream_changes: &ChangeSet<T, K>, predicate: &dyn Fn(&T) -> Result<bool, ObservableError>) -> Result<ChangeSet<T, K>, ObservableError> {
    let mut downstream_changes = Vec::with_capacity(upstream_changes.count());
    for change in upstream_changes {
        // Propagate changes as follows:
        //      Don't propagate moves at all, since we don't preserve indexes.
        //      For Updates, propagate...
        //          ...an Add if the new item matches the predicate, but the old one doesn't
        //          ...a Remove if the old item matches the predicate, but the new one doesn't
        //          ...an Update if both items match the predicate
        //          ...nothing if neither items match the predicate
        //      For all other changes, propagate only if the value matches the predicate
        if change.reason() == ChangeReason::Moved {
            continue;
        }
        let downstream_reason = match change.reason() {
            ChangeReason::Update => {
                let previous = change.previous().expect("an update has a previous item");
                match (predicate(previous)?, predicate(change.current())?) {
                    (false, true) => Some(ChangeReason::Add),
                    (true, false) => Some(ChangeReason::Remove),
                    (true, true) => Some(ChangeReason::Update),
                    (false, false) => None,
                }
            }
            reason => predicate(change.current())?.then_some(reason),
        };
        if let Some(reason) = downstream_reason {
            // Do not propagate indexes, we can't guarantee them to be correct, because we aren't
            // caching items.
            //
            // For Update->Remove (Previous matched, Current does not), the item that leaves
            // downstream is Previous; Current never reached downstream. Per the Change<T,K>
            // contract, Remove.Current is the value being removed, so carry Previous here.
            let current = if reason == ChangeReason::Remove && change.reason() == ChangeReason::Update {
                change.previous().expect("an update has a previous item").clone()
            } else {
                change.current().clone()
            };
            let previous = if reason == ChangeReason::Update { change.previous().cloned() } else { None };
            downstream_changes.push(Change::with_previous(reason, change.key().clone(), current, previous, None, None));
        }
    }
    Ok(ChangeSet::from(downstream_changes))
}
