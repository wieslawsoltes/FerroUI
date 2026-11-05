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
//!
//! The dispatcher is also the schedule of its thread (step A5, section 3.1), from its start to its
//! shutdown: the wakes from other threads of the tasks in the queue of the thread
//! (`ferroui_system::task::spawn_local` outside of a job) and the callbacks that other threads send
//! to it (`CancellationToken::register_local`) are jobs at [`DispatcherPriority::DEFAULT`]. While a
//! job runs, the dispatcher at the priority of the job is the current schedule of the thread
//! (`ferroui_system::task::current_schedule`), which a task spawned with
//! `ferroui_system::task::spawn_local` in the job takes: a wake in a job of the same priority polls
//! it at once, any other wake, from any thread, queues a poll as a job at that priority.

use std::future::Future;
use std::sync::Arc;

use ferroui_system::task::{self, AbortHandle, FirstPoll, Id, JoinHandle, Schedule};

use super::{Dispatcher, DispatcherPriority};

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

    /// A job posted at the priority of the schedule, on the dispatcher thread.
    fn schedule_local(&self, work: Box<dyn FnOnce()>) {
        self.dispatcher.post_local(work, self.priority);
    }
}

/// The table of the tasks of a dispatcher is pruned of the ended ones when it reaches this size,
/// and then when it has doubled.
const TASKS_PRUNED_AT: usize = 64;

impl Dispatcher {
    /// The schedule of the tasks of this dispatcher at `priority`: every poll of a task given to it
    /// is a job posted at that priority, and the dispatcher aborts the task when it shuts down;
    /// work that is not `Send` ([`Schedule::schedule_local`]) is a job posted with
    /// [`post_local`](Self::post_local). It is the current schedule of the jobs of that priority
    /// (`ferroui_system::task::current_schedule`); on the dispatcher thread there is one schedule
    /// per priority.
    ///
    /// For `ferroui_system::task::spawn_local_on`, on the dispatcher thread; with
    /// [`FirstPoll::Now`] the future runs up to its first pending await inside the spawning call.
    /// A task given to it on another thread is aborted at once.
    pub fn task_schedule(&self, priority: DispatcherPriority) -> Arc<dyn Schedule> {
        DispatcherPriority::validate(priority, "priority");
        let Some(local) = self.try_local() else {
            return Arc::new(DispatcherSchedule { dispatcher: self.to_arc(), priority });
        };
        let index = (priority.value() - DispatcherPriority::MIN_VALUE.value()) as usize;
        let mut schedules = local.schedules.borrow_mut();
        if schedules.is_empty() {
            let count = (DispatcherPriority::MAX_VALUE.value() - DispatcherPriority::MIN_VALUE.value() + 1) as usize;
            schedules.resize(count, None);
        }
        schedules[index]
            .get_or_insert_with(|| Arc::new(DispatcherSchedule { dispatcher: self.to_arc(), priority }))
            .clone()
    }

    /// Makes this dispatcher, at [`DispatcherPriority::DEFAULT`], the schedule of its thread
    /// (`ferroui_system::task::set_thread_schedule`); on the dispatcher thread, when it starts.
    pub(crate) fn install_thread_schedule(&self) {
        if self.try_local().is_some() {
            task::set_thread_schedule(Some(self.task_schedule(DispatcherPriority::DEFAULT)));
        }
    }

    /// Removes this dispatcher as the schedule of its thread, when it is: at its shutdown, on its
    /// thread. A schedule that someone else set in the meantime stays.
    pub(crate) fn uninstall_thread_schedule(&self) {
        let Some(local) = self.try_local() else { return };
        let Some(current) = task::thread_schedule() else { return };
        let owned = local
            .schedules
            .borrow()
            .iter()
            .flatten()
            .any(|schedule| std::ptr::addr_eq(Arc::as_ptr(schedule), Arc::as_ptr(&current)));
        if owned {
            task::set_thread_schedule(None);
        }
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
    /// inside that job, then from jobs of the same priority (what the deprecated
    /// `invoke_async_task_with_priority` does, with a `JoinHandle`). A panic of
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

    /// Starts `future` on this dispatcher at the priority of the job that calls this
    /// ([`DispatcherPriority::DEFAULT`] outside of a job), and gives its handle; see
    /// [`start_local_with_priority`](Self::start_local_with_priority).
    ///
    /// # Panics
    /// When called from a thread other than the dispatcher thread.
    pub fn start_local<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.start_local_with_priority(future, self.current_job_priority())
    }

    /// Starts `future`, which need not be `Send`, on this dispatcher the way calling an
    /// asynchronous method does, and gives its handle: the future is polled for the first time
    /// before this returns ([`FirstPoll::Now`]), so it runs up to its first pending await inside
    /// the call (and the handle is finished when there is none); every later poll is a job posted
    /// at `priority` by a wake, from any thread. Otherwise the task is the one of
    /// [`spawn_local_with_priority`](Self::spawn_local_with_priority): a panic of the future is
    /// [`JoinError::Panicked`](ferroui_system::task::JoinError::Panicked), an abort or the
    /// shutdown of the dispatcher is [`JoinError::Canceled`](ferroui_system::task::JoinError::Canceled),
    /// and dropping the handle detaches the task.
    ///
    /// What `DispatcherTaskScheduler::start_local` (deprecated) did, with a `JoinHandle`.
    ///
    /// # Panics
    /// When called from a thread other than the dispatcher thread.
    pub fn start_local_with_priority<F>(&self, future: F, priority: DispatcherPriority) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.verify_access();
        task::spawn_local_on(future, self.task_schedule(priority), FirstPoll::Now)
    }

    /// The priority of the job that runs on the calling thread ([`Dispatcher::current_priority`]),
    /// [`DispatcherPriority::DEFAULT`] outside of a job.
    pub(crate) fn current_job_priority(&self) -> DispatcherPriority {
        Self::current_priority().unwrap_or(DispatcherPriority::DEFAULT)
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
    /// after its poll) and ends them canceled, and removes the dispatcher as the schedule of its
    /// thread: what the shutdown of the dispatcher does to its futures.
    pub(crate) fn cancel_tasks(&self) {
        self.uninstall_thread_schedule();
        if let Some(local) = self.try_local() {
            let tasks = std::mem::take(&mut *local.tasks.borrow_mut());
            // Aborted outside of the borrow: futures run arbitrary code when dropped.
            for task in tasks.into_values() {
                task.abort();
            }
        }
    }
}
