//! `DisposeMany<TObject, TKey>` (`Cache/Internal/DisposeMany.cs`): disposes each item when it is
//! removed or replaced, and every item left when the subscription ends.

use crate::cache::{ChangeReason, ChangeSet};
use crate::internal::KeyedDisposable;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{Observable, Observer};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::hash::Hash;
use std::rc::Rc;

/// The disposable of an item, if it has one (`item as IDisposable`).
pub(crate) type ToDisposable<T> = Rc<dyn Fn(&T) -> Option<Rc<dyn IDisposable>>>;

/// `DisposeMany.Run`: each change set is forwarded first, then the items it adds or updates are
/// tracked by key (the replaced one is disposed) and those it removes are disposed.
pub(crate) fn dispose_many<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    to_disposable: ToDisposable<T>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let tracked = Rc::new(KeyedDisposable::<K>::new());
        let (next_observer, error_observer, completed_observer) = (observer.clone(), observer.clone(), observer);
        let target = tracked.clone();
        let to_disposable = to_disposable.clone();
        let source_subscription = source.subscribe(Observer::create_all(
            move |change_set: ChangeSet<T, K>| {
                next_observer.on_next(change_set.clone());
                for change in &change_set {
                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => target.add(change.key().clone(), to_disposable(change.current())),
                        ChangeReason::Remove => target.remove(change.key()),
                        ChangeReason::Refresh | ChangeReason::Moved => {}
                    }
                }
            },
            move |error| error_observer.on_error(error),
            move || completed_observer.on_completed(),
        ));
        let tracked_disposal = Disposable::create(move || tracked.dispose());
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([source_subscription, tracked_disposal]);
        composite
    })
}
