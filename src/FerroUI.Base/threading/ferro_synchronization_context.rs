//! The priority of the running job, and the deprecated synchronization context of the dispatcher.
//!
//! While the dispatcher runs a job, the priority of the job is the current priority of the thread
//! ([`Dispatcher::current_priority`]) and the dispatcher at that priority is the current schedule
//! of `ferroui-system` ([`ferroui_system::task::current_schedule`]), through which the tasks that
//! the job spawns and the reactive libraries continue where they started (step A5 of phase C4 of
//! #423, docs/idiomatic-rust-async.md, sections 3.4 and 3.5). Both are restored when the job ends
//! ([`JobScope`]).
//!
//! [`FerroSynchronizationContext`] and [`RestoreContext`], the port of the upstream context, are
//! deprecated since 0.5.0 and keep working on top of that: the current context of a thread is the
//! context of the current priority.
#![allow(deprecated)]

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ferroui_system::task::{self, Schedule};

use super::dispatcher_task::{new_local_task, new_send_task};
use super::{Dispatcher, DispatcherPriority, DispatcherTask};
use crate::reactive::IDisposable;

static AUTO_INSTALL: AtomicBool = AtomicBool::new(true);

crate::ferro_thread_local! {
    static CURRENT: RefCell<Option<Arc<FerroSynchronizationContext>>> = const { RefCell::new(None) };
}

/// Replaces the current context of the calling thread, and gives the previous one.
fn replace_current_context(context: Option<Arc<FerroSynchronizationContext>>) -> Option<Arc<FerroSynchronizationContext>> {
    CURRENT.try_with(|current| current.replace(context)).ok().flatten()
}

/// The priority of the current context of the calling thread.
fn current_context_priority() -> Option<DispatcherPriority> {
    CURRENT.try_with(|current| current.borrow().as_ref().map(|context| context.priority)).ok().flatten()
}

/// The schedule of the dispatcher of `context` at its priority, for the current schedule of the
/// thread.
fn schedule_of(context: &FerroSynchronizationContext) -> Arc<dyn Schedule> {
    context.dispatcher.task_schedule(context.priority)
}

/// The per-priority contexts of a dispatcher, created on demand on its thread.
#[derive(Default)]
pub(crate) struct PriorityContexts {
    contexts: RefCell<Vec<Option<Arc<FerroSynchronizationContext>>>>,
}

/// The current priority and the current schedule of the thread while a job runs (the context of
/// the reference implementation): set by [`Dispatcher::enter_job`], restored when dropped, also
/// while a panic unwinds.
pub(crate) struct JobScope {
    /// `Some` when the context changed and has to be restored.
    old_context: Option<Option<Arc<FerroSynchronizationContext>>>,
    old_schedule: Option<Option<Arc<dyn Schedule>>>,
}

impl JobScope {
    fn unchanged() -> Self {
        Self { old_context: None, old_schedule: None }
    }

    fn restore(&mut self) {
        if let Some(old_schedule) = self.old_schedule.take() {
            // The replaced values are dropped outside of the borrows of the slots.
            let replaced = task::set_current_schedule(old_schedule);
            drop(replaced);
        }
        if let Some(old_context) = self.old_context.take() {
            let replaced = replace_current_context(old_context);
            drop(replaced);
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
        let same_priority = current_context_priority() == Some(priority);
        if same_priority && !self.check_access() {
            return JobScope::unchanged();
        }
        self.verify_access();
        let schedule = self.task_schedule(priority);
        let old_schedule =
            if task::is_current_schedule(&schedule) { None } else { Some(task::set_current_schedule(Some(schedule))) };
        let old_context = if same_priority {
            None
        } else {
            Some(replace_current_context(Some(self.get_context_with_priority(priority))))
        };
        JobScope { old_context, old_schedule }
    }

    /// The priority of the dispatcher job that runs on the calling thread (`None` outside of a
    /// job): what asynchronous code started by the job continues at. Also the priority set with
    /// [`set_current_priority`](Self::set_current_priority) for the code that runs outside of a
    /// job, and the priority of a nested frame between its jobs ([`DispatcherPriority::NORMAL`]).
    pub fn current_priority() -> Option<DispatcherPriority> {
        current_context_priority()
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
        let context = priority.map(|priority| Dispatcher::current_dispatcher().get_context_with_priority(priority));
        let previous = FerroSynchronizationContext::set_current(context);
        previous.map(|context| context.priority)
    }

    /// Clears the current priority and the current schedule of the calling thread (the reset of
    /// the dispatcher of a thread).
    pub(crate) fn clear_current_priority() {
        let previous = FerroSynchronizationContext::set_current(None);
        drop(previous);
    }
}

/// A dispatcher together with the priority at which work handed to it is
/// queued.
///
/// The *current* context of a thread tells asynchronous code where and at
/// which priority to continue. While the dispatcher executes a job, the
/// current context is the dispatcher's context for the priority of that job;
/// futures started without an explicit priority (see
/// [`Dispatcher::to_task_scheduler`]) therefore continue at the priority of
/// the job that started them.
///
/// What the reference implementation additionally does in its context -
/// replacing the blocking wait of the runtime with one that does not pump
/// messages - has no counterpart: blocking primitives never run a message
/// pump here.
///
/// Deprecated: the current priority is [`Dispatcher::current_priority`] (and
/// [`Dispatcher::set_current_priority`]), posting is [`Dispatcher::post`] and
/// [`Dispatcher::post_local`] at that priority, and the code that does not
/// know the dispatcher takes the current schedule of `ferroui-system`
/// ([`ferroui_system::task::current_schedule`], the dispatcher's
/// [`task_schedule`](Dispatcher::task_schedule) of the priority).
#[deprecated(
    since = "0.5.0",
    note = "use `Dispatcher::current_priority`, `Dispatcher::post`/`post_local` at that priority, or the dispatcher's `task_schedule(priority)`"
)]
pub struct FerroSynchronizationContext {
    dispatcher: Arc<Dispatcher>,
    priority: DispatcherPriority,
}

impl FerroSynchronizationContext {
    /// Creates a context for the current thread's dispatcher at
    /// [`DispatcherPriority::DEFAULT`].
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<Self> {
        Self::with_dispatcher(&Dispatcher::current_dispatcher(), DispatcherPriority::DEFAULT)
    }

    /// Creates a context for the current thread's dispatcher.
    pub fn with_priority(priority: DispatcherPriority) -> Arc<Self> {
        Self::with_dispatcher(&Dispatcher::current_dispatcher(), priority)
    }

    pub fn with_dispatcher(dispatcher: &Arc<Dispatcher>, priority: DispatcherPriority) -> Arc<Self> {
        Arc::new(Self { dispatcher: dispatcher.clone(), priority })
    }

    /// The priority at which this context queues work.
    pub fn priority(&self) -> DispatcherPriority {
        self.priority
    }

    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }

    /// The context of the calling thread.
    pub fn current() -> Option<Arc<FerroSynchronizationContext>> {
        CURRENT.try_with(|current| current.borrow().clone()).ok().flatten()
    }

    /// Sets the context of the calling thread and returns the previous one. The dispatcher of the
    /// context at its priority becomes the current schedule of the thread
    /// ([`ferroui_system::task::current_schedule`]); `None` clears it.
    pub fn set_current(context: Option<Arc<FerroSynchronizationContext>>) -> Option<Arc<FerroSynchronizationContext>> {
        let schedule = context.as_deref().map(schedule_of);
        let replaced = task::set_current_schedule(schedule);
        drop(replaced);
        replace_current_context(context)
    }

    /// Controls whether [`install_if_needed`](Self::install_if_needed) does
    /// anything.
    pub fn auto_install() -> bool {
        AUTO_INSTALL.load(Ordering::Relaxed)
    }

    pub fn set_auto_install(value: bool) {
        AUTO_INSTALL.store(value, Ordering::Relaxed);
    }

    /// Makes the [`DispatcherPriority::NORMAL`] context of the current
    /// thread's dispatcher the current context, unless the thread already
    /// has one.
    pub fn install_if_needed() {
        if !Self::auto_install() || Self::current().is_some() {
            return;
        }

        Self::set_current(Some(Dispatcher::current_dispatcher().get_context_with_priority(DispatcherPriority::NORMAL)));
    }

    /// Queues `callback` on the dispatcher at the priority of this context.
    pub fn post(&self, callback: impl FnOnce() + Send + 'static) {
        self.dispatcher.post(callback, self.priority);
    }

    /// Like [`post`](Self::post) for callbacks that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn post_local(&self, callback: impl FnOnce() + 'static) {
        self.dispatcher.post_local(callback, self.priority);
    }

    /// Runs `callback` on the dispatcher thread and waits for it.
    pub fn send(&self, callback: impl FnOnce() + Send + 'static) {
        if self.dispatcher.check_access() {
            // Same-thread, use send priority to avoid any reentrancy.
            self.dispatcher.send(callback, Some(DispatcherPriority::SEND));
        } else {
            self.dispatcher.send(callback, Some(self.priority));
        }
    }

    /// Like [`send`](Self::send) for callbacks that are not `Send`: runs
    /// `callback` right away.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn send_local(&self, callback: impl FnOnce() + 'static) {
        // Same-thread, use send priority to avoid any reentrancy.
        self.dispatcher.send_local(callback, Some(DispatcherPriority::SEND));
    }

    /// The scheduler that runs futures on the dispatcher at the priority of
    /// this context.
    pub fn to_task_scheduler(self: &Arc<Self>) -> DispatcherTaskScheduler {
        DispatcherTaskScheduler { dispatcher: self.dispatcher.clone(), priority: self.priority }
    }

    /// Makes sure that the current context of the calling thread is the one
    /// of the current dispatcher for `priority`. Disposing or dropping the
    /// returned value restores the previous context.
    pub fn ensure(priority: DispatcherPriority) -> RestoreContext {
        Self::ensure_with_dispatcher(&Dispatcher::current_dispatcher(), priority)
    }

    /// Makes sure that the current context of the calling thread is the one
    /// of `dispatcher` for `priority`. Disposing or dropping the returned
    /// value restores the previous context.
    ///
    /// # Panics
    /// Panics when the context has to change and the calling thread is not
    /// the dispatcher thread.
    pub fn ensure_with_dispatcher(dispatcher: &Dispatcher, priority: DispatcherPriority) -> RestoreContext {
        RestoreContext { scope: Cell::new(Some(dispatcher.enter_job(priority))) }
    }
}

/// The context as a `System.Threading.SynchronizationContext` of `ferroui-system`, for the code
/// that is not tied to the user interface (the reactive libraries, the tasks of `ferroui-system`):
/// each member is the method of the same name above.
impl ferroui_system::threading::SynchronizationContext for FerroSynchronizationContext {
    fn post(&self, work: Box<dyn FnOnce() + Send>) {
        FerroSynchronizationContext::post(self, work);
    }

    fn send(&self, work: Box<dyn FnOnce() + Send>) {
        FerroSynchronizationContext::send(self, work);
    }

    fn post_local(&self, work: Box<dyn FnOnce()>) {
        FerroSynchronizationContext::post_local(self, work);
    }

    fn send_local(&self, work: Box<dyn FnOnce()>) {
        FerroSynchronizationContext::send_local(self, work);
    }
}

/// Restores the previous current context; see
/// [`FerroSynchronizationContext::ensure`].
///
/// The context is restored when the value is disposed or dropped, whichever
/// comes first.
#[deprecated(
    since = "0.5.0",
    note = "the dispatcher sets the current priority of its jobs itself; outside of a job use `Dispatcher::set_current_priority`, which gives the previous one"
)]
#[derive(Default)]
pub struct RestoreContext {
    /// `Some` while the context still has to be restored.
    scope: Cell<Option<JobScope>>,
}

impl IDisposable for RestoreContext {
    fn dispose(&self) {
        // Restored outside of the cell.
        let scope = self.scope.take();
        drop(scope);
    }
}

impl Drop for RestoreContext {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// Runs futures on a [`Dispatcher`] at a fixed priority: every poll of a
/// future started here is a dispatcher job of that priority.
///
/// The futures are tasks of the dispatcher
/// ([`Dispatcher::spawn_local_with_priority`],
/// [`Dispatcher::spawn_with_priority`]), with handles that keep the contract
/// of [`DispatcherTask`].
///
/// Obtained from [`Dispatcher::to_task_scheduler`] and
/// [`Dispatcher::to_task_scheduler_with_priority`].
#[derive(Clone)]
pub struct DispatcherTaskScheduler {
    dispatcher: Arc<Dispatcher>,
    priority: DispatcherPriority,
}

impl DispatcherTaskScheduler {
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }

    pub fn priority(&self) -> DispatcherPriority {
        self.priority
    }

    /// Runs `future` on the dispatcher thread. Can be called from any
    /// thread.
    pub fn spawn<Fut>(&self, future: Fut) -> DispatcherTask<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let dispatcher = &self.dispatcher;
        let (task, start) = new_send_task::<Fut::Output>(dispatcher);
        // When the job is aborted its closure is dropped, which cancels the task.
        drop(dispatcher.spawn_with_priority(move || start.body(future), self.priority));
        task
    }

    /// Runs a future that is not `Send` on the dispatcher thread.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn spawn_local<Fut>(&self, future: Fut) -> DispatcherTask<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let dispatcher = &self.dispatcher;
        dispatcher.verify_access();
        let (task, start) = new_local_task::<Fut::Output>(dispatcher);
        drop(dispatcher.spawn_local_with_priority(start.body(future), self.priority));
        task
    }

    /// Starts a future that is not `Send` on the dispatcher thread the way
    /// calling an asynchronous method does: the future is polled for the
    /// first time before this returns, so it runs up to its first pending
    /// await right away (and the returned task is already complete if there
    /// is none). Afterwards it is resumed from dispatcher jobs at the
    /// priority of the scheduler.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn start_local<Fut>(&self, future: Fut) -> DispatcherTask<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let dispatcher = &self.dispatcher;
        dispatcher.verify_access();
        let (task, start) = new_local_task::<Fut::Output>(dispatcher);
        drop(dispatcher.start_local_with_priority(start.body(future), self.priority));
        task
    }
}

impl Dispatcher {
    /// The context of this dispatcher for `priority`. On the dispatcher
    /// thread the contexts are cached, one per priority.
    pub(crate) fn get_context_with_priority(&self, priority: DispatcherPriority) -> Arc<FerroSynchronizationContext> {
        DispatcherPriority::validate(priority, "priority");
        let Some(local) = self.try_local() else {
            return FerroSynchronizationContext::with_dispatcher(&self.to_arc(), priority);
        };
        let index = (priority.value() - DispatcherPriority::MIN_VALUE.value()) as usize;
        let mut contexts = local.priority_contexts.contexts.borrow_mut();
        if contexts.is_empty() {
            let count = (DispatcherPriority::MAX_VALUE.value() - DispatcherPriority::MIN_VALUE.value() + 1) as usize;
            contexts.resize(count, None);
        }
        contexts[index]
            .get_or_insert_with(|| FerroSynchronizationContext::with_dispatcher(&self.to_arc(), priority))
            .clone()
    }

    /// Gets a scheduler which runs futures on this dispatcher. The priority
    /// is the one of the job that calls this ([`Dispatcher::current_priority`]),
    /// [`DispatcherPriority::DEFAULT`] outside of a job.
    pub fn to_task_scheduler(&self) -> DispatcherTaskScheduler {
        self.to_task_scheduler_with_priority(self.current_job_priority())
    }

    /// Gets a scheduler which runs futures on this dispatcher with the
    /// specified priority.
    pub fn to_task_scheduler_with_priority(&self, priority: DispatcherPriority) -> DispatcherTaskScheduler {
        DispatcherPriority::validate(priority, "priority");
        DispatcherTaskScheduler { dispatcher: self.to_arc(), priority }
    }
}
