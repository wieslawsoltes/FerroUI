//! WinUI's animations from, to or by a value: `DoubleAnimation`, `ColorAnimation` and
//! `PointAnimation` (`Microsoft.UI.Xaml.Media.Animation`; `dxaml/xcp/core/animation/animation.cpp`
//! of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{EasingFunctionBase, Timeline};
use ferroui_base::media::Color;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    Point, Ref, StyledProperty,
};

/// Animates a `f64` property (WinUI's `DoubleAnimation`).
///
/// It animates from `From` (or the value the property has when it begins) to `To`, or by
/// `By` from the start; with neither `To` nor `By` it animates back to the value of the
/// property without animations. `EasingFunction` eases the progress.
/// `EnableDependentAnimation` is accepted and has no effect: every property is animated on
/// the UI thread.
#[repr(C)]
pub struct DoubleAnimation {
    base: Timeline,
}

ferro_class!(DoubleAnimation: Timeline);
ferro_class_info!(DoubleAnimation { new: DoubleAnimation::new, markup: { namespace: "FerroUI.Animation" } });
ferro_impl_classes!(DoubleAnimation: FerroObjectImpl);

ferro_properties! { impl DoubleAnimation {
    /// Defines the `From` property: the start value. Default `None` (the current value).
    pub fn from_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register::<DoubleAnimation, _>("From", None)
    }

    /// Defines the `To` property: the end value. Default `None`.
    pub fn to_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register::<DoubleAnimation, _>("To", None)
    }

    /// Defines the `By` property: the change from the start value, when `To` is not set.
    /// Default `None`.
    pub fn by_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register::<DoubleAnimation, _>("By", None)
    }

    /// Defines the `EasingFunction` property. Default `None` (linear).
    pub fn easing_function_property() -> StyledProperty<Option<Ref<EasingFunctionBase>>> {
        FerroProperty::register::<DoubleAnimation, _>("EasingFunction", None)
    }

    /// Defines the `EnableDependentAnimation` property (no effect). Default `false`.
    pub fn enable_dependent_animation_property() -> StyledProperty<bool> {
        FerroProperty::register::<DoubleAnimation, _>("EnableDependentAnimation", false)
    }
} }

impl DoubleAnimation {
    pub fn construct() -> Self {
        Self {
            base: Timeline::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn from(&self) -> Option<f64> {
        self.get_value(Self::from_property())
    }

    pub fn set_from(&self, value: Option<f64>) {
        self.set_value(Self::from_property(), value);
    }

    pub fn to(&self) -> Option<f64> {
        self.get_value(Self::to_property())
    }

    pub fn set_to(&self, value: Option<f64>) {
        self.set_value(Self::to_property(), value);
    }

    pub fn by(&self) -> Option<f64> {
        self.get_value(Self::by_property())
    }

    pub fn set_by(&self, value: Option<f64>) {
        self.set_value(Self::by_property(), value);
    }

    pub fn easing_function(&self) -> Option<Ref<EasingFunctionBase>> {
        self.get_value(Self::easing_function_property())
    }

    pub fn set_easing_function(&self, value: Option<Ref<EasingFunctionBase>>) {
        self.set_value(Self::easing_function_property(), value);
    }

    pub fn enable_dependent_animation(&self) -> bool {
        self.get_value(Self::enable_dependent_animation_property())
    }

    pub fn set_enable_dependent_animation(&self, value: bool) {
        self.set_value(Self::enable_dependent_animation_property(), value);
    }
}

/// Animates a [`Color`] property (WinUI's `ColorAnimation`), channel by channel.
///
/// It animates from `From` (or the value the property has when it begins) to `To`, or by
/// `By` from the start; with neither `To` nor `By` it animates back to the value of the
/// property without animations. `EasingFunction` eases the progress.
/// `EnableDependentAnimation` is accepted and has no effect: every property is animated on
/// the UI thread.
#[repr(C)]
pub struct ColorAnimation {
    base: Timeline,
}

ferro_class!(ColorAnimation: Timeline);
ferro_class_info!(ColorAnimation { new: ColorAnimation::new, markup: { namespace: "FerroUI.Animation" } });
ferro_impl_classes!(ColorAnimation: FerroObjectImpl);

ferro_properties! { impl ColorAnimation {
    /// Defines the `From` property: the start value. Default `None` (the current value).
    pub fn from_property() -> StyledProperty<Option<Color>> {
        FerroProperty::register::<ColorAnimation, _>("From", None)
    }

    /// Defines the `To` property: the end value. Default `None`.
    pub fn to_property() -> StyledProperty<Option<Color>> {
        FerroProperty::register::<ColorAnimation, _>("To", None)
    }

    /// Defines the `By` property: the change from the start value, when `To` is not set.
    /// Default `None`.
    pub fn by_property() -> StyledProperty<Option<Color>> {
        FerroProperty::register::<ColorAnimation, _>("By", None)
    }

    /// Defines the `EasingFunction` property. Default `None` (linear).
    pub fn easing_function_property() -> StyledProperty<Option<Ref<EasingFunctionBase>>> {
        FerroProperty::register::<ColorAnimation, _>("EasingFunction", None)
    }

    /// Defines the `EnableDependentAnimation` property (no effect). Default `false`.
    pub fn enable_dependent_animation_property() -> StyledProperty<bool> {
        FerroProperty::register::<ColorAnimation, _>("EnableDependentAnimation", false)
    }
} }

impl ColorAnimation {
    pub fn construct() -> Self {
        Self {
            base: Timeline::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn from(&self) -> Option<Color> {
        self.get_value(Self::from_property())
    }

    pub fn set_from(&self, value: Option<Color>) {
        self.set_value(Self::from_property(), value);
    }

    pub fn to(&self) -> Option<Color> {
        self.get_value(Self::to_property())
    }

    pub fn set_to(&self, value: Option<Color>) {
        self.set_value(Self::to_property(), value);
    }

    pub fn by(&self) -> Option<Color> {
        self.get_value(Self::by_property())
    }

    pub fn set_by(&self, value: Option<Color>) {
        self.set_value(Self::by_property(), value);
    }

    pub fn easing_function(&self) -> Option<Ref<EasingFunctionBase>> {
        self.get_value(Self::easing_function_property())
    }

    pub fn set_easing_function(&self, value: Option<Ref<EasingFunctionBase>>) {
        self.set_value(Self::easing_function_property(), value);
    }

    pub fn enable_dependent_animation(&self) -> bool {
        self.get_value(Self::enable_dependent_animation_property())
    }

    pub fn set_enable_dependent_animation(&self, value: bool) {
        self.set_value(Self::enable_dependent_animation_property(), value);
    }
}

/// Animates a [`Point`] property (WinUI's `PointAnimation`).
///
/// It animates from `From` (or the value the property has when it begins) to `To`, or by
/// `By` from the start; with neither `To` nor `By` it animates back to the value of the
/// property without animations. `EasingFunction` eases the progress.
/// `EnableDependentAnimation` is accepted and has no effect: every property is animated on
/// the UI thread.
#[repr(C)]
pub struct PointAnimation {
    base: Timeline,
}

ferro_class!(PointAnimation: Timeline);
ferro_class_info!(PointAnimation { new: PointAnimation::new, markup: { namespace: "FerroUI.Animation" } });
ferro_impl_classes!(PointAnimation: FerroObjectImpl);

ferro_properties! { impl PointAnimation {
    /// Defines the `From` property: the start value. Default `None` (the current value).
    pub fn from_property() -> StyledProperty<Option<Point>> {
        FerroProperty::register::<PointAnimation, _>("From", None)
    }

    /// Defines the `To` property: the end value. Default `None`.
    pub fn to_property() -> StyledProperty<Option<Point>> {
        FerroProperty::register::<PointAnimation, _>("To", None)
    }

    /// Defines the `By` property: the change from the start value, when `To` is not set.
    /// Default `None`.
    pub fn by_property() -> StyledProperty<Option<Point>> {
        FerroProperty::register::<PointAnimation, _>("By", None)
    }

    /// Defines the `EasingFunction` property. Default `None` (linear).
    pub fn easing_function_property() -> StyledProperty<Option<Ref<EasingFunctionBase>>> {
        FerroProperty::register::<PointAnimation, _>("EasingFunction", None)
    }

    /// Defines the `EnableDependentAnimation` property (no effect). Default `false`.
    pub fn enable_dependent_animation_property() -> StyledProperty<bool> {
        FerroProperty::register::<PointAnimation, _>("EnableDependentAnimation", false)
    }
} }

impl PointAnimation {
    pub fn construct() -> Self {
        Self {
            base: Timeline::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn from(&self) -> Option<Point> {
        self.get_value(Self::from_property())
    }

    pub fn set_from(&self, value: Option<Point>) {
        self.set_value(Self::from_property(), value);
    }

    pub fn to(&self) -> Option<Point> {
        self.get_value(Self::to_property())
    }

    pub fn set_to(&self, value: Option<Point>) {
        self.set_value(Self::to_property(), value);
    }

    pub fn by(&self) -> Option<Point> {
        self.get_value(Self::by_property())
    }

    pub fn set_by(&self, value: Option<Point>) {
        self.set_value(Self::by_property(), value);
    }

    pub fn easing_function(&self) -> Option<Ref<EasingFunctionBase>> {
        self.get_value(Self::easing_function_property())
    }

    pub fn set_easing_function(&self, value: Option<Ref<EasingFunctionBase>>) {
        self.set_value(Self::easing_function_property(), value);
    }

    pub fn enable_dependent_animation(&self) -> bool {
        self.get_value(Self::enable_dependent_animation_property())
    }

    pub fn set_enable_dependent_animation(&self, value: bool) {
        self.set_value(Self::enable_dependent_animation_property(), value);
    }
}
