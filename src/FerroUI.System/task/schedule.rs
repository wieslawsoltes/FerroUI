//! Where the polls of a task run: [`Schedule`] and [`FirstPoll`].

use super::join::AbortHandle;

/// Where the polls of a task run: called from any thread, it queues `poll` to run on the task's
/// thread, in order with the other work of that thread.
///
/// A poll that runs on another thread finds no task there and does nothing, so an implementation
/// must run its work on the thread of the tasks it was given to. A closure of the right shape is
/// a schedule.
pub trait Schedule: Send + Sync + 'static {
    /// Queues `poll` to run on the thread of the task.
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>);

    /// Told on the thread of a task, before its first poll, when the task is given to this
    /// schedule: a schedule that ends its tasks itself (a dispatcher that shuts down aborts them)
    /// keeps the handle, and may abort the task here. Does nothing by default.
    fn spawned(&self, task: AbortHandle) {
        let _ = task;
    }
}

impl<F> Schedule for F
where
    F: Fn(Box<dyn FnOnce() + Send>) + Send + Sync + 'static,
{
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>) {
        self(poll);
    }
}

/// When the first poll of a task given a [`Schedule`] runs ([`spawn_local_on`](super::spawn_local_on)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirstPoll {
    /// Inside the call that spawns the task: the future runs up to its first pending await before
    /// the call returns.
    Now,
    /// Queued on the schedule, like the polls of the wakes that follow.
    Scheduled,
}
