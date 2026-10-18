//! `SkipUntil` (`Linq/Observable/SkipUntil.cs`).

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{subscribe_sink, HalfSerializedSink, HalfSerializer, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;
use std::time::SystemTime;

// ---------------------------------------------------------------------------------------------
// SkipUntil(IObservable<TOther>)
// ---------------------------------------------------------------------------------------------

/// The sink of `SkipUntil` with another sequence (`SkipUntil<TSource, TOther>._`): the values of
/// the source once the other sequence has produced a value. A source that completes before then
/// is let go without ending the sequence.
struct SkipUntilSink<T> {
    sink: Sink<T>,
    other_disposable: SingleAssignmentDisposableValue,
    forward: Cell<bool>,
    serializer: HalfSerializer,
}

impl<T: 'static> SkipUntilSink<T> {
    fn dispose_other(&self) {
        if !self.other_disposable.is_disposed() {
            self.other_disposable.dispose();
        }
    }
}

impl<T: 'static> HalfSerializedSink<T> for SkipUntilSink<T> {
    fn forward_on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn forward_on_error(&self, error: ObservableError) {
        let observer = self.sink.observer();
        observer.on_error(error);
        self.dispose();
    }

    fn forward_on_completed(&self) {
        let observer = self.sink.observer();
        observer.on_completed();
        self.dispose();
    }
}

impl<T: 'static> IDisposable for SkipUntilSink<T> {
    fn dispose(&self) {
        self.dispose_other();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for SkipUntilSink<T> {
    fn on_next(&self, value: T) {
        if self.forward.get() {
            self.serializer.on_next(self, value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.serializer.on_error(self, error);
    }

    fn on_completed(&self) {
        if self.forward.get() {
            self.serializer.on_completed(self);
        } else {
            self.sink.dispose_upstream();
        }
    }
}

/// The observer of the other sequence (`SkipUntil<TSource, TOther>._.OtherObserver`).
struct SkipUntilOtherObserver<T> {
    parent: Rc<SkipUntilSink<T>>,
}

impl<T: 'static, U> IObserver<U> for SkipUntilOtherObserver<T> {
    fn on_next(&self, _value: U) {
        self.parent.forward.set(true);
        self.parent.dispose_other();
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.serializer.on_error(&*self.parent, error);
    }

    fn on_completed(&self) {
        self.parent.dispose_other();
    }
}

/// `SkipUntil<TSource, TOther>`.
struct SkipUntil<T, U> {
    source: Rc<dyn IObservable<T>>,
    other: Rc<dyn IObservable<U>>,
}

impl<T: 'static, U: 'static> IObservable<T> for SkipUntil<T, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let other = self.other.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(SkipUntilSink {
                    sink: Sink::new(observer),
                    other_disposable: SingleAssignmentDisposableValue::new(),
                    forward: Cell::new(false),
                    serializer: HalfSerializer::new(),
                })
            },
            move |sink: &Rc<SkipUntilSink<T>>| {
                let other_observer: Rc<dyn IObserver<U>> = Rc::new(SkipUntilOtherObserver { parent: sink.clone() });
                sink.other_disposable.set_disposable(Some(other.subscribe(other_observer)));
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

pub(crate) fn skip_until<T: 'static, U: 'static>(source: Rc<dyn IObservable<T>>, other: Rc<dyn IObservable<U>>) -> Rc<dyn IObservable<T>> {
    Rc::new(SkipUntil { source, other })
}

// ---------------------------------------------------------------------------------------------
// SkipUntil(DateTimeOffset)
// ---------------------------------------------------------------------------------------------

/// The sink of `SkipUntil` with a start time (`SkipUntil<TSource>._`): the values before the timer
/// has fired are dropped.
struct SkipUntilTimeSink<T> {
    sink: Sink<T>,
    open: Cell<bool>,
    task: SingleAssignmentDisposableValue,
}

impl<T: 'static> SkipUntilTimeSink<T> {
    fn forward_on_error(&self, error: ObservableError) {
        let observer = self.sink.observer();
        observer.on_error(error);
        self.dispose();
    }

    fn forward_on_completed(&self) {
        let observer = self.sink.observer();
        observer.on_completed();
        self.dispose();
    }
}

impl<T: 'static> IDisposable for SkipUntilTimeSink<T> {
    fn dispose(&self) {
        self.task.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for SkipUntilTimeSink<T> {
    fn on_next(&self, value: T) {
        if self.open.get() {
            self.sink.forward_on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.forward_on_completed();
    }
}

/// `SkipUntil<TSource>` (with a start time).
struct SkipUntilTime<T> {
    source: Rc<dyn IObservable<T>>,
    start_time: SystemTime,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for SkipUntilTime<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let start_time = self.start_time;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| Rc::new(SkipUntilTimeSink { sink: Sink::new(observer), open: Cell::new(false), task: SingleAssignmentDisposableValue::new() }),
            move |sink: &Rc<SkipUntilTimeSink<T>>| {
                let target = sink.clone();
                sink.task.set_disposable(Some(scheduler.schedule_action_at(start_time, move || target.open.set(true))));
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

// Deviation (DEVIATIONS.md, Reactive): upstream fuses `SkipUntil(t1).SkipUntil(t2)` on the same
// scheduler into one operator (`Combine`); the port chains them, with the same notifications.
pub(crate) fn skip_until_at<T: 'static>(source: Rc<dyn IObservable<T>>, start_time: SystemTime, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(SkipUntilTime { source, start_time, scheduler })
}

/// The methods of `SkipUntil` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! skip_until_methods {
    () => {
        /// The values once `other` has produced a value (`SkipUntil<TSource, TOther>(IObservable<TOther>)`).
        fn skip_until<U: 'static>(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_until::skip_until(self.observable(), other)
        }

        /// The values from `start_time`, timed on the default scheduler of time-based operations
        /// (`SkipUntil(DateTimeOffset)`).
        fn skip_until_at(&self, start_time: std::time::SystemTime) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_until::skip_until_at(self.observable(), start_time, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// The values from `start_time`, timed on `scheduler` (`SkipUntil(DateTimeOffset, IScheduler)`).
        fn skip_until_at_on(
            &self,
            start_time: std::time::SystemTime,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_until::skip_until_at(self.observable(), start_time, scheduler)
        }
    };
}
pub(crate) use skip_until_methods;
