use super::{RefreshInfoProvider, RefreshRequestedEventArgs, RefreshVisualizerOrientation, RefreshVisualizerState};
use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{ContentControl, ContentControlImpl, Control, ControlImpl, Grid};
use ferroui_base::input::{InputElementImpl, PullDirection};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::TranslateTransform;
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, BoxedValue, DirectProperty,
    FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, Vector3D, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;

const MINIMUM_INDICATOR_OPACITY: f32 = 0.4;

/// Shows the progress of a pull-to-refresh interaction and of the refresh
/// it starts.
#[repr(C)]
pub struct RefreshVisualizer {
    base: ContentControl,
    executing_ratio: Cell<f64>,
    initial_visual_offset: Cell<Vector3D>,
    refresh_visualizer_state: Cell<RefreshVisualizerState>,
    refresh_info_provider: RefCell<Option<Ref<RefreshInfoProvider>>>,
    is_interacting_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    interaction_ratio_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    is_interacting_for_refresh: Cell<bool>,
    root: RefCell<Option<Ref<Grid>>>,
    content: RefCell<Option<Ref<Control>>>,
    /// The subscription to the loaded event of the content.
    content_loaded: Cell<Option<RoutedEventHandlerToken>>,
    orientation: Cell<RefreshVisualizerOrientation>,
    starting_rotation_angle: Cell<f32>,
    interaction_ratio: Cell<f64>,
    played: Cell<bool>,
}

ferro_class!(RefreshVisualizer: ContentControl);
ferroui_base::ferro_class_info!(RefreshVisualizer { new: RefreshVisualizer::new });
ferro_impl_classes!(
    RefreshVisualizer: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    ContentControlImpl
);

impl TemplatedControlImpl for RefreshVisualizer {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        this.set_clip_to_bounds(false);

        let root = e.name_scope().find_as::<Grid>("PART_Root");
        drop(this.root.replace(root.clone()));

        if let Some(root) = root {
            this.on_orientation_changed();

            let content = this.content.borrow().clone();
            if let Some(content) = content {
                root.children().insert(0, content.clone());
                content.set_vertical_alignment(VerticalAlignment::Center);
                content.set_horizontal_alignment(HorizontalAlignment::Center);

                this.update_content();
            }
        }
    }
}

impl VisualImpl for RefreshVisualizer {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        this.update_content();
    }
}

impl FerroObjectImpl for RefreshVisualizer {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::refresh_info_provider_property().as_property() {
            this.on_refresh_info_provider_changed();
        } else if change.property() == ContentControl::content_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<BoxedValue>>();

            if let Some(c) = old_value.as_ref().and_then(Control::from_boxed) {
                if let Some(token) = this.content_loaded.take() {
                    c.remove_handler(Control::loaded_event(), token);
                }
                let root = this.root.borrow().clone();
                if let Some(root) = root {
                    root.children().remove(c);
                }
            }

            let content = new_value.as_ref().and_then(Control::from_boxed);
            drop(this.content.replace(content.clone()));

            if let Some(content) = &content {
                let weak = this.to_ref().downgrade();
                this.content_loaded.set(Some(content.loaded(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.on_content_loaded();
                    }
                })));
            }

            let root = this.root.borrow().clone();
            if let (Some(root), Some(content)) = (root, content) {
                root.children().insert(0, content.clone());
                content.set_vertical_alignment(VerticalAlignment::Center);
                content.set_horizontal_alignment(HorizontalAlignment::Center);

                this.update_content();
            }
        } else if change.property() == Self::orientation_property().as_property() {
            this.on_orientation_changed();

            this.update_content();
        } else if change.property() == Visual::bounds_property().as_property() {
            this.on_bounds_changed();

            this.update_content();
        } else if change.property() == Self::pull_direction_property().as_property() {
            this.on_orientation_changed();

            this.on_bounds_changed();

            this.update_content();
        }
    }
}

ferroui_base::ferro_properties! { impl RefreshVisualizer {
    ferro_property!(
        /// Defines the `PullDirection` property.
        pub(crate) fn pull_direction_property() -> StyledProperty<PullDirection> {
            FerroProperty::register::<RefreshVisualizer, _>("PullDirection", PullDirection::TopToBottom)
        }
    );

    ferro_property!(
        /// Defines the `RefreshVisualizerState` property.
        pub fn refresh_visualizer_state_property() -> DirectProperty<RefreshVisualizer, RefreshVisualizerState> {
            FerroProperty::register_direct::<RefreshVisualizer, _>(
                "RefreshVisualizerState",
                |s| s.refresh_visualizer_state(),
                None,
                RefreshVisualizerState::Idle,
            )
        }
    );

    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> DirectProperty<RefreshVisualizer, RefreshVisualizerOrientation> {
            FerroProperty::register_direct::<RefreshVisualizer, _>(
                "Orientation",
                |s| s.orientation(),
                Some(|s, o| s.set_orientation(o)),
                RefreshVisualizerOrientation::Auto,
            )
        }
    );

    ferro_property!(
        /// Defines the `RefreshInfoProvider` property.
        pub(crate) fn refresh_info_provider_property() -> DirectProperty<RefreshVisualizer, Option<Ref<RefreshInfoProvider>>> {
            FerroProperty::register_direct::<RefreshVisualizer, _>(
                "RefreshInfoProvider",
                |s| s.refresh_info_provider(),
                Some(|s, o| s.set_refresh_info_provider(o)),
                None,
            )
        }
    );
} }

impl RefreshVisualizer {
    ferro_routed_event!(
        /// Defines the `RefreshRequested` event.
        pub fn refresh_requested_event() -> RoutedEvent<RefreshRequestedEventArgs> {
            RoutedEvent::register::<RefreshVisualizer, _>("RefreshRequested", RoutingStrategies::BUBBLE)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            executing_ratio: Cell::new(0.8),
            initial_visual_offset: Cell::new(Vector3D::default()),
            refresh_visualizer_state: Cell::new(RefreshVisualizerState::Idle),
            refresh_info_provider: RefCell::new(None),
            is_interacting_subscription: RefCell::new(None),
            interaction_ratio_subscription: RefCell::new(None),
            is_interacting_for_refresh: Cell::new(false),
            root: RefCell::new(None),
            content: RefCell::new(None),
            content_loaded: Cell::new(None),
            orientation: Cell::new(RefreshVisualizerOrientation::Auto),
            starting_rotation_angle: Cell::new(0.0),
            interaction_ratio: Cell::new(0.0),
            played: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn is_pull_direction_vertical(&self) -> bool {
        let pull_direction = self.pull_direction();
        pull_direction == PullDirection::TopToBottom || pull_direction == PullDirection::BottomToTop
    }

    fn is_pull_direction_far(&self) -> bool {
        let pull_direction = self.pull_direction();
        pull_direction == PullDirection::BottomToTop || pull_direction == PullDirection::RightToLeft
    }

    /// Gets a value that indicates the refresh state of the visualizer.
    ///
    /// Protected upstream; the property itself is public.
    pub fn refresh_visualizer_state(&self) -> RefreshVisualizerState {
        self.refresh_visualizer_state.get()
    }

    fn set_refresh_visualizer_state(&self, value: RefreshVisualizerState) {
        self.set_and_raise_cell(Self::refresh_visualizer_state_property(), &self.refresh_visualizer_state, value);
        self.update_content();
    }

    /// Gets or sets a value that indicates the orientation of the
    /// visualizer.
    pub fn orientation(&self) -> RefreshVisualizerOrientation {
        self.orientation.get()
    }

    pub fn set_orientation(&self, value: RefreshVisualizerOrientation) {
        self.set_and_raise_cell(Self::orientation_property(), &self.orientation, value);
    }

    pub(crate) fn pull_direction(&self) -> PullDirection {
        self.get_value(Self::pull_direction_property())
    }

    pub(crate) fn set_pull_direction(&self, value: PullDirection) {
        self.set_value(Self::pull_direction_property(), value)
    }

    pub(crate) fn refresh_info_provider(&self) -> Option<Ref<RefreshInfoProvider>> {
        self.refresh_info_provider.borrow().clone()
    }

    pub(crate) fn set_refresh_info_provider(&self, value: Option<Ref<RefreshInfoProvider>>) {
        let current = self.refresh_info_provider.borrow().clone();
        if let Some(current) = current {
            current.set_render_transform(None);
        }
        self.set_and_raise(Self::refresh_info_provider_property(), &self.refresh_info_provider, value);
    }

    /// Occurs when an update of the content has been initiated.
    pub fn refresh_requested(
        &self,
        handler: impl Fn(&Interactive, &RefreshRequestedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::refresh_requested_event(), handler)
    }

    fn on_content_loaded(&self) {
        let content = self.content.borrow().clone();
        let Some(content) = content else { return };

        let Some(composition) = ElementComposition::get_element_visual(&content) else { return };

        composition.set_opacity(0.0);

        // COMPOSITION-SEAM: upstream `OnContentLoaded` (RefreshVisualizer.cs
        // lines 158-184) gives the composition visual of the content an
        // implicit animation collection
        // (`Compositor.CreateImplicitAnimationCollection`,
        // `CompositionObject.ImplicitAnimations`) with four key frame
        // animations, each with the single expression key frame
        // (1.0, "this.FinalValue", linear easing):
        //   "RotationAngle" -> `ScalarKeyFrameAnimation`, 100 ms
        //   "Offset"        -> `Vector3KeyFrameAnimation`, 150 ms
        //   "Scale"         -> `Vector3KeyFrameAnimation`, 100 ms
        //   "Opacity"       -> `ScalarKeyFrameAnimation`, 100 ms
        // Key frame animations and implicit animation collections are not
        // ported yet, so the values `update_content` sets apply at once.

        self.update_content();
    }

    fn update_content(&self) {
        let content = self.content.borrow().clone();
        let root = self.root.borrow().clone();
        let (Some(content), Some(root)) = (content, root) else { return };

        let visual = self.refresh_info_provider().and_then(|provider| provider.visual());
        let content_visual = ElementComposition::get_element_visual(&content);
        let visualizer_visual = ElementComposition::get_element_visual(self);
        let (Some(visual), Some(content_visual), Some(visualizer_visual)) = (visual, content_visual, visualizer_visual)
        else {
            return;
        };

        let starting_rotation_angle = self.starting_rotation_angle.get();

        content_visual.set_center_point(Vector3D::new(
            content.bounds().width / 2.0,
            content.bounds().height / 2.0,
            0.0,
        ));
        match self.refresh_visualizer_state() {
            RefreshVisualizerState::Idle => {
                self.played.set(false);
                // COMPOSITION-SEAM: upstream (RefreshVisualizer.cs lines
                // 211-215) ends the endless rotation started in the
                // Refreshing state here: `_rotateAnimation.IterationBehavior
                // = AnimationIterationBehavior.Count; _rotateAnimation =
                // null`. There is no rotation animation to end until key
                // frame animations are ported.

                content_visual.set_opacity(MINIMUM_INDICATOR_OPACITY);
                content_visual.set_rotation_angle(starting_rotation_angle);

                if visualizer_visual.offset().x != 0.0 || visualizer_visual.offset().y != 0.0 {
                    visual.set_offset(self.initial_visual_offset.get());
                    visualizer_visual.set_offset(Vector3D::new(0.0, 0.0, 0.0));
                }

                content.invalidate_measure();
            }
            RefreshVisualizerState::Interacting => {
                self.played.set(false);
                content_visual.set_opacity(MINIMUM_INDICATOR_OPACITY);
                content_visual
                    .set_rotation_angle((starting_rotation_angle as f64 + self.interaction_ratio.get() * 2.0 * PI) as f32);

                self.calculate_and_set_offsets(&root, &visual, &visualizer_visual);
            }
            RefreshVisualizerState::Pending => {
                content_visual.set_opacity(1.0);
                content_visual.set_rotation_angle(starting_rotation_angle + (2.0 * PI) as f32);

                self.calculate_and_set_offsets(&root, &visual, &visualizer_visual);

                if !self.played.get() {
                    self.played.set(true);
                    // COMPOSITION-SEAM: upstream (RefreshVisualizer.cs lines
                    // 246-252) plays a pulse on the content visual here: a
                    // `Vector3KeyFrameAnimation` with target "Scale", the
                    // key frames (0.5, (1.5, 1.5, 1)) and (1.0, (1, 1, 1)),
                    // a duration of 0.3 s, started with
                    // `contentVisual.StartAnimation("Scale", ..)`. Key frame
                    // animations are not ported yet.
                }
            }
            RefreshVisualizerState::Refreshing => {
                // COMPOSITION-SEAM: upstream (RefreshVisualizer.cs lines
                // 256-264) starts the endless rotation of the content
                // visual here and keeps it in `_rotateAnimation`: a
                // `ScalarKeyFrameAnimation` with target "RotationAngle",
                // the linear key frames (0, starting angle) and
                // (1, starting angle + 2 pi), `IterationBehavior = Forever`,
                // `StopBehavior = LeaveCurrentValue`, a duration of 0.5 s,
                // started with `contentVisual.StartAnimation("RotationAngle",
                // ..)`. Key frame animations are not ported yet.
                content_visual.set_opacity(1.0);
                // Upstream computes a translation ratio here that it never
                // uses.

                self.calculate_and_set_offsets(&root, &visual, &visualizer_visual);
            }
            RefreshVisualizerState::Peeking => {
                content_visual.set_opacity(1.0);
                content_visual.set_rotation_angle(starting_rotation_angle);
            }
        }
    }

    fn calculate_and_set_offsets(&self, root: &Grid, visual: &CompositionVisual, indicator_visualizer: &CompositionVisual) {
        let direction = if self.is_pull_direction_far() { -1.0 } else { 1.0 };

        if self.is_pull_direction_vertical() {
            // As long as the indicator container isn't visible, the initial
            // offset is the current offset.
            if indicator_visualizer.offset().y == 0.0 {
                self.initial_visual_offset.set(visual.offset());
            }

            let offset = self.interaction_ratio.get() * direction * root.bounds().height;
            indicator_visualizer.set_offset(Vector3D { y: offset, ..indicator_visualizer.offset() });

            let initial_visual_offset = self.initial_visual_offset.get();
            visual.set_offset(Vector3D { y: initial_visual_offset.y + offset, ..initial_visual_offset });
        } else {
            if indicator_visualizer.offset().x == 0.0 {
                self.initial_visual_offset.set(visual.offset());
            }

            let offset = self.interaction_ratio.get() * direction * root.bounds().width;
            indicator_visualizer.set_offset(Vector3D { x: offset, ..indicator_visualizer.offset() });

            let initial_visual_offset = self.initial_visual_offset.get();
            visual.set_offset(Vector3D { x: initial_visual_offset.x + offset, ..initial_visual_offset });
        }
    }

    /// Initiates an update of the content.
    pub fn request_refresh(&self) {
        if let Some(provider) = self.refresh_info_provider() {
            provider.set_interaction_ratio(1.0);
        }
        self.set_refresh_visualizer_state(RefreshVisualizerState::Refreshing);
        if let Some(provider) = self.refresh_info_provider() {
            provider.on_refresh_started();
        }

        self.raise_refresh_requested();
    }

    fn refresh_completed(&self) {
        if let Some(provider) = self.refresh_info_provider() {
            provider.set_interaction_ratio(0.0);
        }
        self.set_refresh_visualizer_state(RefreshVisualizerState::Idle);
        if let Some(provider) = self.refresh_info_provider() {
            provider.on_refresh_completed();
        }
    }

    fn raise_refresh_requested(&self) {
        let this = self.to_ref();
        let refresh_args =
            RefreshRequestedEventArgs::new(move || this.refresh_completed(), Some(Self::refresh_requested_event()));

        refresh_args.increment_count();

        self.raise_event(&refresh_args);

        refresh_args.decrement_count();
    }

    fn on_bounds_changed(&self) {
        let bounds = self.bounds();
        let transform = match self.pull_direction() {
            PullDirection::TopToBottom => TranslateTransform::with_offset(0.0, -bounds.height),
            PullDirection::BottomToTop => TranslateTransform::with_offset(0.0, bounds.height),
            PullDirection::LeftToRight => TranslateTransform::with_offset(-bounds.width, 0.0),
            PullDirection::RightToLeft => TranslateTransform::with_offset(bounds.width, 0.0),
        };
        self.set_render_transform(Some(transform.into()));
    }

    fn on_orientation_changed(&self) {
        let starting_rotation_angle = match self.orientation.get() {
            RefreshVisualizerOrientation::Auto => match self.pull_direction() {
                PullDirection::TopToBottom | PullDirection::BottomToTop => 0.0,
                PullDirection::LeftToRight => (-PI / 2.0) as f32,
                PullDirection::RightToLeft => (PI / 2.0) as f32,
            },
            RefreshVisualizerOrientation::Normal => 0.0,
            RefreshVisualizerOrientation::Rotate90DegreesCounterclockwise => (PI / 2.0) as f32,
            RefreshVisualizerOrientation::Rotate270DegreesCounterclockwise => (-PI / 2.0) as f32,
        };
        self.starting_rotation_angle.set(starting_rotation_angle);
    }

    fn on_refresh_info_provider_changed(&self) {
        let subscription = self.is_interacting_subscription.replace(None);
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let subscription = self.interaction_ratio_subscription.replace(None);
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        if let Some(provider) = self.refresh_info_provider() {
            let object: &FerroObject = &provider;

            let weak = self.to_ref().downgrade();
            let subscription = FerroObjectExtensions::get_observable(
                object,
                RefreshInfoProvider::is_interacting_for_refresh_property(),
            )
            .subscribe_fn(move |value| {
                if let Some(this) = weak.upgrade() {
                    this.interacting_for_refresh_observer(value);
                }
            });
            drop(self.is_interacting_subscription.replace(Some(subscription)));

            let weak = self.to_ref().downgrade();
            let subscription =
                FerroObjectExtensions::get_observable(object, RefreshInfoProvider::interaction_ratio_property())
                    .subscribe_fn(move |value| {
                        if let Some(this) = weak.upgrade() {
                            this.interaction_ratio_observer(value);
                        }
                    });
            drop(self.interaction_ratio_subscription.replace(Some(subscription)));

            self.executing_ratio.set(provider.execution_ratio());
        } else {
            self.executing_ratio.set(1.0);
        }
    }

    fn interaction_ratio_observer(&self, obj: f64) {
        let was_at_zero = self.interaction_ratio.get() == 0.0;
        self.interaction_ratio.set(obj);

        let interaction_ratio = obj;
        let executing_ratio = self.executing_ratio.get();

        if self.is_interacting_for_refresh.get() {
            match self.refresh_visualizer_state() {
                RefreshVisualizerState::Idle => {
                    if was_at_zero {
                        if interaction_ratio > executing_ratio {
                            self.set_refresh_visualizer_state(RefreshVisualizerState::Pending);
                        } else if interaction_ratio > 0.0 {
                            self.set_refresh_visualizer_state(RefreshVisualizerState::Interacting);
                        }
                    } else if interaction_ratio > 0.0 {
                        self.set_refresh_visualizer_state(RefreshVisualizerState::Peeking);
                    }
                }
                RefreshVisualizerState::Interacting => {
                    if interaction_ratio <= 0.0 {
                        self.set_refresh_visualizer_state(RefreshVisualizerState::Idle);
                    } else if interaction_ratio > executing_ratio {
                        self.set_refresh_visualizer_state(RefreshVisualizerState::Pending);
                    } else {
                        self.update_content();
                    }
                }
                RefreshVisualizerState::Pending => {
                    if interaction_ratio <= executing_ratio {
                        self.set_refresh_visualizer_state(RefreshVisualizerState::Interacting);
                    } else if interaction_ratio <= 0.0 {
                        self.set_refresh_visualizer_state(RefreshVisualizerState::Idle);
                    } else {
                        self.update_content();
                    }
                }
                RefreshVisualizerState::Peeking | RefreshVisualizerState::Refreshing => {}
            }
        } else if self.refresh_visualizer_state() != RefreshVisualizerState::Refreshing {
            if interaction_ratio > 0.0 {
                self.set_refresh_visualizer_state(RefreshVisualizerState::Peeking);
            } else {
                self.set_refresh_visualizer_state(RefreshVisualizerState::Idle);
            }
        }
    }

    fn interacting_for_refresh_observer(&self, obj: bool) {
        self.is_interacting_for_refresh.set(obj);

        if !obj {
            match self.refresh_visualizer_state.get() {
                RefreshVisualizerState::Pending => self.request_refresh(),
                RefreshVisualizerState::Refreshing => {
                    // We don't want to interrupt a currently executing
                    // refresh.
                }
                _ => self.set_refresh_visualizer_state(RefreshVisualizerState::Idle),
            }
        }
    }
}
