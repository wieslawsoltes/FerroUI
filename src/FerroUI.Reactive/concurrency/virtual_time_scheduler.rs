//! `VirtualTimeSchedulerBase<TAbsolute, TRelative>` and `VirtualTimeScheduler<TAbsolute, TRelative>`
//! (`VirtualTimeScheduler.cs`).

use super::{IScheduler, IStopwatch, IStopwatchProvider, ScheduledAction, ScheduledItem};
use crate::internal::PriorityQueue;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// The time of a virtual-time scheduler: what the abstract members of upstream's
/// `VirtualTimeSchedulerBase` state (`Add`, `ToDateTimeOffset`, `ToRelative`), and how an absolute
/// schedule treats a due time (`ScheduleAbsolute`, which a derived scheduler may override).
pub trait VirtualTimeClock: 'static {
    /// The absolute time (`TAbsolute`).
    type Absolute: Ord + Copy + 'static;
    /// The relative time (`TRelative`).
    type Relative: Copy + 'static;

    /// `absolute` moved by `relative` (`Add`).
    fn add(absolute: Self::Absolute, relative: Self::Relative) -> Self::Absolute;

    /// The absolute time as a time of the system (`ToDateTimeOffset`).
    fn to_system_time(absolute: Self::Absolute) -> SystemTime;

    /// A duration as a relative time (`ToRelative`).
    fn to_relative(duration: Duration) -> Self::Relative;

    /// `absolute` moved back by `duration`: what upstream's `Add` does with the negative relative
    /// time of a due time in the past. The default keeps `absolute` (the past is due now).
    fn subtract(absolute: Self::Absolute, _duration: Duration) -> Self::Absolute {
        absolute
    }

    /// The due time an absolute schedule uses for `due_time` when the clock is at `clock`
    /// (`ScheduleAbsolute`): `due_time` itself. The test scheduler moves work due now or earlier to
    /// one tick later.
    fn adjust_due_time(due_time: Self::Absolute, _clock: Self::Absolute) -> Self::Absolute {
        due_time
    }
}

/// A scheduler of virtual time (`VirtualTimeScheduler<TAbsolute, TRelative>`): work runs when the
/// clock is advanced to its due time (`start`, `advance_to`, `advance_by`), in the order of due
/// times and, for equal ones, in the order it was scheduled.
pub struct VirtualTimeScheduler<C: VirtualTimeClock> {
    this: Weak<VirtualTimeScheduler<C>>,
    clock: Cell<C::Absolute>,
    is_enabled: Cell<bool>,
    queue: RefCell<PriorityQueue<C::Absolute, Rc<ScheduledItem<C::Absolute>>>>,
}

impl<C: VirtualTimeClock> VirtualTimeScheduler<C> {
    /// A scheduler with its clock at `initial_clock`.
    pub fn new(initial_clock: C::Absolute) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            clock: Cell::new(initial_clock),
            is_enabled: Cell::new(false),
            queue: RefCell::new(PriorityQueue::new()),
        })
    }

    fn this(&self) -> Rc<dyn IScheduler> {
        self.this.upgrade().expect("the scheduler is alive while it runs work")
    }

    /// The current time of the clock (`Clock`).
    pub fn clock(&self) -> C::Absolute {
        self.clock.get()
    }

    /// Sets the clock (`Clock`'s protected setter), for derived schedulers.
    pub fn set_clock(&self, clock: C::Absolute) {
        self.clock.set(clock);
    }

    /// Whether the scheduler is running work (`IsEnabled`).
    pub fn is_enabled(&self) -> bool {
        self.is_enabled.get()
    }

    /// Schedules work at an absolute virtual time (`ScheduleAbsolute`).
    pub fn schedule_absolute(&self, due_time: C::Absolute, action: ScheduledAction) -> Rc<dyn IDisposable> {
        let due_time = C::adjust_due_time(due_time, self.clock.get());
        let item = ScheduledItem::new(due_time, action);
        self.queue.borrow_mut().push(due_time, item.clone());
        item
    }

    /// Schedules work at a virtual time relative to the clock (`ScheduleRelative`).
    pub fn schedule_relative(&self, due_time: C::Relative, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_absolute(C::add(self.clock.get(), due_time), action)
    }

    /// The next work that has not been cancelled (`GetNext`), left in the queue.
    fn peek_next(&self) -> Option<(C::Absolute, Rc<ScheduledItem<C::Absolute>>)> {
        let mut queue = self.queue.borrow_mut();
        loop {
            let (due, item) = queue.peek().map(|(due, item)| (*due, item.clone()))?;
            if item.is_canceled() {
                queue.pop();
                continue;
            }
            return Some((due, item));
        }
    }

    fn run_next(&self, due: C::Absolute, item: Rc<ScheduledItem<C::Absolute>>) {
        // The work leaves the queue as it starts (upstream removes it in the wrapper of the
        // action): what it schedules for the same time runs after it.
        self.queue.borrow_mut().remove_where(|queued| Rc::ptr_eq(queued, &item));
        if due > self.clock.get() {
            self.clock.set(due);
        }
        item.invoke(&self.this());
    }

    /// Runs the queued work, moving the clock to the due time of each, until there is none or the
    /// scheduler is stopped (`Start`).
    pub fn start(&self) {
        if self.is_enabled.replace(true) {
            return;
        }
        while self.is_enabled.get() {
            match self.peek_next() {
                Some((due, item)) => self.run_next(due, item),
                None => self.is_enabled.set(false),
            }
        }
    }

    /// Stops the scheduler after the work that runs (`Stop`).
    pub fn stop(&self) {
        self.is_enabled.set(false);
    }

    /// Runs the work due up to `time` and moves the clock to it (`AdvanceTo`). Panics when `time`
    /// is before the clock or the scheduler is running, where upstream throws.
    pub fn advance_to(&self, time: C::Absolute) {
        if time < self.clock.get() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'time')");
        }
        if time == self.clock.get() {
            return;
        }
        if self.is_enabled.replace(true) {
            panic!("Cannot advance while running: AdvanceTo.");
        }
        while self.is_enabled.get() {
            match self.peek_next() {
                Some((due, item)) if due <= time => self.run_next(due, item),
                _ => self.is_enabled.set(false),
            }
        }
        self.clock.set(time);
    }

    /// Runs the work due within `time` from the clock and moves the clock by it (`AdvanceBy`).
    pub fn advance_by(&self, time: C::Relative) {
        let target = C::add(self.clock.get(), time);
        if target < self.clock.get() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'time')");
        }
        if target == self.clock.get() {
            return;
        }
        if self.is_enabled.get() {
            panic!("Cannot advance while running: AdvanceBy.");
        }
        self.advance_to(target);
    }

    /// Moves the clock by `time` without running work (`Sleep`).
    pub fn sleep(&self, time: C::Relative) {
        let target = C::add(self.clock.get(), time);
        if target < self.clock.get() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'time')");
        }
        self.clock.set(target);
    }
}

impl<C: VirtualTimeClock> IScheduler for VirtualTimeScheduler<C> {
    fn now(&self) -> SystemTime {
        C::to_system_time(self.clock.get())
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_absolute(self.clock.get(), action)
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_relative(C::to_relative(due_time), action)
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        match due_time.duration_since(self.now()) {
            Ok(relative) => self.schedule_relative(C::to_relative(relative), action),
            // A due time in the past keeps its place before the work due now
            // (`ScheduleRelative(ToRelative(dueTime - Now))` with a negative time).
            Err(past) => self.schedule_absolute(C::subtract(self.clock.get(), past.duration()), action),
        }
    }

    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        Some(self)
    }
}

impl<C: VirtualTimeClock> IStopwatchProvider for VirtualTimeScheduler<C> {
    fn start_stopwatch(&self) -> Box<dyn IStopwatch> {
        Box::new(VirtualTimeStopwatch::<C> { scheduler: self.this.clone(), start: self.now() })
    }
}

/// A stopwatch on the virtual clock (`VirtualTimeStopwatch`).
struct VirtualTimeStopwatch<C: VirtualTimeClock> {
    scheduler: Weak<VirtualTimeScheduler<C>>,
    start: SystemTime,
}

impl<C: VirtualTimeClock> IStopwatch for VirtualTimeStopwatch<C> {
    fn elapsed(&self) -> Duration {
        match self.scheduler.upgrade() {
            Some(scheduler) => scheduler.now().duration_since(self.start).unwrap_or(Duration::ZERO),
            None => Duration::ZERO,
        }
    }
}
