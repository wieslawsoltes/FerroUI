//! `RxSchedulers` (`ReactiveUI.Shared/RxSchedulers.cs`).

use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The scheduler set for the main thread, if any (`_mainThreadScheduler`).
    static MAIN_THREAD_SCHEDULER: RefCell<Option<Rc<dyn IScheduler>>> = const { RefCell::new(None) };
    /// The scheduler set for background work, if any (`_taskpoolScheduler`).
    static TASKPOOL_SCHEDULER: RefCell<Option<Rc<dyn IScheduler>>> = const { RefCell::new(None) };
    /// `SuppressViewCommandBindingMessage`.
    static SUPPRESS_VIEW_COMMAND_BINDING_MESSAGE: Cell<bool> = const { Cell::new(false) };
}

/// The schedulers that ReactiveUI's objects run their work on (`RxSchedulers`), settable for
/// tests and by the integration with the user interface.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the schedulers are per thread, as the observables are
/// (`docs/porting/reactive.md`, section 5.1). Until one is set, both are the default scheduler of
/// the thread ([`SchedulerDefaults::default_scheduler`]), the port of upstream's
/// `Scheduler.Default` and `TaskPoolScheduler.Default`: the current-thread scheduler unless the
/// application set another one. Unit tests therefore run on the current-thread scheduler unless a
/// test sets one; the integration with FerroUI sets the scheduler of the dispatcher.
pub struct RxSchedulers;

impl RxSchedulers {
    /// The scheduler of the user-interface thread (`MainThreadScheduler`).
    pub fn main_thread_scheduler() -> Rc<dyn IScheduler> {
        MAIN_THREAD_SCHEDULER.with(|slot| slot.borrow().clone()).unwrap_or_else(SchedulerDefaults::default_scheduler)
    }

    /// Sets the scheduler of the user-interface thread (`MainThreadScheduler { set; }`).
    pub fn set_main_thread_scheduler(scheduler: Rc<dyn IScheduler>) {
        MAIN_THREAD_SCHEDULER.with(|slot| *slot.borrow_mut() = Some(scheduler));
    }

    /// The scheduler of background work (`TaskpoolScheduler`).
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI): the work of the thread pool cannot run
    /// single-threaded observables, so this is a scheduler of the current thread unless one is
    /// set; work for another thread goes through `ferroui_system::threading::tasks::run`.
    pub fn task_pool_scheduler() -> Rc<dyn IScheduler> {
        TASKPOOL_SCHEDULER.with(|slot| slot.borrow().clone()).unwrap_or_else(SchedulerDefaults::default_scheduler)
    }

    /// Sets the scheduler of background work (`TaskpoolScheduler { set; }`).
    pub fn set_task_pool_scheduler(scheduler: Rc<dyn IScheduler>) {
        TASKPOOL_SCHEDULER.with(|slot| *slot.borrow_mut() = Some(scheduler));
    }

    /// Whether the bindings of commands skip the message they log when a view has no command of
    /// the name (`SuppressViewCommandBindingMessage`).
    pub fn suppress_view_command_binding_message() -> bool {
        SUPPRESS_VIEW_COMMAND_BINDING_MESSAGE.with(Cell::get)
    }

    /// Sets [`suppress_view_command_binding_message`](Self::suppress_view_command_binding_message).
    pub fn set_suppress_view_command_binding_message(value: bool) {
        SUPPRESS_VIEW_COMMAND_BINDING_MESSAGE.with(|slot| slot.set(value));
    }

    /// Restores the default schedulers (`ResetForTesting`); for tests.
    #[doc(hidden)]
    pub fn reset_for_testing() {
        MAIN_THREAD_SCHEDULER.with(|slot| *slot.borrow_mut() = None);
        TASKPOOL_SCHEDULER.with(|slot| *slot.borrow_mut() = None);
    }
}
