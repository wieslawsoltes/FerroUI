use super::Page;
use ferroui_base::animation::IPageTransition;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTask, FerroSynchronizationContext};
use ferroui_base::{BoxedValue, Ref};
use std::future::Future;
use std::rc::Rc;

/// Provides navigation operations for stack-based and modal page
/// navigation. Exposed via the `Navigation` property of a page when the
/// page is hosted in a navigation page.
///
/// Not meant to be implemented by client code.
pub trait INavigation: 'static {
    /// The current navigation stack. The root page is at index 0; the
    /// visible page is last.
    fn navigation_stack(&self) -> Rc<Vec<Ref<Page>>>;

    /// The current modal stack. Index 0 is the oldest (bottom-most) modal;
    /// the last index is the most recently pushed (topmost) modal.
    fn modal_stack(&self) -> Rc<Vec<Ref<Page>>>;

    /// The number of pages in the navigation stack.
    fn stack_depth(&self) -> i32;

    /// Whether a pop operation is possible (the stack has more than one
    /// entry).
    fn can_go_back(&self) -> bool;

    /// Pushes `page` using the host's default transition.
    fn push_async(&self, page: Ref<Page>) -> DispatcherTask<()>;

    /// Pushes `page` using `transition`. Pass `None` for no animation.
    fn push_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()>;

    /// Pushes `page` using `transition` with an optional `parameter`.
    fn push_async_with_parameter(
        &self,
        _page: Ref<Page>,
        _transition: Option<Rc<dyn IPageTransition>>,
        _parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        completed_task()
    }

    /// Pops the top page using the host's default transition.
    fn pop_async(&self) -> DispatcherTask<Option<Ref<Page>>>;

    /// Pops the top page using `transition`. Pass `None` for no animation.
    fn pop_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<Option<Ref<Page>>>;

    /// Pops all pages above the root using the host's default transition.
    fn pop_to_root_async(&self) -> DispatcherTask<()>;

    /// Pops all pages above the root using `transition`. Pass `None` for no
    /// animation.
    fn pop_to_root_async_with_transition(&self, transition: Option<Rc<dyn IPageTransition>>) -> DispatcherTask<()>;

    /// Pops all pages above `page` using the host's default transition.
    fn pop_to_page_async(&self, page: Ref<Page>) -> DispatcherTask<()>;

    /// Pops all pages above `page` using `transition`. Pass `None` for no
    /// animation.
    fn pop_to_page_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()>;

    /// Replaces the current top page with `page` using the host's default
    /// transition.
    fn replace_async(&self, page: Ref<Page>) -> DispatcherTask<()>;

    /// Replaces the current top page with `page` using `transition`. Pass
    /// `None` for no animation.
    fn replace_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()>;

    /// Replaces the current top page with `page` using `transition` with an
    /// optional `parameter`.
    fn replace_async_with_parameter(
        &self,
        _page: Ref<Page>,
        _transition: Option<Rc<dyn IPageTransition>>,
        _parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        completed_task()
    }

    /// Pushes `page` as a modal using the host's modal transition.
    fn push_modal_async(&self, page: Ref<Page>) -> DispatcherTask<()>;

    /// Pushes `page` as a modal using `transition`. Pass `None` for no
    /// animation.
    fn push_modal_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()>;

    /// Pushes `page` as a modal using `transition` with an optional
    /// `parameter`.
    fn push_modal_async_with_parameter(
        &self,
        _page: Ref<Page>,
        _transition: Option<Rc<dyn IPageTransition>>,
        _parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        completed_task()
    }

    /// Pops the top modal page using the host's modal transition.
    fn pop_modal_async(&self) -> DispatcherTask<Option<Ref<Page>>>;

    /// Pops the top modal page using `transition`. Pass `None` for no
    /// animation.
    fn pop_modal_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<Option<Ref<Page>>>;

    /// Pops all modal pages, animating only the topmost dismissal.
    fn pop_all_modals_async(&self) -> DispatcherTask<()>;

    /// Pops all modal pages using `transition`. Pass `None` for no
    /// animation.
    fn pop_all_modals_async_with_transition(&self, transition: Option<Rc<dyn IPageTransition>>)
        -> DispatcherTask<()>;

    /// Inserts `page` immediately before `before` in the stack. Does not
    /// change the currently visible page.
    ///
    /// # Panics
    /// Panics when `before` is not in the navigation stack, or `page` is
    /// already hosted by this navigation page.
    fn insert_page(&self, page: Ref<Page>, before: Ref<Page>);

    /// Removes `page` from the navigation stack without animation. If
    /// `page` is not in the stack the call is a no-op.
    fn remove_page(&self, page: Ref<Page>);

    /// The identity of the navigation service: handles to the same service
    /// compare equal.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

impl PartialEq for dyn INavigation {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

/// Runs the body of an asynchronous navigation member the way calling an
/// asynchronous method does: synchronously up to its first pending await,
/// then resumed through the synchronization context of the calling thread
/// (the dispatcher's context when the thread has none).
pub(crate) fn start_async<T: 'static>(future: impl Future<Output = T> + 'static) -> DispatcherTask<T> {
    let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
        FerroSynchronizationContext::with_dispatcher(&Dispatcher::current_dispatcher(), DispatcherPriority::NORMAL)
    });
    context.to_task_scheduler().start_local(future)
}

/// A task that has already completed.
pub(crate) fn completed_task() -> DispatcherTask<()> {
    start_async(async {})
}
