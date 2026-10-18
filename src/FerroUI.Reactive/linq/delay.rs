//! `Delay` (`Linq/Observable/Delay.cs`): the notifications of the source shifted in time, by a
//! duration or until an absolute time (`Delay<TSource>`), or each value until a sequence of its own
//! produces or ends (`Delay<TSource, TDelay>`).
//!
//! The port has no long-running schedulers (`ISchedulerLongRunning`), so the time-based forms are
//! the forms of upstream for other schedulers (`Delay<TSource>.Base<TParent>.S`): a queue of values
//! with their due times on a stopwatch of the scheduler, drained by recursive scheduling.

use crate::concurrency::{IScheduler, IStopwatch, SchedulerExt};
use crate::disposables::{CompositeDisposable, SerialDisposableValue, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, OperatorSink, Sink};
use crate::TimeInterval;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// The delay of the time-based forms.
#[derive(Clone, Copy)]
pub(crate) enum DueTime {
    /// By a duration (`Delay<TSource>.Relative`).
    Relative(Duration),
    /// Until an absolute time (`Delay<TSource>.Absolute`).
    Absolute(SystemTime),
}

/// What one step of the drain loop does, decided with the state of the sink and acted on after.
enum DrainStep<T> {
    Value(T),
    Completed,
    Failed(ObservableError),
    Recurse(Duration),
    Idle,
}

/// The sink of the time-based forms (`Delay<TSource>.Base<TParent>.S` with `Absolute.S` and
/// `Relative.S`): each value is queued with its due time, the elapsed time of the stopwatch plus
/// the delay; the queue is drained on the scheduler, one value per scheduled step. An error is
/// forwarded at once and drops the queued values.
struct DelaySink<T> {
    this: Weak<DelaySink<T>>,
    sink: Sink<T>,
    scheduler: Rc<dyn IScheduler>,
    watch: RefCell<Option<Box<dyn IStopwatch>>>,
    cancelable: SerialDisposableValue,
    delay: Cell<Duration>,
    ready: Cell<bool>,
    active: Cell<bool>,
    running: Cell<bool>,
    queue: RefCell<VecDeque<TimeInterval<T>>>,
    has_completed: Cell<bool>,
    complete_at: Cell<Duration>,
    has_failed: Cell<bool>,
    exception: RefCell<Option<ObservableError>>,
}

impl<T: 'static> DelaySink<T> {
    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the sink is alive while it runs")
    }

    fn elapsed(&self) -> Duration {
        self.watch.borrow().as_ref().map_or(Duration::ZERO, |watch| watch.elapsed())
    }

    /// `Base<TParent>._.Run`: the stopwatch, the part of each form, then the subscription.
    fn run(&self, source: &Rc<dyn IObservable<T>>, due_time: DueTime) {
        *self.watch.borrow_mut() = Some(self.scheduler.start_stopwatch());
        match due_time {
            // `Relative.S.RunCore`.
            DueTime::Relative(due) => {
                self.ready.set(true);
                self.delay.set(due);
            }
            // `Absolute.S.RunCore`: the values queue until the due time, when the delay becomes
            // the time elapsed until then.
            DueTime::Absolute(due) => {
                self.ready.set(false);
                let this = self.this();
                let start = self.scheduler.schedule_action_at(due, move || this.start());
                self.cancelable.try_set_first(start);
            }
        }
        let upstream = source.subscribe(self.this());
        self.sink.set_upstream(upstream);
    }

    /// `Absolute.S.Start`.
    fn start(&self) {
        let mut next = Duration::ZERO;
        let mut should_run = false;
        {
            let delay = self.elapsed();
            self.delay.set(delay);
            let old_queue = std::mem::take(&mut *self.queue.borrow_mut());
            if let Some(first_due) = old_queue.front().map(|item| item.interval) {
                next = first_due;
                let mut queue = self.queue.borrow_mut();
                for item in old_queue {
                    queue.push_back(TimeInterval::new(item.value, item.interval + delay));
                }
                should_run = true;
                self.active.set(true);
            }
            self.ready.set(true);
        }
        if should_run {
            self.drain_queue(next);
        }
    }

    /// Schedules the drain loop after `next` (`DrainQueue(TimeSpan)`).
    fn drain_queue(&self, next: Duration) {
        let this = self.this();
        let drain = self.scheduler.schedule_recursive_after(next, move |recurse| this.drain(recurse));
        self.cancelable.set_disposable(Some(drain));
    }

    /// The drain loop (`DrainQueue(Action<S, TimeSpan>)`): the values that are due, the first one
    /// at once and the others in later steps (so that one step does not run for long), then the
    /// end, and a recursion to the due time of what is still queued.
    fn drain(&self, recurse: &dyn Fn(Duration)) {
        if self.has_failed.get() {
            return;
        }
        self.running.set(true);

        let mut should_yield = false;
        loop {
            let step = if self.has_failed.get() {
                self.running.set(false);
                DrainStep::Failed(self.exception.borrow().clone().expect("a failed sink has its error"))
            } else {
                let now = self.elapsed();
                let next_due = self.queue.borrow().front().map(|item| item.interval);
                if let Some(next_due) = next_due {
                    if next_due <= now && !should_yield {
                        let item = self.queue.borrow_mut().pop_front().expect("the queue has a value");
                        DrainStep::Value(item.value)
                    } else {
                        self.running.set(false);
                        DrainStep::Recurse(next_due.saturating_sub(now))
                    }
                } else if self.has_completed.get() {
                    let complete_at = self.complete_at.get();
                    if complete_at <= now && !should_yield {
                        DrainStep::Completed
                    } else {
                        self.running.set(false);
                        DrainStep::Recurse(complete_at.saturating_sub(now))
                    }
                } else {
                    self.running.set(false);
                    self.active.set(false);
                    DrainStep::Idle
                }
            };

            match step {
                DrainStep::Value(value) => {
                    self.sink.forward_on_next(value);
                    should_yield = true;
                }
                DrainStep::Completed => {
                    self.sink.forward_on_completed();
                    return;
                }
                DrainStep::Failed(error) => {
                    self.sink.forward_on_error(error);
                    return;
                }
                DrainStep::Recurse(due) => {
                    recurse(due);
                    return;
                }
                DrainStep::Idle => return,
            }
        }
    }
}

impl<T: 'static> OperatorSink<T, T> for DelaySink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IDisposable for DelaySink<T> {
    fn dispose(&self) {
        self.sink.dispose();
        self.cancelable.dispose();
    }
}

impl<T: 'static> IObserver<T> for DelaySink<T> {
    fn on_next(&self, value: T) {
        let next = self.elapsed() + self.delay.get();
        self.queue.borrow_mut().push_back(TimeInterval::new(value, next));
        let should_run = self.ready.get() && !self.active.get();
        self.active.set(true);
        if should_run {
            self.drain_queue(self.delay.get());
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.dispose_upstream();
        self.queue.borrow_mut().clear();
        *self.exception.borrow_mut() = Some(error.clone());
        self.has_failed.set(true);
        let should_run = !self.running.get();
        if should_run {
            self.sink.forward_on_error(error);
        }
    }

    fn on_completed(&self) {
        self.sink.dispose_upstream();
        let next = self.elapsed() + self.delay.get();
        self.complete_at.set(next);
        self.has_completed.set(true);
        let should_run = self.ready.get() && !self.active.get();
        self.active.set(true);
        if should_run {
            self.drain_queue(self.delay.get());
        }
    }
}

/// The observable of the time-based forms (`Delay<TSource>.Relative` and `Delay<TSource>.Absolute`).
struct DelayObservable<T> {
    source: Rc<dyn IObservable<T>>,
    due_time: DueTime,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for DelayObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (source, due_time, scheduler) = (self.source.clone(), self.due_time, self.scheduler.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| DelaySink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    scheduler,
                    watch: RefCell::new(None),
                    cancelable: SerialDisposableValue::new(),
                    delay: Cell::new(Duration::ZERO),
                    ready: Cell::new(false),
                    active: Cell::new(false),
                    running: Cell::new(false),
                    queue: RefCell::new(VecDeque::new()),
                    has_completed: Cell::new(false),
                    complete_at: Cell::new(Duration::ZERO),
                    has_failed: Cell::new(false),
                    exception: RefCell::new(None),
                })
            },
            move |sink: &Rc<DelaySink<T>>| sink.run(&source, due_time),
        )
    }
}

pub(crate) fn delay<T: 'static>(source: Rc<dyn IObservable<T>>, due_time: DueTime, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(DelayObservable { source, due_time, scheduler })
}

type DelaySelector<T, U> = Rc<dyn Fn(&T) -> Result<Rc<dyn IObservable<U>>, ObservableError>>;

/// The sink of the forms with a delay selector (`Delay<TSource, TDelay>.Base<TParent>._`): each
/// value waits for the first value or the end of its delay sequence; the sequence ends once the
/// source has ended and no value waits. A selector that fails ends the sequence with its error.
/// The subscription to the source (or to the subscription delay, which subscribes to the source)
/// is the sink's subscription upstream.
struct DelaySelectorSink<T, U> {
    this: Weak<DelaySelectorSink<T, U>>,
    sink: Sink<T>,
    delays: CompositeDisposable,
    selector: DelaySelector<T, U>,
    at_end: Cell<bool>,
}

impl<T: 'static, U: 'static> DelaySelectorSink<T, U> {
    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the sink is alive while it runs")
    }

    fn check_done(&self) {
        if self.at_end.get() && self.delays.count() == 0 {
            self.sink.forward_on_completed();
        }
    }
}

impl<T: 'static, U: 'static> OperatorSink<T, T> for DelaySelectorSink<T, U> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static, U: 'static> IDisposable for DelaySelectorSink<T, U> {
    fn dispose(&self) {
        self.sink.dispose_upstream();
        self.delays.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static, U: 'static> IObserver<T> for DelaySelectorSink<T, U> {
    fn on_next(&self, value: T) {
        let delay = match (self.selector)(&value) {
            Ok(delay) => delay,
            Err(error) => {
                self.sink.forward_on_error(error);
                return;
            }
        };

        let parent = self.this();
        let observer = Rc::new_cyclic(|this| DelayObserver {
            this: this.clone(),
            parent,
            value: RefCell::new(Some(value)),
            once: Cell::new(false),
            resource: SingleAssignmentDisposableValue::new(),
        });
        self.delays.add(observer.clone());
        let subscription = delay.subscribe(observer.clone());
        observer.resource.set_disposable(Some(subscription));
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.at_end.set(true);
        self.sink.dispose_upstream();
        self.check_done();
    }
}

/// The observer of the delay sequence of a value (`Delay<TSource, TDelay>.Base<TParent>._.DelayObserver`).
struct DelayObserver<T, U> {
    this: Weak<DelayObserver<T, U>>,
    parent: Rc<DelaySelectorSink<T, U>>,
    value: RefCell<Option<T>>,
    once: Cell<bool>,
    resource: SingleAssignmentDisposableValue,
}

impl<T: 'static, U: 'static> DelayObserver<T, U> {
    /// Forwards the value, leaves the group of waiting values, and ends the sequence if it was the
    /// last one after the end of the source.
    fn release(&self) {
        let value = self.value.borrow_mut().take();
        if let Some(value) = value {
            self.parent.sink.forward_on_next(value);
        }
        if let Some(this) = self.this.upgrade() {
            let this: Rc<dyn IDisposable> = this;
            self.parent.delays.remove(&this);
        }
        self.parent.check_done();
    }
}

impl<T: 'static, U: 'static> IObserver<U> for DelayObserver<T, U> {
    fn on_next(&self, _value: U) {
        if !self.once.replace(true) {
            self.release();
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        if !self.once.get() {
            self.release();
        }
    }
}

impl<T: 'static, U: 'static> IDisposable for DelayObserver<T, U> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observer of the subscription delay (`SelectorWithSubscriptionDelay._.SubscriptionDelayObserver`):
/// its first value or its end subscribes to the source.
struct SubscriptionDelayObserver<T, U> {
    parent: Rc<DelaySelectorSink<T, U>>,
    source: Rc<dyn IObservable<T>>,
    subscription: SerialDisposableValue,
}

impl<T: 'static, U: 'static> IObserver<U> for SubscriptionDelayObserver<T, U> {
    fn on_next(&self, _value: U) {
        self.subscription.set_disposable(Some(self.source.subscribe(self.parent.clone())));
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.subscription.set_disposable(Some(self.source.subscribe(self.parent.clone())));
    }
}

impl<T: 'static, U: 'static> IDisposable for SubscriptionDelayObserver<T, U> {
    fn dispose(&self) {
        self.subscription.dispose();
    }
}

/// The observable of the forms with a delay selector (`Delay<TSource, TDelay>.Selector` and
/// `Delay<TSource, TDelay>.SelectorWithSubscriptionDelay`).
struct DelaySelectorObservable<T, U> {
    source: Rc<dyn IObservable<T>>,
    subscription_delay: Option<Rc<dyn IObservable<U>>>,
    selector: DelaySelector<T, U>,
}

impl<T: 'static, U: 'static> IObservable<T> for DelaySelectorObservable<T, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (source, subscription_delay, selector) = (self.source.clone(), self.subscription_delay.clone(), self.selector.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| DelaySelectorSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    delays: CompositeDisposable::default(),
                    selector,
                    at_end: Cell::new(false),
                })
            },
            move |sink: &Rc<DelaySelectorSink<T, U>>| {
                sink.at_end.set(false);
                let subscription: Rc<dyn IDisposable> = match subscription_delay {
                    // `Selector._.RunCore`.
                    None => source.subscribe(sink.clone()),
                    // `SelectorWithSubscriptionDelay._.RunCore`.
                    Some(subscription_delay) => {
                        let delay_consumer = Rc::new(SubscriptionDelayObserver {
                            parent: sink.clone(),
                            source,
                            subscription: SerialDisposableValue::new(),
                        });
                        let first = subscription_delay.subscribe(delay_consumer.clone());
                        delay_consumer.subscription.try_set_first(first);
                        delay_consumer as Rc<dyn IDisposable>
                    }
                };
                sink.sink.set_upstream(subscription);
            },
        )
    }
}

pub(crate) fn delay_by<T: 'static, U: 'static>(
    source: Rc<dyn IObservable<T>>,
    subscription_delay: Option<Rc<dyn IObservable<U>>>,
    selector: DelaySelector<T, U>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(DelaySelectorObservable { source, subscription_delay, selector })
}

/// The methods of `Delay` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! delay_methods {
    () => {
        /// Shifts the notifications by `due_time`, on the scheduler of time-based operations; an
        /// error is not delayed (`Delay(TimeSpan)`).
        fn delay(&self, due_time: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay::delay(
                self.observable(),
                crate::linq::delay::DueTime::Relative(due_time),
                crate::concurrency::SchedulerDefaults::time_based_operations(),
            )
        }

        /// Shifts the notifications by `due_time` on `scheduler` (`Delay(TimeSpan, IScheduler)`).
        fn delay_on(
            &self,
            due_time: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay::delay(self.observable(), crate::linq::delay::DueTime::Relative(due_time), scheduler)
        }

        /// Shifts the notifications by the time from the subscription to `due_time`, on the
        /// scheduler of time-based operations (`Delay(DateTimeOffset)`).
        fn delay_at(&self, due_time: std::time::SystemTime) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay::delay(
                self.observable(),
                crate::linq::delay::DueTime::Absolute(due_time),
                crate::concurrency::SchedulerDefaults::time_based_operations(),
            )
        }

        /// Shifts the notifications by the time from the subscription to `due_time` on `scheduler`
        /// (`Delay(DateTimeOffset, IScheduler)`).
        fn delay_at_on(
            &self,
            due_time: std::time::SystemTime,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay::delay(self.observable(), crate::linq::delay::DueTime::Absolute(due_time), scheduler)
        }

        /// Delays each value until the sequence `delay_duration_selector` gives for it produces or
        /// ends (`Delay(Func<TSource, IObservable<TDelay>>)`).
        fn delay_by<U: 'static>(
            &self,
            delay_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay::delay_by(self.observable(), None, std::rc::Rc::new(move |value: &T| Ok(delay_duration_selector(value))))
        }

        /// [`delay_by`](Self::delay_by) with a selector that may fail; a failure ends the sequence
        /// with its error (`Delay(Func<TSource, IObservable<TDelay>>)` with a selector that throws).
        fn try_delay_by<U: 'static>(
            &self,
            delay_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay::delay_by(self.observable(), None, std::rc::Rc::new(delay_duration_selector))
        }

        /// Subscribes to the sequence when `subscription_delay` produces or ends, then delays each
        /// value until the sequence `delay_duration_selector` gives for it produces or ends
        /// (`Delay(IObservable<TDelay>, Func<TSource, IObservable<TDelay>>)`).
        fn delay_by_with_subscription_delay<U: 'static>(
            &self,
            subscription_delay: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            delay_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay::delay_by(
                self.observable(),
                Some(subscription_delay),
                std::rc::Rc::new(move |value: &T| Ok(delay_duration_selector(value))),
            )
        }

        /// [`delay_by_with_subscription_delay`](Self::delay_by_with_subscription_delay) with a
        /// selector that may fail.
        fn try_delay_by_with_subscription_delay<U: 'static>(
            &self,
            subscription_delay: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            delay_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay::delay_by(self.observable(), Some(subscription_delay), std::rc::Rc::new(delay_duration_selector))
        }
    };
}
pub(crate) use delay_methods;
