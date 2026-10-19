//! `TransformImmutable<TDestination, TSource, TKey>` (`Cache/Internal/TransformImmutable.cs`).

use crate::cache::{Change, ChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::hash::Hash;
use std::rc::Rc;

/// Each change of the source with its items transformed, and every other part kept: the reason,
/// the key and the indexes (`TransformImmutable.Run`). Nothing is cached: the factory runs for
/// each change, the previous item of an update included. An error of the factory ends the
/// sequence with it; the change set that raised it is not emitted.
pub(crate) fn transform_immutable<D: 'static, S: 'static, K: Clone + Eq + Hash + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, K>>>,
    transform_factory: Rc<dyn Fn(&S) -> Result<D, ObservableError>>,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, K>>>| {
        let transform_factory = transform_factory.clone();
        let (on_error, on_completed) = (observer.clone(), observer.clone());
        source.subscribe_all(
            move |upstream_changes: ChangeSet<S, K>| {
                // The factory is "untrusted" code of the consumer: its errors are propagated.
                match transform_changes(&upstream_changes, &*transform_factory) {
                    Ok(downstream_changes) => observer.on_next(downstream_changes),
                    Err(error) => observer.on_error(error),
                }
            },
            move |error| on_error.on_error(error),
            move || on_completed.on_completed(),
        )
    })
}

fn transform_changes<D, S, K: Clone>(upstream_changes: &ChangeSet<S, K>, transform_factory: &dyn Fn(&S) -> Result<D, ObservableError>) -> Result<ChangeSet<D, K>, ObservableError> {
    let mut downstream_changes = Vec::with_capacity(upstream_changes.count());
    for change in upstream_changes {
        let current = transform_factory(change.current())?;
        let previous = match change.previous() {
            Some(previous) => Some(transform_factory(previous)?),
            None => None,
        };
        downstream_changes.push(Change::with_previous(change.reason(), change.key().clone(), current, previous, change.current_index(), change.previous_index()));
    }
    Ok(ChangeSet::from(downstream_changes))
}
