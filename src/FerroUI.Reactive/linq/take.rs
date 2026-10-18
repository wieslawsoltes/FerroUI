//! `Take` (`Linq/Observable/Take.cs`).

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{impl_sink_dispose, subscribe_sink, unary, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The sink of `Take` with a count (`Take<TSource>.Count._`): the count is decremented before the
/// value is forwarded, and the sequence completes after the last value.
struct TakeCountSink<T> {
    sink: Sink<T>,
    remaining: Cell<usize>,
}

impl<T: 'static> OperatorSink<T, T> for TakeCountSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(TakeCountSink<T>);

impl<T: 'static> IObserver<T> for TakeCountSink<T> {
    fn on_next(&self, value: T) {
        let remaining = self.remaining.get();
        if remaining > 0 {
            self.remaining.set(remaining - 1);
            self.sink.forward_on_next(value);
            if remaining == 1 {
                self.sink.forward_on_completed();
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

// Deviation (DEVIATIONS.md, Reactive): upstream fuses `Take(n).Take(m)` into one operator
// (`Count.Combine`, and `Time.Combine` for the timed form on the same scheduler); a handle of the
// port cannot be inspected for the operator behind it, so the operators are chained. The
// notifications and the subscriptions to the source are the same.
fn take_count<T: 'static>(source: Rc<dyn IObservable<T>>, count: usize) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(TakeCountSink { sink: Sink::new(observer), remaining: Cell::new(count) }))
}

/// `Take(int)`: no value at all is an empty sequence (on the immediate scheduler).
pub(crate) fn take<T: 'static>(source: Rc<dyn IObservable<T>>, count: usize) -> Rc<dyn IObservable<T>> {
    if count == 0 {
        return crate::linq::Observable::empty();
    }
    take_count(source, count)
}

/// `Take(int, IScheduler)`: no value at all is an empty sequence on `scheduler`; the scheduler is
/// not used otherwise.
pub(crate) fn take_on<T: 'static>(source: Rc<dyn IObservable<T>>, count: usize, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    if count == 0 {
        return crate::linq::Observable::empty_on(scheduler);
    }
    take_count(source, count)
}

/// The sink of `Take` with a duration (`Take<TSource>.Time._`): the values until the timer fires,
/// which completes the sequence.
struct TakeTimeSink<T> {
    sink: Sink<T>,
    task: SingleAssignmentDisposableValue,
}

impl<T: 'static> TakeTimeSink<T> {
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

impl<T: 'static> IDisposable for TakeTimeSink<T> {
    fn dispose(&self) {
        self.task.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for TakeTimeSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.forward_on_completed();
    }
}

/// `Take<TSource>.Time`.
struct TakeTime<T> {
    source: Rc<dyn IObservable<T>>,
    duration: Duration,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for TakeTime<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let duration = self.duration;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| Rc::new(TakeTimeSink { sink: Sink::new(observer), task: SingleAssignmentDisposableValue::new() }),
            move |sink: &Rc<TakeTimeSink<T>>| {
                let target = sink.clone();
                sink.task.set_disposable(Some(scheduler.schedule_action_after(duration, move || target.forward_on_completed())));
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

pub(crate) fn take_time<T: 'static>(source: Rc<dyn IObservable<T>>, duration: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(TakeTime { source, duration, scheduler })
}

/// The methods of `Take` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! take_methods {
    () => {
        /// The first `count` values (`Take(int)`); `take(0)` is an empty sequence that completes at
        /// once.
        fn take(&self, count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take::take(self.observable(), count)
        }

        /// The first `count` values; `take_on(0, scheduler)` completes on `scheduler`
        /// (`Take(int, IScheduler)`).
        fn take_on(&self, count: usize, scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take::take_on(self.observable(), count, scheduler)
        }

        /// The values of the first `duration` after the subscription, timed on the default
        /// scheduler of time-based operations (`Take(TimeSpan)`).
        fn take_time(&self, duration: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take::take_time(self.observable(), duration, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// The values of the first `duration` after the subscription, timed on `scheduler`
        /// (`Take(TimeSpan, IScheduler)`).
        fn take_time_on(
            &self,
            duration: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take::take_time(self.observable(), duration, scheduler)
        }
    };
}
pub(crate) use take_methods;
