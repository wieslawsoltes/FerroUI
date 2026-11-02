//! Where the polls of a task run: [`Schedule`] and [`FirstPoll`].

/// Where the polls of a task run: called from any thread, it queues `poll` to run on the task's
/// thread, in order with the other work of that thread.
///
/// A poll that runs on another thread finds no task there and does nothing, so an implementation
/// must run its work on the thread of the tasks it was given to. A closure of the right shape is
/// a schedule.
pub trait Schedule: Send + Sync + 'static {
    /// Queues `poll` to run on the thread of the task.
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>);
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
