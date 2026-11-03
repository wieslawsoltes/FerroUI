use super::Page;
use ferroui_base::animation::IPageTransition;
#[allow(deprecated)]
use ferroui_base::threading::DispatcherTask;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{BoxedValue, Ref};
use ferroui_system::task::JoinHandle;
use std::future::Future;
use std::rc::Rc;

/// Provides navigation operations for stack-based and modal page
/// navigation. Exposed via the `Navigation` property of a page when the
/// page is hosted in a navigation page.
///
/// Not meant to be implemented by client code.
///
/// The asynchronous members give the [`JoinHandle`] of a task of the
/// dispatcher of the calling thread: awaited, it gives the outcome of the
/// navigation, or a `JoinError` (`Panicked` when the navigation failed,
/// `Canceled` when its task was aborted or the dispatcher shut down);
/// dropped, the navigation goes on. Their deprecated `..._task` forms give a
/// `DispatcherTask`, which raises a failure again when it is observed.
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
    fn push_async(&self, page: Ref<Page>) -> JoinHandle<()>;

    /// Pushes `page` using `transition`. Pass `None` for no animation.
    fn push_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> JoinHandle<()>;

    /// Pushes `page` using `transition` with an optional `parameter`.
    fn push_async_with_parameter(
        &self,
        _page: Ref<Page>,
        _transition: Option<Rc<dyn IPageTransition>>,
        _parameter: Option<BoxedValue>,
    ) -> JoinHandle<()> {
        completed_task()
    }

    /// Pops the top page using the host's default transition.
    fn pop_async(&self) -> JoinHandle<Option<Ref<Page>>>;

    /// Pops the top page using `transition`. Pass `None` for no animation.
    fn pop_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> JoinHandle<Option<Ref<Page>>>;

    /// Pops all pages above the root using the host's default transition.
    fn pop_to_root_async(&self) -> JoinHandle<()>;

    /// Pops all pages above the root using `transition`. Pass `None` for no
    /// animation.
    fn pop_to_root_async_with_transition(&self, transition: Option<Rc<dyn IPageTransition>>) -> JoinHandle<()>;

    /// Pops all pages above `page` using the host's default transition.
    fn pop_to_page_async(&self, page: Ref<Page>) -> JoinHandle<()>;

    /// Pops all pages above `page` using `transition`. Pass `None` for no
    /// animation.
    fn pop_to_page_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> JoinHandle<()>;

    /// Replaces the current top page with `page` using the host's default
    /// transition.
    fn replace_async(&self, page: Ref<Page>) -> JoinHandle<()>;

    /// Replaces the current top page with `page` using `transition`. Pass
    /// `None` for no animation.
    fn replace_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> JoinHandle<()>;

    /// Replaces the current top page with `page` using `transition` with an
    /// optional `parameter`.
    fn replace_async_with_parameter(
        &self,
        _page: Ref<Page>,
        _transition: Option<Rc<dyn IPageTransition>>,
        _parameter: Option<BoxedValue>,
    ) -> JoinHandle<()> {
        completed_task()
    }

    /// Pushes `page` as a modal using the host's modal transition.
    fn push_modal_async(&self, page: Ref<Page>) -> JoinHandle<()>;

    /// Pushes `page` as a modal using `transition`. Pass `None` for no
    /// animation.
    fn push_modal_async_with_transition(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> JoinHandle<()>;

    /// Pushes `page` as a modal using `transition` with an optional
    /// `parameter`.
    fn push_modal_async_with_parameter(
        &self,
        _page: Ref<Page>,
        _transition: Option<Rc<dyn IPageTransition>>,
        _parameter: Option<BoxedValue>,
    ) -> JoinHandle<()> {
        completed_task()
    }

    /// Pops the top modal page using the host's modal transition.
    fn pop_modal_async(&self) -> JoinHandle<Option<Ref<Page>>>;

    /// Pops the top modal page using `transition`. Pass `None` for no
    /// animation.
    fn pop_modal_async_with_transition(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> JoinHandle<Option<Ref<Page>>>;

    /// Pops all modal pages, animating only the topmost dismissal.
    fn pop_all_modals_async(&self) -> JoinHandle<()>;

    /// Pops all modal pages using `transition`. Pass `None` for no
    /// animation.
    fn pop_all_modals_async_with_transition(&self, transition: Option<Rc<dyn IPageTransition>>)
        -> JoinHandle<()>;

    // The deprecated forms of the asynchronous members, which give a `DispatcherTask` (step A6 of
    // phase C4 of #423): each converts the handle of its member.

    /// The deprecated form of [`push_async`](Self::push_async): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `push_async`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn push_async_task(&self, page: Ref<Page>) -> DispatcherTask<()> {
        self.push_async(page).into()
    }

    /// The deprecated form of [`push_async_with_transition`](Self::push_async_with_transition): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `push_async_with_transition`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn push_async_with_transition_task(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        self.push_async_with_transition(page, transition).into()
    }

    /// The deprecated form of [`push_async_with_parameter`](Self::push_async_with_parameter): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `push_async_with_parameter`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn push_async_with_parameter_task(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        self.push_async_with_parameter(page, transition, parameter).into()
    }

    /// The deprecated form of [`pop_async`](Self::pop_async): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_async`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_async_task(&self) -> DispatcherTask<Option<Ref<Page>>> {
        self.pop_async().into()
    }

    /// The deprecated form of [`pop_async_with_transition`](Self::pop_async_with_transition): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_async_with_transition`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_async_with_transition_task(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<Option<Ref<Page>>> {
        self.pop_async_with_transition(transition).into()
    }

    /// The deprecated form of [`pop_to_root_async`](Self::pop_to_root_async): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_to_root_async`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_to_root_async_task(&self) -> DispatcherTask<()> {
        self.pop_to_root_async().into()
    }

    /// The deprecated form of [`pop_to_root_async_with_transition`](Self::pop_to_root_async_with_transition):
    /// a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_to_root_async_with_transition`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_to_root_async_with_transition_task(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        self.pop_to_root_async_with_transition(transition).into()
    }

    /// The deprecated form of [`pop_to_page_async`](Self::pop_to_page_async): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_to_page_async`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_to_page_async_task(&self, page: Ref<Page>) -> DispatcherTask<()> {
        self.pop_to_page_async(page).into()
    }

    /// The deprecated form of [`pop_to_page_async_with_transition`](Self::pop_to_page_async_with_transition):
    /// a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_to_page_async_with_transition`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_to_page_async_with_transition_task(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        self.pop_to_page_async_with_transition(page, transition).into()
    }

    /// The deprecated form of [`replace_async`](Self::replace_async): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `replace_async`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn replace_async_task(&self, page: Ref<Page>) -> DispatcherTask<()> {
        self.replace_async(page).into()
    }

    /// The deprecated form of [`replace_async_with_transition`](Self::replace_async_with_transition):
    /// a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `replace_async_with_transition`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn replace_async_with_transition_task(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        self.replace_async_with_transition(page, transition).into()
    }

    /// The deprecated form of [`replace_async_with_parameter`](Self::replace_async_with_parameter): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `replace_async_with_parameter`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn replace_async_with_parameter_task(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        self.replace_async_with_parameter(page, transition, parameter).into()
    }

    /// The deprecated form of [`push_modal_async`](Self::push_modal_async): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `push_modal_async`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn push_modal_async_task(&self, page: Ref<Page>) -> DispatcherTask<()> {
        self.push_modal_async(page).into()
    }

    /// The deprecated form of [`push_modal_async_with_transition`](Self::push_modal_async_with_transition):
    /// a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `push_modal_async_with_transition`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn push_modal_async_with_transition_task(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        self.push_modal_async_with_transition(page, transition).into()
    }

    /// The deprecated form of [`push_modal_async_with_parameter`](Self::push_modal_async_with_parameter):
    /// a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `push_modal_async_with_parameter`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn push_modal_async_with_parameter_task(
        &self,
        page: Ref<Page>,
        transition: Option<Rc<dyn IPageTransition>>,
        parameter: Option<BoxedValue>,
    ) -> DispatcherTask<()> {
        self.push_modal_async_with_parameter(page, transition, parameter).into()
    }

    /// The deprecated form of [`pop_modal_async`](Self::pop_modal_async): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_modal_async`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_modal_async_task(&self) -> DispatcherTask<Option<Ref<Page>>> {
        self.pop_modal_async().into()
    }

    /// The deprecated form of [`pop_modal_async_with_transition`](Self::pop_modal_async_with_transition):
    /// a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_modal_async_with_transition`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_modal_async_with_transition_task(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<Option<Ref<Page>>> {
        self.pop_modal_async_with_transition(transition).into()
    }

    /// The deprecated form of [`pop_all_modals_async`](Self::pop_all_modals_async): a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_all_modals_async`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_all_modals_async_task(&self) -> DispatcherTask<()> {
        self.pop_all_modals_async().into()
    }

    /// The deprecated form of [`pop_all_modals_async_with_transition`](Self::pop_all_modals_async_with_transition):
    /// a `DispatcherTask`.
    #[deprecated(since = "0.5.0", note = "use `pop_all_modals_async_with_transition`, which gives a `JoinHandle`")]
    #[allow(deprecated)]
    fn pop_all_modals_async_with_transition_task(
        &self,
        transition: Option<Rc<dyn IPageTransition>>,
    ) -> DispatcherTask<()> {
        self.pop_all_modals_async_with_transition(transition).into()
    }

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
/// then resumed by the dispatcher of the calling thread at the priority of
/// the running job (the synchronization context of the original), Normal
/// outside of one.
pub(crate) fn start_async<T: 'static>(future: impl Future<Output = T> + 'static) -> JoinHandle<T> {
    let priority = Dispatcher::current_priority().unwrap_or(DispatcherPriority::NORMAL);
    Dispatcher::current_dispatcher().start_local_with_priority(future, priority)
}

/// A task that has already completed.
pub(crate) fn completed_task() -> JoinHandle<()> {
    start_async(async {})
}
