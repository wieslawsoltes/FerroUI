//! The enumerations, options and event arguments of WinUI's `ScrollView` and `ScrollPresenter`
//! (`controls/dev/ScrollPresenter/ScrollPresenter.idl`, `ScrollingScrollOptions.cpp`,
//! `ScrollingZoomOptions.cpp`, the `Scrolling*EventArgs.cpp` files and
//! `controls/dev/ScrollView/ScrollView.idl` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use ferroui_base::numerics::Vector2;
use ferroui_base::rendering::composition::animations::{
    ScalarKeyFrameAnimation, Vector3KeyFrameAnimation,
};
use ferroui_base::{ferro_markup_enum, ferro_markup_type};
use std::cell::RefCell;
use std::rc::Rc;

/// The visibility of a scroll bar of a `ScrollView` (WinUI's `ScrollingScrollBarVisibility`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingScrollBarVisibility {
    /// The bar is shown when the content can be scrolled on its axis.
    #[default]
    Auto = 0,
    /// The bar is always shown.
    Visible = 1,
    /// The bar is never shown; the content can still be scrolled.
    Hidden = 2,
}

/// The axes on which the content of a scroll presenter is not constrained to the viewport
/// (WinUI's `ScrollingContentOrientation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingContentOrientation {
    /// The content is as wide as the viewport and scrolls vertically.
    #[default]
    Vertical = 0,
    /// The content is as tall as the viewport and scrolls horizontally.
    Horizontal = 1,
    /// The content is measured without constraint and scrolls on both axes.
    None = 2,
    /// The content is constrained to the viewport on both axes (it scrolls only when zoomed).
    Both = 3,
}

/// What moves the view of a scroll presenter (WinUI's `ScrollingInteractionState`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingInteractionState {
    /// Nothing moves the view.
    #[default]
    Idle = 0,
    /// The user moves the view (touch or pen).
    Interaction = 1,
    /// The view moves by inertia (a velocity added by `AddScrollVelocity` or `AddZoomVelocity`).
    Inertia = 2,
    /// The view moves by an animated `ScrollTo`, `ScrollBy`, `ZoomTo` or `ZoomBy`.
    Animation = 3,
}

/// Whether the user can scroll on an axis (WinUI's `ScrollingScrollMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingScrollMode {
    /// The user can scroll.
    Enabled = 0,
    /// The user cannot scroll; the view can still be changed in code.
    Disabled = 1,
    /// The user can scroll when the content orientation lets the content scroll on the axis.
    #[default]
    Auto = 2,
}

/// Whether the user can zoom (WinUI's `ScrollingZoomMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingZoomMode {
    /// The user can zoom (Ctrl and the mouse wheel).
    Enabled = 0,
    /// The user cannot zoom; the zoom can still be changed in code.
    #[default]
    Disabled = 1,
}

/// Whether a scroll or zoom past the end chains to an outer scroller (WinUI's
/// `ScrollingChainMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingChainMode {
    /// Chains when the scroller cannot move further.
    #[default]
    Auto = 0,
    /// Always chains.
    Always = 1,
    /// Never chains.
    Never = 2,
}

/// Whether a pan sticks to the axis it starts on (WinUI's `ScrollingRailMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingRailMode {
    /// The pan sticks to its axis.
    #[default]
    Enabled = 0,
    /// The pan moves freely.
    Disabled = 1,
}

bitflags::bitflags! {
    /// The kinds of input a scroll presenter ignores (WinUI's `ScrollingInputKinds`).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct ScrollingInputKinds: u32 {
        /// No input is ignored.
        const NONE = 0;
        /// Touch.
        const TOUCH = 1;
        /// Pen.
        const PEN = 2;
        /// The mouse wheel.
        const MOUSE_WHEEL = 4;
        /// The keyboard.
        const KEYBOARD = 8;
        /// The game pad.
        const GAMEPAD = 16;
        /// All input.
        const ALL = 255;
    }
}

/// Whether a programmatic view change is animated (WinUI's `ScrollingAnimationMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingAnimationMode {
    /// The change is applied at once.
    Disabled = 0,
    /// The change is animated.
    Enabled = 1,
    /// The change is animated unless the platform turns animations off.
    #[default]
    Auto = 2,
}

/// Whether a programmatic view change lands on the snap points (WinUI's
/// `ScrollingSnapPointsMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollingSnapPointsMode {
    /// The snap points apply.
    #[default]
    Default = 0,
    /// The snap points are ignored.
    Ignore = 1,
}

ferro_markup_enum!(ScrollingScrollBarVisibility { Auto, Visible, Hidden }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(ScrollingContentOrientation { Vertical, Horizontal, None, Both }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(
    ScrollingInteractionState { Idle, Interaction, Inertia, Animation },
    { namespace: "FerroUI.Controls" }
);
ferro_markup_enum!(ScrollingScrollMode { Enabled, Disabled, Auto }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(ScrollingZoomMode { Enabled, Disabled }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(ScrollingChainMode { Auto, Always, Never }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(ScrollingRailMode { Enabled, Disabled }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(flags ScrollingInputKinds {
    None = ScrollingInputKinds::NONE, Touch = ScrollingInputKinds::TOUCH, Pen = ScrollingInputKinds::PEN,
    MouseWheel = ScrollingInputKinds::MOUSE_WHEEL, Keyboard = ScrollingInputKinds::KEYBOARD,
    Gamepad = ScrollingInputKinds::GAMEPAD, All = ScrollingInputKinds::ALL
}, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(ScrollingAnimationMode { Disabled, Enabled, Auto }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(ScrollingSnapPointsMode { Default, Ignore }, { namespace: "FerroUI.Controls" });

/// The options of a `ScrollTo` or `ScrollBy` (WinUI's `ScrollingScrollOptions`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollingScrollOptions {
    /// Whether the change is animated.
    pub animation_mode: ScrollingAnimationMode,
    /// Whether the change lands on the snap points.
    pub snap_points_mode: ScrollingSnapPointsMode,
}

impl ScrollingScrollOptions {
    /// `ScrollingScrollOptions(animationMode)`.
    pub fn new(animation_mode: ScrollingAnimationMode) -> Self {
        Self {
            animation_mode,
            snap_points_mode: ScrollingSnapPointsMode::Default,
        }
    }

    /// `ScrollingScrollOptions(animationMode, snapPointsMode)`.
    pub fn with_snap_points_mode(
        animation_mode: ScrollingAnimationMode,
        snap_points_mode: ScrollingSnapPointsMode,
    ) -> Self {
        Self {
            animation_mode,
            snap_points_mode,
        }
    }
}

/// The options of a `ZoomTo` or `ZoomBy` (WinUI's `ScrollingZoomOptions`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollingZoomOptions {
    /// Whether the change is animated.
    pub animation_mode: ScrollingAnimationMode,
    /// Whether the change lands on the snap points.
    pub snap_points_mode: ScrollingSnapPointsMode,
}

impl ScrollingZoomOptions {
    /// `ScrollingZoomOptions(animationMode)`.
    pub fn new(animation_mode: ScrollingAnimationMode) -> Self {
        Self {
            animation_mode,
            snap_points_mode: ScrollingSnapPointsMode::Default,
        }
    }

    /// `ScrollingZoomOptions(animationMode, snapPointsMode)`.
    pub fn with_snap_points_mode(
        animation_mode: ScrollingAnimationMode,
        snap_points_mode: ScrollingSnapPointsMode,
    ) -> Self {
        Self {
            animation_mode,
            snap_points_mode,
        }
    }
}

/// The arguments of `ScrollAnimationStarting` (WinUI's
/// `ScrollingScrollAnimationStartingEventArgs`): the animation of the position, which a handler
/// may change or replace (`Animation`), from `StartPosition` to `EndPosition`.
pub struct ScrollingScrollAnimationStartingEventArgs {
    animation: RefCell<Option<Rc<Vector3KeyFrameAnimation>>>,
    start_position: Vector2,
    end_position: Vector2,
    correlation_id: i32,
}

impl ScrollingScrollAnimationStartingEventArgs {
    pub(crate) fn new(
        animation: Rc<Vector3KeyFrameAnimation>,
        start_position: Vector2,
        end_position: Vector2,
        correlation_id: i32,
    ) -> Self {
        Self {
            animation: RefCell::new(Some(animation)),
            start_position,
            end_position,
            correlation_id,
        }
    }

    /// The animation of the position (x and y are the offsets; z is not used).
    pub fn animation(&self) -> Option<Rc<Vector3KeyFrameAnimation>> {
        self.animation.borrow().clone()
    }

    /// Replaces the animation; `None` applies the change at once.
    pub fn set_animation(&self, value: Option<Rc<Vector3KeyFrameAnimation>>) {
        *self.animation.borrow_mut() = value;
    }

    /// The offsets the animation starts from.
    pub fn start_position(&self) -> Vector2 {
        self.start_position
    }

    /// The offsets the animation ends at.
    pub fn end_position(&self) -> Vector2 {
        self.end_position
    }

    /// The id the `ScrollTo` or `ScrollBy` returned.
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id
    }
}

/// The arguments of `ZoomAnimationStarting` (WinUI's `ScrollingZoomAnimationStartingEventArgs`).
pub struct ScrollingZoomAnimationStartingEventArgs {
    animation: RefCell<Option<Rc<ScalarKeyFrameAnimation>>>,
    center_point: Vector2,
    start_zoom_factor: f32,
    end_zoom_factor: f32,
    correlation_id: i32,
}

impl ScrollingZoomAnimationStartingEventArgs {
    pub(crate) fn new(
        animation: Rc<ScalarKeyFrameAnimation>,
        center_point: Vector2,
        start_zoom_factor: f32,
        end_zoom_factor: f32,
        correlation_id: i32,
    ) -> Self {
        Self {
            animation: RefCell::new(Some(animation)),
            center_point,
            start_zoom_factor,
            end_zoom_factor,
            correlation_id,
        }
    }

    /// The animation of the zoom factor.
    pub fn animation(&self) -> Option<Rc<ScalarKeyFrameAnimation>> {
        self.animation.borrow().clone()
    }

    /// Replaces the animation; `None` applies the change at once.
    pub fn set_animation(&self, value: Option<Rc<ScalarKeyFrameAnimation>>) {
        *self.animation.borrow_mut() = value;
    }

    /// The point of the viewport that the zoom keeps in place.
    pub fn center_point(&self) -> Vector2 {
        self.center_point
    }

    /// The zoom factor the animation starts from.
    pub fn start_zoom_factor(&self) -> f32 {
        self.start_zoom_factor
    }

    /// The zoom factor the animation ends at.
    pub fn end_zoom_factor(&self) -> f32 {
        self.end_zoom_factor
    }

    /// The id the `ZoomTo` or `ZoomBy` returned.
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id
    }
}

/// The arguments of `ScrollCompleted` (WinUI's `ScrollingScrollCompletedEventArgs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollingScrollCompletedEventArgs {
    correlation_id: i32,
}

impl ScrollingScrollCompletedEventArgs {
    pub(crate) fn new(correlation_id: i32) -> Self {
        Self { correlation_id }
    }

    /// The id the request returned.
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id
    }
}

/// The arguments of `ZoomCompleted` (WinUI's `ScrollingZoomCompletedEventArgs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollingZoomCompletedEventArgs {
    correlation_id: i32,
}

impl ScrollingZoomCompletedEventArgs {
    pub(crate) fn new(correlation_id: i32) -> Self {
        Self { correlation_id }
    }

    /// The id the request returned.
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id
    }
}

/// The arguments of `ScrollStarting` (WinUI's `ScrollingScrollStartingEventArgs`): the view when
/// the scroll starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollingScrollStartingEventArgs {
    correlation_id: i32,
    horizontal_offset: f64,
    vertical_offset: f64,
    zoom_factor: f32,
}

impl ScrollingScrollStartingEventArgs {
    pub(crate) fn new(
        correlation_id: i32,
        horizontal_offset: f64,
        vertical_offset: f64,
        zoom_factor: f32,
    ) -> Self {
        Self {
            correlation_id,
            horizontal_offset,
            vertical_offset,
            zoom_factor,
        }
    }

    /// The id the request returned.
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id
    }

    /// The horizontal offset when the scroll starts.
    pub fn horizontal_offset(&self) -> f64 {
        self.horizontal_offset
    }

    /// The vertical offset when the scroll starts.
    pub fn vertical_offset(&self) -> f64 {
        self.vertical_offset
    }

    /// The zoom factor when the scroll starts.
    pub fn zoom_factor(&self) -> f32 {
        self.zoom_factor
    }
}

/// The arguments of `ZoomStarting` (WinUI's `ScrollingZoomStartingEventArgs`).
pub type ScrollingZoomStartingEventArgs = ScrollingScrollStartingEventArgs;

ferro_markup_type!(class ScrollingScrollCompletedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ScrollingScrollCompletedEventArgs],
    properties: [
        CorrelationId: i32 { get: ScrollingScrollCompletedEventArgs::correlation_id },
    ],
});

ferro_markup_type!(class ScrollingZoomCompletedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ScrollingZoomCompletedEventArgs],
    properties: [
        CorrelationId: i32 { get: ScrollingZoomCompletedEventArgs::correlation_id },
    ],
});

ferro_markup_type!(class ScrollingScrollStartingEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ScrollingScrollStartingEventArgs],
    properties: [
        CorrelationId: i32 { get: ScrollingScrollStartingEventArgs::correlation_id },
        HorizontalOffset: f64 { get: ScrollingScrollStartingEventArgs::horizontal_offset },
        VerticalOffset: f64 { get: ScrollingScrollStartingEventArgs::vertical_offset },
        ZoomFactor: f32 { get: ScrollingScrollStartingEventArgs::zoom_factor },
    ],
});
