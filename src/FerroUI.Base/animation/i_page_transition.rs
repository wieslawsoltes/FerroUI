use crate::animation::AnimationTask;
use crate::threading::{
    CancellationToken, Dispatcher, DispatcherPriority, DispatcherTask, FerroSynchronizationContext,
};
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
    /// The result tracks the progress of the animation: it completes when
    /// the transition has ended or was cancelled, and it fails (re-raising
    /// the failure to whoever observes it) when an animation of the
    /// transition could not be run.
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()>;

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

/// The synchronization context that asynchronous code started on the calling
/// thread continues in: the current one, or the dispatcher's context for
/// [`DispatcherPriority::NORMAL`] when the thread has none.
pub(crate) fn continuation_context() -> std::sync::Arc<FerroSynchronizationContext> {
    FerroSynchronizationContext::current().unwrap_or_else(|| {
        FerroSynchronizationContext::with_dispatcher(&Dispatcher::current_dispatcher(), DispatcherPriority::NORMAL)
    })
}

/// Runs the body of an asynchronous page transition: synchronously up to its
/// first pending await, then resumed through the synchronization context.
pub(crate) fn start_async(future: impl Future<Output = ()> + 'static) -> DispatcherTask<()> {
    continuation_context().to_task_scheduler().start_local(future)
}

/// Awaits the animations of a page transition. A failure to run one of them
/// fails the transition.
pub(crate) async fn when_all(tasks: Vec<AnimationTask>) {
    if let Err(error) = AnimationTask::when_all(tasks).await {
        panic!("{error}");
    }
}
