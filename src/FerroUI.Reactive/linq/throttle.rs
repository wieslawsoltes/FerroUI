//! `Throttle` (`Linq/Observable/Throttle.cs`): a value is forwarded once no other value has followed
//! it for a time (`Throttle<TSource>`), or until a sequence of its own produces (`Throttle<TSource,
//! TThrottle>`); the last value is forwarded when the source completes.

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::{SerialDisposableValue, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The sink of `Throttle(TimeSpan)` (`Throttle<TSource>._`): each value replaces the pending one
/// and restarts the timer; the timer forwards the pending value if no value came since.
struct ThrottleSink<T> {
    this: Weak<ThrottleSink<T>>,
    sink: Sink<T>,
    due_time: Duration,
    scheduler: Rc<dyn IScheduler>,
    value: RefCell<Option<T>>,
    has_value: Cell<bool>,
    id: Cell<u64>,
    serial_cancelable: SerialDisposableValue,
}

impl<T: 'static> ThrottleSink<T> {
    fn propagate(&self, current_id: u64) {
        if self.has_value.get() && self.id.get() == current_id {
            let value = self.value.borrow_mut().take();
            if let Some(value) = value {
                self.sink.forward_on_next(value);
            }
            self.has_value.set(false);
        }
    }
}

impl<T: 'static> OperatorSink<T, T> for ThrottleSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IDisposable for ThrottleSink<T> {
    fn dispose(&self) {
        self.serial_cancelable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for ThrottleSink<T> {
    fn on_next(&self, value: T) {
        self.has_value.set(true);
        *self.value.borrow_mut() = Some(value);
        let current_id = self.id.get().wrapping_add(1);
        self.id.set(current_id);

        self.serial_cancelable.set_disposable(None);
        let this = self.this.upgrade().expect("the sink is alive while it receives values");
        let timer = self.scheduler.schedule_action_after(self.due_time, move || this.propagate(current_id));
        self.serial_cancelable.set_disposable(Some(timer));
    }

    fn on_error(&self, error: ObservableError) {
        self.serial_cancelable.dispose();
        self.sink.forward_on_error(error);
        self.has_value.set(false);
        self.id.set(self.id.get().wrapping_add(1));
    }

    fn on_completed(&self) {
        self.serial_cancelable.dispose();
        if self.has_value.get() {
            let value = self.value.borrow_mut().take();
            if let Some(value) = value {
                self.sink.forward_on_next(value);
            }
        }
        self.sink.forward_on_completed();
        self.has_value.set(false);
        self.id.set(self.id.get().wrapping_add(1));
    }
}

/// The observable of `Throttle(TimeSpan)` (`Throttle<TSource>`).
struct ThrottleObservable<T> {
    source: Rc<dyn IObservable<T>>,
    due_time: Duration,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for ThrottleObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let (due_time, scheduler) = (self.due_time, self.scheduler.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| ThrottleSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    due_time,
                    scheduler,
                    value: RefCell::new(None),
                    has_value: Cell::new(false),
                    id: Cell::new(0),
                    serial_cancelable: SerialDisposableValue::new(),
                })
            },
            move |sink: &Rc<ThrottleSink<T>>| {
                let upstream = source.subscribe(sink.clone());
                sink.sink.set_upstream(upstream);
            },
        )
    }
}

pub(crate) fn throttle<T: 'static>(source: Rc<dyn IObservable<T>>, due_time: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(ThrottleObservable { source, due_time, scheduler })
}

type ThrottleSelector<T, U> = Rc<dyn Fn(&T) -> Result<Rc<dyn IObservable<U>>, ObservableError>>;

/// The sink of `Throttle` with a duration selector (`Throttle<TSource, TThrottle>._`): each value
/// replaces the pending one and subscribes to its own throttle sequence, whose first value or end
/// forwards the pending value if no value came since; a selector that fails ends the sequence with
/// its error.
struct ThrottleSelectorSink<T, U> {
    this: Weak<ThrottleSelectorSink<T, U>>,
    sink: Sink<T>,
    selector: ThrottleSelector<T, U>,
    value: RefCell<Option<T>>,
    has_value: Cell<bool>,
    id: Cell<u64>,
    serial_cancelable: SerialDisposableValue,
}

impl<T: 'static, U: 'static> ThrottleSelectorSink<T, U> {
    /// Forwards the pending value if it is the one of `current_id` (the first part of
    /// `ThrottleObserver.OnNext` and `OnCompleted`).
    fn propagate(&self, current_id: u64) {
        if self.has_value.get() && self.id.get() == current_id {
            let value = self.value.borrow_mut().take();
            if let Some(value) = value {
                self.sink.forward_on_next(value);
            }
        }
        self.has_value.set(false);
    }
}

impl<T: 'static, U: 'static> OperatorSink<T, T> for ThrottleSelectorSink<T, U> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static, U: 'static> IDisposable for ThrottleSelectorSink<T, U> {
    fn dispose(&self) {
        self.serial_cancelable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static, U: 'static> IObserver<T> for ThrottleSelectorSink<T, U> {
    fn on_next(&self, value: T) {
        let throttle = match (self.selector)(&value) {
            Ok(throttle) => throttle,
            Err(error) => {
                self.sink.forward_on_error(error);
                return;
            }
        };

        self.has_value.set(true);
        *self.value.borrow_mut() = Some(value);
        let current_id = self.id.get().wrapping_add(1);
        self.id.set(current_id);

        self.serial_cancelable.set_disposable(None);

        let parent = self.this.upgrade().expect("the sink is alive while it receives values");
        let inner = Rc::new(ThrottleObserver { parent, current_id, resource: SingleAssignmentDisposableValue::new() });
        inner.resource.set_disposable(Some(throttle.subscribe(inner.clone())));

        self.serial_cancelable.set_disposable(Some(inner));
    }

    fn on_error(&self, error: ObservableError) {
        self.serial_cancelable.dispose();
        self.sink.forward_on_error(error);
        self.has_value.set(false);
        self.id.set(self.id.get().wrapping_add(1));
    }

    fn on_completed(&self) {
        self.serial_cancelable.dispose();
        if self.has_value.get() {
            let value = self.value.borrow_mut().take();
            if let Some(value) = value {
                self.sink.forward_on_next(value);
            }
        }
        self.sink.forward_on_completed();
        self.has_value.set(false);
        self.id.set(self.id.get().wrapping_add(1));
    }
}

/// The observer of the throttle sequence of a value (`ThrottleObserver`). Upstream keeps a copy of
/// the value; the port forwards the pending value of the sink, which is that value whenever the
/// identifiers match.
struct ThrottleObserver<T, U> {
    parent: Rc<ThrottleSelectorSink<T, U>>,
    current_id: u64,
    resource: SingleAssignmentDisposableValue,
}

impl<T: 'static, U: 'static> IObserver<U> for ThrottleObserver<T, U> {
    fn on_next(&self, _value: U) {
        self.parent.propagate(self.current_id);
        self.dispose();
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.parent.propagate(self.current_id);
        self.dispose();
    }
}

impl<T: 'static, U: 'static> IDisposable for ThrottleObserver<T, U> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observable of `Throttle` with a duration selector (`Throttle<TSource, TThrottle>`).
struct ThrottleSelectorObservable<T, U> {
    source: Rc<dyn IObservable<T>>,
    selector: ThrottleSelector<T, U>,
}

impl<T: 'static, U: 'static> IObservable<T> for ThrottleSelectorObservable<T, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let selector = self.selector.clone();
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| ThrottleSelectorSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    selector,
                    value: RefCell::new(None),
                    has_value: Cell::new(false),
                    id: Cell::new(0),
                    serial_cancelable: SerialDisposableValue::new(),
                })
            },
            move |sink: &Rc<ThrottleSelectorSink<T, U>>| {
                let upstream = source.subscribe(sink.clone());
                sink.sink.set_upstream(upstream);
            },
        )
    }
}

pub(crate) fn throttle_by<T: 'static, U: 'static>(source: Rc<dyn IObservable<T>>, selector: ThrottleSelector<T, U>) -> Rc<dyn IObservable<T>> {
    Rc::new(ThrottleSelectorObservable { source, selector })
}

/// The methods of `Throttle` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! throttle_methods {
    () => {
        /// Forwards a value once `due_time` has passed without another value, on the scheduler of
        /// time-based operations (`Throttle(TimeSpan)`).
        fn throttle(&self, due_time: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::throttle::throttle(self.observable(), due_time, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// Forwards a value once `due_time` has passed on `scheduler` without another value
        /// (`Throttle(TimeSpan, IScheduler)`).
        fn throttle_on(
            &self,
            due_time: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::throttle::throttle(self.observable(), due_time, scheduler)
        }

        /// Forwards a value once the sequence `throttle_duration_selector` gives for it produces a
        /// value or ends without another value having come
        /// (`Throttle(Func<TSource, IObservable<TThrottle>>)`).
        fn throttle_by<U: 'static>(
            &self,
            throttle_duration_selector: impl Fn(&T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::throttle::throttle_by(self.observable(), std::rc::Rc::new(move |value: &T| Ok(throttle_duration_selector(value))))
        }

        /// [`throttle_by`](Self::throttle_by) with a selector that may fail; a failure ends the
        /// sequence with its error (`Throttle(Func<TSource, IObservable<TThrottle>>)` with a
        /// selector that throws).
        fn try_throttle_by<U: 'static>(
            &self,
            throttle_duration_selector: impl Fn(&T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::throttle::throttle_by(self.observable(), std::rc::Rc::new(throttle_duration_selector))
        }
    };
}
pub(crate) use throttle_methods;
