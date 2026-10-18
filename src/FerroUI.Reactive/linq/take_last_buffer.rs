//! `TakeLastBuffer` (`Linq/Observable/TakeLastBuffer.cs`).

use crate::concurrency::{IScheduler, IStopwatch, SchedulerExt};
use crate::internal::{impl_sink_dispose, subscribe_sink, unary, OperatorSink, Sink};
use crate::TimeInterval;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

/// The sink of `TakeLastBuffer` with a count (`TakeLastBuffer<TSource>.Count._`): the last `count`
/// values, as one list when the source completes.
struct TakeLastBufferCountSink<T> {
    sink: Sink<Vec<T>>,
    count: usize,
    queue: RefCell<VecDeque<T>>,
}

impl<T: 'static> OperatorSink<T, Vec<T>> for TakeLastBufferCountSink<T> {
    fn sink(&self) -> &Sink<Vec<T>> {
        &self.sink
    }
}

impl_sink_dispose!(TakeLastBufferCountSink<T>);

impl<T: 'static> IObserver<T> for TakeLastBufferCountSink<T> {
    fn on_next(&self, value: T) {
        let mut queue = self.queue.borrow_mut();
        queue.push_back(value);
        if queue.len() > self.count {
            queue.pop_front();
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let res: Vec<T> = self.queue.borrow_mut().drain(..).collect();
        self.sink.forward_on_next(res);
        self.sink.forward_on_completed();
    }
}

pub(crate) fn take_last_buffer<T: 'static>(source: Rc<dyn IObservable<T>>, count: usize) -> Rc<dyn IObservable<Vec<T>>> {
    unary(source, move |observer| Rc::new(TakeLastBufferCountSink { sink: Sink::new(observer), count, queue: RefCell::new(VecDeque::new()) }))
}

/// The sink of `TakeLastBuffer` with a duration (`TakeLastBuffer<TSource>.Time._`): the values of
/// the last `duration` before the completion, as one list.
struct TakeLastBufferTimeSink<T> {
    sink: Sink<Vec<T>>,
    duration: Duration,
    queue: RefCell<VecDeque<TimeInterval<T>>>,
    watch: RefCell<Option<Box<dyn IStopwatch>>>,
}

impl<T: 'static> TakeLastBufferTimeSink<T> {
    fn elapsed(&self) -> Duration {
        self.watch.borrow().as_ref().map_or(Duration::ZERO, |watch| watch.elapsed())
    }

    fn trim(&self, now: Duration) {
        let mut queue = self.queue.borrow_mut();
        while queue.front().is_some_and(|front| now.saturating_sub(front.interval) >= self.duration) {
            queue.pop_front();
        }
    }
}

impl<T: 'static> OperatorSink<T, Vec<T>> for TakeLastBufferTimeSink<T> {
    fn sink(&self) -> &Sink<Vec<T>> {
        &self.sink
    }
}

impl_sink_dispose!(TakeLastBufferTimeSink<T>);

impl<T: 'static> IObserver<T> for TakeLastBufferTimeSink<T> {
    fn on_next(&self, value: T) {
        let now = self.elapsed();
        self.queue.borrow_mut().push_back(TimeInterval::new(value, now));
        self.trim(now);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let now = self.elapsed();
        self.trim(now);
        let res: Vec<T> = self.queue.borrow_mut().drain(..).map(|item| item.value).collect();
        self.sink.forward_on_next(res);
        self.sink.forward_on_completed();
    }
}

/// `TakeLastBuffer<TSource>.Time`.
struct TakeLastBufferTime<T> {
    source: Rc<dyn IObservable<T>>,
    duration: Duration,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<Vec<T>> for TakeLastBufferTime<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Vec<T>>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let duration = self.duration;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(TakeLastBufferTimeSink { sink: Sink::new(observer), duration, queue: RefCell::new(VecDeque::new()), watch: RefCell::new(None) })
            },
            move |sink: &Rc<TakeLastBufferTimeSink<T>>| {
                *sink.watch.borrow_mut() = Some(scheduler.start_stopwatch());
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

pub(crate) fn take_last_buffer_time<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    duration: Duration,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<Vec<T>>> {
    Rc::new(TakeLastBufferTime { source, duration, scheduler })
}

/// The methods of `TakeLastBuffer` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! take_last_buffer_methods {
    () => {
        /// The last `count` values as one list, produced when the source completes
        /// (`TakeLastBuffer(int)`).
        fn take_last_buffer(&self, count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::take_last_buffer::take_last_buffer(self.observable(), count)
        }

        /// The values of the last `duration` before the completion as one list, timed on the
        /// default scheduler of time-based operations (`TakeLastBuffer(TimeSpan)`).
        fn take_last_buffer_time(&self, duration: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::take_last_buffer::take_last_buffer_time(self.observable(), duration, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// The values of the last `duration` before the completion as one list, timed on
        /// `scheduler` (`TakeLastBuffer(TimeSpan, IScheduler)`).
        fn take_last_buffer_time_on(
            &self,
            duration: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::vec::Vec<T>>> {
            crate::linq::take_last_buffer::take_last_buffer_time(self.observable(), duration, scheduler)
        }
    };
}
pub(crate) use take_last_buffer_methods;
