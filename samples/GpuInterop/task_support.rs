//! What the asynchronous members of the sample are built from (not a port).
//!
//! Upstream writes `async` methods over `Task` and `ValueTask` (`DisposeAsync` of the swapchain
//! images and of the resources of the demos, the initialization of a demo). Here a method that
//! awaits is a future started on the dispatcher ([`start`]), and a job of the compositor
//! ([`ServerJobTask`]) is awaited through [`ServerJobTaskFuture`], as the OpenGL control of the
//! framework does it (`ferroui-opengl`, `composition/task_support.rs`).

use ferroui_base::rendering::composition::{ServerJobError, ServerJobTask};
use ferroui_base::rendering::SwapchainImagePresentStatus;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTask};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Starts an asynchronous method on the current thread: the future runs up to its first
/// pending await right away, as an `async` method of the original does when it is called,
/// and is resumed by the dispatcher. The returned task is the `Task` of the method.
///
/// # Panics
/// Panics when called from a thread other than the dispatcher thread.
pub fn start<T: 'static>(future: impl Future<Output = T> + 'static) -> DispatcherTask<T> {
    let priority = Dispatcher::current_priority().unwrap_or(DispatcherPriority::NORMAL);
    let scheduler = Dispatcher::current_dispatcher().to_task_scheduler_with_priority(priority);
    scheduler.start_local(future)
}

/// The state of a presentation of a swapchain image (`LastPresent` upstream, a `Task`): `None`
/// before the first.
pub fn present_status(last_present: Option<&ServerJobTask<()>>) -> Option<SwapchainImagePresentStatus> {
    let task = last_present?;
    Some(if !task.is_completed() {
        SwapchainImagePresentStatus::Pending
    } else if task.is_faulted() {
        SwapchainImagePresentStatus::Faulted
    } else {
        SwapchainImagePresentStatus::RanToCompletion
    })
}

/// `await task` for a job of the compositor: completes when the job has run, with the error
/// of a job that failed. The result of the job is left in the task.
pub struct ServerJobTaskFuture<T> {
    task: ServerJobTask<T>,
    registered: bool,
}

impl<T: Send + 'static> ServerJobTaskFuture<T> {
    pub fn new(task: ServerJobTask<T>) -> Self {
        Self { task, registered: false }
    }
}

impl<T: Send + 'static> Future for ServerJobTaskFuture<T> {
    type Output = Result<(), ServerJobError>;

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
