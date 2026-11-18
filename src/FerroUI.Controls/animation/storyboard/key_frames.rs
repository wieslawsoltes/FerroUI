//! WinUI's key frame animations: `DoubleAnimationUsingKeyFrames`, `ColorAnimationUsingKeyFrames`,
//! `PointAnimationUsingKeyFrames` with their `Discrete`, `Linear`, `Spline` and `Easing` key
//! frames, and `ObjectAnimationUsingKeyFrames` with `DiscreteObjectKeyFrame`
//! (`Microsoft.UI.Xaml.Media.Animation`; `dxaml/xcp/core/animation/animation.cpp`,
//! `KeyFrame.cpp` of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md).

use super::{EasingFunctionBase, Timeline};
use ferroui_base::animation::{KeyFrameEase, KeySpline, SignedDuration};
use ferroui_base::collections::FerroList;
use ferroui_base::media::Color;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, FerroProperty, Point, Ref, StyledProperty,
};
use std::rc::Rc;

/// How a key frame reaches its value: the key frame class it is.
pub(crate) fn ease_of(frame: &FerroObject) -> KeyFrameEase {
    let spline = |spline: Option<Ref<KeySpline>>| match spline {
        Some(spline) => KeyFrameEase::Spline(spline),
        // A spline key frame without a spline is linear (WinUI's default spline is the line).
        None => KeyFrameEase::Linear,
    };
    let easing = |easing: Option<Ref<EasingFunctionBase>>| match easing.and_then(|easing| easing.curve()) {
        Some(curve) => KeyFrameEase::Easing(Rc::new(curve)),
        None => KeyFrameEase::Linear,
    };
    if let Some(frame) = frame.downcast_ref::<SplineDoubleKeyFrame>() {
        spline(frame.key_spline())
    } else if let Some(frame) = frame.downcast_ref::<SplineColorKeyFrame>() {
        spline(frame.key_spline())
    } else if let Some(frame) = frame.downcast_ref::<SplinePointKeyFrame>() {
        spline(frame.key_spline())
    } else if let Some(frame) = frame.downcast_ref::<EasingDoubleKeyFrame>() {
        easing(frame.easing_function())
    } else if let Some(frame) = frame.downcast_ref::<EasingColorKeyFrame>() {
        easing(frame.easing_function())
    } else if let Some(frame) = frame.downcast_ref::<EasingPointKeyFrame>() {
        easing(frame.easing_function())
    } else if frame.downcast_ref::<LinearDoubleKeyFrame>().is_some()
        || frame.downcast_ref::<LinearColorKeyFrame>().is_some()
        || frame.downcast_ref::<LinearPointKeyFrame>().is_some()
    {
        KeyFrameEase::Linear
    } else {
        KeyFrameEase::Discrete
    }
}

/// The base class of the key frames of a `f64` property (WinUI's `DoubleKeyFrame`): a `Value` reached at a
/// `KeyTime`.
#[repr(C)]
pub struct DoubleKeyFrame {
    base: FerroObject,
}

ferro_class!(DoubleKeyFrame: FerroObject);
ferro_class_info!(DoubleKeyFrame { markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(DoubleKeyFrame: FerroObjectImpl);

ferro_properties! { impl DoubleKeyFrame {
    /// Defines the `KeyTime` property: when the value is reached, from the start of the
    /// animation. Default zero.
    pub fn key_time_property() -> StyledProperty<SignedDuration> {
        FerroProperty::register::<DoubleKeyFrame, _>("KeyTime", SignedDuration::ZERO)
    }

    /// Defines the `Value` property: the value of the key frame.
    pub fn value_property() -> StyledProperty<f64> {
        FerroProperty::register::<DoubleKeyFrame, _>("Value", f64::default())
    }
} }

impl DoubleKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }

    pub fn key_time(&self) -> SignedDuration {
        self.get_value(Self::key_time_property())
    }

    pub fn set_key_time(&self, value: SignedDuration) {
        self.set_value(Self::key_time_property(), value);
    }

    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }

    pub fn set_frame_value(&self, value: f64) {
        self.set_value(Self::value_property(), value);
    }
}

/// Holds the value before it until its key time, then jumps.
#[repr(C)]
pub struct DiscreteDoubleKeyFrame {
    base: DoubleKeyFrame,
}

ferro_class!(DiscreteDoubleKeyFrame: DoubleKeyFrame);
ferro_class_info!(DiscreteDoubleKeyFrame {
    new: DiscreteDoubleKeyFrame::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(DiscreteDoubleKeyFrame: FerroObjectImpl);

impl DiscreteDoubleKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: DoubleKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a key frame of `value` at `key_time`.
    pub fn with(key_time: SignedDuration, value: f64) -> Ref<Self> {
        let frame = Self::new();
        frame.set_key_time(key_time);
        frame.set_frame_value(value);
        frame
    }
}

/// Interpolates linearly from the value before it.
#[repr(C)]
pub struct LinearDoubleKeyFrame {
    base: DoubleKeyFrame,
}

ferro_class!(LinearDoubleKeyFrame: DoubleKeyFrame);
ferro_class_info!(LinearDoubleKeyFrame { new: LinearDoubleKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(LinearDoubleKeyFrame: FerroObjectImpl);

impl LinearDoubleKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: DoubleKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a key frame of `value` at `key_time`.
    pub fn with(key_time: SignedDuration, value: f64) -> Ref<Self> {
        let frame = Self::new();
        frame.set_key_time(key_time);
        frame.set_frame_value(value);
        frame
    }
}

/// Interpolates along a `KeySpline` from the value before it (WinUI's `SplineDoubleKeyFrame`).
#[repr(C)]
pub struct SplineDoubleKeyFrame {
    base: DoubleKeyFrame,
}

ferro_class!(SplineDoubleKeyFrame: DoubleKeyFrame);
ferro_class_info!(SplineDoubleKeyFrame { new: SplineDoubleKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(SplineDoubleKeyFrame: FerroObjectImpl);

ferro_properties! { impl SplineDoubleKeyFrame {
    /// Defines the `KeySpline` property: the Bezier curve of the progress. Default `None`
    /// (linear).
    pub fn key_spline_property() -> StyledProperty<Option<Ref<KeySpline>>> {
        FerroProperty::register::<SplineDoubleKeyFrame, _>("KeySpline", None)
    }
} }

impl SplineDoubleKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: DoubleKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn key_spline(&self) -> Option<Ref<KeySpline>> {
        self.get_value(Self::key_spline_property())
    }

    pub fn set_key_spline(&self, value: Option<Ref<KeySpline>>) {
        self.set_value(Self::key_spline_property(), value);
    }
}

/// Interpolates with an easing function from the value before it (WinUI's `EasingDoubleKeyFrame`).
#[repr(C)]
pub struct EasingDoubleKeyFrame {
    base: DoubleKeyFrame,
}

ferro_class!(EasingDoubleKeyFrame: DoubleKeyFrame);
ferro_class_info!(EasingDoubleKeyFrame { new: EasingDoubleKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(EasingDoubleKeyFrame: FerroObjectImpl);

ferro_properties! { impl EasingDoubleKeyFrame {
    /// Defines the `EasingFunction` property. Default `None` (linear).
    pub fn easing_function_property() -> StyledProperty<Option<Ref<EasingFunctionBase>>> {
        FerroProperty::register::<EasingDoubleKeyFrame, _>("EasingFunction", None)
    }
} }

impl EasingDoubleKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: DoubleKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn easing_function(&self) -> Option<Ref<EasingFunctionBase>> {
        self.get_value(Self::easing_function_property())
    }

    pub fn set_easing_function(&self, value: Option<Ref<EasingFunctionBase>>) {
        self.set_value(Self::easing_function_property(), value);
    }
}

/// Animates a `f64` property through its `KeyFrames` (WinUI's `DoubleAnimationUsingKeyFrames`).
///
/// Before the first key frame the value goes from the value the property has when the
/// animation begins; after the last it holds the last value. With an automatic `Duration`
/// the animation lasts until its last key time. `EnableDependentAnimation` is accepted and
/// has no effect.
#[repr(C)]
pub struct DoubleAnimationUsingKeyFrames {
    base: Timeline,
    key_frames: FerroList<Ref<DoubleKeyFrame>>,
}

ferro_class!(DoubleAnimationUsingKeyFrames: Timeline);
ferro_class_info!(DoubleAnimationUsingKeyFrames {
    new: DoubleAnimationUsingKeyFrames::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: KeyFrames,
        properties: [
            KeyFrames: FerroList<Ref<DoubleKeyFrame>> { get: DoubleAnimationUsingKeyFrames::key_frames },
        ],
    },
});
ferro_impl_classes!(DoubleAnimationUsingKeyFrames: FerroObjectImpl);

ferro_properties! { impl DoubleAnimationUsingKeyFrames {
    /// Defines the `EnableDependentAnimation` property (no effect). Default `false`.
    pub fn enable_dependent_animation_property() -> StyledProperty<bool> {
        FerroProperty::register::<DoubleAnimationUsingKeyFrames, _>("EnableDependentAnimation", false)
    }
} }

impl DoubleAnimationUsingKeyFrames {
    pub fn construct() -> Self {
        Self {
            base: Timeline::construct(),
            key_frames: FerroList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The key frames (in any order: they are sorted by their key times).
    pub fn key_frames(&self) -> FerroList<Ref<DoubleKeyFrame>> {
        self.key_frames.clone()
    }

    pub fn enable_dependent_animation(&self) -> bool {
        self.get_value(Self::enable_dependent_animation_property())
    }

    pub fn set_enable_dependent_animation(&self, value: bool) {
        self.set_value(Self::enable_dependent_animation_property(), value);
    }
}

/// The base class of the key frames of a `Color` property (WinUI's `ColorKeyFrame`): a `Value` reached at a
/// `KeyTime`.
#[repr(C)]
pub struct ColorKeyFrame {
    base: FerroObject,
}

ferro_class!(ColorKeyFrame: FerroObject);
ferro_class_info!(ColorKeyFrame { markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(ColorKeyFrame: FerroObjectImpl);

ferro_properties! { impl ColorKeyFrame {
    /// Defines the `KeyTime` property: when the value is reached, from the start of the
    /// animation. Default zero.
    pub fn key_time_property() -> StyledProperty<SignedDuration> {
        FerroProperty::register::<ColorKeyFrame, _>("KeyTime", SignedDuration::ZERO)
    }

    /// Defines the `Value` property: the value of the key frame.
    pub fn value_property() -> StyledProperty<Color> {
        FerroProperty::register::<ColorKeyFrame, _>("Value", Color::default())
    }
} }

impl ColorKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }

    pub fn key_time(&self) -> SignedDuration {
        self.get_value(Self::key_time_property())
    }

    pub fn set_key_time(&self, value: SignedDuration) {
        self.set_value(Self::key_time_property(), value);
    }

    pub fn value(&self) -> Color {
        self.get_value(Self::value_property())
    }

    pub fn set_frame_value(&self, value: Color) {
        self.set_value(Self::value_property(), value);
    }
}

/// Holds the value before it until its key time, then jumps.
#[repr(C)]
pub struct DiscreteColorKeyFrame {
    base: ColorKeyFrame,
}

ferro_class!(DiscreteColorKeyFrame: ColorKeyFrame);
ferro_class_info!(DiscreteColorKeyFrame {
    new: DiscreteColorKeyFrame::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(DiscreteColorKeyFrame: FerroObjectImpl);

impl DiscreteColorKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: ColorKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a key frame of `value` at `key_time`.
    pub fn with(key_time: SignedDuration, value: Color) -> Ref<Self> {
        let frame = Self::new();
        frame.set_key_time(key_time);
        frame.set_frame_value(value);
        frame
    }
}

/// Interpolates linearly from the value before it.
#[repr(C)]
pub struct LinearColorKeyFrame {
    base: ColorKeyFrame,
}

ferro_class!(LinearColorKeyFrame: ColorKeyFrame);
ferro_class_info!(LinearColorKeyFrame { new: LinearColorKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(LinearColorKeyFrame: FerroObjectImpl);

impl LinearColorKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: ColorKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a key frame of `value` at `key_time`.
    pub fn with(key_time: SignedDuration, value: Color) -> Ref<Self> {
        let frame = Self::new();
        frame.set_key_time(key_time);
        frame.set_frame_value(value);
        frame
    }
}

/// Interpolates along a `KeySpline` from the value before it (WinUI's `SplineColorKeyFrame`).
#[repr(C)]
pub struct SplineColorKeyFrame {
    base: ColorKeyFrame,
}

ferro_class!(SplineColorKeyFrame: ColorKeyFrame);
ferro_class_info!(SplineColorKeyFrame { new: SplineColorKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(SplineColorKeyFrame: FerroObjectImpl);

ferro_properties! { impl SplineColorKeyFrame {
    /// Defines the `KeySpline` property: the Bezier curve of the progress. Default `None`
    /// (linear).
    pub fn key_spline_property() -> StyledProperty<Option<Ref<KeySpline>>> {
        FerroProperty::register::<SplineColorKeyFrame, _>("KeySpline", None)
    }
} }

impl SplineColorKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: ColorKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn key_spline(&self) -> Option<Ref<KeySpline>> {
        self.get_value(Self::key_spline_property())
    }

    pub fn set_key_spline(&self, value: Option<Ref<KeySpline>>) {
        self.set_value(Self::key_spline_property(), value);
    }
}

/// Interpolates with an easing function from the value before it (WinUI's `EasingColorKeyFrame`).
#[repr(C)]
pub struct EasingColorKeyFrame {
    base: ColorKeyFrame,
}

ferro_class!(EasingColorKeyFrame: ColorKeyFrame);
ferro_class_info!(EasingColorKeyFrame { new: EasingColorKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(EasingColorKeyFrame: FerroObjectImpl);

ferro_properties! { impl EasingColorKeyFrame {
    /// Defines the `EasingFunction` property. Default `None` (linear).
    pub fn easing_function_property() -> StyledProperty<Option<Ref<EasingFunctionBase>>> {
        FerroProperty::register::<EasingColorKeyFrame, _>("EasingFunction", None)
    }
} }

impl EasingColorKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: ColorKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn easing_function(&self) -> Option<Ref<EasingFunctionBase>> {
        self.get_value(Self::easing_function_property())
    }

    pub fn set_easing_function(&self, value: Option<Ref<EasingFunctionBase>>) {
        self.set_value(Self::easing_function_property(), value);
    }
}

/// Animates a `Color` property through its `KeyFrames` (WinUI's `ColorAnimationUsingKeyFrames`).
///
/// Before the first key frame the value goes from the value the property has when the
/// animation begins; after the last it holds the last value. With an automatic `Duration`
/// the animation lasts until its last key time. `EnableDependentAnimation` is accepted and
/// has no effect.
#[repr(C)]
pub struct ColorAnimationUsingKeyFrames {
    base: Timeline,
    key_frames: FerroList<Ref<ColorKeyFrame>>,
}

ferro_class!(ColorAnimationUsingKeyFrames: Timeline);
ferro_class_info!(ColorAnimationUsingKeyFrames {
    new: ColorAnimationUsingKeyFrames::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: KeyFrames,
        properties: [
            KeyFrames: FerroList<Ref<ColorKeyFrame>> { get: ColorAnimationUsingKeyFrames::key_frames },
        ],
    },
});
ferro_impl_classes!(ColorAnimationUsingKeyFrames: FerroObjectImpl);

ferro_properties! { impl ColorAnimationUsingKeyFrames {
    /// Defines the `EnableDependentAnimation` property (no effect). Default `false`.
    pub fn enable_dependent_animation_property() -> StyledProperty<bool> {
        FerroProperty::register::<ColorAnimationUsingKeyFrames, _>("EnableDependentAnimation", false)
    }
} }

impl ColorAnimationUsingKeyFrames {
    pub fn construct() -> Self {
        Self {
            base: Timeline::construct(),
            key_frames: FerroList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The key frames (in any order: they are sorted by their key times).
    pub fn key_frames(&self) -> FerroList<Ref<ColorKeyFrame>> {
        self.key_frames.clone()
    }

    pub fn enable_dependent_animation(&self) -> bool {
        self.get_value(Self::enable_dependent_animation_property())
    }

    pub fn set_enable_dependent_animation(&self, value: bool) {
        self.set_value(Self::enable_dependent_animation_property(), value);
    }
}

/// The base class of the key frames of a `Point` property (WinUI's `PointKeyFrame`): a `Value` reached at a
/// `KeyTime`.
#[repr(C)]
pub struct PointKeyFrame {
    base: FerroObject,
}

ferro_class!(PointKeyFrame: FerroObject);
ferro_class_info!(PointKeyFrame { markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(PointKeyFrame: FerroObjectImpl);

ferro_properties! { impl PointKeyFrame {
    /// Defines the `KeyTime` property: when the value is reached, from the start of the
    /// animation. Default zero.
    pub fn key_time_property() -> StyledProperty<SignedDuration> {
        FerroProperty::register::<PointKeyFrame, _>("KeyTime", SignedDuration::ZERO)
    }

    /// Defines the `Value` property: the value of the key frame.
    pub fn value_property() -> StyledProperty<Point> {
        FerroProperty::register::<PointKeyFrame, _>("Value", Point::default())
    }
} }

impl PointKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }

    pub fn key_time(&self) -> SignedDuration {
        self.get_value(Self::key_time_property())
    }

    pub fn set_key_time(&self, value: SignedDuration) {
        self.set_value(Self::key_time_property(), value);
    }

    pub fn value(&self) -> Point {
        self.get_value(Self::value_property())
    }

    pub fn set_frame_value(&self, value: Point) {
        self.set_value(Self::value_property(), value);
    }
}

/// Holds the value before it until its key time, then jumps.
#[repr(C)]
pub struct DiscretePointKeyFrame {
    base: PointKeyFrame,
}

ferro_class!(DiscretePointKeyFrame: PointKeyFrame);
ferro_class_info!(DiscretePointKeyFrame {
    new: DiscretePointKeyFrame::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(DiscretePointKeyFrame: FerroObjectImpl);

impl DiscretePointKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: PointKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a key frame of `value` at `key_time`.
    pub fn with(key_time: SignedDuration, value: Point) -> Ref<Self> {
        let frame = Self::new();
        frame.set_key_time(key_time);
        frame.set_frame_value(value);
        frame
    }
}

/// Interpolates linearly from the value before it.
#[repr(C)]
pub struct LinearPointKeyFrame {
    base: PointKeyFrame,
}

ferro_class!(LinearPointKeyFrame: PointKeyFrame);
ferro_class_info!(LinearPointKeyFrame { new: LinearPointKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(LinearPointKeyFrame: FerroObjectImpl);

impl LinearPointKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: PointKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a key frame of `value` at `key_time`.
    pub fn with(key_time: SignedDuration, value: Point) -> Ref<Self> {
        let frame = Self::new();
        frame.set_key_time(key_time);
        frame.set_frame_value(value);
        frame
    }
}

/// Interpolates along a `KeySpline` from the value before it (WinUI's `SplinePointKeyFrame`).
#[repr(C)]
pub struct SplinePointKeyFrame {
    base: PointKeyFrame,
}

ferro_class!(SplinePointKeyFrame: PointKeyFrame);
ferro_class_info!(SplinePointKeyFrame { new: SplinePointKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(SplinePointKeyFrame: FerroObjectImpl);

ferro_properties! { impl SplinePointKeyFrame {
    /// Defines the `KeySpline` property: the Bezier curve of the progress. Default `None`
    /// (linear).
    pub fn key_spline_property() -> StyledProperty<Option<Ref<KeySpline>>> {
        FerroProperty::register::<SplinePointKeyFrame, _>("KeySpline", None)
    }
} }

impl SplinePointKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: PointKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn key_spline(&self) -> Option<Ref<KeySpline>> {
        self.get_value(Self::key_spline_property())
    }

    pub fn set_key_spline(&self, value: Option<Ref<KeySpline>>) {
        self.set_value(Self::key_spline_property(), value);
    }
}

/// Interpolates with an easing function from the value before it (WinUI's `EasingPointKeyFrame`).
#[repr(C)]
pub struct EasingPointKeyFrame {
    base: PointKeyFrame,
}

ferro_class!(EasingPointKeyFrame: PointKeyFrame);
ferro_class_info!(EasingPointKeyFrame { new: EasingPointKeyFrame::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(EasingPointKeyFrame: FerroObjectImpl);

ferro_properties! { impl EasingPointKeyFrame {
    /// Defines the `EasingFunction` property. Default `None` (linear).
    pub fn easing_function_property() -> StyledProperty<Option<Ref<EasingFunctionBase>>> {
        FerroProperty::register::<EasingPointKeyFrame, _>("EasingFunction", None)
    }
} }

impl EasingPointKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: PointKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn easing_function(&self) -> Option<Ref<EasingFunctionBase>> {
        self.get_value(Self::easing_function_property())
    }

    pub fn set_easing_function(&self, value: Option<Ref<EasingFunctionBase>>) {
        self.set_value(Self::easing_function_property(), value);
    }
}

/// Animates a `Point` property through its `KeyFrames` (WinUI's `PointAnimationUsingKeyFrames`).
///
/// Before the first key frame the value goes from the value the property has when the
/// animation begins; after the last it holds the last value. With an automatic `Duration`
/// the animation lasts until its last key time. `EnableDependentAnimation` is accepted and
/// has no effect.
#[repr(C)]
pub struct PointAnimationUsingKeyFrames {
    base: Timeline,
    key_frames: FerroList<Ref<PointKeyFrame>>,
}

ferro_class!(PointAnimationUsingKeyFrames: Timeline);
ferro_class_info!(PointAnimationUsingKeyFrames {
    new: PointAnimationUsingKeyFrames::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: KeyFrames,
        properties: [
            KeyFrames: FerroList<Ref<PointKeyFrame>> { get: PointAnimationUsingKeyFrames::key_frames },
        ],
    },
});
ferro_impl_classes!(PointAnimationUsingKeyFrames: FerroObjectImpl);

ferro_properties! { impl PointAnimationUsingKeyFrames {
    /// Defines the `EnableDependentAnimation` property (no effect). Default `false`.
    pub fn enable_dependent_animation_property() -> StyledProperty<bool> {
        FerroProperty::register::<PointAnimationUsingKeyFrames, _>("EnableDependentAnimation", false)
    }
} }

impl PointAnimationUsingKeyFrames {
    pub fn construct() -> Self {
        Self {
            base: Timeline::construct(),
            key_frames: FerroList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The key frames (in any order: they are sorted by their key times).
    pub fn key_frames(&self) -> FerroList<Ref<PointKeyFrame>> {
        self.key_frames.clone()
    }

    pub fn enable_dependent_animation(&self) -> bool {
        self.get_value(Self::enable_dependent_animation_property())
    }

    pub fn set_enable_dependent_animation(&self, value: bool) {
        self.set_value(Self::enable_dependent_animation_property(), value);
    }
}

/// The base class of the key frames of an object animation (WinUI's `ObjectKeyFrame`): a
/// `Value` of any type, set at a `KeyTime`.
///
/// A value written as text in markup (`Value="False"`) is converted to the type of the animated
/// property when the animation begins.
#[repr(C)]
pub struct ObjectKeyFrame {
    base: FerroObject,
}

ferro_class!(ObjectKeyFrame: FerroObject);
ferro_class_info!(ObjectKeyFrame { markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(ObjectKeyFrame: FerroObjectImpl);

ferro_properties! { impl ObjectKeyFrame {
    /// Defines the `KeyTime` property: when the value is set, from the start of the animation.
    /// Default zero.
    pub fn key_time_property() -> StyledProperty<SignedDuration> {
        FerroProperty::register::<ObjectKeyFrame, _>("KeyTime", SignedDuration::ZERO)
    }

    /// Defines the `Value` property. Default `None` (null).
    pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<ObjectKeyFrame, _>("Value", None)
    }
} }

impl ObjectKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }

    pub fn key_time(&self) -> SignedDuration {
        self.get_value(Self::key_time_property())
    }

    pub fn set_key_time(&self, value: SignedDuration) {
        self.set_value(Self::key_time_property(), value);
    }

    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    pub fn set_frame_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value);
    }
}

/// Sets its value at its key time (WinUI's `DiscreteObjectKeyFrame`).
#[repr(C)]
pub struct DiscreteObjectKeyFrame {
    base: ObjectKeyFrame,
}

ferro_class!(DiscreteObjectKeyFrame: ObjectKeyFrame);
ferro_class_info!(DiscreteObjectKeyFrame {
    new: DiscreteObjectKeyFrame::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(DiscreteObjectKeyFrame: FerroObjectImpl);

impl DiscreteObjectKeyFrame {
    pub fn construct() -> Self {
        Self {
            base: ObjectKeyFrame::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a key frame of `value` at `key_time`.
    pub fn with(key_time: SignedDuration, value: Option<BoxedValue>) -> Ref<Self> {
        let frame = Self::new();
        frame.set_key_time(key_time);
        frame.set_frame_value(value);
        frame
    }
}

/// Sets a property of any type to the values of its `KeyFrames` (WinUI's
/// `ObjectAnimationUsingKeyFrames`; every key frame is discrete).
#[repr(C)]
pub struct ObjectAnimationUsingKeyFrames {
    base: Timeline,
    key_frames: FerroList<Ref<ObjectKeyFrame>>,
}

ferro_class!(ObjectAnimationUsingKeyFrames: Timeline);
ferro_class_info!(ObjectAnimationUsingKeyFrames {
    new: ObjectAnimationUsingKeyFrames::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: KeyFrames,
        properties: [
            KeyFrames: FerroList<Ref<ObjectKeyFrame>> { get: ObjectAnimationUsingKeyFrames::key_frames },
        ],
    },
});
ferro_impl_classes!(ObjectAnimationUsingKeyFrames: FerroObjectImpl);

impl ObjectAnimationUsingKeyFrames {
    pub fn construct() -> Self {
        Self {
            base: Timeline::construct(),
            key_frames: FerroList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The key frames (in any order: they are sorted by their key times).
    pub fn key_frames(&self) -> FerroList<Ref<ObjectKeyFrame>> {
        self.key_frames.clone()
    }
}
