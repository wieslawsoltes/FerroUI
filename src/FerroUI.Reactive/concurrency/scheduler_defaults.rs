//! `SchedulerDefaults` (`SchedulerDefaults.cs`) and the default scheduler of the thread
//! (`DefaultScheduler.Instance`).

use super::{CurrentThreadScheduler, IScheduler, ImmediateScheduler};
use std::cell::RefCell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The scheduler of the operators whose overload names none and that upstream runs on the
    /// thread pool (`DefaultScheduler.Instance`).
    static DEFAULT: RefCell<Option<Rc<dyn IScheduler>>> = const { RefCell::new(None) };
}

/// The schedulers that operators use when the caller names none (`SchedulerDefaults`).
///
/// Deviation (DEVIATIONS.md, Reactive): upstream runs time-based operators and asynchronous
/// conversions on the thread pool (`DefaultScheduler`). The port is single-threaded: those use the
/// default scheduler of the thread, which an application sets (the integration with FerroUI sets
/// the scheduler of the dispatcher of the user-interface thread) and which is the current-thread
/// scheduler until it does.
pub struct SchedulerDefaults;

impl SchedulerDefaults {
    /// Operators that take constant time (`ConstantTimeOperations`): the immediate scheduler.
    pub fn constant_time_operations() -> Rc<dyn IScheduler> {
        ImmediateScheduler::instance()
    }

    /// Tail recursion (`TailRecursion`): the immediate scheduler.
    pub fn tail_recursion() -> Rc<dyn IScheduler> {
        ImmediateScheduler::instance()
    }

    /// Iteration (`Iteration`): the current-thread scheduler.
    pub fn iteration() -> Rc<dyn IScheduler> {
        CurrentThreadScheduler::instance()
    }

    /// Time-based operations (`TimeBasedOperations`): the default scheduler of the thread.
    pub fn time_based_operations() -> Rc<dyn IScheduler> {
        Self::default_scheduler()
    }

    /// Asynchronous conversions (`AsyncConversions`): the default scheduler of the thread.
    pub fn async_conversions() -> Rc<dyn IScheduler> {
        Self::default_scheduler()
    }

    /// The default scheduler of the thread (`DefaultScheduler.Instance`): the one set with
    /// [`set_default_scheduler`](Self::set_default_scheduler), else the current-thread scheduler.
    pub fn default_scheduler() -> Rc<dyn IScheduler> {
        DEFAULT.with(|default| default.borrow().clone()).unwrap_or_else(CurrentThreadScheduler::instance)
    }

    /// Sets the default scheduler of the thread; `None` restores the current-thread scheduler.
    pub fn set_default_scheduler(scheduler: Option<Rc<dyn IScheduler>>) {
        DEFAULT.with(|default| *default.borrow_mut() = scheduler);
    }
}
