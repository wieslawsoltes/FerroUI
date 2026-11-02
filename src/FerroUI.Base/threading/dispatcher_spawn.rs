//! Futures on a dispatcher: the dispatcher is a [`Schedule`] per priority of the executor of
//! `ferroui-system` (`ferroui_system::task`; step A2 of phase C4 of #423,
//! docs/idiomatic-rust-async.md, section 3.1).
//!
//! A task of a dispatcher lives in the task table of the dispatcher thread, with the schedule of
//! its priority: every wake, from any thread, posts one poll of the task as a job at that
//! priority (at most one queued poll at a time), so a continuation never runs inside the call
//! that completed what it awaited, and runs behind the work of its priority that was queued
//! before the wake. A queued poll that runs while its task is being polled (a nested frame or a
//! pumping wait inside the poll) marks the task, which is polled again through its schedule once
//! that poll has returned. The dispatcher keeps the abort handles of its tasks and aborts them
//! when it shuts down, which drops their futures; a task spawned after the shutdown is aborted at
//! once.

use std::future::Future;
use std::sync::Arc;

use ferroui_system::task::{self, AbortHandle, FirstPoll, Id, JoinHandle, Schedule};

use super::{Dispatcher, DispatcherPriority, FerroSynchronizationContext};

/// The schedule of the tasks of a dispatcher at one priority.
struct DispatcherSchedule {
    dispatcher: Arc<Dispatcher>,
    priority: DispatcherPriority,
}

impl Schedule for DispatcherSchedule {
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>) {
        self.dispatcher.post(poll, self.priority);
    }

    fn spawned(&self, task: AbortHandle) {
        self.dispatcher.track_task(task);
    }
}

/// The table of the tasks of a dispatcher is pruned of the ended ones when it reaches this size,
/// and then when it has doubled.
const TASKS_PRUNED_AT: usize = 64;

impl Dispatcher {
    /// The schedule of the tasks of this dispatcher at `priority`: every poll of a task given to it
    /// is a job posted at that priority, and the dispatcher aborts the task when it shuts down.
    ///
    /// For `ferroui_system::task::spawn_local_on`, on the dispatcher thread; with
    /// [`FirstPoll::Now`] the future runs up to its first pending await inside the spawning call.
    /// A task given to it on another thread is aborted at once.
    pub fn task_schedule(&self, priority: DispatcherPriority) -> Arc<dyn Schedule> {
        DispatcherPriority::validate(priority, "priority");
        Arc::new(DispatcherSchedule { dispatcher: self.to_arc(), priority })
    }

    /// Runs `future` on this dispatcher at the priority of the job that calls this
    /// ([`DispatcherPriority::DEFAULT`] outside of a job), and gives its handle; see
    /// [`spawn_local_with_priority`](Self::spawn_local_with_priority).
    ///
    /// # Panics
    /// When called from a thread other than the dispatcher thread.
    pub fn spawn_local<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.spawn_local_with_priority(future, self.current_job_priority())
    }

    /// Runs `future`, which need not be `Send`, on this dispatcher, and gives its handle. Its
    /// first poll is a job posted at `priority`, and so is every poll that a wake queues, from any
    /// thread. The handle is a future of the output, or of a
    /// [`JoinError`](ferroui_system::task::JoinError): canceled when the task is aborted or the
    /// dispatcher shuts down first, panicked when the future panics. Dropping the handle detaches
    /// the task, which keeps running.
    ///
    /// # Panics
    /// When called from a thread other than the dispatcher thread.
    pub fn spawn_local_with_priority<F>(&self, future: F, priority: DispatcherPriority) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.verify_access();
        task::spawn_local_on(future, self.task_schedule(priority), FirstPoll::Scheduled)
    }

    /// Runs the future that `create` makes on this dispatcher at [`DispatcherPriority::DEFAULT`],
    /// from any thread; see [`spawn_with_priority`](Self::spawn_with_priority).
    pub fn spawn<C, F>(&self, create: C) -> JoinHandle<F::Output>
    where
        C: FnOnce() -> F + Send + 'static,
        F: Future + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_with_priority(create, DispatcherPriority::DEFAULT)
    }

    /// Runs the future that `create` makes on this dispatcher, and gives its handle; callable from
    /// any thread. `create` is `Send` and runs in a job posted at `priority`; the future it makes
    /// need not be `Send`, never leaves the dispatcher thread, and is polled for the first time
    /// inside that job, then from jobs of the same priority (what
    /// [`invoke_async_task_with_priority`](Self::invoke_async_task_with_priority) does). A panic of
    /// `create` faults the task like a panic of the future; the task is canceled when the
    /// dispatcher shuts down before the job runs.
    pub fn spawn_with_priority<C, F>(&self, create: C, priority: DispatcherPriority) -> JoinHandle<F::Output>
    where
        C: FnOnce() -> F + Send + 'static,
        F: Future + 'static,
        F::Output: Send + 'static,
    {
        task::spawn_on(create, self.task_schedule(priority))
    }

    /// Runs `future` on this dispatcher at `priority`, polling it for the first time before this
    /// returns ([`FirstPoll::Now`]). Dispatcher thread only.
    pub(crate) fn start_local_with_priority<F>(&self, future: F, priority: DispatcherPriority) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.verify_access();
        task::spawn_local_on(future, self.task_schedule(priority), FirstPoll::Now)
    }

    /// The priority of the job that runs on the calling thread: the one of the current
    /// [`FerroSynchronizationContext`], [`DispatcherPriority::DEFAULT`] when there is none.
    pub(crate) fn current_job_priority(&self) -> DispatcherPriority {
        match FerroSynchronizationContext::current() {
            Some(context) => context.priority(),
            None => DispatcherPriority::DEFAULT,
        }
    }

    /// Keeps the abort handle of a task given to a schedule of this dispatcher, or aborts the task
    /// when it cannot run here: the dispatcher has shut down or been reset, or the task was
    /// spawned on another thread.
    fn track_task(&self, task: AbortHandle) {
        let Some(local) = self.try_local() else {
            task.abort();
            return;
        };
        if self.lock().has_shutdown_finished {
            task.abort();
            return;
        }
        let mut tasks = local.tasks.borrow_mut();
        if tasks.len() >= local.tasks_pruned_at.get() {
            tasks.retain(|_, task| !task.is_finished());
            local.tasks_pruned_at.set((tasks.len() * 2).max(TASKS_PRUNED_AT));
        }
        tasks.insert(task.id(), task);
    }

    /// Whether `id` is a task of this dispatcher. Dispatcher thread only (false elsewhere).
    pub(crate) fn owns_task(&self, id: Id) -> bool {
        self.try_local().is_some_and(|local| local.tasks.borrow().contains_key(&id))
    }

    /// Aborts every task of this dispatcher, which drops their futures (a task being polled, right
    /// after its poll) and ends them canceled.
    pub(crate) fn cancel_tasks(&self) {
        if let Some(local) = self.try_local() {
            let tasks = std::mem::take(&mut *local.tasks.borrow_mut());
            // Aborted outside of the borrow: futures run arbitrary code when dropped.
            for task in tasks.into_values() {
                task.abort();
            }
        }
    }
}
