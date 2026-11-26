//! The priority of the running job.
//!
//! While the dispatcher runs a job, the priority of the job is the current priority of the thread
//! ([`Dispatcher::current_priority`]) and the dispatcher at that priority is the current schedule
//! of `ferroui-system` ([`ferroui_system::task::current_schedule`]), through which the tasks that
//! the job spawns and the reactive libraries continue where they started (step A5 of phase C4 of
//! #423, docs/idiomatic-rust-async.md, sections 3.4 and 3.5). Both are restored when the job ends
//! ([`JobScope`]). The synchronization context of the dispatcher, which carried the priority
//! before, was deprecated in 0.5.0 and removed in 0.7.0 (#625).

use std::cell::Cell;
use std::sync::Arc;

use ferroui_system::task::{self, Schedule};

use super::{Dispatcher, DispatcherPriority};

crate::ferro_thread_local! {
    static CURRENT: Cell<Option<DispatcherPriority>> = const { Cell::new(None) };
}

/// Replaces the current priority of the calling thread, and gives the previous one.
fn replace_current_priority(priority: Option<DispatcherPriority>) -> Option<DispatcherPriority> {
    CURRENT.try_with(|current| current.replace(priority)).ok().flatten()
}

/// The current priority of the calling thread.
fn current_thread_priority() -> Option<DispatcherPriority> {
    CURRENT.try_with(Cell::get).ok().flatten()
}

/// The current priority and the current schedule of the thread while a job runs (the context of
/// the reference implementation): set by [`Dispatcher::enter_job`], restored when dropped, also
/// while a panic unwinds.
pub(crate) struct JobScope {
    /// `Some` when the priority changed and has to be restored.
    old_priority: Option<Option<DispatcherPriority>>,
    old_schedule: Option<Option<Arc<dyn Schedule>>>,
}

impl JobScope {
    fn unchanged() -> Self {
        Self { old_priority: None, old_schedule: None }
    }

    fn restore(&mut self) {
        if let Some(old_schedule) = self.old_schedule.take() {
            // The replaced value is dropped outside of the borrow of the slot.
            let replaced = task::set_current_schedule(old_schedule);
            drop(replaced);
        }
        if let Some(old_priority) = self.old_priority.take() {
            replace_current_priority(old_priority);
        }
    }
}

impl Drop for JobScope {
    fn drop(&mut self) {
        self.restore();
    }
}

impl Dispatcher {
    /// Makes `priority` the current priority of the calling thread, and this dispatcher at that
    /// priority its current schedule, until the returned scope is dropped: what the dispatcher
    /// does around each job it runs. What is already current is not changed, and not restored.
    ///
    /// # Panics
    /// When the priority has to change and the calling thread is not the dispatcher thread.
    pub(crate) fn enter_job(&self, priority: DispatcherPriority) -> JobScope {
        let same_priority = current_thread_priority() == Some(priority);
        if same_priority && !self.check_access() {
            return JobScope::unchanged();
        }
        self.verify_access();
        let schedule = self.task_schedule(priority);
        let old_schedule =
            if task::is_current_schedule(&schedule) { None } else { Some(task::set_current_schedule(Some(schedule))) };
        let old_priority = if same_priority { None } else { Some(replace_current_priority(Some(priority))) };
        JobScope { old_priority, old_schedule }
    }

    /// The priority of the dispatcher job that runs on the calling thread (`None` outside of a
    /// job): what asynchronous code started by the job continues at. Also the priority set with
    /// [`set_current_priority`](Self::set_current_priority) for the code that runs outside of a
    /// job, and the priority of a nested frame between its jobs ([`DispatcherPriority::NORMAL`]).
    pub fn current_priority() -> Option<DispatcherPriority> {
        current_thread_priority()
    }

    /// Sets the priority of the code that runs on the calling thread outside of a dispatcher job,
    /// and gives the previous one: the dispatcher of the thread at that priority becomes the
    /// current schedule of the thread ([`ferroui_system::task::current_schedule`]), which the
    /// tasks spawned there and the reactive libraries continue in; `None` clears both. What the
    /// application does when it starts (at [`DispatcherPriority::NORMAL`], unless the thread has a
    /// priority), as the context that the upstream application installs; the jobs of the
    /// dispatcher set their own priority while they run, and restore this one.
    ///
    /// A priority creates the dispatcher of the calling thread when it has none.
    pub fn set_current_priority(priority: Option<DispatcherPriority>) -> Option<DispatcherPriority> {
        let schedule = priority.map(|priority| Dispatcher::current_dispatcher().task_schedule(priority));
        let replaced = task::set_current_schedule(schedule);
        drop(replaced);
        replace_current_priority(priority)
    }

    /// Clears the current priority and the current schedule of the calling thread (the reset of
    /// the dispatcher of a thread).
    pub(crate) fn clear_current_priority() {
        Self::set_current_priority(None);
    }
}
