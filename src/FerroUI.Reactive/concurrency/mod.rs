//! `System.Reactive.Concurrency`: the schedulers, which decide when and where work runs.
//!
//! Work is scheduled as a [`ScheduledAction`], which receives the scheduler it runs on and returns
//! the disposable that cancels what it started. All schedulers here run their work on the thread
//! that scheduled it: the immediate and current-thread schedulers on the calling thread, the
//! virtual-time schedulers when their clock is advanced. The scheduler of the user-interface thread
//! (the dispatcher) is given by the integration of the reactive libraries with the framework.

mod async_lock;
mod current_thread_scheduler;
mod historical_scheduler;
mod immediate_scheduler;
mod local_scheduler;
mod scheduled_item;
mod scheduler;
mod scheduler_defaults;
mod virtual_time_scheduler;

pub use async_lock::AsyncLock;
pub use current_thread_scheduler::CurrentThreadScheduler;
pub use historical_scheduler::{HistoricalScheduler, HistoricalTime};
pub use immediate_scheduler::ImmediateScheduler;
pub use scheduled_item::ScheduledItem;
pub use scheduler::{
    IScheduler, ISchedulerPeriodic, IStopwatch, IStopwatchProvider, ScheduledAction, Scheduler, SchedulerExt,
};
pub use scheduler_defaults::SchedulerDefaults;
pub use virtual_time_scheduler::{VirtualTimeClock, VirtualTimeScheduler};
