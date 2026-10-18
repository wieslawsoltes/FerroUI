//! `TakeLast` (`Linq/Observable/TakeLast.cs`).

use crate::concurrency::{IScheduler, IStopwatch, SchedulerExt};
use crate::disposables::{Disposable, MultipleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use crate::TimeInterval;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// What the sink keeps of the source: its last `count` values (`TakeLast<TSource>.Count`), or the
/// values of its last `duration`, measured with a stopwatch of the timer scheduler
/// (`TakeLast<TSource>.Time`).
enum Window {
    Count(usize),
    Time { duration: Duration, watch: RefCell<Option<Box<dyn IStopwatch>>> },
}

/// The sink of `TakeLast` (`TakeLast<TSource>.Count._` and `Time._`): the values are kept until
/// the source completes, and then produced one by one on the loop scheduler, each in work of its
/// own, followed by the completion.
///
/// Upstream runs the loop as one long-running work item when the loop scheduler is long-running
/// (`ISchedulerLongRunning`); no scheduler of the port is, so the loop is always the recursive one.
struct TakeLastSink<T> {
    sink: Sink<T>,
    window: Window,
    loop_scheduler: Rc<dyn IScheduler>,
    queue: RefCell<VecDeque<TimeInterval<T>>>,
    loop_disposable: MultipleAssignmentDisposableValue,
    this: Weak<Self>,
}

impl<T: 'static> TakeLastSink<T> {
    fn elapsed(&self) -> Duration {
        match &self.window {
            Window::Count(_) => Duration::ZERO,
            Window::Time { watch, .. } => watch.borrow().as_ref().map_or(Duration::ZERO, |watch| watch.elapsed()),
        }
    }

    /// Drops the values that are no longer among the last ones (`Trim`, and the dequeue of the
    /// count form).
    fn trim(&self, now: Duration) {
        let mut queue = self.queue.borrow_mut();
        match &self.window {
            Window::Count(count) => {
                if queue.len() > *count {
                    queue.pop_front();
                }
            }
            Window::Time { duration, .. } => {
                while queue.front().is_some_and(|front| now.saturating_sub(front.interval) >= *duration) {
                    queue.pop_front();
                }
            }
        }
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

    /// One step of the loop (`LoopRec`): the next value, and the next step scheduled; or the
    /// completion.
    fn loop_rec(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        let next = self.queue.borrow_mut().pop_front();
        if let Some(item) = next {
            self.sink.forward_on_next(item.value);
            let this = self.clone();
            let next = scheduler.schedule(Box::new(move |inner: &Rc<dyn IScheduler>| this.loop_rec(inner)));
            self.loop_disposable.set_disposable(Some(next));
        } else {
            self.forward_on_completed();
        }
        Disposable::empty()
    }

    /// Assigns the first step of the loop unless the loop has assigned a later one already
    /// (`MultipleAssignmentDisposableValue.TrySetFirst`); a slot disposed already disposes it.
    fn set_first_loop_step(&self, first: Rc<dyn IDisposable>) {
        if self.loop_disposable.is_disposed() || self.loop_disposable.disposable().is_none() {
            self.loop_disposable.set_disposable(Some(first));
        }
    }
}

impl<T: 'static> IDisposable for TakeLastSink<T> {
    fn dispose(&self) {
        self.loop_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for TakeLastSink<T> {
    fn on_next(&self, value: T) {
        let now = self.elapsed();
        self.queue.borrow_mut().push_back(TimeInterval::new(value, now));
        self.trim(now);
    }

    fn on_error(&self, error: ObservableError) {
        self.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.dispose_upstream();
        if matches!(self.window, Window::Time { .. }) {
            let now = self.elapsed();
            self.trim(now);
        }
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let first = self.loop_scheduler.schedule(Box::new(move |inner: &Rc<dyn IScheduler>| this.loop_rec(inner)));
        self.set_first_loop_step(first);
    }
}

/// `TakeLast<TSource>.Count` and `TakeLast<TSource>.Time`; the timer scheduler is the one whose
/// stopwatch measures the time form.
struct TakeLast<T> {
    source: Rc<dyn IObservable<T>>,
    count: Option<usize>,
    duration: Duration,
    timer_scheduler: Option<Rc<dyn IScheduler>>,
    loop_scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for TakeLast<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let window = match self.count {
            Some(count) => Window::Count(count),
            None => Window::Time { duration: self.duration, watch: RefCell::new(None) },
        };
        let timer_scheduler = self.timer_scheduler.clone();
        let loop_scheduler = self.loop_scheduler.clone();
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| TakeLastSink {
                    sink: Sink::new(observer),
                    window,
                    loop_scheduler,
                    queue: RefCell::new(VecDeque::new()),
                    loop_disposable: MultipleAssignmentDisposableValue::new(),
                    this: this.clone(),
                })
            },
            move |sink: &Rc<TakeLastSink<T>>| {
                if let (Window::Time { watch, .. }, Some(timer_scheduler)) = (&sink.window, &timer_scheduler) {
                    *watch.borrow_mut() = Some(timer_scheduler.start_stopwatch());
                }
                sink.sink.set_upstream(source.subscribe(sink.clone()));
            },
        )
    }
}

pub(crate) fn take_last<T: 'static>(source: Rc<dyn IObservable<T>>, count: usize, loop_scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(TakeLast { source, count: Some(count), duration: Duration::ZERO, timer_scheduler: None, loop_scheduler })
}

pub(crate) fn take_last_time<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    duration: Duration,
    timer_scheduler: Rc<dyn IScheduler>,
    loop_scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(TakeLast { source, count: None, duration, timer_scheduler: Some(timer_scheduler), loop_scheduler })
}

/// The methods of `TakeLast` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! take_last_methods {
    () => {
        /// The last `count` values, produced when the source completes, on the scheduler of
        /// iteration (`TakeLast(int)`).
        fn take_last(&self, count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_last::take_last(self.observable(), count, crate::concurrency::SchedulerDefaults::iteration())
        }

        /// The last `count` values, produced on `scheduler` when the source completes
        /// (`TakeLast(int, IScheduler)`).
        fn take_last_on(&self, count: usize, scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_last::take_last(self.observable(), count, scheduler)
        }

        /// The values of the last `duration` before the completion, timed on the default scheduler
        /// of time-based operations and produced on the scheduler of iteration
        /// (`TakeLast(TimeSpan)`).
        fn take_last_time(&self, duration: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_last::take_last_time(
                self.observable(),
                duration,
                crate::concurrency::SchedulerDefaults::time_based_operations(),
                crate::concurrency::SchedulerDefaults::iteration(),
            )
        }

        /// The values of the last `duration` before the completion, timed on `scheduler` and
        /// produced on the scheduler of iteration (`TakeLast(TimeSpan, IScheduler)`).
        fn take_last_time_on(
            &self,
            duration: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_last::take_last_time(self.observable(), duration, scheduler, crate::concurrency::SchedulerDefaults::iteration())
        }

        /// The values of the last `duration` before the completion, timed on `timer_scheduler` and
        /// produced on `loop_scheduler` (`TakeLast(TimeSpan, IScheduler, IScheduler)`).
        fn take_last_time_on_with_loop(
            &self,
            duration: std::time::Duration,
            timer_scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
            loop_scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::take_last::take_last_time(self.observable(), duration, timer_scheduler, loop_scheduler)
        }
    };
}
pub(crate) use take_last_methods;
