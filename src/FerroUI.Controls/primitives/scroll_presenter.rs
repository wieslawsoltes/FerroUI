//! WinUI's `ScrollPresenter` (`controls/dev/ScrollPresenter/ScrollPresenter.cpp`, `.h` and `.idl`
//! of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{IScrollController, ScrollBarVisibility};
use crate::scroll_view::{
    ScrollingAnimationMode, ScrollingChainMode, ScrollingContentOrientation, ScrollingInputKinds,
    ScrollingInteractionState, ScrollingRailMode, ScrollingScrollAnimationStartingEventArgs,
    ScrollingScrollCompletedEventArgs, ScrollingScrollMode, ScrollingScrollOptions,
    ScrollingScrollStartingEventArgs, ScrollingZoomAnimationStartingEventArgs,
    ScrollingZoomCompletedEventArgs, ScrollingZoomMode, ScrollingZoomOptions,
    ScrollingZoomStartingEventArgs,
};
use crate::{Control, ControlImpl, ScrollViewer, TopLevel, ZoomMode};
use ferroui_base::animation::easings::IEasing;
use ferroui_base::input::{InputElement, InputElementImpl, KeyModifiers, PointerWheelEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::{LayoutHelper, Layoutable, LayoutableImpl};
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::rendering::composition::animations::{KeyFrame, Vector3KeyFrameAnimation};
use ferroui_base::rendering::composition::{Compositor, ElementComposition};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl,
    StyledProperty, Thickness, Upcast, Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// `s_offsetsChangeMsPerUnit`, `s_offsetsChangeMinMs` and `s_offsetsChangeMaxMs`: the duration of
/// the default scroll animation, 5 ms per pixel between 50 ms and 1 s.
const OFFSETS_CHANGE_MS_PER_UNIT: f64 = 5.0;
const OFFSETS_CHANGE_MIN_MS: f64 = 50.0;
const OFFSETS_CHANGE_MAX_MS: f64 = 1000.0;
/// `s_zoomFactorChangeMsPerUnit` and its bounds: 250 ms per unit of zoom.
const ZOOM_FACTOR_CHANGE_MS_PER_UNIT: f64 = 250.0;
const ZOOM_FACTOR_CHANGE_MIN_MS: f64 = 50.0;
const ZOOM_FACTOR_CHANGE_MAX_MS: f64 = 1000.0;
/// `c_scrollPresenterDefaultInertiaDecayRate`.
const DEFAULT_INERTIA_DECAY_RATE: f32 = 0.95;
/// The speed under which a velocity change ends, in pixels (or zoom units) per second.
const MIN_INERTIA_VELOCITY: f64 = 1.0;

/// Evaluates the key frames of a composition animation at `progress` (0 to 1), from `start`, the
/// value of the animated property when the animation starts (the implicit key frame at 0), as
/// the compositor does: each segment eases with the easing of its end key frame.
pub(crate) fn sample_key_frames<T: Copy>(
    frames: &[KeyFrame<T>],
    start: T,
    progress: f32,
    lerp: impl Fn(T, T, f64) -> T,
) -> T {
    let progress = progress.clamp(0.0, 1.0);
    let mut previous = (0.0f32, start);
    for frame in frames {
        if progress <= frame.normalized_progress_key {
            let span = frame.normalized_progress_key - previous.0;
            let local = if span <= 0.0 {
                1.0
            } else {
                f64::from((progress - previous.0) / span)
            };
            return lerp(previous.1, frame.value, frame.easing_function.ease(local));
        }
        previous = (frame.normalized_progress_key, frame.value);
    }
    previous.1
}

fn lerp_vector3(from: Vector3, to: Vector3, t: f64) -> Vector3 {
    let t = t as f32;
    Vector3::new(
        from.x + (to.x - from.x) * t,
        from.y + (to.y - from.y) * t,
        from.z + (to.z - from.z) * t,
    )
}

fn lerp_f32(from: f32, to: f32, t: f64) -> f32 {
    from + (to - from) * t as f32
}

/// The duration of a default animation over `distance` units.
fn animation_duration(distance: f64, ms_per_unit: f64, min_ms: f64, max_ms: f64) -> Duration {
    Duration::from_secs_f64((distance.trunc() * ms_per_unit).clamp(min_ms, max_ms) / 1000.0)
}

/// Which scroll controller requested a scroll (the presenter asks it for the animation and tells
/// it when the scroll completes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Requester {
    Code,
    HorizontalController,
    VerticalController,
}

/// A view change that runs over frames.
enum ViewOperation {
    Scroll {
        id: i32,
        requester: Requester,
        frames: Vec<KeyFrame<Vector3>>,
        from: Vector3,
        duration: f64,
        start: Option<f64>,
    },
    Zoom {
        id: i32,
        frames: Vec<KeyFrame<f32>>,
        from: f32,
        from_offset: Vector,
        center: Vector2,
        duration: f64,
        start: Option<f64>,
    },
    ScrollVelocity {
        id: i32,
        requester: Requester,
        velocity: (f64, f64),
        decay: (f64, f64),
        last: Option<f64>,
    },
    ZoomVelocity {
        id: i32,
        velocity: f64,
        decay: f64,
        center: Vector2,
        last: Option<f64>,
    },
}

impl ViewOperation {
    fn id(&self) -> i32 {
        match self {
            Self::Scroll { id, .. }
            | Self::Zoom { id, .. }
            | Self::ScrollVelocity { id, .. }
            | Self::ZoomVelocity { id, .. } => *id,
        }
    }

    fn is_zoom(&self) -> bool {
        matches!(self, Self::Zoom { .. } | Self::ZoomVelocity { .. })
    }

    fn requester(&self) -> Requester {
        match self {
            Self::Scroll { requester, .. } | Self::ScrollVelocity { requester, .. } => *requester,
            _ => Requester::Code,
        }
    }
}

type PresenterHandler = dyn Fn(&ScrollPresenter);

/// The scroller of a [`ScrollView`](crate::ScrollView) (WinUI's `ScrollPresenter`): it shows its
/// `Content` through a viewport, scrolls and zooms it (with the wheel, Ctrl and the wheel when
/// `ZoomMode` is enabled, touch and the keyboard), changes the view in code (`ScrollTo`,
/// `ScrollBy`, `AddScrollVelocity`, `ZoomTo`, `ZoomBy`, `AddZoomVelocity`, each returning the
/// correlation id of the change) and drives a scroll controller on each axis
/// (`HorizontalScrollController`, `VerticalScrollController`).
///
/// `ContentOrientation` constrains the content to the viewport on the axes that do not scroll;
/// `HorizontalScrollMode` and `VerticalScrollMode` say whether the user scrolls on each axis
/// (`Auto`: when the content is larger than the viewport, or zooming is enabled).
///
/// An animated change raises `ScrollAnimationStarting` (or `ZoomAnimationStarting`) with WinUI's
/// default composition animation (one key frame at the end, 5 ms per pixel between 50 ms and
/// 1 s; 250 ms per unit of zoom), which a handler can change or replace; the key frames are then
/// evaluated on each frame and set as the offsets, which the compositor scrolls as for any other
/// scroll.
///
/// Differences (docs/porting/winui-controls.md, section 10.12): the scrolling is FerroUI's
/// `ScrollViewer` (its presenter and gestures), which the presenter hosts without scroll bars,
/// where WinUI drives an `InteractionTracker`; `ExpressionAnimationSources`, the snap points
/// collections, `BringingIntoView` and `AnchorRequested` are left out; the chain and rail modes
/// are kept (chaining is off only when both axes are `Never`); `IgnoredInputKinds` and a
/// `Disabled` scroll mode stop the mouse wheel; the anchor ratios are kept without effect.
#[repr(C)]
pub struct ScrollPresenter {
    base: Control,
    engine: RefCell<Option<Ref<ScrollViewer>>>,
    state: Cell<ScrollingInteractionState>,
    last_view: Cell<(Vector, f32, Size)>,
    last_extent: Cell<Size>,
    next_correlation_id: Cell<i32>,
    operation: RefCell<Option<ViewOperation>>,
    frame_requested: Cell<bool>,
    /// Set by the tests, which call [`advance`](Self::advance) themselves.
    #[cfg(test)]
    manual_frames: Cell<bool>,
    horizontal_scroll_controller: RefCell<Option<(Rc<dyn IScrollController>, Vec<Subscription>)>>,
    vertical_scroll_controller: RefCell<Option<(Rc<dyn IScrollController>, Vec<Subscription>)>>,
    extent_changed: HandlerList<PresenterHandler>,
    state_changed: HandlerList<PresenterHandler>,
    view_changed: HandlerList<PresenterHandler>,
    scroll_animation_starting:
        HandlerList<dyn Fn(&ScrollPresenter, &ScrollingScrollAnimationStartingEventArgs)>,
    zoom_animation_starting:
        HandlerList<dyn Fn(&ScrollPresenter, &ScrollingZoomAnimationStartingEventArgs)>,
    scroll_completed: HandlerList<dyn Fn(&ScrollPresenter, &ScrollingScrollCompletedEventArgs)>,
    zoom_completed: HandlerList<dyn Fn(&ScrollPresenter, &ScrollingZoomCompletedEventArgs)>,
    scroll_starting: HandlerList<dyn Fn(&ScrollPresenter, &ScrollingScrollStartingEventArgs)>,
    zoom_starting: HandlerList<dyn Fn(&ScrollPresenter, &ScrollingZoomStartingEventArgs)>,
}

/// A markup event of the presenter without arguments.
fn markup_view_event(handler: MarkupDelegate) -> impl Fn(&ScrollPresenter) {
    move |sender: &ScrollPresenter| {
        handler.invoke(&[
            into_markup_value(Some(sender.to_ref())),
            into_markup_value(None::<BoxedValue>),
        ]);
    }
}

ferro_class!(ScrollPresenter: Control);
ferroui_base::ferro_class_info!(ScrollPresenter {
    new: ScrollPresenter::new,
    markup: {
        namespace: "FerroUI.Controls.Primitives",
        content: Content,
        events: [
            ExtentChanged(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<ScrollPresenter>, handler: MarkupDelegate| {
                this.extent_changed(markup_view_event(handler))
            },
            StateChanged(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<ScrollPresenter>, handler: MarkupDelegate| {
                this.state_changed(markup_view_event(handler))
            },
            ViewChanged(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<ScrollPresenter>, handler: MarkupDelegate| {
                this.view_changed(markup_view_event(handler))
            },
            ScrollCompleted(Option<BoxedValue>, ScrollingScrollCompletedEventArgs) =>
                |this: &Ref<ScrollPresenter>, handler: MarkupDelegate| {
                this.scroll_completed(move |sender: &ScrollPresenter, args: &ScrollingScrollCompletedEventArgs| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(*args)]);
                })
            },
            ZoomCompleted(Option<BoxedValue>, ScrollingZoomCompletedEventArgs) =>
                |this: &Ref<ScrollPresenter>, handler: MarkupDelegate| {
                this.zoom_completed(move |sender: &ScrollPresenter, args: &ScrollingZoomCompletedEventArgs| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(*args)]);
                })
            },
        ],
    },
});
ferro_impl_classes!(ScrollPresenter: StyledElementImpl, VisualImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for ScrollPresenter {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let engine = ScrollViewer::new();
        engine.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        engine.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        engine.set_parent(this.to_ref());
        this.visual_children().add(engine.clone().upcast());
        this.logical_children().add(engine.clone().upcast());
        let weak = this.to_ref().downgrade();
        let _scroll_changed = engine.scroll_changed(move |_, args| {
            let Some(this) = weak.upgrade() else { return };
            let from_engine = args.source().is_some_and(|source| {
                this.engine().is_some_and(|engine| {
                    std::ptr::eq(
                        &*source as *const _ as *const (),
                        &*engine as *const ScrollViewer as *const (),
                    )
                })
            });
            if from_engine {
                this.on_engine_view_changed();
            }
        });
        *this.engine.borrow_mut() = Some(engine);
        this.update_engine_modes();
        this.update_engine_zoom();
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::pointer_wheel_changed_event(),
            move |_, args| {
                if let Some(this) = weak.upgrade() {
                    this.on_preview_pointer_wheel_changed(args);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        let Some(engine) = this.engine() else { return };
        if property == Self::content_property().as_property() {
            engine.set_content(this.content().map(Control::boxed));
        } else if property == Self::background_property().as_property() {
            engine.set_background(this.background());
        } else if property == Self::content_orientation_property().as_property()
            || property == Self::horizontal_scroll_chain_mode_property().as_property()
            || property == Self::vertical_scroll_chain_mode_property().as_property()
        {
            this.update_engine_modes();
        } else if property == Self::zoom_mode_property().as_property()
            || property == Self::min_zoom_factor_property().as_property()
            || property == Self::max_zoom_factor_property().as_property()
        {
            this.update_engine_zoom();
            this.update_computed_scroll_modes();
        } else if property == Self::horizontal_scroll_mode_property().as_property()
            || property == Self::vertical_scroll_mode_property().as_property()
        {
            this.update_computed_scroll_modes();
        }
    }
}

impl InputElementImpl for ScrollPresenter {}

impl LayoutableImpl for ScrollPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let engine = this.engine();
        LayoutHelper::measure_child(
            engine.as_deref().map(|e| -> &Layoutable { e }),
            available_size,
            Thickness::default(),
        )
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let engine = this.engine();
        LayoutHelper::arrange_child(
            engine.as_deref().map(|e| -> &Layoutable { e }),
            final_size,
            Thickness::default(),
        )
    }
}

ferroui_base::ferro_properties! { impl ScrollPresenter {
    ferro_property!(
        /// Defines the `Content` property: the element the presenter scrolls.
        pub fn content_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<ScrollPresenter, _>("Content", None)
        }
    );
    ferro_property!(
        /// Defines the `Background` property: the brush behind the content.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<ScrollPresenter, _>("Background", None)
        }
    );
    ferro_property!(
        /// Defines the `ContentOrientation` property (default `Vertical`).
        pub fn content_orientation_property() -> StyledProperty<ScrollingContentOrientation> {
            FerroProperty::register::<ScrollPresenter, _>("ContentOrientation", ScrollingContentOrientation::Vertical)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalScrollChainMode` property (default `Auto`).
        pub fn horizontal_scroll_chain_mode_property() -> StyledProperty<ScrollingChainMode> {
            FerroProperty::register::<ScrollPresenter, _>("HorizontalScrollChainMode", ScrollingChainMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `VerticalScrollChainMode` property (default `Auto`).
        pub fn vertical_scroll_chain_mode_property() -> StyledProperty<ScrollingChainMode> {
            FerroProperty::register::<ScrollPresenter, _>("VerticalScrollChainMode", ScrollingChainMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalScrollRailMode` property (default `Enabled`).
        pub fn horizontal_scroll_rail_mode_property() -> StyledProperty<ScrollingRailMode> {
            FerroProperty::register::<ScrollPresenter, _>("HorizontalScrollRailMode", ScrollingRailMode::Enabled)
        }
    );
    ferro_property!(
        /// Defines the `VerticalScrollRailMode` property (default `Enabled`).
        pub fn vertical_scroll_rail_mode_property() -> StyledProperty<ScrollingRailMode> {
            FerroProperty::register::<ScrollPresenter, _>("VerticalScrollRailMode", ScrollingRailMode::Enabled)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalScrollMode` property (default `Auto`).
        pub fn horizontal_scroll_mode_property() -> StyledProperty<ScrollingScrollMode> {
            FerroProperty::register::<ScrollPresenter, _>("HorizontalScrollMode", ScrollingScrollMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `VerticalScrollMode` property (default `Auto`).
        pub fn vertical_scroll_mode_property() -> StyledProperty<ScrollingScrollMode> {
            FerroProperty::register::<ScrollPresenter, _>("VerticalScrollMode", ScrollingScrollMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the read-only `ComputedHorizontalScrollMode` property: `HorizontalScrollMode`
        /// with `Auto` resolved.
        pub fn computed_horizontal_scroll_mode_property() -> StyledProperty<ScrollingScrollMode> {
            FerroProperty::register::<ScrollPresenter, _>("ComputedHorizontalScrollMode", ScrollingScrollMode::Disabled)
        }
    );
    ferro_property!(
        /// Defines the read-only `ComputedVerticalScrollMode` property.
        pub fn computed_vertical_scroll_mode_property() -> StyledProperty<ScrollingScrollMode> {
            FerroProperty::register::<ScrollPresenter, _>("ComputedVerticalScrollMode", ScrollingScrollMode::Disabled)
        }
    );
    ferro_property!(
        /// Defines the `ZoomChainMode` property (default `Auto`).
        pub fn zoom_chain_mode_property() -> StyledProperty<ScrollingChainMode> {
            FerroProperty::register::<ScrollPresenter, _>("ZoomChainMode", ScrollingChainMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `ZoomMode` property (default `Disabled`).
        pub fn zoom_mode_property() -> StyledProperty<ScrollingZoomMode> {
            FerroProperty::register::<ScrollPresenter, _>("ZoomMode", ScrollingZoomMode::Disabled)
        }
    );
    ferro_property!(
        /// Defines the `IgnoredInputKinds` property (default `None`).
        pub fn ignored_input_kinds_property() -> StyledProperty<ScrollingInputKinds> {
            FerroProperty::register::<ScrollPresenter, _>("IgnoredInputKinds", ScrollingInputKinds::NONE)
        }
    );
    ferro_property!(
        /// Defines the `MinZoomFactor` property (default 0.1).
        pub fn min_zoom_factor_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollPresenter, _>("MinZoomFactor", 0.1)
        }
    );
    ferro_property!(
        /// Defines the `MaxZoomFactor` property (default 10).
        pub fn max_zoom_factor_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollPresenter, _>("MaxZoomFactor", 10.0)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalAnchorRatio` property (default 0).
        pub fn horizontal_anchor_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollPresenter, _>("HorizontalAnchorRatio", 0.0)
        }
    );
    ferro_property!(
        /// Defines the `VerticalAnchorRatio` property (default 0).
        pub fn vertical_anchor_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollPresenter, _>("VerticalAnchorRatio", 0.0)
        }
    );
} }

macro_rules! accessors {
    ($($(#[$doc:meta])* $get:ident, $set:ident, $property:ident: $ty:ty;)*) => {$(
        $(#[$doc])*
        pub fn $get(&self) -> $ty {
            self.get_value(Self::$property())
        }

        $(#[$doc])*
        pub fn $set(&self, value: $ty) {
            self.set_value(Self::$property(), value);
        }
    )*};
}

impl ScrollPresenter {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            engine: RefCell::new(None),
            state: Cell::new(ScrollingInteractionState::Idle),
            last_view: Cell::new((Vector::default(), 1.0, Size::default())),
            last_extent: Cell::new(Size::default()),
            next_correlation_id: Cell::new(0),
            operation: RefCell::new(None),
            frame_requested: Cell::new(false),
            #[cfg(test)]
            manual_frames: Cell::new(false),
            horizontal_scroll_controller: RefCell::new(None),
            vertical_scroll_controller: RefCell::new(None),
            extent_changed: HandlerList::new(),
            state_changed: HandlerList::new(),
            view_changed: HandlerList::new(),
            scroll_animation_starting: HandlerList::new(),
            zoom_animation_starting: HandlerList::new(),
            scroll_completed: HandlerList::new(),
            zoom_completed: HandlerList::new(),
            scroll_starting: HandlerList::new(),
            zoom_starting: HandlerList::new(),
        }
    }

    /// Creates a scroll presenter without content.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    accessors! {
        /// The element the presenter scrolls (`Content`).
        content, set_content, content_property: Option<Ref<Control>>;
        /// The brush behind the content (`Background`).
        background, set_background, background_property: Option<Rc<dyn IBrush>>;
        /// The axes on which the content is not constrained to the viewport (`ContentOrientation`).
        content_orientation, set_content_orientation, content_orientation_property: ScrollingContentOrientation;
        /// `HorizontalScrollChainMode`.
        horizontal_scroll_chain_mode, set_horizontal_scroll_chain_mode,
            horizontal_scroll_chain_mode_property: ScrollingChainMode;
        /// `VerticalScrollChainMode`.
        vertical_scroll_chain_mode, set_vertical_scroll_chain_mode,
            vertical_scroll_chain_mode_property: ScrollingChainMode;
        /// `HorizontalScrollRailMode`.
        horizontal_scroll_rail_mode, set_horizontal_scroll_rail_mode,
            horizontal_scroll_rail_mode_property: ScrollingRailMode;
        /// `VerticalScrollRailMode`.
        vertical_scroll_rail_mode, set_vertical_scroll_rail_mode, vertical_scroll_rail_mode_property: ScrollingRailMode;
        /// Whether the user scrolls horizontally (`HorizontalScrollMode`).
        horizontal_scroll_mode, set_horizontal_scroll_mode, horizontal_scroll_mode_property: ScrollingScrollMode;
        /// Whether the user scrolls vertically (`VerticalScrollMode`).
        vertical_scroll_mode, set_vertical_scroll_mode, vertical_scroll_mode_property: ScrollingScrollMode;
        /// `ZoomChainMode`.
        zoom_chain_mode, set_zoom_chain_mode, zoom_chain_mode_property: ScrollingChainMode;
        /// Whether the user zooms (`ZoomMode`).
        zoom_mode, set_zoom_mode, zoom_mode_property: ScrollingZoomMode;
        /// The kinds of input the presenter ignores (`IgnoredInputKinds`).
        ignored_input_kinds, set_ignored_input_kinds, ignored_input_kinds_property: ScrollingInputKinds;
        /// The smallest zoom factor (`MinZoomFactor`).
        min_zoom_factor, set_min_zoom_factor, min_zoom_factor_property: f64;
        /// The largest zoom factor (`MaxZoomFactor`).
        max_zoom_factor, set_max_zoom_factor, max_zoom_factor_property: f64;
        /// `HorizontalAnchorRatio`.
        horizontal_anchor_ratio, set_horizontal_anchor_ratio, horizontal_anchor_ratio_property: f64;
        /// `VerticalAnchorRatio`.
        vertical_anchor_ratio, set_vertical_anchor_ratio, vertical_anchor_ratio_property: f64;
    }

    /// `HorizontalScrollMode` with `Auto` resolved (`ComputedHorizontalScrollMode`).
    pub fn computed_horizontal_scroll_mode(&self) -> ScrollingScrollMode {
        self.get_value(Self::computed_horizontal_scroll_mode_property())
    }

    /// `VerticalScrollMode` with `Auto` resolved (`ComputedVerticalScrollMode`).
    pub fn computed_vertical_scroll_mode(&self) -> ScrollingScrollMode {
        self.get_value(Self::computed_vertical_scroll_mode_property())
    }

    /// The scroll viewer that scrolls the content.
    pub(crate) fn engine(&self) -> Option<Ref<ScrollViewer>> {
        self.engine.borrow().clone()
    }

    /// The horizontal offset of the view, in zoomed units (`HorizontalOffset`).
    pub fn horizontal_offset(&self) -> f64 {
        self.engine().map_or(0.0, |engine| engine.offset().x)
    }

    /// The vertical offset of the view, in zoomed units (`VerticalOffset`).
    pub fn vertical_offset(&self) -> f64 {
        self.engine().map_or(0.0, |engine| engine.offset().y)
    }

    /// The zoom factor of the content (`ZoomFactor`).
    pub fn zoom_factor(&self) -> f32 {
        self.engine().map_or(1.0, |engine| engine.zoom_factor())
    }

    /// The width of the content, zoomed (`ExtentWidth`).
    pub fn extent_width(&self) -> f64 {
        self.engine().map_or(0.0, |engine| engine.extent().width)
    }

    /// The height of the content, zoomed (`ExtentHeight`).
    pub fn extent_height(&self) -> f64 {
        self.engine().map_or(0.0, |engine| engine.extent().height)
    }

    /// The width of the viewport (`ViewportWidth`).
    pub fn viewport_width(&self) -> f64 {
        self.engine().map_or(0.0, |engine| engine.viewport().width)
    }

    /// The height of the viewport (`ViewportHeight`).
    pub fn viewport_height(&self) -> f64 {
        self.engine().map_or(0.0, |engine| engine.viewport().height)
    }

    /// The largest horizontal offset (`ScrollableWidth`).
    pub fn scrollable_width(&self) -> f64 {
        (self.extent_width() - self.viewport_width()).max(0.0)
    }

    /// The largest vertical offset (`ScrollableHeight`).
    pub fn scrollable_height(&self) -> f64 {
        (self.extent_height() - self.viewport_height()).max(0.0)
    }

    /// What moves the view (`State`).
    pub fn state(&self) -> ScrollingInteractionState {
        self.state.get()
    }

    /// The element the view is anchored to (`CurrentAnchor`).
    pub fn current_anchor(&self) -> Option<Ref<Control>> {
        self.engine().and_then(|engine| engine.current_anchor())
    }

    /// `RegisterAnchorCandidate`.
    pub fn register_anchor_candidate(&self, element: &Ref<Control>) {
        if let Some(engine) = self.engine() {
            engine.register_anchor_candidate(element);
        }
    }

    /// `UnregisterAnchorCandidate`.
    pub fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        if let Some(engine) = self.engine() {
            engine.unregister_anchor_candidate(element);
        }
    }

    /// The scroll controller of the horizontal axis (`HorizontalScrollController`).
    pub fn horizontal_scroll_controller(&self) -> Option<Rc<dyn IScrollController>> {
        self.horizontal_scroll_controller
            .borrow()
            .as_ref()
            .map(|(controller, _)| controller.clone())
    }

    /// Sets the scroll controller of the horizontal axis.
    pub fn set_horizontal_scroll_controller(&self, value: Option<Rc<dyn IScrollController>>) {
        self.set_scroll_controller(value, Requester::HorizontalController);
    }

    /// The scroll controller of the vertical axis (`VerticalScrollController`).
    pub fn vertical_scroll_controller(&self) -> Option<Rc<dyn IScrollController>> {
        self.vertical_scroll_controller
            .borrow()
            .as_ref()
            .map(|(controller, _)| controller.clone())
    }

    /// Sets the scroll controller of the vertical axis.
    pub fn set_vertical_scroll_controller(&self, value: Option<Rc<dyn IScrollController>>) {
        self.set_scroll_controller(value, Requester::VerticalController);
    }

    fn set_scroll_controller(&self, value: Option<Rc<dyn IScrollController>>, axis: Requester) {
        let slot = if axis == Requester::HorizontalController {
            &self.horizontal_scroll_controller
        } else {
            &self.vertical_scroll_controller
        };
        // Dropping the subscriptions disconnects the old controller.
        slot.borrow_mut().take();
        let Some(controller) = value else { return };
        let horizontal = axis == Requester::HorizontalController;
        let mut subscriptions = Vec::new();
        let weak = self.to_ref().downgrade();
        subscriptions.push(controller.scroll_to_requested(Rc::new(move |args| {
            let Some(this) = weak.upgrade() else { return };
            let (h, v) = if horizontal {
                (args.offset(), this.vertical_offset())
            } else {
                (this.horizontal_offset(), args.offset())
            };
            args.set_correlation_id(this.scroll_to_core(h, v, args.options(), axis));
        })));
        let weak = self.to_ref().downgrade();
        subscriptions.push(controller.scroll_by_requested(Rc::new(move |args| {
            let Some(this) = weak.upgrade() else { return };
            let (dh, dv) = if horizontal {
                (args.offset_delta(), 0.0)
            } else {
                (0.0, args.offset_delta())
            };
            let (h, v) = this.target_offsets();
            args.set_correlation_id(this.scroll_to_core(h + dh, v + dv, args.options(), axis));
        })));
        let weak = self.to_ref().downgrade();
        subscriptions.push(
            controller.add_scroll_velocity_requested(Rc::new(move |args| {
                let Some(this) = weak.upgrade() else { return };
                let velocity = f64::from(args.offset_velocity());
                let decay = args
                    .inertia_decay_rate()
                    .unwrap_or(DEFAULT_INERTIA_DECAY_RATE);
                let (velocity, decay) = if horizontal {
                    (
                        Vector2::new(velocity as f32, 0.0),
                        Vector2::new(decay, DEFAULT_INERTIA_DECAY_RATE),
                    )
                } else {
                    (
                        Vector2::new(0.0, velocity as f32),
                        Vector2::new(DEFAULT_INERTIA_DECAY_RATE, decay),
                    )
                };
                args.set_correlation_id(this.add_scroll_velocity_core(velocity, Some(decay), axis));
            })),
        );
        *slot.borrow_mut() = Some((controller, subscriptions));
        self.update_scroll_controllers();
    }

    // --- events ---

    fn subscribe(
        &self,
        add: impl FnOnce(&Self) -> u64,
        remove: impl Fn(&Self, u64) + 'static,
    ) -> Subscription {
        let token = add(self);
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                remove(&this, token);
            }
        }))
    }

    /// Adds a handler of `ExtentChanged`, raised when the extent of the content changes.
    pub fn extent_changed(&self, handler: impl Fn(&ScrollPresenter) + 'static) -> Subscription {
        self.subscribe(
            |this| this.extent_changed.add(Rc::new(handler)),
            |this, token| {
                this.extent_changed.remove(token);
            },
        )
    }

    /// Adds a handler of `StateChanged`, raised when [`state`](Self::state) changes.
    pub fn state_changed(&self, handler: impl Fn(&ScrollPresenter) + 'static) -> Subscription {
        self.subscribe(
            |this| this.state_changed.add(Rc::new(handler)),
            |this, token| {
                this.state_changed.remove(token);
            },
        )
    }

    /// Adds a handler of `ViewChanged`, raised when the offsets, the zoom factor or the viewport
    /// change.
    pub fn view_changed(&self, handler: impl Fn(&ScrollPresenter) + 'static) -> Subscription {
        self.subscribe(
            |this| this.view_changed.add(Rc::new(handler)),
            |this, token| {
                this.view_changed.remove(token);
            },
        )
    }

    /// Adds a handler of `ScrollAnimationStarting`, raised before an animated scroll of code
    /// starts, with the animation it runs.
    pub fn scroll_animation_starting(
        &self,
        handler: impl Fn(&ScrollPresenter, &ScrollingScrollAnimationStartingEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.scroll_animation_starting.add(Rc::new(handler)),
            |this, token| {
                this.scroll_animation_starting.remove(token);
            },
        )
    }

    /// Adds a handler of `ZoomAnimationStarting`, raised before an animated zoom starts.
    pub fn zoom_animation_starting(
        &self,
        handler: impl Fn(&ScrollPresenter, &ScrollingZoomAnimationStartingEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.zoom_animation_starting.add(Rc::new(handler)),
            |this, token| {
                this.zoom_animation_starting.remove(token);
            },
        )
    }

    /// Adds a handler of `ScrollCompleted`, raised when a `ScrollTo`, `ScrollBy` or
    /// `AddScrollVelocity` ends (also when another change interrupts it).
    pub fn scroll_completed(
        &self,
        handler: impl Fn(&ScrollPresenter, &ScrollingScrollCompletedEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.scroll_completed.add(Rc::new(handler)),
            |this, token| {
                this.scroll_completed.remove(token);
            },
        )
    }

    /// Adds a handler of `ZoomCompleted`.
    pub fn zoom_completed(
        &self,
        handler: impl Fn(&ScrollPresenter, &ScrollingZoomCompletedEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.zoom_completed.add(Rc::new(handler)),
            |this, token| {
                this.zoom_completed.remove(token);
            },
        )
    }

    /// Adds a handler of `ScrollStarting`, raised when a scroll of code starts.
    pub fn scroll_starting(
        &self,
        handler: impl Fn(&ScrollPresenter, &ScrollingScrollStartingEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.scroll_starting.add(Rc::new(handler)),
            |this, token| {
                this.scroll_starting.remove(token);
            },
        )
    }

    /// Adds a handler of `ZoomStarting`, raised when a zoom of code starts.
    pub fn zoom_starting(
        &self,
        handler: impl Fn(&ScrollPresenter, &ScrollingZoomStartingEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.zoom_starting.add(Rc::new(handler)),
            |this, token| {
                this.zoom_starting.remove(token);
            },
        )
    }

    fn raise(&self, list: &HandlerList<PresenterHandler>) {
        for (_, handler) in list.snapshot().iter() {
            handler(self);
        }
    }

    fn set_state(&self, state: ScrollingInteractionState) {
        if self.state.replace(state) != state {
            self.raise(&self.state_changed);
        }
    }

    // --- view changes ---

    fn next_id(&self) -> i32 {
        let id = self.next_correlation_id.get() + 1;
        self.next_correlation_id.set(id);
        id
    }

    /// The offsets a running scroll ends at (the current ones otherwise): `ScrollBy` adds to them.
    fn target_offsets(&self) -> (f64, f64) {
        if let Some(ViewOperation::Scroll { frames, .. }) = self.operation.borrow().as_ref() {
            if let Some(last) = frames.last() {
                return (f64::from(last.value.x), f64::from(last.value.y));
            }
        }
        (self.horizontal_offset(), self.vertical_offset())
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        ElementComposition::get_element_visual(self).map(|visual| visual.compositor().clone())
    }

    /// `ScrollTo(horizontalOffset, verticalOffset)`: an animated scroll to the offsets; the
    /// correlation id of the scroll.
    pub fn scroll_to(&self, horizontal_offset: f64, vertical_offset: f64) -> i32 {
        self.scroll_to_with(
            horizontal_offset,
            vertical_offset,
            ScrollingScrollOptions::default(),
        )
    }

    /// `ScrollTo(horizontalOffset, verticalOffset, options)`.
    pub fn scroll_to_with(
        &self,
        horizontal_offset: f64,
        vertical_offset: f64,
        options: ScrollingScrollOptions,
    ) -> i32 {
        self.scroll_to_core(horizontal_offset, vertical_offset, options, Requester::Code)
    }

    /// `ScrollBy(horizontalOffsetDelta, verticalOffsetDelta)`.
    pub fn scroll_by(&self, horizontal_offset_delta: f64, vertical_offset_delta: f64) -> i32 {
        self.scroll_by_with(
            horizontal_offset_delta,
            vertical_offset_delta,
            ScrollingScrollOptions::default(),
        )
    }

    /// `ScrollBy(horizontalOffsetDelta, verticalOffsetDelta, options)`: from the offsets a
    /// running scroll ends at.
    pub fn scroll_by_with(
        &self,
        horizontal_offset_delta: f64,
        vertical_offset_delta: f64,
        options: ScrollingScrollOptions,
    ) -> i32 {
        let (h, v) = self.target_offsets();
        self.scroll_to_with(
            h + horizontal_offset_delta,
            v + vertical_offset_delta,
            options,
        )
    }

    fn scroll_to_core(
        &self,
        horizontal_offset: f64,
        vertical_offset: f64,
        options: ScrollingScrollOptions,
        requester: Requester,
    ) -> i32 {
        let id = self.next_id();
        self.complete_operation();
        for (_, handler) in self.scroll_starting.snapshot().iter() {
            handler(
                self,
                &ScrollingScrollStartingEventArgs::new(
                    id,
                    self.horizontal_offset(),
                    self.vertical_offset(),
                    self.zoom_factor(),
                ),
            );
        }
        let target = (
            if horizontal_offset.is_finite() {
                horizontal_offset.clamp(0.0, self.scrollable_width())
            } else {
                0.0
            },
            if vertical_offset.is_finite() {
                vertical_offset.clamp(0.0, self.scrollable_height())
            } else {
                0.0
            },
        );
        let from = Vector3::new(
            self.horizontal_offset() as f32,
            self.vertical_offset() as f32,
            0.0,
        );
        let end = Vector3::new(target.0 as f32, target.1 as f32, 0.0);
        let animation = match (options.animation_mode, self.compositor()) {
            (ScrollingAnimationMode::Disabled, _) | (_, None) => None,
            (_, Some(compositor)) => {
                let distance = ((target.0 - f64::from(from.x)).powi(2)
                    + (target.1 - f64::from(from.y)).powi(2))
                .sqrt();
                let animation = compositor.create_vector3_key_frame_animation();
                animation.insert_key_frame(1.0, end);
                animation.set_duration(animation_duration(
                    distance,
                    OFFSETS_CHANGE_MS_PER_UNIT,
                    OFFSETS_CHANGE_MIN_MS,
                    OFFSETS_CHANGE_MAX_MS,
                ));
                let start_position = Vector2::new(from.x, from.y);
                let end_position = Vector2::new(end.x, end.y);
                match requester {
                    Requester::Code => {
                        let args = ScrollingScrollAnimationStartingEventArgs::new(
                            animation,
                            start_position,
                            end_position,
                            id,
                        );
                        for (_, handler) in self.scroll_animation_starting.snapshot().iter() {
                            handler(self, &args);
                        }
                        args.animation()
                    }
                    controller => {
                        let custom = self.controller(controller).and_then(|controller| {
                            controller.get_scroll_animation(
                                id,
                                start_position,
                                end_position,
                                &animation,
                            )
                        });
                        Some(custom.unwrap_or(animation))
                    }
                }
            }
        };
        match animation {
            Some(animation) => {
                *self.operation.borrow_mut() = Some(ViewOperation::Scroll {
                    id,
                    requester,
                    frames: animation.key_frames(),
                    from,
                    duration: animation.duration().as_secs_f64(),
                    start: None,
                });
                self.set_state(ScrollingInteractionState::Animation);
                self.request_frame();
            }
            None => {
                if let Some(engine) = self.engine() {
                    engine.set_offset(Vector::new(target.0, target.1));
                }
                self.post_completed(id, false, requester);
            }
        }
        id
    }

    /// `AddScrollVelocity(offsetsVelocity, inertiaDecayRate)`: adds a velocity, in pixels per
    /// second, to the offsets; `inertiaDecayRate` is the share of the velocity lost per second
    /// (0.95 when `None`; 0 keeps the velocity until the view reaches the end of the content).
    pub fn add_scroll_velocity(
        &self,
        offsets_velocity: Vector2,
        inertia_decay_rate: Option<Vector2>,
    ) -> i32 {
        self.add_scroll_velocity_core(offsets_velocity, inertia_decay_rate, Requester::Code)
    }

    fn add_scroll_velocity_core(
        &self,
        velocity: Vector2,
        decay: Option<Vector2>,
        requester: Requester,
    ) -> i32 {
        let decay = decay.unwrap_or(Vector2::new(
            DEFAULT_INERTIA_DECAY_RATE,
            DEFAULT_INERTIA_DECAY_RATE,
        ));
        let decay = (
            f64::from(decay.x).clamp(0.0, 1.0),
            f64::from(decay.y).clamp(0.0, 1.0),
        );
        let added = (f64::from(velocity.x), f64::from(velocity.y));
        // A velocity added to a running one keeps its correlation id.
        let running = match self.operation.borrow_mut().as_mut() {
            Some(ViewOperation::ScrollVelocity {
                id,
                velocity,
                decay: running_decay,
                ..
            }) => {
                velocity.0 += added.0;
                velocity.1 += added.1;
                *running_decay = decay;
                Some(*id)
            }
            _ => None,
        };
        if let Some(id) = running {
            return id;
        }
        let id = self.next_id();
        self.complete_operation();
        *self.operation.borrow_mut() = Some(ViewOperation::ScrollVelocity {
            id,
            requester,
            velocity: added,
            decay,
            last: None,
        });
        self.set_state(ScrollingInteractionState::Inertia);
        self.request_frame();
        id
    }

    /// `ZoomTo(zoomFactor, centerPoint)`: an animated zoom about `centerPoint`, a point of the
    /// viewport (its center when `None`).
    pub fn zoom_to(&self, zoom_factor: f32, center_point: Option<Vector2>) -> i32 {
        self.zoom_to_with(zoom_factor, center_point, ScrollingZoomOptions::default())
    }

    /// `ZoomTo(zoomFactor, centerPoint, options)`.
    pub fn zoom_to_with(
        &self,
        zoom_factor: f32,
        center_point: Option<Vector2>,
        options: ScrollingZoomOptions,
    ) -> i32 {
        let id = self.next_id();
        self.complete_operation();
        for (_, handler) in self.zoom_starting.snapshot().iter() {
            handler(
                self,
                &ScrollingZoomStartingEventArgs::new(
                    id,
                    self.horizontal_offset(),
                    self.vertical_offset(),
                    self.zoom_factor(),
                ),
            );
        }
        let center = self.center_point(center_point);
        let target = self.clamp_zoom(zoom_factor);
        let from = self.zoom_factor();
        let from_offset = Vector::new(self.horizontal_offset(), self.vertical_offset());
        let animation = match (options.animation_mode, self.compositor()) {
            (ScrollingAnimationMode::Disabled, _) | (_, None) => None,
            (_, Some(compositor)) => {
                let animation = compositor.create_scalar_key_frame_animation();
                animation.insert_key_frame(1.0, target);
                animation.set_duration(animation_duration(
                    f64::from((target - from).abs()),
                    ZOOM_FACTOR_CHANGE_MS_PER_UNIT,
                    ZOOM_FACTOR_CHANGE_MIN_MS,
                    ZOOM_FACTOR_CHANGE_MAX_MS,
                ));
                let args = ScrollingZoomAnimationStartingEventArgs::new(
                    animation, center, from, target, id,
                );
                for (_, handler) in self.zoom_animation_starting.snapshot().iter() {
                    handler(self, &args);
                }
                args.animation()
            }
        };
        match animation {
            Some(animation) => {
                *self.operation.borrow_mut() = Some(ViewOperation::Zoom {
                    id,
                    frames: animation.key_frames(),
                    from,
                    from_offset,
                    center,
                    duration: animation.duration().as_secs_f64(),
                    start: None,
                });
                self.set_state(ScrollingInteractionState::Animation);
                self.request_frame();
            }
            None => {
                self.apply_zoom(target, from, from_offset, center);
                self.post_completed(id, true, Requester::Code);
            }
        }
        id
    }

    /// `ZoomBy(zoomFactorDelta, centerPoint)`.
    pub fn zoom_by(&self, zoom_factor_delta: f32, center_point: Option<Vector2>) -> i32 {
        self.zoom_by_with(
            zoom_factor_delta,
            center_point,
            ScrollingZoomOptions::default(),
        )
    }

    /// `ZoomBy(zoomFactorDelta, centerPoint, options)`.
    pub fn zoom_by_with(
        &self,
        zoom_factor_delta: f32,
        center_point: Option<Vector2>,
        options: ScrollingZoomOptions,
    ) -> i32 {
        let target = match self.operation.borrow().as_ref() {
            Some(ViewOperation::Zoom { frames, .. }) => frames.last().map(|frame| frame.value),
            _ => None,
        }
        .unwrap_or_else(|| self.zoom_factor());
        self.zoom_to_with(target + zoom_factor_delta, center_point, options)
    }

    /// `AddZoomVelocity(zoomFactorVelocity, centerPoint, inertiaDecayRate)`: adds a velocity, in
    /// zoom units per second, to the zoom factor.
    pub fn add_zoom_velocity(
        &self,
        zoom_factor_velocity: f32,
        center_point: Option<Vector2>,
        inertia_decay_rate: Option<f32>,
    ) -> i32 {
        let id = self.next_id();
        self.complete_operation();
        *self.operation.borrow_mut() = Some(ViewOperation::ZoomVelocity {
            id,
            velocity: f64::from(zoom_factor_velocity),
            decay: f64::from(inertia_decay_rate.unwrap_or(DEFAULT_INERTIA_DECAY_RATE))
                .clamp(0.0, 1.0),
            center: self.center_point(center_point),
            last: None,
        });
        self.set_state(ScrollingInteractionState::Inertia);
        self.request_frame();
        id
    }

    fn center_point(&self, center_point: Option<Vector2>) -> Vector2 {
        center_point.unwrap_or_else(|| {
            Vector2::new(
                (self.viewport_width() / 2.0) as f32,
                (self.viewport_height() / 2.0) as f32,
            )
        })
    }

    fn clamp_zoom(&self, value: f32) -> f32 {
        let min = self.min_zoom_factor().max(f64::from(f32::MIN_POSITIVE)) as f32;
        let max = (self.max_zoom_factor() as f32).max(min);
        if value.is_finite() {
            value.clamp(min, max)
        } else {
            self.zoom_factor()
        }
    }

    /// Zooms to `zoom` from the view (`from`, `from_offset`) the change started at, keeping
    /// `center` in place.
    fn apply_zoom(&self, zoom: f32, from: f32, from_offset: Vector, center: Vector2) {
        let Some(engine) = self.engine() else { return };
        let ratio = f64::from(zoom) / f64::from(from.max(f32::MIN_POSITIVE));
        let (cx, cy) = (f64::from(center.x), f64::from(center.y));
        engine.change_view(
            Some((from_offset.x + cx) * ratio - cx),
            Some((from_offset.y + cy) * ratio - cy),
            Some(zoom),
        );
    }

    fn controller(&self, requester: Requester) -> Option<Rc<dyn IScrollController>> {
        match requester {
            Requester::Code => None,
            Requester::HorizontalController => self.horizontal_scroll_controller(),
            Requester::VerticalController => self.vertical_scroll_controller(),
        }
    }

    /// Raises the completion of a change applied at once, after the request returned (WinUI
    /// completes every change asynchronously).
    fn post_completed(&self, id: i32, zoom: bool, requester: Requester) {
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.raise_completed(id, zoom, requester);
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    fn raise_completed(&self, id: i32, zoom: bool, requester: Requester) {
        if zoom {
            let args = ScrollingZoomCompletedEventArgs::new(id);
            for (_, handler) in self.zoom_completed.snapshot().iter() {
                handler(self, &args);
            }
        } else {
            let args = ScrollingScrollCompletedEventArgs::new(id);
            for (_, handler) in self.scroll_completed.snapshot().iter() {
                handler(self, &args);
            }
            if let Some(controller) = self.controller(requester) {
                controller.notify_requested_scroll_completed(id);
            }
        }
    }

    /// Ends the running change, where it is, and raises its completion.
    fn complete_operation(&self) {
        let Some(operation) = self.operation.borrow_mut().take() else {
            return;
        };
        self.set_state(ScrollingInteractionState::Idle);
        self.raise_completed(operation.id(), operation.is_zoom(), operation.requester());
    }

    fn request_frame(&self) {
        #[cfg(test)]
        if self.manual_frames.get() {
            return;
        }
        if self.frame_requested.replace(true) {
            return;
        }
        let weak = self.to_ref().downgrade();
        match TopLevel::get_top_level(Some(self)) {
            Some(top_level) => top_level.request_animation_frame(move |time| {
                if let Some(this) = weak.upgrade() {
                    this.frame_requested.set(false);
                    this.advance(time.total_seconds());
                }
            }),
            // Not in a window: the change ends at once.
            None => {
                self.frame_requested.set(false);
                self.finish_now();
            }
        }
    }

    /// Applies the end of the running change and completes it.
    fn finish_now(&self) {
        let operation = self.operation.borrow_mut().take();
        if let Some(operation) = operation {
            match &operation {
                ViewOperation::Scroll { frames, from, .. } => {
                    let end = sample_key_frames(frames, *from, 1.0, lerp_vector3);
                    self.set_offsets(f64::from(end.x), f64::from(end.y));
                }
                ViewOperation::Zoom {
                    frames,
                    from,
                    from_offset,
                    center,
                    ..
                } => {
                    let end = sample_key_frames(frames, *from, 1.0, lerp_f32);
                    self.apply_zoom(self.clamp_zoom(end), *from, *from_offset, *center);
                }
                _ => {}
            }
            *self.operation.borrow_mut() = Some(operation);
            self.complete_operation();
        }
    }

    fn set_offsets(&self, horizontal_offset: f64, vertical_offset: f64) {
        if let Some(engine) = self.engine() {
            engine.set_offset(Vector::new(horizontal_offset, vertical_offset));
        }
    }

    /// Moves the running change to `time` (seconds, the clock of the animation frames); the
    /// first frame starts it.
    pub(crate) fn advance(&self, time: f64) {
        enum Step {
            Offsets(f64, f64, bool),
            Zoom(f32, f32, Vector, Vector2, bool),
            Velocity(f64, f64),
            ZoomVelocity(f64, Vector2),
            None,
        }
        let step = match self.operation.borrow_mut().as_mut() {
            Some(ViewOperation::Scroll {
                frames,
                from,
                duration,
                start,
                ..
            }) => {
                let begun = *start.get_or_insert(time);
                let progress = if *duration <= 0.0 {
                    1.0
                } else {
                    ((time - begun) / *duration) as f32
                };
                let value = sample_key_frames(frames, *from, progress, lerp_vector3);
                Step::Offsets(f64::from(value.x), f64::from(value.y), progress >= 1.0)
            }
            Some(ViewOperation::Zoom {
                frames,
                from,
                from_offset,
                center,
                duration,
                start,
                ..
            }) => {
                let begun = *start.get_or_insert(time);
                let progress = if *duration <= 0.0 {
                    1.0
                } else {
                    ((time - begun) / *duration) as f32
                };
                let value = sample_key_frames(frames, *from, progress, lerp_f32);
                Step::Zoom(value, *from, *from_offset, *center, progress >= 1.0)
            }
            Some(ViewOperation::ScrollVelocity {
                velocity,
                decay,
                last,
                ..
            }) => {
                let dt = last
                    .replace(time)
                    .map_or(0.0, |last| (time - last).max(0.0));
                let moved = (velocity.0 * dt, velocity.1 * dt);
                velocity.0 *= (1.0 - decay.0).powf(dt);
                velocity.1 *= (1.0 - decay.1).powf(dt);
                Step::Velocity(moved.0, moved.1)
            }
            Some(ViewOperation::ZoomVelocity {
                velocity,
                decay,
                center,
                last,
                ..
            }) => {
                let dt = last
                    .replace(time)
                    .map_or(0.0, |last| (time - last).max(0.0));
                let moved = *velocity * dt;
                *velocity *= (1.0 - *decay).powf(dt);
                Step::ZoomVelocity(moved, *center)
            }
            None => Step::None,
        };
        let done = match step {
            Step::Offsets(h, v, done) => {
                self.set_offsets(h, v);
                done
            }
            Step::Zoom(zoom, from, from_offset, center, done) => {
                self.apply_zoom(self.clamp_zoom(zoom), from, from_offset, center);
                done
            }
            Step::Velocity(dh, dv) => {
                let (h, v) = (self.horizontal_offset(), self.vertical_offset());
                let target = (
                    (h + dh).clamp(0.0, self.scrollable_width()),
                    (v + dv).clamp(0.0, self.scrollable_height()),
                );
                self.set_offsets(target.0, target.1);
                // An axis that reached the end of the content stops.
                let mut stopped = false;
                if let Some(ViewOperation::ScrollVelocity { velocity, .. }) =
                    self.operation.borrow_mut().as_mut()
                {
                    if (target.0 - (h + dh)).abs() > f64::EPSILON {
                        velocity.0 = 0.0;
                    }
                    if (target.1 - (v + dv)).abs() > f64::EPSILON {
                        velocity.1 = 0.0;
                    }
                    stopped = velocity.0.abs() < MIN_INERTIA_VELOCITY
                        && velocity.1.abs() < MIN_INERTIA_VELOCITY;
                }
                stopped
            }
            Step::ZoomVelocity(dz, center) => {
                let from = self.zoom_factor();
                let target = self.clamp_zoom(from + dz as f32);
                self.apply_zoom(
                    target,
                    from,
                    Vector::new(self.horizontal_offset(), self.vertical_offset()),
                    center,
                );
                let mut stopped = true;
                if let Some(ViewOperation::ZoomVelocity { velocity, .. }) =
                    self.operation.borrow_mut().as_mut()
                {
                    if (f64::from(target) - (f64::from(from) + dz)).abs() > 1e-6 {
                        *velocity = 0.0;
                    }
                    stopped = velocity.abs() < MIN_INERTIA_VELOCITY / 100.0;
                }
                stopped
            }
            Step::None => return,
        };
        if done {
            self.complete_operation();
        } else {
            self.request_frame();
        }
    }

    /// `AddScrollVelocity` for the tests, which drive the frames themselves ([`advance`](Self::advance)).
    #[cfg(test)]
    pub(crate) fn add_scroll_velocity_for_test(
        &self,
        velocity: Vector2,
        decay: Option<Vector2>,
    ) -> i32 {
        let decay = decay.unwrap_or(Vector2::new(
            DEFAULT_INERTIA_DECAY_RATE,
            DEFAULT_INERTIA_DECAY_RATE,
        ));
        let id = self.next_id();
        self.complete_operation();
        *self.operation.borrow_mut() = Some(ViewOperation::ScrollVelocity {
            id,
            requester: Requester::Code,
            velocity: (f64::from(velocity.x), f64::from(velocity.y)),
            decay: (f64::from(decay.x), f64::from(decay.y)),
            last: None,
        });
        self.set_state(ScrollingInteractionState::Inertia);
        id
    }

    /// Starts a scroll of code with `frames` over `duration` seconds, for the tests.
    #[cfg(test)]
    pub(crate) fn start_scroll_animation_for_test(
        &self,
        frames: Vec<KeyFrame<Vector3>>,
        duration: f64,
    ) -> i32 {
        let id = self.next_id();
        self.complete_operation();
        *self.operation.borrow_mut() = Some(ViewOperation::Scroll {
            id,
            requester: Requester::Code,
            frames,
            from: Vector3::new(
                self.horizontal_offset() as f32,
                self.vertical_offset() as f32,
                0.0,
            ),
            duration,
            start: None,
        });
        self.set_state(ScrollingInteractionState::Animation);
        id
    }

    /// Lets the tests drive the frames.
    #[cfg(test)]
    pub(crate) fn set_manual_frames_for_test(&self) {
        self.manual_frames.set(true);
    }

    // --- the engine ---

    fn update_engine_modes(&self) {
        let Some(engine) = self.engine() else { return };
        let orientation = self.content_orientation();
        let constrained = |axis_constrained: bool| {
            if axis_constrained {
                ScrollBarVisibility::Disabled
            } else {
                ScrollBarVisibility::Hidden
            }
        };
        engine.set_horizontal_scroll_bar_visibility(constrained(matches!(
            orientation,
            ScrollingContentOrientation::Vertical | ScrollingContentOrientation::Both
        )));
        engine.set_vertical_scroll_bar_visibility(constrained(matches!(
            orientation,
            ScrollingContentOrientation::Horizontal | ScrollingContentOrientation::Both
        )));
        engine.set_is_scroll_chaining_enabled(
            self.horizontal_scroll_chain_mode() != ScrollingChainMode::Never
                || self.vertical_scroll_chain_mode() != ScrollingChainMode::Never,
        );
    }

    fn update_engine_zoom(&self) {
        let Some(engine) = self.engine() else { return };
        engine.set_zoom_mode(if self.zoom_mode() == ScrollingZoomMode::Enabled {
            ZoomMode::Enabled
        } else {
            ZoomMode::Disabled
        });
        engine.set_min_zoom_factor(self.min_zoom_factor() as f32);
        engine.set_max_zoom_factor(self.max_zoom_factor() as f32);
    }

    /// `GetComputedScrollMode`: `Auto` is `Enabled` when zooming is enabled or the content is
    /// larger than the viewport on the axis.
    fn update_computed_scroll_modes(&self) {
        let compute = |mode: ScrollingScrollMode, scrollable: f64| match mode {
            ScrollingScrollMode::Auto
                if self.zoom_mode() == ScrollingZoomMode::Enabled || scrollable > 0.0 =>
            {
                ScrollingScrollMode::Enabled
            }
            ScrollingScrollMode::Auto => ScrollingScrollMode::Disabled,
            mode => mode,
        };
        let horizontal = compute(self.horizontal_scroll_mode(), self.scrollable_width());
        let vertical = compute(self.vertical_scroll_mode(), self.scrollable_height());
        self.set_value(Self::computed_horizontal_scroll_mode_property(), horizontal);
        self.set_value(Self::computed_vertical_scroll_mode_property(), vertical);
        self.update_scroll_controllers();
    }

    /// `UpdateScrollControllerValues` and `UpdateScrollControllerIsScrollable`.
    fn update_scroll_controllers(&self) {
        if let Some(controller) = self.horizontal_scroll_controller() {
            controller.set_is_scrollable(
                self.computed_horizontal_scroll_mode() == ScrollingScrollMode::Enabled,
            );
            controller.set_values(
                0.0,
                self.scrollable_width(),
                self.horizontal_offset(),
                self.viewport_width(),
            );
        }
        if let Some(controller) = self.vertical_scroll_controller() {
            controller.set_is_scrollable(
                self.computed_vertical_scroll_mode() == ScrollingScrollMode::Enabled,
            );
            controller.set_values(
                0.0,
                self.scrollable_height(),
                self.vertical_offset(),
                self.viewport_height(),
            );
        }
    }

    fn on_engine_view_changed(&self) {
        let Some(engine) = self.engine() else { return };
        let extent = engine.extent();
        let view = (engine.offset(), engine.zoom_factor(), engine.viewport());
        let extent_changed = self.last_extent.replace(extent) != extent;
        let view_changed = self.last_view.replace(view) != view;
        if extent_changed || view_changed {
            self.update_computed_scroll_modes();
        }
        if extent_changed {
            self.raise(&self.extent_changed);
        }
        if view_changed {
            self.raise(&self.view_changed);
        }
    }

    /// The mouse wheel is ignored by `IgnoredInputKinds` and on an axis whose scroll mode is
    /// disabled (Ctrl and the wheel zoom, which `ZoomMode` decides).
    fn on_preview_pointer_wheel_changed(&self, args: &PointerWheelEventArgs) {
        if args.key_modifiers().contains(KeyModifiers::CONTROL) {
            return;
        }
        let delta = args.delta();
        let blocked = self
            .ignored_input_kinds()
            .contains(ScrollingInputKinds::MOUSE_WHEEL)
            || (delta.y != 0.0
                && self.computed_vertical_scroll_mode() == ScrollingScrollMode::Disabled
                && self.computed_horizontal_scroll_mode() == ScrollingScrollMode::Disabled)
            || (delta.x != 0.0
                && delta.y == 0.0
                && self.computed_horizontal_scroll_mode() == ScrollingScrollMode::Disabled);
        if blocked {
            args.set_handled(true);
        }
    }
}
