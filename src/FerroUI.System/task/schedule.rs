//! Where the polls of a task run: [`Schedule`] and [`FirstPoll`].

use super::executor::{LocalCallback, RunCallback};
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

    /// Queues `work`, which need not be `Send`, to run on the thread of this schedule, in order
    /// with its other work; called on that thread (a `SynchronizationContext.Post` from the thread
    /// of the context). By default the work is kept as a callback of the calling thread, and the
    /// schedule is given a piece of work that runs it there; work that the schedule drops without
    /// running it is dropped on its thread, the next time that thread runs its queue
    /// ([`run_pending`](super::run_pending)). A schedule that can hold work that is not `Send` (a
    /// dispatcher) queues it directly.
    fn schedule_local(&self, work: Box<dyn FnOnce()>) {
        let run = RunCallback::new(LocalCallback::register(work).handle());
        self.schedule(Box::new(move || run.run()));
    }

    /// The identity of the schedule, by which [`is_current_schedule`](super::is_current_schedule)
    /// compares two of them: by default the address of the schedule. A schedule that stands for
    /// another object (an adapter) gives the address of that object, so that two of them for the
    /// same object are the same schedule.
    fn identity(&self) -> *const () {
        (self as *const Self).cast()
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
