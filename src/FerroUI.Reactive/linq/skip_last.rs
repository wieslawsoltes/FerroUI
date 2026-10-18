//! `SkipLast` (`Linq/Observable/SkipLast.cs`).

use crate::concurrency::{IScheduler, IStopwatch, SchedulerExt};
use crate::internal::{impl_sink_dispose, subscribe_sink, unary, OperatorSink, Sink};
use crate::TimeInterval;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

/// The sink of `SkipLast` with a count (`SkipLast<TSource>.Count._`): a value is forwarded once
/// `count` values have arrived after it.
struct SkipLastCountSink<T> {
    sink: Sink<T>,
    count: usize,
    queue: RefCell<VecDeque<T>>,
}

impl<T: 'static> OperatorSink<T, T> for SkipLastCountSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(SkipLastCountSink<T>);

impl<T: 'static> IObserver<T> for SkipLastCountSink<T> {
    fn on_next(&self, value: T) {
        let ready = {
            let mut queue = self.queue.borrow_mut();
            queue.push_back(value);
            if queue.len() > self.count {
                queue.pop_front()
            } else {
                None
            }
        };
        if let Some(value) = ready {
            self.sink.forward_on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn skip_last<T: 'static>(source: Rc<dyn IObservable<T>>, count: usize) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| Rc::new(SkipLastCountSink { sink: Sink::new(observer), count, queue: RefCell::new(VecDeque::new()) }))
}

/// The sink of `SkipLast` with a duration (`SkipLast<TSource>.Time._`): a value is forwarded once
/// `duration` has passed since it arrived, when a later value or the completion shows it.
struct SkipLastTimeSink<T> {
    sink: Sink<T>,
    duration: Duration,
    queue: RefCell<VecDeque<TimeInterval<T>>>,
    watch: RefCell<Option<Box<dyn IStopwatch>>>,
}

impl<T: 'static> SkipLastTimeSink<T> {
    fn elapsed(&self) -> Duration {
        self.watch.borrow().as_ref().map_or(Duration::ZERO, |watch| watch.elapsed())
    }

    /// The oldest value that has been held for `duration` at `now`, taken out of the queue.
    fn next_ready(&self, now: Duration) -> Option<T> {
        let mut queue = self.queue.borrow_mut();
        match queue.front() {
            Some(front) if now.saturating_sub(front.interval) >= self.duration => queue.pop_front().map(|item| item.value),
            _ => None,
        }
    }

    fn drain(&self, now: Duration) {
        while let Some(value) = self.next_ready(now) {
            self.sink.forward_on_next(value);
        }
    }
}

impl_sink_dispose!(SkipLastTimeSink<T>);

impl<T: 'static> OperatorSink<T, T> for SkipLastTimeSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IObserver<T> for SkipLastTimeSink<T> {
    fn on_next(&self, value: T) {
        let now = self.elapsed();
        self.queue.borrow_mut().push_back(TimeInterval::new(value, now));
        self.drain(now);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let now = self.elapsed();
        self.drain(now);
        self.sink.forward_on_completed();
    }
}

/// `SkipLast<TSource>.Time`.
struct SkipLastTime<T> {
    source: Rc<dyn IObservable<T>>,
    duration: Duration,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for SkipLastTime<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let duration = self.duration;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(SkipLastTimeSink { sink: Sink::new(observer), duration, queue: RefCell::new(VecDeque::new()), watch: RefCell::new(None) })
            },
            move |sink: &Rc<SkipLastTimeSink<T>>| {
                *sink.watch.borrow_mut() = Some(scheduler.start_stopwatch());
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

pub(crate) fn skip_last_time<T: 'static>(source: Rc<dyn IObservable<T>>, duration: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(SkipLastTime { source, duration, scheduler })
}

/// The methods of `SkipLast` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! skip_last_methods {
    () => {
        /// The values but the last `count` (`SkipLast(int)`); a value is produced once `count`
        /// values have followed it.
        fn skip_last(&self, count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_last::skip_last(self.observable(), count)
        }

        /// The values but those of the last `duration` before the completion, timed on the default
        /// scheduler of time-based operations (`SkipLast(TimeSpan)`).
        fn skip_last_time(&self, duration: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_last::skip_last_time(self.observable(), duration, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// The values but those of the last `duration` before the completion, timed on `scheduler`
        /// (`SkipLast(TimeSpan, IScheduler)`).
        fn skip_last_time_on(
            &self,
            duration: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::skip_last::skip_last_time(self.observable(), duration, scheduler)
        }
    };
}
pub(crate) use skip_last_methods;
