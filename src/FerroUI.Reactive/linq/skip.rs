//! `Skip` (`Linq/Observable/Skip.cs`).

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{impl_sink_dispose, subscribe_sink, unary, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The sink of `Skip` with a count (`Skip<TSource>.Count._`).
struct SkipCountSink<T> {
    sink: Sink<T>,
    remaining: Cell<usize>,
}

impl<T: 'static> OperatorSink<T, T> for SkipCountSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(SkipCountSink<T>);

impl<T: 'static> IObserver<T> for SkipCountSink<T> {
    fn on_next(&self, value: T) {
        let remaining = self.remaining.get();
        if remaining == 0 {
            self.sink.forward_on_next(value);
        } else {
            self.remaining.set(remaining - 1);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

// Deviation (DEVIATIONS.md, Reactive): upstream fuses `Skip(n).Skip(m)` into one operator
// (`Count.Combine`, and `Time.Combine` for the timed form on the same scheduler); a handle of the
// port cannot be inspected for the operator behind it, so the operators are chained. The
// notifications and the subscriptions to the source are the same.
pub(crate) fn skip<T: 'static>(source: Rc<dyn IObservable<T>>, count: usize) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(SkipCountSink { sink: Sink::new(observer), remaining: Cell::new(count) }))
}

/// The sink of `Skip` with a duration (`Skip<TSource>.Time._`): the values before the timer has
/// fired are dropped. The timer is the upstream of the sink; the subscription to the source is kept
/// apart.
struct SkipTimeSink<T> {
    sink: Sink<T>,
    open: Cell<bool>,
    source_disposable: SingleAssignmentDisposableValue,
}

impl<T: 'static> SkipTimeSink<T> {
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

impl<T: 'static> IDisposable for SkipTimeSink<T> {
    fn dispose(&self) {
        self.source_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for SkipTimeSink<T> {
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

/// `Skip<TSource>.Time`.
struct SkipTime<T> {
    source: Rc<dyn IObservable<T>>,
    duration: Duration,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for SkipTime<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let duration = self.duration;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(SkipTimeSink {
                    sink: Sink::new(observer),
                    open: Cell::new(false),
                    source_disposable: SingleAssignmentDisposableValue::new(),
                })
            },
            move |sink: &Rc<SkipTimeSink<T>>| {
                let target = sink.clone();
                sink.sink.set_upstream(scheduler.schedule_action_after(duration, move || target.open.set(true)));
                sink.source_disposable.set_disposable(Some(source.subscribe(sink.clone())));
            },
        )
    }
}

pub(crate) fn skip_time<T: 'static>(source: Rc<dyn IObservable<T>>, duration: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(SkipTime { source, duration, scheduler })
}

/// The methods of `Skip` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! skip_methods {
    () => {
        /// The values after the first `count` (`Skip(int)`).
        fn skip(&self, count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip::skip(self.observable(), count)
        }

        /// The values produced once `duration` has passed since the subscription, timed on the
        /// default scheduler of time-based operations (`Skip(TimeSpan)`).
        fn skip_time(&self, duration: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip::skip_time(self.observable(), duration, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// The values produced once `duration` has passed since the subscription, timed on
        /// `scheduler` (`Skip(TimeSpan, IScheduler)`).
        fn skip_time_on(
            &self,
            duration: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip::skip_time(self.observable(), duration, scheduler)
        }
    };
}
pub(crate) use skip_methods;
