// Code based on https://github.com/dotnet/reactive/blob/main/Rx.NET/Source/src/System.Reactive/Linq/Observable/Switch.cs

use super::sink::{ISink, Sink, SubscriptionSlot};
use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Forwards the values of the most recent of the observables that an
/// observable of observables produces.
pub(crate) struct Switch<TSource> {
    sources: Rc<dyn IObservable<Rc<dyn IObservable<TSource>>>>,
}

impl<TSource: 'static> Switch<TSource> {
    pub fn new(sources: Rc<dyn IObservable<Rc<dyn IObservable<TSource>>>>) -> Self {
        Self { sources }
    }
}

impl<TSource: 'static> IObservable<TSource> for Switch<TSource> {
    fn subscribe(&self, observer: Rc<dyn IObserver<TSource>>) -> Rc<dyn IDisposable> {
        self.sources.subscribe(SwitchSink::new(observer))
    }
}

/// The subscription of a [`Switch`] (the original's nested class `_`).
struct SwitchSink<TSource> {
    sink: Sink<TSource>,
    weak_self: Weak<SwitchSink<TSource>>,
    inner_serial_disposable: RefCell<Option<Rc<SubscriptionSlot>>>,
    is_stopped: Cell<bool>,
    latest: Cell<u64>,
    has_latest: Cell<bool>,
}

impl<TSource: 'static> SwitchSink<TSource> {
    fn new(observer: Rc<dyn IObserver<TSource>>) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            sink: Sink::new(observer),
            weak_self: weak_self.clone(),
            inner_serial_disposable: RefCell::new(None),
            is_stopped: Cell::new(false),
            latest: Cell::new(0),
            has_latest: Cell::new(false),
        })
    }
}

impl<TSource: 'static> ISink<TSource> for SwitchSink<TSource> {
    fn sink(&self) -> &Sink<TSource> {
        &self.sink
    }

    fn dispose(&self) {
        let inner = self.inner_serial_disposable.borrow().clone();
        if let Some(inner) = inner {
            inner.dispose();
        }

        self.sink.dispose();
    }
}

impl<TSource: 'static> IObserver<Rc<dyn IObservable<TSource>>> for SwitchSink<TSource> {
    fn on_next(&self, value: Rc<dyn IObservable<TSource>>) {
        let id = self.latest.get().wrapping_add(1);
        self.latest.set(id);
        self.has_latest.set(true);

        let Some(parent) = self.weak_self.upgrade() else {
            return;
        };
        let disposable = SubscriptionSlot::new();
        let inner_observer = Rc::new(InnerObserver { parent, id, disposable: disposable.clone() });

        let previous = self.inner_serial_disposable.replace(Some(disposable.clone()));
        if let Some(previous) = previous {
            previous.dispose();
        }
        disposable.set(value.subscribe(inner_observer));
    }

    fn on_error(&self, error: ObservableError) {
        self.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.dispose_upstream();

        self.is_stopped.set(true);
        if !self.has_latest.get() {
            self.forward_on_completed();
        }
    }
}

struct InnerObserver<TSource> {
    parent: Rc<SwitchSink<TSource>>,
    id: u64,
    disposable: Rc<SubscriptionSlot>,
}

impl<TSource: 'static> IObserver<TSource> for InnerObserver<TSource> {
    fn on_next(&self, value: TSource) {
        if self.parent.latest.get() == self.id {
            self.parent.forward_on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.disposable.dispose();

        if self.parent.latest.get() == self.id {
            self.parent.forward_on_error(error);
        }
    }

    fn on_completed(&self) {
        self.disposable.dispose();

        if self.parent.latest.get() == self.id {
            self.parent.has_latest.set(false);

            if self.parent.is_stopped.get() {
                self.parent.forward_on_completed();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;
    use crate::reactive::{AnonymousObserver, LightweightSubject};

    type Inner = Rc<dyn IObservable<i32>>;

    fn subscribe(
        sources: &LightweightSubject<Inner>,
    ) -> (Rc<RefCell<Vec<i32>>>, Rc<Cell<bool>>, Rc<dyn IDisposable>) {
        let received = Rc::new(RefCell::new(Vec::new()));
        let completed = Rc::new(Cell::new(false));
        let (r, c) = (received.clone(), completed.clone());
        let observer: Rc<dyn IObserver<i32>> = Rc::new(AnonymousObserver::new_with_completed(
            move |value: i32| r.borrow_mut().push(value),
            move || c.set(true),
        ));
        let outer: Rc<dyn IObservable<Inner>> = Rc::new(sources.clone());
        let subscription = Switch::new(outer).subscribe(observer);
        (received, completed, subscription)
    }

    #[test]
    fn forwards_the_values_of_the_latest_source_only() {
        let sources = LightweightSubject::<Inner>::new();
        let first = LightweightSubject::<i32>::new();
        let second = LightweightSubject::<i32>::new();
        let (received, _, _subscription) = subscribe(&sources);

        sources.on_next(Rc::new(first.clone()));
        first.on_next(1);
        sources.on_next(Rc::new(second.clone()));
        assert!(!first.has_observers());
        first.on_next(2);
        second.on_next(3);

        assert_eq!(vec![1, 3], *received.borrow());
    }

    #[test]
    fn completes_when_the_sources_and_the_latest_source_have_completed() {
        let sources = LightweightSubject::<Inner>::new();
        let inner = LightweightSubject::<i32>::new();
        let (_, completed, _subscription) = subscribe(&sources);

        sources.on_next(Rc::new(inner.clone()));
        sources.on_completed();
        assert!(!completed.get());

        inner.on_completed();
        assert!(completed.get());
    }

    #[test]
    fn completes_with_the_sources_when_there_is_no_source() {
        let sources = LightweightSubject::<Inner>::new();
        let (_, completed, _subscription) = subscribe(&sources);

        sources.on_completed();
        assert!(completed.get());
    }
}
