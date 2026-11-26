//! WinUI's `ScrollView` (`controls/dev/ScrollView/ScrollView.cpp`, `.h` and `.idl` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::scroll_bar_controller::ScrollBarController;
use super::{
    ScrollingChainMode, ScrollingContentOrientation, ScrollingInputKinds,
    ScrollingInteractionState, ScrollingRailMode, ScrollingScrollAnimationStartingEventArgs,
    ScrollingScrollBarVisibility, ScrollingScrollCompletedEventArgs, ScrollingScrollMode,
    ScrollingScrollOptions, ScrollingScrollStartingEventArgs,
    ScrollingZoomAnimationStartingEventArgs, ScrollingZoomCompletedEventArgs, ScrollingZoomMode,
    ScrollingZoomOptions, ScrollingZoomStartingEventArgs,
};
use crate::primitives::{
    IScrollController, ScrollBar, ScrollPresenter, TemplateAppliedEventArgs, TemplatedControl,
    TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::numerics::Vector2;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

const SCROLL_PRESENTER_PART_NAME: &str = "PART_ScrollPresenter";
const HORIZONTAL_SCROLL_BAR_PART_NAME: &str = "PART_HorizontalScrollBar";
const VERTICAL_SCROLL_BAR_PART_NAME: &str = "PART_VerticalScrollBar";
const SCROLL_BARS_SEPARATOR_PART_NAME: &str = "PART_ScrollBarsSeparator";

type ViewHandler = dyn Fn(&ScrollView);

/// A scroller with scroll bars (WinUI's `ScrollView`): it shows its `Content` through a
/// [`ScrollPresenter`] (`PART_ScrollPresenter` of the template, `ScrollPresenter` here), which
/// scrolls and zooms it, and a scroll bar on each axis (`PART_HorizontalScrollBar` and
/// `PART_VerticalScrollBar`), whose scroll controllers drive the presenter. The properties,
/// methods and events are the presenter's: `ScrollTo`, `ScrollBy`, `AddScrollVelocity`,
/// `ZoomTo`, `ZoomBy` and `AddZoomVelocity` change the view, `ViewChanged`, `ExtentChanged`,
/// `StateChanged`, `ScrollAnimationStarting`, `ZoomAnimationStarting`, `ScrollCompleted`,
/// `ZoomCompleted`, `ScrollStarting` and `ZoomStarting` report it.
///
/// `HorizontalScrollBarVisibility` and `VerticalScrollBarVisibility` decide each bar:
/// `Auto` shows it when its axis can scroll (`ComputedHorizontalScrollBarVisibility` and
/// `ComputedVerticalScrollBarVisibility`, which the template binds); the separator in the corner
/// shows when both bars do.
///
/// The template is the control theme of the WinUI theme (`Controls/ScrollView.xaml`, from WinUI's
/// `ScrollView.xaml`); the Fluent and Simple themes have none (docs/porting/winui-controls.md,
/// section 5.1). Differences: section 10.12 of that document and [`ScrollPresenter`].
#[repr(C)]
pub struct ScrollView {
    base: TemplatedControl,
    scroll_presenter: RefCell<Option<Ref<ScrollPresenter>>>,
    horizontal_scroll_bar_controller: RefCell<Option<Rc<ScrollBarController>>>,
    vertical_scroll_bar_controller: RefCell<Option<Rc<ScrollBarController>>>,
    scroll_bars_separator: RefCell<Option<Ref<Control>>>,
    presenter_subscriptions: RefCell<Vec<Subscription>>,
    extent_changed: HandlerList<ViewHandler>,
    state_changed: HandlerList<ViewHandler>,
    view_changed: HandlerList<ViewHandler>,
    scroll_animation_starting:
        HandlerList<dyn Fn(&ScrollView, &ScrollingScrollAnimationStartingEventArgs)>,
    zoom_animation_starting:
        HandlerList<dyn Fn(&ScrollView, &ScrollingZoomAnimationStartingEventArgs)>,
    scroll_completed: HandlerList<dyn Fn(&ScrollView, &ScrollingScrollCompletedEventArgs)>,
    zoom_completed: HandlerList<dyn Fn(&ScrollView, &ScrollingZoomCompletedEventArgs)>,
    scroll_starting: HandlerList<dyn Fn(&ScrollView, &ScrollingScrollStartingEventArgs)>,
    zoom_starting: HandlerList<dyn Fn(&ScrollView, &ScrollingZoomStartingEventArgs)>,
}

fn markup_view_event(handler: MarkupDelegate) -> impl Fn(&ScrollView) {
    move |sender: &ScrollView| {
        handler.invoke(&[
            into_markup_value(Some(sender.to_ref())),
            into_markup_value(None::<BoxedValue>),
        ]);
    }
}

ferro_class!(ScrollView: TemplatedControl);
ferroui_base::ferro_class_info!(ScrollView {
    new: ScrollView::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        events: [
            ExtentChanged(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<ScrollView>, handler: MarkupDelegate| {
                this.extent_changed(markup_view_event(handler))
            },
            StateChanged(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<ScrollView>, handler: MarkupDelegate| {
                this.state_changed(markup_view_event(handler))
            },
            ViewChanged(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<ScrollView>, handler: MarkupDelegate| {
                this.view_changed(markup_view_event(handler))
            },
            ScrollCompleted(Option<BoxedValue>, ScrollingScrollCompletedEventArgs) =>
                |this: &Ref<ScrollView>, handler: MarkupDelegate| {
                this.scroll_completed(move |sender: &ScrollView, args: &ScrollingScrollCompletedEventArgs| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(*args)]);
                })
            },
            ZoomCompleted(Option<BoxedValue>, ScrollingZoomCompletedEventArgs) =>
                |this: &Ref<ScrollView>, handler: MarkupDelegate| {
                this.zoom_completed(move |sender: &ScrollView, args: &ScrollingZoomCompletedEventArgs| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(*args)]);
                })
            },
        ],
        attributes: [
            TemplatePart("PART_ScrollPresenter", type(Ref<ScrollPresenter>)),
            TemplatePart("PART_HorizontalScrollBar", type(Ref<ScrollBar>)),
            TemplatePart("PART_VerticalScrollBar", type(Ref<ScrollBar>)),
            TemplatePart("PART_ScrollBarsSeparator", type(Ref<Control>)),
        ],
    },
});
ferro_impl_classes!(ScrollView: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl ControlImpl for ScrollView {}

impl FerroObjectImpl for ScrollView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::horizontal_scroll_bar_visibility_property().as_property()
            || property == Self::vertical_scroll_bar_visibility_property().as_property()
        {
            this.update_scroll_controllers_visibility();
        }
    }
}

impl TemplatedControlImpl for ScrollView {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        if let Some(old) = this.scroll_presenter.borrow_mut().take() {
            old.set_horizontal_scroll_controller(None);
            old.set_vertical_scroll_controller(None);
        }
        this.presenter_subscriptions.borrow_mut().clear();
        let horizontal = scope
            .find_as::<ScrollBar>(HORIZONTAL_SCROLL_BAR_PART_NAME)
            .map(ScrollBarController::new);
        let vertical = scope
            .find_as::<ScrollBar>(VERTICAL_SCROLL_BAR_PART_NAME)
            .map(ScrollBarController::new);
        *this.horizontal_scroll_bar_controller.borrow_mut() = horizontal.clone();
        *this.vertical_scroll_bar_controller.borrow_mut() = vertical.clone();
        *this.scroll_bars_separator.borrow_mut() =
            scope.find_as::<Control>(SCROLL_BARS_SEPARATOR_PART_NAME);
        let presenter = scope.find_as::<ScrollPresenter>(SCROLL_PRESENTER_PART_NAME);
        if let Some(presenter) = &presenter {
            this.hook_scroll_presenter(presenter);
            presenter.set_horizontal_scroll_controller(
                horizontal.map(|c| c as Rc<dyn IScrollController>),
            );
            presenter
                .set_vertical_scroll_controller(vertical.map(|c| c as Rc<dyn IScrollController>));
        }
        *this.scroll_presenter.borrow_mut() = presenter;
        this.update_scroll_controllers_visibility();
    }
}

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

ferroui_base::ferro_properties! { impl ScrollView {
    ferro_property!(
        /// Defines the `Content` property: the element the view scrolls.
        pub fn content_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<ScrollView, _>("Content", None)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalScrollBarVisibility` property (default `Auto`).
        pub fn horizontal_scroll_bar_visibility_property() -> StyledProperty<ScrollingScrollBarVisibility> {
            FerroProperty::register::<ScrollView, _>(
                "HorizontalScrollBarVisibility",
                ScrollingScrollBarVisibility::Auto,
            )
        }
    );
    ferro_property!(
        /// Defines the `VerticalScrollBarVisibility` property (default `Auto`).
        pub fn vertical_scroll_bar_visibility_property() -> StyledProperty<ScrollingScrollBarVisibility> {
            FerroProperty::register::<ScrollView, _>("VerticalScrollBarVisibility", ScrollingScrollBarVisibility::Auto)
        }
    );
    ferro_property!(
        /// Defines the read-only `ComputedHorizontalScrollBarVisibility` property: whether the
        /// horizontal bar shows (WinUI's `Visibility`).
        pub fn computed_horizontal_scroll_bar_visibility_property() -> StyledProperty<bool> {
            FerroProperty::register::<ScrollView, _>("ComputedHorizontalScrollBarVisibility", false)
        }
    );
    ferro_property!(
        /// Defines the read-only `ComputedVerticalScrollBarVisibility` property.
        pub fn computed_vertical_scroll_bar_visibility_property() -> StyledProperty<bool> {
            FerroProperty::register::<ScrollView, _>("ComputedVerticalScrollBarVisibility", false)
        }
    );
    ferro_property!(
        /// Defines the read-only `ComputedHorizontalScrollMode` property (the presenter's).
        pub fn computed_horizontal_scroll_mode_property() -> StyledProperty<ScrollingScrollMode> {
            FerroProperty::register::<ScrollView, _>("ComputedHorizontalScrollMode", ScrollingScrollMode::Disabled)
        }
    );
    ferro_property!(
        /// Defines the read-only `ComputedVerticalScrollMode` property (the presenter's).
        pub fn computed_vertical_scroll_mode_property() -> StyledProperty<ScrollingScrollMode> {
            FerroProperty::register::<ScrollView, _>("ComputedVerticalScrollMode", ScrollingScrollMode::Disabled)
        }
    );
    ferro_property!(
        /// Defines the `ContentOrientation` property (default `Vertical`).
        pub fn content_orientation_property() -> StyledProperty<ScrollingContentOrientation> {
            FerroProperty::register::<ScrollView, _>("ContentOrientation", ScrollingContentOrientation::Vertical)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalScrollChainMode` property (default `Auto`).
        pub fn horizontal_scroll_chain_mode_property() -> StyledProperty<ScrollingChainMode> {
            FerroProperty::register::<ScrollView, _>("HorizontalScrollChainMode", ScrollingChainMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `VerticalScrollChainMode` property (default `Auto`).
        pub fn vertical_scroll_chain_mode_property() -> StyledProperty<ScrollingChainMode> {
            FerroProperty::register::<ScrollView, _>("VerticalScrollChainMode", ScrollingChainMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalScrollRailMode` property (default `Enabled`).
        pub fn horizontal_scroll_rail_mode_property() -> StyledProperty<ScrollingRailMode> {
            FerroProperty::register::<ScrollView, _>("HorizontalScrollRailMode", ScrollingRailMode::Enabled)
        }
    );
    ferro_property!(
        /// Defines the `VerticalScrollRailMode` property (default `Enabled`).
        pub fn vertical_scroll_rail_mode_property() -> StyledProperty<ScrollingRailMode> {
            FerroProperty::register::<ScrollView, _>("VerticalScrollRailMode", ScrollingRailMode::Enabled)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalScrollMode` property (default `Auto`).
        pub fn horizontal_scroll_mode_property() -> StyledProperty<ScrollingScrollMode> {
            FerroProperty::register::<ScrollView, _>("HorizontalScrollMode", ScrollingScrollMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `VerticalScrollMode` property (default `Auto`).
        pub fn vertical_scroll_mode_property() -> StyledProperty<ScrollingScrollMode> {
            FerroProperty::register::<ScrollView, _>("VerticalScrollMode", ScrollingScrollMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `ZoomChainMode` property (default `Auto`).
        pub fn zoom_chain_mode_property() -> StyledProperty<ScrollingChainMode> {
            FerroProperty::register::<ScrollView, _>("ZoomChainMode", ScrollingChainMode::Auto)
        }
    );
    ferro_property!(
        /// Defines the `ZoomMode` property (default `Disabled`).
        pub fn zoom_mode_property() -> StyledProperty<ScrollingZoomMode> {
            FerroProperty::register::<ScrollView, _>("ZoomMode", ScrollingZoomMode::Disabled)
        }
    );
    ferro_property!(
        /// Defines the `IgnoredInputKinds` property (default `None`).
        pub fn ignored_input_kinds_property() -> StyledProperty<ScrollingInputKinds> {
            FerroProperty::register::<ScrollView, _>("IgnoredInputKinds", ScrollingInputKinds::NONE)
        }
    );
    ferro_property!(
        /// Defines the `MinZoomFactor` property (default 0.1).
        pub fn min_zoom_factor_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollView, _>("MinZoomFactor", 0.1)
        }
    );
    ferro_property!(
        /// Defines the `MaxZoomFactor` property (default 10).
        pub fn max_zoom_factor_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollView, _>("MaxZoomFactor", 10.0)
        }
    );
    ferro_property!(
        /// Defines the `HorizontalAnchorRatio` property (default 0).
        pub fn horizontal_anchor_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollView, _>("HorizontalAnchorRatio", 0.0)
        }
    );
    ferro_property!(
        /// Defines the `VerticalAnchorRatio` property (default 0).
        pub fn vertical_anchor_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ScrollView, _>("VerticalAnchorRatio", 0.0)
        }
    );
} }

impl ScrollView {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            scroll_presenter: RefCell::new(None),
            horizontal_scroll_bar_controller: RefCell::new(None),
            vertical_scroll_bar_controller: RefCell::new(None),
            scroll_bars_separator: RefCell::new(None),
            presenter_subscriptions: RefCell::new(Vec::new()),
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

    /// Creates a scroll view without content.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    accessors! {
        /// The element the view scrolls (`Content`).
        content, set_content, content_property: Option<Ref<Control>>;
        /// `HorizontalScrollBarVisibility`.
        horizontal_scroll_bar_visibility, set_horizontal_scroll_bar_visibility,
            horizontal_scroll_bar_visibility_property:
            ScrollingScrollBarVisibility;
        /// `VerticalScrollBarVisibility`.
        vertical_scroll_bar_visibility, set_vertical_scroll_bar_visibility, vertical_scroll_bar_visibility_property:
            ScrollingScrollBarVisibility;
        /// `ContentOrientation`.
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
        /// `HorizontalScrollMode`.
        horizontal_scroll_mode, set_horizontal_scroll_mode, horizontal_scroll_mode_property: ScrollingScrollMode;
        /// `VerticalScrollMode`.
        vertical_scroll_mode, set_vertical_scroll_mode, vertical_scroll_mode_property: ScrollingScrollMode;
        /// `ZoomChainMode`.
        zoom_chain_mode, set_zoom_chain_mode, zoom_chain_mode_property: ScrollingChainMode;
        /// `ZoomMode`.
        zoom_mode, set_zoom_mode, zoom_mode_property: ScrollingZoomMode;
        /// `IgnoredInputKinds`.
        ignored_input_kinds, set_ignored_input_kinds, ignored_input_kinds_property: ScrollingInputKinds;
        /// `MinZoomFactor`.
        min_zoom_factor, set_min_zoom_factor, min_zoom_factor_property: f64;
        /// `MaxZoomFactor`.
        max_zoom_factor, set_max_zoom_factor, max_zoom_factor_property: f64;
        /// `HorizontalAnchorRatio`.
        horizontal_anchor_ratio, set_horizontal_anchor_ratio, horizontal_anchor_ratio_property: f64;
        /// `VerticalAnchorRatio`.
        vertical_anchor_ratio, set_vertical_anchor_ratio, vertical_anchor_ratio_property: f64;
    }

    /// Whether the horizontal bar shows (`ComputedHorizontalScrollBarVisibility`).
    pub fn computed_horizontal_scroll_bar_visibility(&self) -> bool {
        self.get_value(Self::computed_horizontal_scroll_bar_visibility_property())
    }

    /// Whether the vertical bar shows (`ComputedVerticalScrollBarVisibility`).
    pub fn computed_vertical_scroll_bar_visibility(&self) -> bool {
        self.get_value(Self::computed_vertical_scroll_bar_visibility_property())
    }

    /// `ComputedHorizontalScrollMode`.
    pub fn computed_horizontal_scroll_mode(&self) -> ScrollingScrollMode {
        self.get_value(Self::computed_horizontal_scroll_mode_property())
    }

    /// `ComputedVerticalScrollMode`.
    pub fn computed_vertical_scroll_mode(&self) -> ScrollingScrollMode {
        self.get_value(Self::computed_vertical_scroll_mode_property())
    }

    /// The presenter of the template (`ScrollPresenter`).
    pub fn scroll_presenter(&self) -> Option<Ref<ScrollPresenter>> {
        self.scroll_presenter.borrow().clone()
    }

    fn presenter_or<T>(&self, default: T, f: impl FnOnce(&ScrollPresenter) -> T) -> T {
        self.scroll_presenter()
            .map_or(default, |presenter| f(&presenter))
    }

    /// `HorizontalOffset`.
    pub fn horizontal_offset(&self) -> f64 {
        self.presenter_or(0.0, ScrollPresenter::horizontal_offset)
    }

    /// `VerticalOffset`.
    pub fn vertical_offset(&self) -> f64 {
        self.presenter_or(0.0, ScrollPresenter::vertical_offset)
    }

    /// `ZoomFactor`.
    pub fn zoom_factor(&self) -> f32 {
        self.presenter_or(1.0, ScrollPresenter::zoom_factor)
    }

    /// `ExtentWidth`.
    pub fn extent_width(&self) -> f64 {
        self.presenter_or(0.0, ScrollPresenter::extent_width)
    }

    /// `ExtentHeight`.
    pub fn extent_height(&self) -> f64 {
        self.presenter_or(0.0, ScrollPresenter::extent_height)
    }

    /// `ViewportWidth`.
    pub fn viewport_width(&self) -> f64 {
        self.presenter_or(0.0, ScrollPresenter::viewport_width)
    }

    /// `ViewportHeight`.
    pub fn viewport_height(&self) -> f64 {
        self.presenter_or(0.0, ScrollPresenter::viewport_height)
    }

    /// `ScrollableWidth`.
    pub fn scrollable_width(&self) -> f64 {
        self.presenter_or(0.0, ScrollPresenter::scrollable_width)
    }

    /// `ScrollableHeight`.
    pub fn scrollable_height(&self) -> f64 {
        self.presenter_or(0.0, ScrollPresenter::scrollable_height)
    }

    /// `State`.
    pub fn state(&self) -> ScrollingInteractionState {
        self.presenter_or(ScrollingInteractionState::Idle, ScrollPresenter::state)
    }

    /// `CurrentAnchor`.
    pub fn current_anchor(&self) -> Option<Ref<Control>> {
        self.presenter_or(None, ScrollPresenter::current_anchor)
    }

    /// `RegisterAnchorCandidate`.
    pub fn register_anchor_candidate(&self, element: &Ref<Control>) {
        self.presenter_or((), |presenter| presenter.register_anchor_candidate(element));
    }

    /// `UnregisterAnchorCandidate`.
    pub fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        self.presenter_or((), |presenter| {
            presenter.unregister_anchor_candidate(element)
        });
    }

    /// `ScrollTo(horizontalOffset, verticalOffset)`; -1 before the template is applied.
    pub fn scroll_to(&self, horizontal_offset: f64, vertical_offset: f64) -> i32 {
        self.presenter_or(-1, |p| p.scroll_to(horizontal_offset, vertical_offset))
    }

    /// `ScrollTo(horizontalOffset, verticalOffset, options)`.
    pub fn scroll_to_with(
        &self,
        horizontal_offset: f64,
        vertical_offset: f64,
        options: ScrollingScrollOptions,
    ) -> i32 {
        self.presenter_or(-1, |p| {
            p.scroll_to_with(horizontal_offset, vertical_offset, options)
        })
    }

    /// `ScrollBy(horizontalOffsetDelta, verticalOffsetDelta)`.
    pub fn scroll_by(&self, horizontal_offset_delta: f64, vertical_offset_delta: f64) -> i32 {
        self.presenter_or(-1, |p| {
            p.scroll_by(horizontal_offset_delta, vertical_offset_delta)
        })
    }

    /// `ScrollBy(horizontalOffsetDelta, verticalOffsetDelta, options)`.
    pub fn scroll_by_with(
        &self,
        horizontal_offset_delta: f64,
        vertical_offset_delta: f64,
        options: ScrollingScrollOptions,
    ) -> i32 {
        self.presenter_or(-1, |p| {
            p.scroll_by_with(horizontal_offset_delta, vertical_offset_delta, options)
        })
    }

    /// `AddScrollVelocity(offsetsVelocity, inertiaDecayRate)`.
    pub fn add_scroll_velocity(
        &self,
        offsets_velocity: Vector2,
        inertia_decay_rate: Option<Vector2>,
    ) -> i32 {
        self.presenter_or(-1, |p| {
            p.add_scroll_velocity(offsets_velocity, inertia_decay_rate)
        })
    }

    /// `ZoomTo(zoomFactor, centerPoint)`.
    pub fn zoom_to(&self, zoom_factor: f32, center_point: Option<Vector2>) -> i32 {
        self.presenter_or(-1, |p| p.zoom_to(zoom_factor, center_point))
    }

    /// `ZoomTo(zoomFactor, centerPoint, options)`.
    pub fn zoom_to_with(
        &self,
        zoom_factor: f32,
        center_point: Option<Vector2>,
        options: ScrollingZoomOptions,
    ) -> i32 {
        self.presenter_or(-1, |p| p.zoom_to_with(zoom_factor, center_point, options))
    }

    /// `ZoomBy(zoomFactorDelta, centerPoint)`.
    pub fn zoom_by(&self, zoom_factor_delta: f32, center_point: Option<Vector2>) -> i32 {
        self.presenter_or(-1, |p| p.zoom_by(zoom_factor_delta, center_point))
    }

    /// `ZoomBy(zoomFactorDelta, centerPoint, options)`.
    pub fn zoom_by_with(
        &self,
        zoom_factor_delta: f32,
        center_point: Option<Vector2>,
        options: ScrollingZoomOptions,
    ) -> i32 {
        self.presenter_or(-1, |p| {
            p.zoom_by_with(zoom_factor_delta, center_point, options)
        })
    }

    /// `AddZoomVelocity(zoomFactorVelocity, centerPoint, inertiaDecayRate)`.
    pub fn add_zoom_velocity(
        &self,
        zoom_factor_velocity: f32,
        center_point: Option<Vector2>,
        inertia_decay_rate: Option<f32>,
    ) -> i32 {
        self.presenter_or(-1, |p| {
            p.add_zoom_velocity(zoom_factor_velocity, center_point, inertia_decay_rate)
        })
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

    /// Adds a handler of `ExtentChanged`.
    pub fn extent_changed(&self, handler: impl Fn(&ScrollView) + 'static) -> Subscription {
        self.subscribe(
            |this| this.extent_changed.add(Rc::new(handler)),
            |this, token| {
                this.extent_changed.remove(token);
            },
        )
    }

    /// Adds a handler of `StateChanged`.
    pub fn state_changed(&self, handler: impl Fn(&ScrollView) + 'static) -> Subscription {
        self.subscribe(
            |this| this.state_changed.add(Rc::new(handler)),
            |this, token| {
                this.state_changed.remove(token);
            },
        )
    }

    /// Adds a handler of `ViewChanged`.
    pub fn view_changed(&self, handler: impl Fn(&ScrollView) + 'static) -> Subscription {
        self.subscribe(
            |this| this.view_changed.add(Rc::new(handler)),
            |this, token| {
                this.view_changed.remove(token);
            },
        )
    }

    /// Adds a handler of `ScrollAnimationStarting`.
    pub fn scroll_animation_starting(
        &self,
        handler: impl Fn(&ScrollView, &ScrollingScrollAnimationStartingEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.scroll_animation_starting.add(Rc::new(handler)),
            |this, token| {
                this.scroll_animation_starting.remove(token);
            },
        )
    }

    /// Adds a handler of `ZoomAnimationStarting`.
    pub fn zoom_animation_starting(
        &self,
        handler: impl Fn(&ScrollView, &ScrollingZoomAnimationStartingEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.zoom_animation_starting.add(Rc::new(handler)),
            |this, token| {
                this.zoom_animation_starting.remove(token);
            },
        )
    }

    /// Adds a handler of `ScrollCompleted`.
    pub fn scroll_completed(
        &self,
        handler: impl Fn(&ScrollView, &ScrollingScrollCompletedEventArgs) + 'static,
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
        handler: impl Fn(&ScrollView, &ScrollingZoomCompletedEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.zoom_completed.add(Rc::new(handler)),
            |this, token| {
                this.zoom_completed.remove(token);
            },
        )
    }

    /// Adds a handler of `ScrollStarting`.
    pub fn scroll_starting(
        &self,
        handler: impl Fn(&ScrollView, &ScrollingScrollStartingEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.scroll_starting.add(Rc::new(handler)),
            |this, token| {
                this.scroll_starting.remove(token);
            },
        )
    }

    /// Adds a handler of `ZoomStarting`.
    pub fn zoom_starting(
        &self,
        handler: impl Fn(&ScrollView, &ScrollingZoomStartingEventArgs) + 'static,
    ) -> Subscription {
        self.subscribe(
            |this| this.zoom_starting.add(Rc::new(handler)),
            |this, token| {
                this.zoom_starting.remove(token);
            },
        )
    }

    /// `HookScrollPresenterEvents`: the presenter's events are raised as the view's.
    fn hook_scroll_presenter(&self, presenter: &Ref<ScrollPresenter>) {
        macro_rules! forward {
            ($subscribe:ident, $list:ident) => {{
                let weak = self.to_ref().downgrade();
                presenter.$subscribe(move |_, args| {
                    if let Some(this) = weak.upgrade() {
                        for (_, handler) in this.$list.snapshot().iter() {
                            handler(&this, args);
                        }
                    }
                })
            }};
        }
        let weak = self.to_ref().downgrade();
        let view_changed = presenter.view_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.update_scroll_controllers_visibility();
                for (_, handler) in this.view_changed.snapshot().iter() {
                    handler(&this);
                }
            }
        });
        let weak = self.to_ref().downgrade();
        let extent_changed = presenter.extent_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.update_scroll_controllers_visibility();
                for (_, handler) in this.extent_changed.snapshot().iter() {
                    handler(&this);
                }
            }
        });
        let weak = self.to_ref().downgrade();
        let state_changed = presenter.state_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                for (_, handler) in this.state_changed.snapshot().iter() {
                    handler(&this);
                }
            }
        });
        let mut subscriptions = vec![
            view_changed,
            extent_changed,
            state_changed,
            forward!(scroll_animation_starting, scroll_animation_starting),
            forward!(zoom_animation_starting, zoom_animation_starting),
            forward!(scroll_completed, scroll_completed),
            forward!(zoom_completed, zoom_completed),
            forward!(scroll_starting, scroll_starting),
            forward!(zoom_starting, zoom_starting),
        ];
        self.presenter_subscriptions
            .borrow_mut()
            .append(&mut subscriptions);
    }

    /// `UpdateScrollControllersVisibility`: `Auto` shows a bar whose controller can scroll; the
    /// separator shows when both bars do. The computed scroll modes follow the presenter.
    fn update_scroll_controllers_visibility(&self) {
        let compute =
            |visibility: ScrollingScrollBarVisibility,
             controller: &Option<Rc<ScrollBarController>>| match visibility {
                ScrollingScrollBarVisibility::Auto => controller
                    .as_ref()
                    .is_some_and(|controller| controller.can_scroll()),
                ScrollingScrollBarVisibility::Visible => true,
                ScrollingScrollBarVisibility::Hidden => false,
            };
        let horizontal = compute(
            self.horizontal_scroll_bar_visibility(),
            &self.horizontal_scroll_bar_controller.borrow(),
        );
        let vertical = compute(
            self.vertical_scroll_bar_visibility(),
            &self.vertical_scroll_bar_controller.borrow(),
        );
        self.set_value(
            Self::computed_horizontal_scroll_bar_visibility_property(),
            horizontal,
        );
        self.set_value(
            Self::computed_vertical_scroll_bar_visibility_property(),
            vertical,
        );
        if let Some(separator) = self.scroll_bars_separator.borrow().as_ref() {
            separator.set_is_visible(horizontal && vertical);
        }
        if let Some(presenter) = self.scroll_presenter() {
            self.set_value(
                Self::computed_horizontal_scroll_mode_property(),
                presenter.computed_horizontal_scroll_mode(),
            );
            self.set_value(
                Self::computed_vertical_scroll_mode_property(),
                presenter.computed_vertical_scroll_mode(),
            );
        }
    }

    /// The controller of a bar of the template, for the tests.
    #[cfg(test)]
    pub(crate) fn scroll_bar_controller(
        &self,
        horizontal: bool,
    ) -> Option<Rc<ScrollBarController>> {
        if horizontal {
            self.horizontal_scroll_bar_controller.borrow().clone()
        } else {
            self.vertical_scroll_bar_controller.borrow().clone()
        }
    }
}
