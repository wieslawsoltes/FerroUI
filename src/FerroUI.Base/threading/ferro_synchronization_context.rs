use std::cell::{Cell, RefCell};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::dispatcher_task::{wrap_local_future, wrap_send_future};
use super::{Dispatcher, DispatcherPriority, DispatcherTask};
use crate::reactive::IDisposable;

static AUTO_INSTALL: AtomicBool = AtomicBool::new(true);

thread_local! {
    static CURRENT: RefCell<Option<Arc<FerroSynchronizationContext>>> = const { RefCell::new(None) };
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

    /// Sets the context of the calling thread and returns the previous one.
    pub fn set_current(context: Option<Arc<FerroSynchronizationContext>>) -> Option<Arc<FerroSynchronizationContext>> {
        CURRENT.try_with(|current| current.replace(context)).ok().flatten()
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
        DispatcherTaskScheduler { context: self.clone() }
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
        let unchanged = CURRENT
            .try_with(|current| current.borrow().as_ref().is_some_and(|context| context.priority == priority))
            .unwrap_or(true);
        if unchanged {
            return RestoreContext::default();
        }
        dispatcher.verify_access();
        let old_context = Self::set_current(Some(dispatcher.get_context_with_priority(priority)));
        RestoreContext { old_context: Cell::new(Some(old_context)) }
    }
}

/// Restores the previous current context; see
/// [`FerroSynchronizationContext::ensure`].
///
/// The context is restored when the value is disposed or dropped, whichever
/// comes first.
#[derive(Default)]
pub struct RestoreContext {
    /// `Some` while the context still has to be restored.
    old_context: Cell<Option<Option<Arc<FerroSynchronizationContext>>>>,
}

impl IDisposable for RestoreContext {
    fn dispose(&self) {
        if let Some(old_context) = self.old_context.take() {
            FerroSynchronizationContext::set_current(old_context);
        }
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
/// Obtained from [`Dispatcher::to_task_scheduler`] or
/// [`FerroSynchronizationContext::to_task_scheduler`].
#[derive(Clone)]
pub struct DispatcherTaskScheduler {
    context: Arc<FerroSynchronizationContext>,
}

impl DispatcherTaskScheduler {
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.context.dispatcher
    }

    pub fn priority(&self) -> DispatcherPriority {
        self.context.priority
    }

    /// Runs `future` on the dispatcher thread. Can be called from any
    /// thread.
    pub fn spawn<Fut>(&self, future: Fut) -> DispatcherTask<Fut::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let dispatcher = self.context.dispatcher.clone();
        let priority = self.context.priority;
        let (task, completion, wrap) = wrap_send_future::<Fut>(&dispatcher);
        let target = dispatcher.clone();
        // When the job is aborted its closure is dropped, which cancels the task.
        dispatcher.post(move || target.start_task(priority, wrap(future), completion, true), priority);
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
        let dispatcher = &self.context.dispatcher;
        dispatcher.verify_access();
        let (task, completion, wrap) = wrap_local_future::<Fut>(dispatcher);
        dispatcher.start_task(self.context.priority, wrap(future), completion, false);
        task
    }

    /// Starts a future that is not `Send` on the dispatcher thread the way
    /// calling an asynchronous method does: the future is polled for the
    /// first time before this returns, so it runs up to its first pending
    /// await right away (and the returned task is already complete if there
    /// is none). Afterwards it is resumed from dispatcher jobs at the
    /// priority of the context.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn start_local<Fut>(&self, future: Fut) -> DispatcherTask<Fut::Output>
    where
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        let dispatcher = &self.context.dispatcher;
        dispatcher.verify_access();
        let (task, completion, wrap) = wrap_local_future::<Fut>(dispatcher);
        dispatcher.start_task(self.context.priority, wrap(future), completion, true);
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
        let mut contexts = local.priority_contexts.borrow_mut();
        if contexts.is_empty() {
            let count = (DispatcherPriority::MAX_VALUE.value() - DispatcherPriority::MIN_VALUE.value() + 1) as usize;
            contexts.resize(count, None);
        }
        contexts[index]
            .get_or_insert_with(|| FerroSynchronizationContext::with_dispatcher(&self.to_arc(), priority))
            .clone()
    }

    /// Gets a scheduler which runs futures on this dispatcher. The priority
    /// is taken from the current [`FerroSynchronizationContext`] if one is
    /// available. Otherwise, [`DispatcherPriority::DEFAULT`] is used.
    pub fn to_task_scheduler(&self) -> DispatcherTaskScheduler {
        match FerroSynchronizationContext::current() {
            Some(context) => self.to_task_scheduler_with_priority(context.priority()),
            None => self.to_task_scheduler_with_priority(DispatcherPriority::DEFAULT),
        }
    }

    /// Gets a scheduler which runs futures on this dispatcher with the
    /// specified priority.
    pub fn to_task_scheduler_with_priority(&self, priority: DispatcherPriority) -> DispatcherTaskScheduler {
        self.get_context_with_priority(priority).to_task_scheduler()
    }
}
