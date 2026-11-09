use crate::animation::AnimationTask;
use crate::threading::{CancellationToken, Dispatcher, DispatcherPriority};
use ferroui_system::task::{JoinError, JoinHandle};
use std::future::Future;
use crate::{Ref, Visual};

/// Interface for animations that transition between two pages.
pub trait IPageTransition: 'static {
    /// Starts the animation.
    ///
    /// `from` is the control that is being transitioned away from, if any,
    /// and `to` the control that is being transitioned to, if any. `forward`
    /// tells whether the transition goes forwards or backwards. The
    /// transition stops as it is when `cancellation_token` is cancelled.
    ///
    /// The result tracks the progress of the animation: the handle of a task
    /// of the dispatcher that ends with `Ok(())` when the transition has
    /// ended or was cancelled, and with `JoinError::Panicked` when an
    /// animation of the transition could not be run (the failure is a value
    /// of the handle, which the caller handles).
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<()>;

    /// The transition as a progress-driven transition, if it is one.
    fn as_progress_page_transition(&self) -> Option<&dyn crate::animation::IProgressPageTransition> {
        None
    }

    /// The transition as a page slide, if it is one.
    fn as_page_slide(&self) -> Option<&crate::animation::PageSlide> {
        None
    }

    /// The transition as a composite page transition, if it is one.
    fn as_composite_page_transition(&self) -> Option<&crate::animation::CompositePageTransition> {
        None
    }
}

impl PartialEq for dyn IPageTransition {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const dyn IPageTransition, other as *const dyn IPageTransition)
    }
}

/// The priority at which asynchronous code started on the calling thread
/// continues on the dispatcher of the thread: the one of the running job
/// ([`Dispatcher::current_priority`]), or [`DispatcherPriority::NORMAL`]
/// outside of a job.
pub(crate) fn continuation_priority() -> DispatcherPriority {
    Dispatcher::current_priority().unwrap_or(DispatcherPriority::NORMAL)
}

/// Runs the body of an asynchronous page transition: synchronously up to its
/// first pending await, then resumed by the dispatcher at the priority it
/// started at. An `Err` of the body is the failure of the handle
/// (`JoinError::Panicked` when an animation could not be run), without a
/// panic (#567).
pub(crate) fn start_async(future: impl Future<Output = Result<(), JoinError>> + 'static) -> JoinHandle<()> {
    Dispatcher::current_dispatcher().start_local_on_result_with_priority(future, continuation_priority())
}

/// Awaits the animations of a page transition. A failure to run one of them
/// fails the transition: `JoinError::Panicked` with the message of the error
/// (the payload a `panic!` with it had, before #567).
pub(crate) async fn when_all(tasks: Vec<AnimationTask>) -> Result<(), JoinError> {
    AnimationTask::when_all(tasks).await.map_err(|error| JoinError::Panicked(Box::new(error.to_string())))
}
