use ferroui_base::input::{PullDirection, PullGestureEndedEventArgs, PullGestureEventArgs};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::rendering::composition::CompositionVisual;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, DirectProperty, FerroObjectImpl,
    FerroProperty, Ref, Size, StyledElementImpl, Vector, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// What a refresh visualizer needs to know about the interaction that
/// drives it: whether the user is pulling, and how far.
#[repr(C)]
pub struct RefreshInfoProvider {
    base: Interactive,
    refresh_pull_direction: Cell<PullDirection>,
    refresh_visualizer_size: Cell<Size>,
    visual: Option<Rc<CompositionVisual>>,
    is_interacting_for_refresh: Cell<bool>,
    interaction_ratio: Cell<f64>,
    entered: Cell<bool>,
    peeking_mode: Cell<bool>,
}

ferro_class!(RefreshInfoProvider: Interactive);
ferroui_base::ferro_class_info!(RefreshInfoProvider {});
ferro_impl_classes!(
    RefreshInfoProvider: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl
);

ferroui_base::ferro_properties! { impl RefreshInfoProvider {
    ferro_property!(
        pub fn is_interacting_for_refresh_property() -> DirectProperty<RefreshInfoProvider, bool> {
            FerroProperty::register_direct::<RefreshInfoProvider, _>(
                "IsInteractingForRefresh",
                |s| s.is_interacting_for_refresh(),
                Some(|s, o| s.set_is_interacting_for_refresh(o)),
                false,
            )
        }
    );

    ferro_property!(
        pub fn execution_ratio_property() -> DirectProperty<RefreshInfoProvider, f64> {
            FerroProperty::register_direct::<RefreshInfoProvider, _>("ExecutionRatio", |s| s.execution_ratio(), None, 0.0)
        }
    );

    ferro_property!(
        pub fn interaction_ratio_property() -> DirectProperty<RefreshInfoProvider, f64> {
            FerroProperty::register_direct::<RefreshInfoProvider, _>(
                "InteractionRatio",
                |s| s.interaction_ratio(),
                Some(|s, o| s.set_interaction_ratio(o)),
                0.0,
            )
        }
    );

    ferro_property!(
        pub fn pull_direction_property() -> DirectProperty<RefreshInfoProvider, PullDirection> {
            FerroProperty::register_direct::<RefreshInfoProvider, _>(
                "PullDirection",
                |s| s.pull_direction(),
                Some(|s, o| s.set_pull_direction(o)),
                PullDirection::TopToBottom,
            )
        }
    );

    ferro_property!(
        pub fn refresh_visualizer_size_property() -> DirectProperty<RefreshInfoProvider, Size> {
            FerroProperty::register_direct::<RefreshInfoProvider, _>(
                "RefreshVisualizerSize",
                |s| s.refresh_visualizer_size(),
                Some(|s, o| s.set_refresh_visualizer_size(o)),
                Size::default(),
            )
        }
    );
} }

impl RefreshInfoProvider {
    pub(crate) const DEFAULT_EXECUTION_RATIO: f64 = 0.8;

    ferro_routed_event!(
        /// Defines the `RefreshStarted` event.
        pub fn refresh_started_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RefreshInfoProvider, _>("RefreshStarted", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `RefreshCompleted` event.
        pub fn refresh_completed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<RefreshInfoProvider, _>("RefreshCompleted", RoutingStrategies::BUBBLE)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(
        refresh_pull_direction: PullDirection,
        refresh_visualizer_size: Option<Size>,
        visual: Option<Rc<CompositionVisual>>,
    ) -> Self {
        Self {
            base: Interactive::construct(),
            refresh_pull_direction: Cell::new(refresh_pull_direction),
            refresh_visualizer_size: Cell::new(refresh_visualizer_size.unwrap_or_default()),
            visual,
            is_interacting_for_refresh: Cell::new(false),
            interaction_ratio: Cell::new(0.0),
            entered: Cell::new(false),
            peeking_mode: Cell::new(false),
        }
    }

    pub fn new(
        refresh_pull_direction: PullDirection,
        refresh_visualizer_size: Option<Size>,
        visual: Option<Rc<CompositionVisual>>,
    ) -> Ref<Self> {
        instantiate(Self::construct(refresh_pull_direction, refresh_visualizer_size, visual))
    }

    pub fn peeking_mode(&self) -> bool {
        self.peeking_mode.get()
    }

    pub(crate) fn set_peeking_mode(&self, value: bool) {
        self.peeking_mode.set(value)
    }

    pub fn is_interacting_for_refresh(&self) -> bool {
        self.is_interacting_for_refresh.get()
    }

    pub(crate) fn set_is_interacting_for_refresh(&self, value: bool) {
        let is_interacting_for_refresh = value && !self.peeking_mode();

        if is_interacting_for_refresh != self.is_interacting_for_refresh.get() {
            self.set_and_raise_cell(
                Self::is_interacting_for_refresh_property(),
                &self.is_interacting_for_refresh,
                is_interacting_for_refresh,
            );

            // Keep `entered` in sync with `IsInteractingForRefresh`. The flag
            // can be cleared by paths other than the end of the pull gesture
            // (the scroll changed and pointer released handlers of the
            // adapter). Without this, `interacting_state_entered` would
            // short-circuit and never re-assert the flag for the rest of the
            // gesture, leaving the visualizer stuck in Idle and the spinner
            // invisible until the next gesture.
            if !is_interacting_for_refresh {
                self.entered.set(false);
            }
        }
    }

    pub fn interaction_ratio(&self) -> f64 {
        self.interaction_ratio.get()
    }

    pub fn set_interaction_ratio(&self, value: f64) {
        self.set_and_raise_cell(Self::interaction_ratio_property(), &self.interaction_ratio, value);
    }

    pub fn refresh_visualizer_size(&self) -> Size {
        self.refresh_visualizer_size.get()
    }

    pub fn set_refresh_visualizer_size(&self, value: Size) {
        self.set_and_raise_cell(Self::refresh_visualizer_size_property(), &self.refresh_visualizer_size, value);
    }

    pub fn pull_direction(&self) -> PullDirection {
        self.refresh_pull_direction.get()
    }

    pub fn set_pull_direction(&self, value: PullDirection) {
        self.set_and_raise_cell(Self::pull_direction_property(), &self.refresh_pull_direction, value);
    }

    pub fn execution_ratio(&self) -> f64 {
        Self::DEFAULT_EXECUTION_RATIO
    }

    pub(crate) fn visual(&self) -> Option<Rc<CompositionVisual>> {
        self.visual.clone()
    }

    // Not used inside the crate (as upstream, where the class is internal).
    #[allow(dead_code)]
    pub fn refresh_started(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::refresh_started_event(), handler)
    }

    // Not used inside the crate (as upstream, where the class is internal).
    #[allow(dead_code)]
    pub fn refresh_completed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::refresh_completed_event(), handler)
    }

    pub(crate) fn interacting_state_entered(&self, e: &PullGestureEventArgs) {
        if !self.entered.get() {
            self.set_is_interacting_for_refresh(true);
            self.entered.set(true);
        }

        self.values_changed(e.delta());
    }

    pub(crate) fn interacting_state_exited(&self, _e: &PullGestureEndedEventArgs) {
        self.set_is_interacting_for_refresh(false);
        self.entered.set(false);

        self.values_changed(Vector::default());
    }

    pub fn on_refresh_started(&self) {
        self.raise_event(&RoutedEventArgs::with_event(Self::refresh_started_event()));
    }

    pub fn on_refresh_completed(&self) {
        self.raise_event(&RoutedEventArgs::with_event(Self::refresh_completed_event()));
    }

    pub(crate) fn values_changed(&self, value: Vector) {
        let size = self.refresh_visualizer_size.get();
        match self.refresh_pull_direction.get() {
            PullDirection::TopToBottom | PullDirection::BottomToTop => {
                self.set_interaction_ratio(if size.height == 0.0 { 1.0 } else { min_with_one(value.y / size.height) });
            }
            PullDirection::LeftToRight | PullDirection::RightToLeft => {
                self.set_interaction_ratio(if size.width == 0.0 { 1.0 } else { min_with_one(value.x / size.width) });
            }
        }
    }
}

/// The smaller of 1 and `value`; not-a-number when `value` is not a number
/// (`f64::min` would return 1).
fn min_with_one(value: f64) -> f64 {
    if value.is_nan() {
        value
    } else {
        value.min(1.0)
    }
}
