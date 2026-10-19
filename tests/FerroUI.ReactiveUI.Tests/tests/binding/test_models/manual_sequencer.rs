//! `ManualSequencer` (`TestModels/ManualSequencer.cs`).

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction, Scheduler};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// The most items [`ManualSequencer::run_until_idle`] runs before it gives up.
const MAX_ITEMS: usize = 1_000;

/// A scheduler that queues its work, whatever its due time, until the test runs it
/// (`ManualSequencer`).
pub struct ManualSequencer {
    this: Weak<ManualSequencer>,
    pending: RefCell<VecDeque<ScheduledAction>>,
}

#[allow(dead_code)]
impl ManualSequencer {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), pending: RefCell::new(VecDeque::new()) })
    }

    /// The scheduler as an `Rc<dyn IScheduler>`.
    pub fn scheduler(self: &Rc<Self>) -> Rc<dyn IScheduler> {
        self.clone()
    }

    /// The number of items waiting.
    pub fn pending_count(&self) -> usize {
        self.pending.borrow().len()
    }

    /// Runs the items waiting now, not those they queue, and returns how many ran (`RunPending`).
    pub fn run_pending(&self) -> usize {
        let count = self.pending.borrow().len();
        for _ in 0..count {
            self.run_next();
        }
        count
    }

    /// Runs passes of the waiting items until none is left, and returns the number of passes; `-1`
    /// when more than `max_passes` passes, or more than a thousand items, ran (`RunUntilIdle`).
    pub fn run_until_idle(&self, max_passes: i32) -> i32 {
        let mut ran = 0;
        for pass in 0..max_passes {
            let count = self.pending.borrow().len();
            if count == 0 {
                return pass;
            }
            for _ in 0..count {
                ran += 1;
                if ran > MAX_ITEMS {
                    return -1;
                }
                self.run_next();
            }
        }
        -1
    }

    fn run_next(&self) {
        let next = self.pending.borrow_mut().pop_front();
        let (Some(action), Some(this)) = (next, self.this.upgrade()) else {
            return;
        };
        let scheduler: Rc<dyn IScheduler> = this;
        let _ = action(&scheduler);
    }
}

impl IScheduler for ManualSequencer {
    fn now(&self) -> SystemTime {
        Scheduler::now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.pending.borrow_mut().push_back(action);
        Disposable::empty()
    }

    fn schedule_after(&self, _due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule(action)
    }

    fn schedule_at(&self, _due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule(action)
    }
}
