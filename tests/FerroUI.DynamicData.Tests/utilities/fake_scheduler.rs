//! `FakeScheduler` (`Utilities/FakeScheduler.cs`).

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction as SchedulerAction};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// One tick of `TimeSpan` (100 nanoseconds).
const TICK: Duration = Duration::from_nanos(100);

/// A scheduler that keeps the actions in a list until the test runs them
/// (`FakeScheduler`): its clock is set by the test and advanced by
/// [`simulate_until_idle`](Self::simulate_until_idle).
///
/// Deviation (DEVIATIONS.md, DynamicData): the clock starts at the epoch of `SystemTime`, where
/// upstream's `DateTimeOffset` starts at year 1.
pub struct FakeScheduler {
    this: Weak<FakeScheduler>,
    scheduled_actions: RefCell<Vec<Rc<ScheduledAction>>>,
    now: Cell<SystemTime>,
}

impl FakeScheduler {
    /// A scheduler without actions, its clock at the epoch (`new FakeScheduler()`).
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), scheduled_actions: RefCell::new(Vec::new()), now: Cell::new(SystemTime::UNIX_EPOCH) })
    }

    /// The scheduler as the handle operators take.
    pub fn scheduler(self: &Rc<Self>) -> Rc<dyn IScheduler> {
        self.clone()
    }

    /// The actions not yet run, in the order they were scheduled (`ScheduledActions`).
    pub fn scheduled_actions(&self) -> Vec<Rc<ScheduledAction>> {
        self.scheduled_actions.borrow().clone()
    }

    /// Sets the clock (`Now`'s setter).
    pub fn set_now(&self, now: SystemTime) {
        self.now.set(now);
    }

    /// Runs the actions, and those they schedule, until none remain (`SimulateUntilIdle()`).
    pub fn simulate_until_idle(&self) {
        self.simulate_until_idle_with(0);
    }

    /// Runs the actions, and those they schedule, until none remain
    /// (`SimulateUntilIdle(TimeSpan inaccuracyOffset)`, with the offset in ticks, which may be
    /// negative).
    ///
    /// The first action runs at once when it has no due time, or when its due time is reached by
    /// the clock offset by the inaccuracy: an offset of -1 ms runs actions 1 ms early. After each
    /// step the clock advances by one tick. As upstream, only the first action is looked at, and a
    /// cancelled action still runs.
    pub fn simulate_until_idle_with(&self, inaccuracy_offset_ticks: i64) {
        loop {
            let first = self.scheduled_actions.borrow().first().cloned();
            let Some(first) = first else { break };
            let due = match first.due_time() {
                None => true,
                Some(due_time) => due_time <= offset_time(self.now.get(), -inaccuracy_offset_ticks),
            };
            if due {
                first.invoke();
                let mut actions = self.scheduled_actions.borrow_mut();
                if actions.first().is_some_and(|action| Rc::ptr_eq(action, &first)) {
                    actions.remove(0);
                }
            }
            // At least one tick after every step, so that the loop ends.
            self.now.set(self.now.get() + TICK);
        }
    }

    fn schedule_core(&self, due_time: Option<SystemTime>, action: SchedulerAction) -> Rc<dyn IDisposable> {
        let this = self.this.clone();
        let scheduled_action = Rc::new(ScheduledAction {
            due_time,
            on_invoked: RefCell::new(Some(Box::new(move || match this.upgrade() {
                Some(scheduler) => {
                    let scheduler: Rc<dyn IScheduler> = scheduler;
                    action(&scheduler)
                }
                None => Disposable::empty(),
            }))),
            has_been_cancelled: Cell::new(false),
        });
        self.scheduled_actions.borrow_mut().push(scheduled_action.clone());
        Disposable::create(move || scheduled_action.cancel())
    }
}

/// `time` moved by `ticks`, which may be negative.
fn offset_time(time: SystemTime, ticks: i64) -> SystemTime {
    let offset = Duration::from_nanos(ticks.unsigned_abs().saturating_mul(100));
    if ticks >= 0 {
        time + offset
    } else {
        time.checked_sub(offset).unwrap_or(SystemTime::UNIX_EPOCH)
    }
}

impl IScheduler for FakeScheduler {
    fn now(&self) -> SystemTime {
        self.now.get()
    }

    fn schedule(&self, action: SchedulerAction) -> Rc<dyn IDisposable> {
        self.schedule_core(None, action)
    }

    fn schedule_after(&self, due_time: Duration, action: SchedulerAction) -> Rc<dyn IDisposable> {
        self.schedule_core(Some(self.now.get() + due_time), action)
    }

    fn schedule_at(&self, due_time: SystemTime, action: SchedulerAction) -> Rc<dyn IDisposable> {
        self.schedule_core(Some(due_time), action)
    }
}

type OnInvoked = Box<dyn FnOnce() -> Rc<dyn IDisposable>>;

/// An action of a [`FakeScheduler`] (`FakeScheduler.ScheduledAction`): its due time, whether it
/// was cancelled, and the work.
pub struct ScheduledAction {
    due_time: Option<SystemTime>,
    on_invoked: RefCell<Option<OnInvoked>>,
    has_been_cancelled: Cell<bool>,
}

impl ScheduledAction {
    /// When the action is due; `None` for at once (`DueTime`).
    pub fn due_time(&self) -> Option<SystemTime> {
        self.due_time
    }

    /// Whether the disposable of the scheduling was disposed (`HasBeenCancelled`).
    pub fn has_been_cancelled(&self) -> bool {
        self.has_been_cancelled.get()
    }

    /// Marks the action cancelled (`Cancel`); it is still in the list.
    pub fn cancel(&self) {
        self.has_been_cancelled.set(true);
    }

    /// Runs the work and returns its disposable (`Invoke`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): the work of a scheduler runs once; a second
    /// invocation does nothing and returns an empty disposable.
    pub fn invoke(&self) -> Rc<dyn IDisposable> {
        let on_invoked = self.on_invoked.borrow_mut().take();
        match on_invoked {
            Some(on_invoked) => on_invoked(),
            None => Disposable::empty(),
        }
    }
}
