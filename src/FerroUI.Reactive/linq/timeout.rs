//! `Timeout` (`Linq/Observable/Timeout.cs`): when the source does not produce in time, the
//! subscription switches to another sequence (by default one that fails with a
//! [`TimeoutException`]). The time is a duration restarted by each value (`Timeout<TSource>.Relative`),
//! an absolute time (`Timeout<TSource>.Absolute`), or the first value or end of a sequence given for
//! the subscription and for each value (`Timeout<TSource, TTimeout>`).

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::{MultipleAssignmentDisposableValue, SerialDisposableValue, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// The error of a sequence that timed out (`System.TimeoutException`), which the overloads of
/// `Timeout` without another sequence end with.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutException;

impl fmt::Display for TimeoutException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("The operation has timed out.")
    }
}

impl std::error::Error for TimeoutException {}

/// The sequence the overloads without another sequence switch to
/// (`Observable.Throw<TSource>(new TimeoutException())`).
pub(crate) fn timeout_error<T: 'static>() -> Rc<dyn IObservable<T>> {
    crate::linq::Observable::throw(Rc::new(TimeoutException))
}

/// The index of a sink that has switched to the other sequence or ended (`long.MaxValue`).
const DONE: u64 = u64::MAX;

/// An observer that forwards to the observer downstream of a sink (`Sink.GetForwarder()`).
struct Forwarder<T, S> {
    target: Rc<S>,
    _marker: PhantomData<fn(T)>,
}

impl<T: 'static, S: OperatorSink<T, T> + 'static> IObserver<T> for Forwarder<T, S> {
    fn on_next(&self, value: T) {
        self.target.sink().forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.target.sink().forward_on_error(error);
    }

    fn on_completed(&self) {
        self.target.sink().forward_on_completed();
    }
}

fn forwarder<T: 'static, S: OperatorSink<T, T> + 'static>(target: Rc<S>) -> Rc<dyn IObserver<T>> {
    Rc::new(Forwarder { target, _marker: PhantomData })
}

/// The sink of `Timeout(TimeSpan)` (`Timeout<TSource>.Relative._`): a timer per value; the timer of
/// the latest value switches to the other sequence.
struct RelativeSink<T> {
    this: Weak<RelativeSink<T>>,
    sink: Sink<T>,
    due_time: Duration,
    other: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
    index: Cell<u64>,
    main_disposable: SingleAssignmentDisposableValue,
    other_disposable: SingleAssignmentDisposableValue,
    timer_disposable: MultipleAssignmentDisposableValue,
}

impl<T: 'static> RelativeSink<T> {
    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the sink is alive while it runs")
    }

    fn create_timer(&self, idx: u64) {
        // `TrySetMultiple(ref _timerDisposable, null)`: nothing to do once the sink is disposed.
        if self.timer_disposable.is_disposed() {
            return;
        }
        self.timer_disposable.set_disposable(None);
        let this = self.this();
        let timer = self.scheduler.schedule_action_after(self.due_time, move || this.timeout(idx));
        self.timer_disposable.set_disposable(Some(timer));
    }

    fn timeout(&self, idx: u64) {
        if self.index.get() == idx {
            self.index.set(DONE);
            self.main_disposable.dispose();
            let subscription = self.other.subscribe(forwarder(self.this()));
            self.other_disposable.set_disposable(Some(subscription));
        }
    }
}

impl<T: 'static> OperatorSink<T, T> for RelativeSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IDisposable for RelativeSink<T> {
    fn dispose(&self) {
        self.main_disposable.dispose();
        self.other_disposable.dispose();
        self.timer_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for RelativeSink<T> {
    fn on_next(&self, value: T) {
        let idx = self.index.get();
        if idx != DONE {
            self.index.set(idx + 1);
            // The timer is disposed but stays in its slot, which holds the next timer.
            if let Some(timer) = self.timer_disposable.disposable() {
                timer.dispose();
            }
            self.sink.forward_on_next(value);
            self.create_timer(idx + 1);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if self.index.replace(DONE) != DONE {
            self.timer_disposable.dispose();
            self.sink.forward_on_error(error);
        }
    }

    fn on_completed(&self) {
        if self.index.replace(DONE) != DONE {
            self.timer_disposable.dispose();
            self.sink.forward_on_completed();
        }
    }
}

/// The observable of `Timeout(TimeSpan)` (`Timeout<TSource>.Relative`).
struct RelativeObservable<T> {
    source: Rc<dyn IObservable<T>>,
    due_time: Duration,
    other: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for RelativeObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let (due_time, other, scheduler) = (self.due_time, self.other.clone(), self.scheduler.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| RelativeSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    due_time,
                    other,
                    scheduler,
                    index: Cell::new(0),
                    main_disposable: SingleAssignmentDisposableValue::new(),
                    other_disposable: SingleAssignmentDisposableValue::new(),
                    timer_disposable: MultipleAssignmentDisposableValue::new(),
                })
            },
            move |sink: &Rc<RelativeSink<T>>| {
                sink.create_timer(0);
                sink.main_disposable.set_disposable(Some(source.subscribe(sink.clone())));
            },
        )
    }
}

pub(crate) fn timeout_relative<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    due_time: Duration,
    other: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(RelativeObservable { source, due_time, other, scheduler })
}

/// The sink of `Timeout(DateTimeOffset)` (`Timeout<TSource>.Absolute._`): one timer, whose
/// subscription is the sink's subscription upstream; the subscription to the source, then to the
/// other sequence, is held in a serial slot.
struct AbsoluteSink<T> {
    this: Weak<AbsoluteSink<T>>,
    sink: Sink<T>,
    other: Rc<dyn IObservable<T>>,
    serial_disposable: SerialDisposableValue,
    wip: Cell<u32>,
}

impl<T: 'static> AbsoluteSink<T> {
    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the sink is alive while it runs")
    }

    fn switch_to_other(&self) {
        let subscription = self.other.subscribe(forwarder(self.this()));
        self.serial_disposable.set_disposable(Some(subscription));
    }

    fn timeout(&self) {
        let wip = self.wip.get() + 1;
        self.wip.set(wip);
        if wip == 1 {
            self.switch_to_other();
        }
    }
}

impl<T: 'static> OperatorSink<T, T> for AbsoluteSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IDisposable for AbsoluteSink<T> {
    fn dispose(&self) {
        self.serial_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for AbsoluteSink<T> {
    fn on_next(&self, value: T) {
        if self.wip.get() == 0 {
            self.wip.set(1);
            self.sink.forward_on_next(value);
            let wip = self.wip.get() - 1;
            self.wip.set(wip);
            if wip != 0 {
                self.switch_to_other();
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        if self.wip.get() == 0 {
            self.wip.set(1);
            self.sink.forward_on_error(error);
        }
    }

    fn on_completed(&self) {
        if self.wip.get() == 0 {
            self.wip.set(1);
            self.sink.forward_on_completed();
        }
    }
}

/// The observable of `Timeout(DateTimeOffset)` (`Timeout<TSource>.Absolute`).
struct AbsoluteObservable<T> {
    source: Rc<dyn IObservable<T>>,
    due_time: SystemTime,
    other: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for AbsoluteObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let (due_time, other, scheduler) = (self.due_time, self.other.clone(), self.scheduler.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| AbsoluteSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    other,
                    serial_disposable: SerialDisposableValue::new(),
                    wip: Cell::new(0),
                })
            },
            move |sink: &Rc<AbsoluteSink<T>>| {
                let target = sink.clone();
                sink.sink.set_upstream(scheduler.schedule_action_at(due_time, move || target.timeout()));
                sink.serial_disposable.set_disposable(Some(source.subscribe(sink.clone())));
            },
        )
    }
}

pub(crate) fn timeout_absolute<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    due_time: SystemTime,
    other: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(AbsoluteObservable { source, due_time, other, scheduler })
}

type TimeoutSelector<T, U> = Rc<dyn Fn(&T) -> Result<Rc<dyn IObservable<U>>, ObservableError>>;

/// The sink of `Timeout` with timeout sequences (`Timeout<TSource, TTimeout>._`): the first value or
/// the end of the timeout sequence of the subscription, then of the latest value, switches to the
/// other sequence; its error ends the sequence.
struct SelectorSink<T, U> {
    this: Weak<SelectorSink<T, U>>,
    sink: Sink<T>,
    selector: TimeoutSelector<T, U>,
    other: Rc<dyn IObservable<T>>,
    source_disposable: SerialDisposableValue,
    timer_disposable: SerialDisposableValue,
    index: Cell<u64>,
}

impl<T: 'static, U: 'static> SelectorSink<T, U> {
    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the sink is alive while it runs")
    }

    fn set_timer(&self, timeout: Rc<dyn IObservable<U>>, idx: u64) {
        let timeout_observer = Rc::new(TimeoutObserver { parent: self.this(), id: idx, resource: SingleAssignmentDisposableValue::new() });
        // `TrySetSerial`: the previous timer is disposed; once the sink is disposed, the new one is
        // disposed at once and not subscribed.
        let disposed = self.timer_disposable.is_disposed();
        self.timer_disposable.set_disposable(Some(timeout_observer.clone()));
        if !disposed {
            let subscription = timeout.subscribe(timeout_observer.clone());
            timeout_observer.resource.set_disposable(Some(subscription));
        }
    }

    fn timeout(&self, idx: u64) {
        if self.index.get() == idx {
            self.index.set(DONE);
            let subscription = self.other.subscribe(forwarder(self.this()));
            self.source_disposable.set_disposable(Some(subscription));
        }
    }

    fn timeout_error(&self, idx: u64, error: ObservableError) -> bool {
        if self.index.get() == idx {
            self.index.set(DONE);
            self.sink.forward_on_error(error);
            return true;
        }
        false
    }
}

impl<T: 'static, U: 'static> OperatorSink<T, T> for SelectorSink<T, U> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static, U: 'static> IDisposable for SelectorSink<T, U> {
    fn dispose(&self) {
        self.source_disposable.dispose();
        self.timer_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static, U: 'static> IObserver<T> for SelectorSink<T, U> {
    fn on_next(&self, value: T) {
        let idx = self.index.get();
        if idx != DONE {
            self.index.set(idx + 1);
            // The timer is disposed but stays in its slot, which holds the next timer.
            if let Some(timer) = self.timer_disposable.disposable() {
                timer.dispose();
            }
            // Deviation (DEVIATIONS.md, Reactive): upstream calls the selector after it forwards
            // the value; the port moves the value into the observer, so the selector runs first
            // and its result is used after the value is forwarded, in the order of upstream.
            let timeout_source = (self.selector)(&value);
            self.sink.forward_on_next(value);
            match timeout_source {
                Ok(timeout_source) => self.set_timer(timeout_source, idx + 1),
                Err(error) => self.sink.forward_on_error(error),
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        if self.index.replace(DONE) != DONE {
            self.sink.forward_on_error(error);
        }
    }

    fn on_completed(&self) {
        if self.index.replace(DONE) != DONE {
            self.sink.forward_on_completed();
        }
    }
}

/// The observer of a timeout sequence (`Timeout<TSource, TTimeout>._.TimeoutObserver`).
struct TimeoutObserver<T, U> {
    parent: Rc<SelectorSink<T, U>>,
    id: u64,
    resource: SingleAssignmentDisposableValue,
}

impl<T: 'static, U: 'static> IObserver<U> for TimeoutObserver<T, U> {
    fn on_next(&self, _value: U) {
        self.on_completed();
    }

    fn on_error(&self, error: ObservableError) {
        if !self.parent.timeout_error(self.id, error) {
            self.dispose();
        }
    }

    fn on_completed(&self) {
        self.parent.timeout(self.id);
        self.dispose();
    }
}

impl<T: 'static, U: 'static> IDisposable for TimeoutObserver<T, U> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observable of `Timeout` with timeout sequences (`Timeout<TSource, TTimeout>`).
struct SelectorObservable<T, U> {
    source: Rc<dyn IObservable<T>>,
    first_timeout: Rc<dyn IObservable<U>>,
    selector: TimeoutSelector<T, U>,
    other: Rc<dyn IObservable<T>>,
}

impl<T: 'static, U: 'static> IObservable<T> for SelectorObservable<T, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (source, first_timeout) = (self.source.clone(), self.first_timeout.clone());
        let (selector, other) = (self.selector.clone(), self.other.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| SelectorSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    selector,
                    other,
                    source_disposable: SerialDisposableValue::new(),
                    timer_disposable: SerialDisposableValue::new(),
                    index: Cell::new(0),
                })
            },
            move |sink: &Rc<SelectorSink<T, U>>| {
                sink.set_timer(first_timeout, 0);
                sink.source_disposable.try_set_first(source.subscribe(sink.clone()));
            },
        )
    }
}

pub(crate) fn timeout_by<T: 'static, U: 'static>(
    source: Rc<dyn IObservable<T>>,
    first_timeout: Rc<dyn IObservable<U>>,
    selector: TimeoutSelector<T, U>,
    other: Rc<dyn IObservable<T>>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(SelectorObservable { source, first_timeout, selector, other })
}

/// The methods of `Timeout` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! timeout_methods {
    () => {
        /// Ends with a `TimeoutException` when `due_time`
        /// passes without a value (from the subscription, then from each value), on the scheduler
        /// of time-based operations (`Timeout(TimeSpan)`).
        fn timeout(&self, due_time: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_relative(
                self.observable(),
                due_time,
                crate::linq::timeout::timeout_error(),
                crate::concurrency::SchedulerDefaults::time_based_operations(),
            )
        }

        /// [`timeout`](Self::timeout) on `scheduler` (`Timeout(TimeSpan, IScheduler)`).
        fn timeout_on(
            &self,
            due_time: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_relative(self.observable(), due_time, crate::linq::timeout::timeout_error(), scheduler)
        }

        /// Switches to `other` when `due_time` passes without a value, on the scheduler of
        /// time-based operations (`Timeout(TimeSpan, IObservable<TSource>)`).
        fn timeout_with(
            &self,
            due_time: std::time::Duration,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_relative(
                self.observable(),
                due_time,
                other,
                crate::concurrency::SchedulerDefaults::time_based_operations(),
            )
        }

        /// [`timeout_with`](Self::timeout_with) on `scheduler`
        /// (`Timeout(TimeSpan, IObservable<TSource>, IScheduler)`).
        fn timeout_with_on(
            &self,
            due_time: std::time::Duration,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_relative(self.observable(), due_time, other, scheduler)
        }

        /// Ends with a `TimeoutException` at `due_time`
        /// unless the sequence ended before, on the scheduler of time-based operations
        /// (`Timeout(DateTimeOffset)`).
        fn timeout_at(&self, due_time: std::time::SystemTime) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_absolute(
                self.observable(),
                due_time,
                crate::linq::timeout::timeout_error(),
                crate::concurrency::SchedulerDefaults::time_based_operations(),
            )
        }

        /// [`timeout_at`](Self::timeout_at) on `scheduler` (`Timeout(DateTimeOffset, IScheduler)`).
        fn timeout_at_on(
            &self,
            due_time: std::time::SystemTime,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_absolute(self.observable(), due_time, crate::linq::timeout::timeout_error(), scheduler)
        }

        /// Switches to `other` at `due_time` unless the sequence ended before, on the scheduler of
        /// time-based operations (`Timeout(DateTimeOffset, IObservable<TSource>)`).
        fn timeout_at_with(
            &self,
            due_time: std::time::SystemTime,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_absolute(
                self.observable(),
                due_time,
                other,
                crate::concurrency::SchedulerDefaults::time_based_operations(),
            )
        }

        /// [`timeout_at_with`](Self::timeout_at_with) on `scheduler`
        /// (`Timeout(DateTimeOffset, IObservable<TSource>, IScheduler)`).
        fn timeout_at_with_on(
            &self,
            due_time: std::time::SystemTime,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_absolute(self.observable(), due_time, other, scheduler)
        }

        /// Ends with a `TimeoutException` when the
        /// sequence `timeout_duration_selector` gives for the latest value produces or ends first
        /// (`Timeout(Func<TSource, IObservable<TTimeout>>)`).
        fn timeout_by<U: 'static>(
            &self,
            timeout_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_by(
                self.observable(),
                crate::linq::Observable::never::<U>(),
                std::rc::Rc::new(move |value: &T| Ok(timeout_duration_selector(value))),
                crate::linq::timeout::timeout_error(),
            )
        }

        /// [`timeout_by`](Self::timeout_by) with a selector that may fail; a failure ends the
        /// sequence with its error (`Timeout(Func<TSource, IObservable<TTimeout>>)` with a selector
        /// that throws).
        fn try_timeout_by<U: 'static>(
            &self,
            timeout_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_by(
                self.observable(),
                crate::linq::Observable::never::<U>(),
                std::rc::Rc::new(timeout_duration_selector),
                crate::linq::timeout::timeout_error(),
            )
        }

        /// Switches to `other` when the sequence `timeout_duration_selector` gives for the latest
        /// value produces or ends first (`Timeout(Func<TSource, IObservable<TTimeout>>,
        /// IObservable<TSource>)`).
        fn timeout_by_with<U: 'static>(
            &self,
            timeout_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_by(
                self.observable(),
                crate::linq::Observable::never::<U>(),
                std::rc::Rc::new(move |value: &T| Ok(timeout_duration_selector(value))),
                other,
            )
        }

        /// [`timeout_by_with`](Self::timeout_by_with) with a selector that may fail.
        fn try_timeout_by_with<U: 'static>(
            &self,
            timeout_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_by(
                self.observable(),
                crate::linq::Observable::never::<U>(),
                std::rc::Rc::new(timeout_duration_selector),
                other,
            )
        }

        /// Ends with a `TimeoutException` when
        /// `first_timeout` produces or ends before the first value, then when the sequence
        /// `timeout_duration_selector` gives for the latest value does
        /// (`Timeout(IObservable<TTimeout>, Func<TSource, IObservable<TTimeout>>)`).
        fn timeout_by_first<U: 'static>(
            &self,
            first_timeout: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            timeout_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_by(
                self.observable(),
                first_timeout,
                std::rc::Rc::new(move |value: &T| Ok(timeout_duration_selector(value))),
                crate::linq::timeout::timeout_error(),
            )
        }

        /// [`timeout_by_first`](Self::timeout_by_first) with a selector that may fail.
        fn try_timeout_by_first<U: 'static>(
            &self,
            first_timeout: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            timeout_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_by(
                self.observable(),
                first_timeout,
                std::rc::Rc::new(timeout_duration_selector),
                crate::linq::timeout::timeout_error(),
            )
        }

        /// Switches to `other` when `first_timeout` produces or ends before the first value, then
        /// when the sequence `timeout_duration_selector` gives for the latest value does
        /// (`Timeout(IObservable<TTimeout>, Func<TSource, IObservable<TTimeout>>, IObservable<TSource>)`).
        fn timeout_by_first_with<U: 'static>(
            &self,
            first_timeout: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            timeout_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_by(
                self.observable(),
                first_timeout,
                std::rc::Rc::new(move |value: &T| Ok(timeout_duration_selector(value))),
                other,
            )
        }

        /// [`timeout_by_first_with`](Self::timeout_by_first_with) with a selector that may fail.
        fn try_timeout_by_first_with<U: 'static>(
            &self,
            first_timeout: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            timeout_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::timeout::timeout_by(self.observable(), first_timeout, std::rc::Rc::new(timeout_duration_selector), other)
        }
    };
}
pub(crate) use timeout_methods;
