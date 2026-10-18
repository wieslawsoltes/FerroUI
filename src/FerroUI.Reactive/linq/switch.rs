//! `Switch` (`Linq/Observable/Switch.cs`).

use crate::disposables::{SerialDisposableValue, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use crate::linq::merge::ObservableSource;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

/// The sink of `Switch` (`Switch<TSource>._`): each inner sequence replaces the previous one, whose
/// subscription is disposed; the sequence ends when the outer sequence and the latest inner one have
/// ended.
struct SwitchSink<S, T> {
    this: Weak<SwitchSink<S, T>>,
    sink: Sink<T>,
    inner_serial: SerialDisposableValue,
    is_stopped: Cell<bool>,
    latest: Cell<u64>,
    has_latest: Cell<bool>,
    _sources: PhantomData<fn(S)>,
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObserver<S> for SwitchSink<S, T> {
    fn on_next(&self, value: S) {
        let id = self.latest.get().wrapping_add(1);
        self.latest.set(id);
        self.has_latest.set(true);
        let parent = self.this.upgrade().expect("the sink is alive while it receives notifications");
        let inner = Rc::new(SwitchInnerObserver { parent, id, resource: SingleAssignmentDisposableValue::new() });
        self.inner_serial.set_disposable(Some(inner.clone()));
        let subscription = value.subscribe_source(inner.clone());
        inner.resource.set_disposable(Some(subscription));
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.dispose_upstream();
        self.is_stopped.set(true);
        if !self.has_latest.get() {
            self.sink.forward_on_completed();
        }
    }
}

impl<S: 'static, T: 'static> IDisposable for SwitchSink<S, T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.inner_serial.dispose();
        }
    }
}

/// The observer of an inner sequence of `Switch` (`InnerObserver`, a `SafeObserver<TSource>`): it
/// notifies only while its sequence is the latest one.
struct SwitchInnerObserver<S, T> {
    parent: Rc<SwitchSink<S, T>>,
    id: u64,
    resource: SingleAssignmentDisposableValue,
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObserver<T> for SwitchInnerObserver<S, T> {
    fn on_next(&self, value: T) {
        if self.parent.latest.get() == self.id {
            self.parent.sink.forward_on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.resource.dispose();
        if self.parent.latest.get() == self.id {
            self.parent.sink.forward_on_error(error);
        }
    }

    fn on_completed(&self) {
        self.resource.dispose();
        let parent = &self.parent;
        if parent.latest.get() == self.id {
            parent.has_latest.set(false);
            if parent.is_stopped.get() {
                parent.sink.forward_on_completed();
            }
        }
    }
}

impl<S, T> IDisposable for SwitchInnerObserver<S, T> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observable of `Switch` (`Switch<TSource>`).
struct SwitchObservable<S, T> {
    sources: Rc<dyn IObservable<S>>,
    _values: PhantomData<fn() -> T>,
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObservable<T> for SwitchObservable<S, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let sources = self.sources.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new_cyclic(|this| SwitchSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    inner_serial: SerialDisposableValue::new(),
                    is_stopped: Cell::new(false),
                    latest: Cell::new(0),
                    has_latest: Cell::new(false),
                    _sources: PhantomData,
                })
            },
            move |sink: &Rc<SwitchSink<S, T>>| {
                let upstream = sources.subscribe(sink.clone());
                sink.sink.set_upstream(upstream);
            },
        )
    }
}

pub(crate) fn switch<S: ObservableSource<T> + 'static, T: 'static>(sources: Rc<dyn IObservable<S>>) -> Rc<dyn IObservable<T>> {
    Rc::new(SwitchObservable { sources, _values: PhantomData })
}

/// The methods of `Switch` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! switch_methods {
    () => {
        /// The values of the latest inner sequence of a sequence of sequences: each inner sequence
        /// replaces the previous one (`Switch(IObservable<IObservable<TSource>>)`).
        fn switch<U: 'static>(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
        where
            T: crate::linq::merge::ObservableSource<U>,
        {
            crate::linq::switch::switch(self.observable())
        }
    };
}
pub(crate) use switch_methods;
