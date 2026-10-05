use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

use super::{Dispatcher, DispatcherOperation, DispatcherPriority};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::BoxedValue;

thread_local! {
    static ACTIVE_TIMERS_COUNT: Cell<i32> = const { Cell::new(0) };
}

/// A timer that is integrated into the [`Dispatcher`] queues, and will be
/// processed after a given amount of time at a specified priority.
///
/// Timers belong to the thread of their dispatcher: they are created,
/// started and stopped there, and that is where `tick` handlers run. To
/// start a timer from another thread, post a job that creates it.
pub struct DispatcherTimer {
    weak_self: Weak<DispatcherTimer>,
    dispatcher: Arc<Dispatcher>,
    priority: DispatcherPriority,
    interval: Cell<Duration>,
    operation: RefCell<Option<DispatcherOperation>>,
    is_enabled: Cell<bool>,
    // used by Dispatcher
    due_time_in_ms: Cell<i64>,
    tick: HandlerList<dyn Fn(&DispatcherTimer)>,
    tag: RefCell<Option<BoxedValue>>,
}

impl DispatcherTimer {
    /// The number of started timers on the calling thread.
    #[allow(dead_code)]
    pub(crate) fn active_timers_count() -> i32 {
        ACTIVE_TIMERS_COUNT.with(Cell::get)
    }

    /// Creates a timer that uses the current thread's dispatcher to process
    /// the timer event at background priority.
    pub fn new() -> Rc<DispatcherTimer> {
        Self::with_interval(Duration::ZERO, DispatcherPriority::BACKGROUND, &Dispatcher::current_dispatcher())
    }

    /// Creates a timer that uses the current thread's dispatcher to process
    /// the timer event at the specified priority.
    pub fn with_priority(priority: DispatcherPriority) -> Rc<DispatcherTimer> {
        Self::with_interval(Duration::ZERO, priority, &Dispatcher::current_dispatcher())
    }

    /// Creates a timer that uses the specified dispatcher to process the
    /// timer event at the specified priority.
    pub fn with_priority_and_dispatcher(
        priority: DispatcherPriority,
        dispatcher: &Arc<Dispatcher>,
    ) -> Rc<DispatcherTimer> {
        Self::with_interval(Duration::ZERO, priority, dispatcher)
    }

    /// Creates a timer that uses the specified dispatcher to process the
    /// timer event at the specified priority after the specified interval.
    ///
    /// # Panics
    /// Panics when the priority is invalid or inactive, when the interval is
    /// longer than `i32::MAX` milliseconds and when called from a thread
    /// other than the dispatcher's.
    pub fn with_interval(
        interval: Duration,
        priority: DispatcherPriority,
        dispatcher: &Arc<Dispatcher>,
    ) -> Rc<DispatcherTimer> {
        dispatcher.verify_access();

        DispatcherPriority::validate(priority, "priority");
        if priority == DispatcherPriority::INACTIVE {
            panic!("Specified priority is not valid. (parameter 'priority')");
        }

        if interval.as_millis() > i32::MAX as u128 {
            panic!("interval: TimeSpan period must be less than or equal to Int32.MaxValue.");
        }

        Rc::new_cyclic(|weak_self| DispatcherTimer {
            weak_self: weak_self.clone(),
            dispatcher: dispatcher.clone(),
            priority,
            interval: Cell::new(interval),
            operation: RefCell::new(None),
            is_enabled: Cell::new(false),
            due_time_in_ms: Cell::new(0),
            tick: HandlerList::new(),
            tag: RefCell::new(None),
        })
    }

    /// Creates a timer that is bound to the current thread's dispatcher and
    /// will be processed after the specified interval, at the specified
    /// priority, calling the specified handler. The timer is started.
    pub fn with_callback(
        interval: Duration,
        priority: DispatcherPriority,
        callback: impl Fn(&DispatcherTimer) + 'static,
    ) -> Rc<DispatcherTimer> {
        Self::with_callback_and_dispatcher(interval, priority, &Dispatcher::current_dispatcher(), callback)
    }

    /// Creates a timer that is bound to the specified dispatcher and will be
    /// processed after the specified interval, at the specified priority,
    /// calling the specified handler. The timer is started.
    pub fn with_callback_and_dispatcher(
        interval: Duration,
        priority: DispatcherPriority,
        dispatcher: &Arc<Dispatcher>,
        callback: impl Fn(&DispatcherTimer) + 'static,
    ) -> Rc<DispatcherTimer> {
        let timer = Self::with_interval(interval, priority, dispatcher);
        timer.tick.add(Rc::new(callback));
        timer.start();
        timer
    }

    /// Gets the dispatcher this timer is associated with.
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }

    /// Gets whether the timer is running.
    pub fn is_enabled(&self) -> bool {
        self.is_enabled.get()
    }

    /// Starts or stops the timer.
    pub fn set_is_enabled(&self, value: bool) {
        if !value && self.is_enabled.get() {
            self.stop();
        } else if value && !self.is_enabled.get() {
            self.start();
        }
    }

    /// Gets the time between timer ticks.
    pub fn interval(&self) -> Duration {
        self.interval.get()
    }

    /// Sets the time between timer ticks.
    pub fn set_interval(&self, value: Duration) {
        self.dispatcher.verify_access();
        let mut update_os_timer = false;

        self.interval.set(value);

        if self.is_enabled.get() {
            self.due_time_in_ms.set(self.dispatcher.now() + value.as_millis() as i64);
            update_os_timer = true;
        }

        if update_os_timer {
            self.dispatcher.reschedule_timers();
        }
    }

    /// Starts the timer.
    pub fn start(&self) {
        self.dispatcher.verify_access();
        if !self.is_enabled.get() {
            self.is_enabled.set(true);
            ACTIVE_TIMERS_COUNT.with(|count| count.set(count.get() + 1));

            self.restart();
        }
    }

    /// Stops the timer.
    pub fn stop(&self) {
        self.dispatcher.verify_access();
        let mut update_os_timer = false;

        if self.is_enabled.get() {
            self.is_enabled.set(false);
            ACTIVE_TIMERS_COUNT.with(|count| count.set(count.get() - 1));
            update_os_timer = true;

            // If the operation is in the queue, abort it.
            let operation = self.operation.borrow_mut().take();
            if let Some(operation) = operation {
                operation.abort();
            }
        }

        if update_os_timer {
            self.dispatcher.remove_timer(self);
        }
    }

    /// Starts a new timer.
    ///
    /// `action` is called on each tick; the timer stops when it returns
    /// `false`. Disposing the returned value stops the timer.
    pub fn run(
        action: impl Fn() -> bool + 'static,
        interval: Duration,
        priority: DispatcherPriority,
    ) -> Rc<dyn IDisposable> {
        let timer = DispatcherTimer::with_priority(priority);
        timer.set_interval(interval);

        timer.tick.add(Rc::new(move |timer: &DispatcherTimer| {
            if !action() {
                timer.stop();
            }
        }));

        timer.start();

        Disposable::create(move || timer.stop())
    }

    /// Runs a method once, after the specified interval.
    ///
    /// Disposing the returned value cancels the timer.
    pub fn run_once(action: impl Fn() + 'static, interval: Duration, priority: DispatcherPriority) -> Rc<dyn IDisposable> {
        // One tick (100ns) when the interval is zero.
        let interval = if interval != Duration::ZERO { interval } else { Duration::from_nanos(100) };

        let timer = DispatcherTimer::with_priority(priority);
        timer.set_interval(interval);

        timer.tick.add(Rc::new(move |timer: &DispatcherTimer| {
            action();
            timer.stop();
        }));

        timer.start();

        Disposable::create(move || timer.stop())
    }

    /// Occurs when the specified timer interval has elapsed and the timer is
    /// enabled.
    pub fn tick(&self, handler: impl Fn(&DispatcherTimer) + 'static) -> Rc<dyn IDisposable> {
        let token = self.tick.add(Rc::new(handler));
        let timer = self.weak_self.clone();
        Disposable::create(move || {
            if let Some(timer) = timer.upgrade() {
                timer.tick.remove(token);
            }
        })
    }

    /// Any data that the caller wants to pass along with the timer.
    pub fn tag(&self) -> Option<BoxedValue> {
        self.tag.borrow().clone()
    }

    pub fn set_tag(&self, value: Option<BoxedValue>) {
        *self.tag.borrow_mut() = value;
    }

    fn restart(&self) {
        if self.operation.borrow().is_some() {
            // Timer has already been restarted, e.g. start was called from the tick handler.
            return;
        }
        let Some(this) = self.weak_self.upgrade() else {
            return;
        };

        // BeginInvoke a new operation.
        let timer = this.clone();
        let operation =
            self.dispatcher.invoke_async_local_with_priority(move || timer.fire_tick(), DispatcherPriority::INACTIVE);
        *self.operation.borrow_mut() = Some(operation);

        self.due_time_in_ms.set(self.dispatcher.now() + self.interval.get().as_millis() as i64);

        if self.interval.get().as_millis() == 0 && self.dispatcher.check_access() {
            // shortcut - just promote the item now
            self.promote();
        } else {
            self.dispatcher.add_timer(&this);
        }
    }

    // called from Dispatcher
    pub(crate) fn promote(&self) {
        // Simply promote the operation to it's desired priority.
        let operation = self.operation.borrow().clone();
        if let Some(operation) = operation {
            operation.set_priority(self.priority);
        }
    }

    pub(crate) fn due_time_in_ms(&self) -> i64 {
        self.due_time_in_ms.get()
    }

    /// Moves the due time when the dispatcher switches to another clock.
    pub(crate) fn shift_due_time(&self, shift: i64) {
        self.due_time_in_ms.set(self.due_time_in_ms.get().wrapping_add(shift));
    }

    fn fire_tick(&self) {
        // The operation has been invoked, so forget about it.
        let operation = self.operation.borrow_mut().take();
        drop(operation);

        // The dispatcher thread is calling us because item's priority
        // was changed from inactive to something else.
        for (_, handler) in self.tick.snapshot().iter() {
            handler(self);
        }

        // If we are still enabled, start the timer again.
        if self.is_enabled.get() {
            self.restart();
        }
    }
}
