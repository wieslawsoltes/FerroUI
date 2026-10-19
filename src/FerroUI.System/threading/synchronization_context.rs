//! `System.Threading.SynchronizationContext`: where work posted from any thread runs, and the
//! current context of a thread (docs/porting/reactive.md, sections 5.1 and 5.8).
//!
//! A context is shared between threads (`Arc<dyn SynchronizationContext>`), as in .NET, and its
//! work comes in two forms: `Send` work that any thread posts ([`post`](SynchronizationContext::post),
//! [`send`](SynchronizationContext::send)), and work that is not `Send`, posted from the thread of
//! the context itself ([`post_local`](SynchronizationContext::post_local),
//! [`send_local`](SynchronizationContext::send_local)). The second form is what the single-threaded
//! code of the workspace (the observables, the tasks of [`tasks`](super::tasks)) uses: it defers
//! its work to the context of its own thread.
//!
//! The current context of a thread ([`current_synchronization_context`],
//! `SynchronizationContext.Current`) is set with [`set_synchronization_context`]
//! (`SynchronizationContext.SetSynchronizationContext`). Setting it also makes it the thread poster
//! of the thread ([`tasks::set_thread_poster`](super::tasks::set_thread_poster)): a wake of a task
//! of the thread, or a cancellation registered there with
//! [`CancellationToken::register_local`](super::CancellationToken::register_local), that comes from
//! another thread is posted to the context. The context of a thread is therefore expected to run
//! the work it is posted on that thread.

use super::tasks::{set_thread_poster, ThreadPoster};
use std::cell::RefCell;
use std::sync::Arc;

/// Where posted work runs (`System.Threading.SynchronizationContext`).
///
/// The base class of .NET posts to the thread pool and sends on the calling thread; here every
/// implementation says where its work runs. An implementation is shared between threads, as the
/// contexts of .NET are.
pub trait SynchronizationContext: Send + Sync {
    /// Queues `work` to run on the context, from any thread, without waiting for it (`Post`).
    fn post(&self, work: Box<dyn FnOnce() + Send>);

    /// Runs `work` on the context, from any thread, and waits for it (`Send`).
    fn send(&self, work: Box<dyn FnOnce() + Send>);

    /// Queues `work`, which is not `Send`, to run on the context (`Post` from the thread of the
    /// context). Implementations may panic when it is called from another thread.
    fn post_local(&self, work: Box<dyn FnOnce()>);

    /// Runs `work`, which is not `Send`, on the context and waits for it (`Send` from the thread of
    /// the context). Implementations may panic when it is called from another thread.
    fn send_local(&self, work: Box<dyn FnOnce()>);

    /// Tells the context that an operation has started (`OperationStarted`); nothing by default.
    fn operation_started(&self) {}

    /// Tells the context that an operation has completed (`OperationCompleted`); nothing by
    /// default.
    fn operation_completed(&self) {}
}

crate::ferro_thread_local! {
    /// The current context of the thread (`SynchronizationContext.Current`).
    static CURRENT: RefCell<Option<Arc<dyn SynchronizationContext>>> = const { RefCell::new(None) };
}

/// The current context of the calling thread (`SynchronizationContext.Current`).
pub fn current_synchronization_context() -> Option<Arc<dyn SynchronizationContext>> {
    CURRENT.try_with(|current| current.borrow().clone()).ok().flatten()
}

/// Sets the current context of the calling thread (`SynchronizationContext.SetSynchronizationContext`),
/// and makes it the thread poster of the thread, through which other threads wake its tasks and
/// run its cancellations; `None` clears both.
pub fn set_synchronization_context(context: Option<Arc<dyn SynchronizationContext>>) {
    let poster: Option<ThreadPoster> = context.as_ref().map(|context| {
        let context = context.clone();
        let poster: ThreadPoster = Arc::new(move |work: Box<dyn FnOnce() + Send>| context.post(work));
        poster
    });
    // The previous context is dropped outside the borrow, in case its drop reads the current one.
    let previous = CURRENT.try_with(|current| current.replace(context)).ok().flatten();
    drop(previous);
    set_thread_poster(poster);
}

impl dyn SynchronizationContext {
    /// The current context of the calling thread (`SynchronizationContext.Current`); the same as
    /// [`current_synchronization_context`].
    pub fn current() -> Option<Arc<dyn SynchronizationContext>> {
        current_synchronization_context()
    }

    /// Sets the current context of the calling thread
    /// (`SynchronizationContext.SetSynchronizationContext`); the same as
    /// [`set_synchronization_context`].
    pub fn set_synchronization_context(context: Option<Arc<dyn SynchronizationContext>>) {
        set_synchronization_context(context);
    }

    /// Whether `context` is the current context of the calling thread
    /// (`context == SynchronizationContext.Current`, a comparison of references).
    pub fn is_current(context: &Arc<dyn SynchronizationContext>) -> bool {
        current_synchronization_context().is_some_and(|current| std::ptr::addr_eq(Arc::as_ptr(&current), Arc::as_ptr(context)))
    }
}
