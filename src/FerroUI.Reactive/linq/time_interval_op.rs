//! `TimeInterval` (`Linq/Observable/TimeInterval.cs`). The module is named `time_interval_op`
//! because `crate::time_interval` holds the type `TimeInterval<T>`.

use crate::concurrency::{IScheduler, IStopwatch, SchedulerExt};
use crate::internal::{impl_sink_dispose, subscribe_sink, OperatorSink, Sink};
use crate::TimeInterval;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The sink of `TimeInterval` (`TimeInterval<TSource>._`): each value with the time since the
/// previous one (or since the subscription, for the first), measured with a stopwatch of the
/// scheduler started when the subscription runs.
struct TimeIntervalSink<T> {
    sink: Sink<TimeInterval<T>>,
    watch: RefCell<Option<Box<dyn IStopwatch>>>,
    last: Cell<Duration>,
}

impl<T: 'static> OperatorSink<T, TimeInterval<T>> for TimeIntervalSink<T> {
    fn sink(&self) -> &Sink<TimeInterval<T>> {
        &self.sink
    }
}

impl_sink_dispose!(TimeIntervalSink<T>);

impl<T: 'static> IObserver<T> for TimeIntervalSink<T> {
    fn on_next(&self, value: T) {
        let now = self.watch.borrow().as_ref().map_or(Duration::ZERO, |watch| watch.elapsed());
        let span = now.saturating_sub(self.last.get());
        self.last.set(now);
        self.sink.forward_on_next(TimeInterval::new(value, span));
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

/// The observable of `TimeInterval` (`TimeInterval<TSource>`).
struct TimeIntervalObservable<T> {
    source: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<TimeInterval<T>> for TimeIntervalObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeInterval<T>>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| Rc::new(TimeIntervalSink { sink: Sink::new(observer), watch: RefCell::new(None), last: Cell::new(Duration::ZERO) }),
            move |sink: &Rc<TimeIntervalSink<T>>| {
                *sink.watch.borrow_mut() = Some(scheduler.start_stopwatch());
                sink.last.set(Duration::ZERO);
                let upstream = source.subscribe(sink.clone());
                sink.sink.set_upstream(upstream);
            },
        )
    }
}

pub(crate) fn time_interval<T: 'static>(source: Rc<dyn IObservable<T>>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<TimeInterval<T>>> {
    Rc::new(TimeIntervalObservable { source, scheduler })
}

/// The methods of `TimeInterval` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! time_interval_methods {
    () => {
        /// Each value with the time since the previous one, measured on the scheduler of
        /// time-based operations (`TimeInterval()`).
        fn time_interval(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::TimeInterval<T>>> {
            crate::linq::time_interval_op::time_interval(self.observable(), crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// Each value with the time since the previous one, measured on `scheduler`
        /// (`TimeInterval(IScheduler)`).
        fn time_interval_on(
            &self,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::TimeInterval<T>>> {
            crate::linq::time_interval_op::time_interval(self.observable(), scheduler)
        }
    };
}
pub(crate) use time_interval_methods;
