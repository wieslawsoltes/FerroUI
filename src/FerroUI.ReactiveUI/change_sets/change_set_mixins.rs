//! `ChangeSetMixins` (`ReactiveUI.Core/Mixins/ChangeSetMixins.cs`).

use super::{IReactiveChangeSet, ReactiveChangeSet};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The filter of the change sets that change the count of a collection (`ChangeSetMixins`, on
/// a stream of change sets).
pub trait ChangeSetMixins<T: 'static> {
    /// The change sets that add or remove items (`WhenCountChanged`).
    fn when_count_changed(&self) -> Rc<dyn IObservable<ReactiveChangeSet<T>>>;
}

impl<T: 'static> ChangeSetMixins<T> for Rc<dyn IObservable<ReactiveChangeSet<T>>> {
    fn when_count_changed(&self) -> Rc<dyn IObservable<ReactiveChangeSet<T>>> {
        Rc::new(CountChangedObservable { source: self.clone() })
    }
}

/// Whether a change set changes the count of its collection (`ChangeSetMixins`, on a change set).
pub trait ChangeSetCountMixins {
    /// Whether the change set adds or removes items (`CountHasChanged`).
    fn count_has_changed(&self) -> bool;
}

impl<C: IReactiveChangeSet + ?Sized> ChangeSetCountMixins for C {
    fn count_has_changed(&self) -> bool {
        self.adds() > 0 || self.removes() > 0
    }
}

/// `CountChangedObservable<T>`.
struct CountChangedObservable<T> {
    source: Rc<dyn IObservable<ReactiveChangeSet<T>>>,
}

impl<T: 'static> IObservable<ReactiveChangeSet<T>> for CountChangedObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<ReactiveChangeSet<T>>>) -> Rc<dyn IDisposable> {
        self.source.subscribe(Rc::new(CountChangedSink { downstream: observer }))
    }
}

/// `CountChangedObservable.Sink`.
struct CountChangedSink<T> {
    downstream: Rc<dyn IObserver<ReactiveChangeSet<T>>>,
}

impl<T> IObserver<ReactiveChangeSet<T>> for CountChangedSink<T> {
    fn on_next(&self, value: ReactiveChangeSet<T>) {
        if value.adds() == 0 && value.removes() == 0 {
            return;
        }
        self.downstream.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}
