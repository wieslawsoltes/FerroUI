//! `TakeUntil` (`Linq/Observable/TakeUntil.cs`, `Linq/Observable/TakeUntilPredicate.cs`,
//! `Linq/Observable/TakeUntilCancellationToken.cs`).

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{impl_sink_dispose, subscribe_sink, unary, HalfSerializedSink, HalfSerializer, OperatorSink, Sink, TokenRegistration};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;
use std::time::SystemTime;

// ---------------------------------------------------------------------------------------------
// TakeUntil(IObservable<TOther>)
// ---------------------------------------------------------------------------------------------

/// The sink of `TakeUntil` with another sequence (`TakeUntil<TSource, TOther>._`): the values of
/// the source until the other sequence produces a value.
struct TakeUntilSink<T> {
    sink: Sink<T>,
    other_disposable: SingleAssignmentDisposableValue,
    serializer: HalfSerializer,
}

impl<T: 'static> TakeUntilSink<T> {
    fn dispose_other(&self) {
        if !self.other_disposable.is_disposed() {
            self.other_disposable.dispose();
        }
    }
}

impl<T: 'static> HalfSerializedSink<T> for TakeUntilSink<T> {
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

impl<T: 'static> IDisposable for TakeUntilSink<T> {
    fn dispose(&self) {
        self.dispose_other();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for TakeUntilSink<T> {
    fn on_next(&self, value: T) {
        self.serializer.on_next(self, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.serializer.on_error(self, error);
    }

    fn on_completed(&self) {
        self.serializer.on_completed(self);
    }
}

/// The observer of the other sequence (`TakeUntil<TSource, TOther>._.OtherObserver`).
struct TakeUntilOtherObserver<T> {
    parent: Rc<TakeUntilSink<T>>,
}

impl<T: 'static, U> IObserver<U> for TakeUntilOtherObserver<T> {
    fn on_next(&self, _value: U) {
        self.parent.serializer.on_completed(&*self.parent);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.serializer.on_error(&*self.parent, error);
    }

    fn on_completed(&self) {
        // The completion of the other sequence does not end the sequence.
        self.parent.other_disposable.dispose();
    }
}

/// `TakeUntil<TSource, TOther>`.
struct TakeUntil<T, U> {
    source: Rc<dyn IObservable<T>>,
    other: Rc<dyn IObservable<U>>,
}

impl<T: 'static, U: 'static> IObservable<T> for TakeUntil<T, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let other = self.other.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(TakeUntilSink { sink: Sink::new(observer), other_disposable: SingleAssignmentDisposableValue::new(), serializer: HalfSerializer::new() })
            },
            move |sink: &Rc<TakeUntilSink<T>>| {
                let other_observer: Rc<dyn IObserver<U>> = Rc::new(TakeUntilOtherObserver { parent: sink.clone() });
                sink.other_disposable.set_disposable(Some(other.subscribe(other_observer)));
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

pub(crate) fn take_until<T: 'static, U: 'static>(source: Rc<dyn IObservable<T>>, other: Rc<dyn IObservable<U>>) -> Rc<dyn IObservable<T>> {
    Rc::new(TakeUntil { source, other })
}

// ---------------------------------------------------------------------------------------------
// TakeUntil(DateTimeOffset)
// ---------------------------------------------------------------------------------------------

/// The sink of `TakeUntil` with an end time (`TakeUntil<TSource>._`).
struct TakeUntilTimeSink<T> {
    sink: Sink<T>,
    timer_disposable: SingleAssignmentDisposableValue,
    serializer: HalfSerializer,
}

impl<T: 'static> HalfSerializedSink<T> for TakeUntilTimeSink<T> {
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

impl<T: 'static> IDisposable for TakeUntilTimeSink<T> {
    fn dispose(&self) {
        self.timer_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for TakeUntilTimeSink<T> {
    fn on_next(&self, value: T) {
        self.serializer.on_next(self, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.serializer.on_error(self, error);
    }

    fn on_completed(&self) {
        self.serializer.on_completed(self);
    }
}

/// `TakeUntil<TSource>` (with an end time).
struct TakeUntilTime<T> {
    source: Rc<dyn IObservable<T>>,
    end_time: SystemTime,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for TakeUntilTime<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let end_time = self.end_time;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(TakeUntilTimeSink { sink: Sink::new(observer), timer_disposable: SingleAssignmentDisposableValue::new(), serializer: HalfSerializer::new() })
            },
            move |sink: &Rc<TakeUntilTimeSink<T>>| {
                let target = sink.clone();
                sink.timer_disposable.set_disposable(Some(scheduler.schedule_action_at(end_time, move || target.on_completed())));
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

// Deviation (DEVIATIONS.md, Reactive): upstream fuses `TakeUntil(t1).TakeUntil(t2)` on the same
// scheduler into one operator (`Combine`); the port chains them, with the same notifications.
pub(crate) fn take_until_at<T: 'static>(source: Rc<dyn IObservable<T>>, end_time: SystemTime, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(TakeUntilTime { source, end_time, scheduler })
}

// ---------------------------------------------------------------------------------------------
// TakeUntil(Func<TSource, bool>)
// ---------------------------------------------------------------------------------------------

type StopPredicate<T> = Rc<dyn Fn(&T) -> Result<bool, ObservableError>>;

/// The sink of `TakeUntil` with a stop predicate (`TakeUntilPredicate<TSource>.TakeUntilPredicateObserver`):
/// each value is forwarded, and the sequence completes after the first value for which the
/// predicate holds. A predicate that fails ends the sequence with its error after the value.
struct TakeUntilPredicateSink<T> {
    sink: Sink<T>,
    stop_predicate: StopPredicate<T>,
}

impl<T: 'static> OperatorSink<T, T> for TakeUntilPredicateSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(TakeUntilPredicateSink<T>);

impl<T: 'static> IObserver<T> for TakeUntilPredicateSink<T> {
    fn on_next(&self, value: T) {
        // Deviation (DEVIATIONS.md, Reactive): upstream forwards the value and then tests it; the
        // port tests it first (so that the value need not be cloned) and then forwards it, and
        // acts on the outcome of the test after the value has been forwarded, as upstream does.
        let should_stop = (self.stop_predicate)(&value);
        self.sink.forward_on_next(value);
        match should_stop {
            Ok(true) => self.sink.forward_on_completed(),
            Ok(false) => {}
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn take_until_predicate<T: 'static>(source: Rc<dyn IObservable<T>>, stop_predicate: StopPredicate<T>) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(TakeUntilPredicateSink { sink: Sink::new(observer), stop_predicate: stop_predicate.clone() }))
}

// ---------------------------------------------------------------------------------------------
// TakeUntil(CancellationToken)
// ---------------------------------------------------------------------------------------------

/// The sink of `TakeUntil` with a cancellation token (`TakeUntilCancellationToken<TSource>._`).
struct TakeUntilTokenSink<T> {
    sink: Sink<T>,
    registration: SingleAssignmentDisposableValue,
    serializer: HalfSerializer,
}

impl<T: 'static> HalfSerializedSink<T> for TakeUntilTokenSink<T> {
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

impl<T: 'static> IDisposable for TakeUntilTokenSink<T> {
    fn dispose(&self) {
        self.registration.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for TakeUntilTokenSink<T> {
    fn on_next(&self, value: T) {
        self.serializer.on_next(self, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.serializer.on_error(self, error);
    }

    fn on_completed(&self) {
        self.serializer.on_completed(self);
    }
}

/// `TakeUntilCancellationToken<TSource>`.
struct TakeUntilToken<T> {
    source: Rc<dyn IObservable<T>>,
    token: CancellationToken,
}

impl<T: 'static> IObservable<T> for TakeUntilToken<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let token = self.token.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(TakeUntilTokenSink { sink: Sink::new(observer), registration: SingleAssignmentDisposableValue::new(), serializer: HalfSerializer::new() })
            },
            move |sink: &Rc<TakeUntilTokenSink<T>>| {
                if token.is_cancellation_requested() {
                    sink.on_completed();
                    return;
                }
                let target = sink.clone();
                let registration = TokenRegistration::register(&token, move || target.on_completed());
                sink.registration.set_disposable(Some(registration));
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

// Deviation (DEVIATIONS.md, Reactive): the sequence is completed by a cancellation requested on the
// thread of the subscription; a cancellation requested on another thread is not seen by it (the
// sequences of the port are single-threaded and cannot be reached from another thread).
pub(crate) fn take_until_cancellation<T: 'static>(source: Rc<dyn IObservable<T>>, token: CancellationToken) -> Rc<dyn IObservable<T>> {
    Rc::new(TakeUntilToken { source, token })
}

/// The methods of `TakeUntil` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! take_until_methods {
    () => {
        /// The values until `other` produces a value (`TakeUntil<TSource, TOther>(IObservable<TOther>)`);
        /// the completion of `other` does not end the sequence, its error does.
        fn take_until<U: 'static>(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_until::take_until(self.observable(), other)
        }

        /// The values until `end_time`, timed on the default scheduler of time-based operations
        /// (`TakeUntil(DateTimeOffset)`).
        fn take_until_at(&self, end_time: std::time::SystemTime) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_until::take_until_at(self.observable(), end_time, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// The values until `end_time`, timed on `scheduler` (`TakeUntil(DateTimeOffset, IScheduler)`).
        fn take_until_at_on(
            &self,
            end_time: std::time::SystemTime,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_until::take_until_at(self.observable(), end_time, scheduler)
        }

        /// The values up to and including the first one for which `stop_predicate` is true
        /// (`TakeUntil(Func<TSource, bool>)`).
        fn take_until_predicate(&self, stop_predicate: impl Fn(&T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_until::take_until_predicate(self.observable(), std::rc::Rc::new(move |value| Ok(stop_predicate(value))))
        }

        /// [`take_until_predicate`](Self::take_until_predicate) with a predicate that may fail; a
        /// failure ends the sequence with its error after the value (`TakeUntil(Func<TSource, bool>)`
        /// with a predicate that throws).
        fn try_take_until_predicate(
            &self,
            stop_predicate: impl Fn(&T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_until::take_until_predicate(self.observable(), std::rc::Rc::new(stop_predicate))
        }

        /// The values until `token` is canceled (`TakeUntil(CancellationToken)`); a token canceled
        /// already gives a sequence that completes at once without subscribing to the source.
        fn take_until_cancellation(
            &self,
            token: ferroui_system::threading::CancellationToken,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_until::take_until_cancellation(self.observable(), token)
        }
    };
}
pub(crate) use take_until_methods;
