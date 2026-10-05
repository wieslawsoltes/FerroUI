use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControlImpl;
use crate::{
    ContentControl, ContentControlImpl, ContentControlImplExt, ControlImpl, TransitionCompletedEventArgs,
};
use ferroui_base::animation::{CrossFade, IPageTransition, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::threading::{
    CancellationToken, CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTask,
    FerroSynchronizationContext,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl,
    StyledProperty, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// Displays the content according to a data template, using a page
/// transition to move between the old and new content.
#[repr(C)]
pub struct TransitioningContentControl {
    base: ContentControl,
    current_transition: RefCell<Option<CancellationTokenSource>>,
    last_presenter: RefCell<Option<Ref<ContentPresenter>>>,
    presenter2: RefCell<Option<Ref<ContentPresenter>>>,
    is_first_full: Cell<bool>,
    should_animate: Cell<bool>,
}

ferro_class!(TransitioningContentControl: ContentControl);
ferro_class_info!(TransitioningContentControl { new: TransitioningContentControl::new });
ferro_impl_classes!(
    TransitioningContentControl: StyledElementImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for TransitioningContentControl {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == ContentControl::content_property().as_property() {
            this.update_content(true);
            return;
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl VisualImpl for TransitioningContentControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.update_content(false);
    }
}

impl LayoutableImpl for TransitioningContentControl {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let result = Self::parent_arrange_override(this, final_size);

        if this.should_animate.get() {
            let current_transition = this.current_transition.borrow().clone();
            if let Some(current_transition) = current_transition {
                current_transition.cancel();
            }

            let presenter2 = this.presenter2.borrow().clone();
            if let (Some(presenter2), Some(presenter), Some(transition)) =
                (presenter2, this.presenter(), this.page_transition())
            {
                this.should_animate.set(false);

                let cancel = CancellationTokenSource::new();
                *this.current_transition.borrow_mut() = Some(cancel.clone());

                let (from, to) =
                    if this.is_first_full.get() { (presenter2, presenter) } else { (presenter, presenter2) };
                let from_content = from.content();
                let to_content = to.content();

                let from: Ref<Visual> = from.upcast();
                let to: Ref<Visual> = to.upcast();
                let task = transition.start(Some(&from), Some(&to), !this.is_transition_reversed(), cancel.token());

                // The continuation always runs as a job of the
                // synchronization context, never inside this arrange pass.
                let this = this.to_ref();
                let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
                    FerroSynchronizationContext::with_dispatcher(
                        &Dispatcher::current_dispatcher(),
                        DispatcherPriority::NORMAL,
                    )
                });
                drop(context.to_task_scheduler().start_local(async move {
                    if task.is_completed() {
                        YieldOnce(false).await;
                    }
                    let ran_to_completion = TransitionEnd { task }.await;

                    this.on_transition_completed(&TransitionCompletedEventArgs::new(
                        from_content,
                        to_content,
                        ran_to_completion && !cancel.is_cancellation_requested(),
                    ));

                    if !cancel.is_cancellation_requested() {
                        this.hide_old_presenter();
                    }
                }));
            }

            this.should_animate.set(false);
        }

        result
    }
}

impl ContentControlImpl for TransitioningContentControl {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        if Self::parent_register_content_presenter(this, presenter) {
            return true;
        }

        if presenter.name().as_deref() == Some("PART_ContentPresenter2") {
            *this.presenter2.borrow_mut() = Some(presenter.to_ref());
            presenter.set_is_visible(false);
            this.update_content(false);
            return true;
        }

        false
    }
}

/// Waits for a page transition to end; the result tells whether it ran to
/// completion. The failure of a transition is not re-raised.
struct TransitionEnd {
    task: DispatcherTask<()>,
}

impl Future for TransitionEnd {
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
        if self.task.is_faulted() {
            return Poll::Ready(false);
        }

        match Pin::new(&mut self.task).poll(cx) {
            Poll::Ready(result) => Poll::Ready(result.is_ok()),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Suspends once: the rest of the asynchronous method runs from a job of its
/// scheduler.
struct YieldOnce(bool);

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }

        self.0 = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// The default page transition: a cross-fade whose settings cannot be
/// changed.
struct ImmutableCrossFade {
    inner: CrossFade,
}

impl ImmutableCrossFade {
    fn new(duration: TimeSpan) -> Self {
        Self { inner: CrossFade::with_duration(duration) }
    }
}

impl IPageTransition for ImmutableCrossFade {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        _forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        self.inner.start_cross_fade(from, to, cancellation_token)
    }
}

ferro_properties! {
    impl TransitioningContentControl {
        /// Defines the `PageTransition` property.
        pub fn page_transition_property() -> StyledProperty<Option<Rc<dyn IPageTransition>>> {
            let default: Rc<dyn IPageTransition> = Rc::new(ImmutableCrossFade::new(TimeSpan::from_milliseconds(125.0)));
            FerroProperty::register::<TransitioningContentControl, _>("PageTransition", Some(default))
        }

        /// Defines the `IsTransitionReversed` property.
        pub fn is_transition_reversed_property() -> StyledProperty<bool> {
            FerroProperty::register::<TransitioningContentControl, _>("IsTransitionReversed", false)
        }
    }
}

impl TransitioningContentControl {
    ferro_routed_event!(
        /// Defines the `TransitionCompleted` routed event.
        pub fn transition_completed_event() -> RoutedEvent<TransitionCompletedEventArgs> {
            RoutedEvent::register::<TransitioningContentControl, _>("TransitionCompleted", RoutingStrategies::DIRECT)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            current_transition: RefCell::new(None),
            last_presenter: RefCell::new(None),
            presenter2: RefCell::new(None),
            is_first_full: Cell::new(false),
            should_animate: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The animation played when content appears and disappears.
    pub fn page_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.get_value(Self::page_transition_property())
    }

    pub fn set_page_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.set_value(Self::page_transition_property(), value)
    }

    /// Whether the control is animated in the reverse direction.
    ///
    /// May not apply to all transitions.
    pub fn is_transition_reversed(&self) -> bool {
        self.get_value(Self::is_transition_reversed_property())
    }

    pub fn set_is_transition_reversed(&self, value: bool) {
        self.set_value(Self::is_transition_reversed_property(), value)
    }

    /// Raised when the old content isn't needed anymore by the control,
    /// because the transition has completed.
    pub fn transition_completed(
        &self,
        handler: impl Fn(&Interactive, &TransitionCompletedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::transition_completed_event(), handler)
    }

    fn update_content(&self, with_transition: bool) {
        let presenter2 = self.presenter2.borrow().clone();
        let (Some(presenter2), Some(presenter)) = (presenter2, self.presenter()) else {
            return;
        };
        if self.visual_root().is_none() {
            return;
        }

        let current_presenter = if self.is_first_full.get() { presenter2 } else { presenter };
        let last_presenter = self.last_presenter.borrow().clone();
        let from_content = last_presenter.as_ref().and_then(|presenter| presenter.content());
        let to_content = self.content();

        if let Some(last_presenter) = &last_presenter {
            if !last_presenter.ptr_eq(&current_presenter)
                && crate::reference_equals(&last_presenter.content(), &to_content) {
                last_presenter.set_content(None);
            }
        }

        current_presenter.set_content(to_content.clone());
        current_presenter.set_is_visible(true);
        *self.last_presenter.borrow_mut() = Some(current_presenter);

        self.is_first_full.set(!self.is_first_full.get());

        if self.page_transition().is_some() && with_transition {
            self.should_animate.set(true);
            self.invalidate_arrange();
        } else {
            self.hide_old_presenter();
            self.on_transition_completed(&TransitionCompletedEventArgs::new(from_content, to_content, false));
        }
    }

    fn hide_old_presenter(&self) {
        let old_presenter = if self.is_first_full.get() { self.presenter2.borrow().clone() } else { self.presenter() };
        if let Some(old_presenter) = old_presenter {
            old_presenter.set_content(None);
            old_presenter.set_is_visible(false);
        }
    }

    fn on_transition_completed(&self, e: &TransitionCompletedEventArgs) {
        self.raise_event(e);
    }
}
