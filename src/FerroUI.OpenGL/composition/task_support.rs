//! What the asynchronous members of this module are built from.
//!
//! Not a port: the original writes `async` methods over `Task`. Here a method that awaits
//! is a future started on the dispatcher ([`start`]), and a job of the compositor
//! ([`ServerJobTask`]) is awaited through [`ServerJobTaskFuture`].

use ferroui_base::rendering::composition::ServerJobTask;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTask, FerroSynchronizationContext};
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// Starts an asynchronous method on the current thread: the future runs up to its first
/// pending await right away, as an `async` method of the original does when it is called,
/// and is resumed by the dispatcher. The returned task is the `Task` of the method.
///
/// # Panics
/// Panics when called from a thread other than the dispatcher thread.
pub(crate) fn start<T: 'static>(future: impl Future<Output = T> + 'static) -> DispatcherTask<T> {
    let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
        FerroSynchronizationContext::with_dispatcher(&Dispatcher::current_dispatcher(), DispatcherPriority::NORMAL)
    });
    context.to_task_scheduler().start_local(future)
}

/// `await task` for a job of the compositor: completes when the job has run, with the error
/// of a job that failed. The result of the job is left in the task.
pub(crate) struct ServerJobTaskFuture<T> {
    task: ServerJobTask<T>,
    registered: bool,
}

impl<T: Send + 'static> ServerJobTaskFuture<T> {
    pub(crate) fn new(task: ServerJobTask<T>) -> Self {
        Self { task, registered: false }
    }
}

impl<T: Send + 'static> Future for ServerJobTaskFuture<T> {
    type Output = Result<(), ferroui_base::rendering::composition::ServerJobError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.task.is_completed() {
            return Poll::Ready(match self.task.exception() {
                Some(error) => Err(error),
                None => Ok(()),
            });
        }
        if !self.registered {
            self.registered = true;
            let waker = cx.waker().clone();
            self.task.on_completed(move || waker.wake());
        }
        Poll::Pending
    }
}
