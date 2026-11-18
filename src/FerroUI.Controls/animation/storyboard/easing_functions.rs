//! WinUI's easing functions (`Microsoft.UI.Xaml.Media.Animation.EasingFunctionBase` and its eleven
//! classes; `dxaml/xcp/components/animation/EasingFunctions.cpp` and
//! `dxaml/xcp/core/inc/EasingFunctions.h` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! Each function is the `EaseInCore` of the source; [`EasingMode`] turns it into the ease-out and
//! ease-in-out curves as `CEasingFunctionImpl::Ease` does. The arithmetic is in `f64` (the source
//! computes in `f32`).

use ferroui_base::animation::easings::{Easing, IEasing, SharedEasing};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use std::f64::consts::{E, PI};
use std::sync::Arc;

const EPSILON: f64 = 0.0001;

/// Which end of an easing function is eased (WinUI's `EasingMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum EasingMode {
    /// The curve of the function, mirrored: fast at the start, slow at the end (WinUI's default).
    #[default]
    EaseOut = 0,
    /// The curve of the function: slow at the start.
    EaseIn = 1,
    /// Ease in for the first half, ease out for the second.
    EaseInOut = 2,
}

ferroui_base::ferro_markup_enum!(EasingMode { EaseOut, EaseIn, EaseInOut }, { namespace: "FerroUI.Controls" });

/// The curve of one of the easing functions, with its parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Curve {
    Back { amplitude: f64 },
    Bounce { bounces: i32, bounciness: f64 },
    Circle,
    Power(f64),
    Elastic { oscillations: i32, springiness: f64 },
    Exponential { exponent: f64 },
    Sine,
}

impl Curve {
    /// WinUI's `EaseInCore` of each function.
    fn ease_in(&self, t: f64) -> f64 {
        match *self {
            Curve::Back { amplitude } => {
                let t = t.max(0.0);
                t * t * t - t * amplitude * (t * PI).sin()
            }
            Curve::Bounce { bounces, bounciness } => bounce(t, bounces, bounciness),
            Curve::Circle => {
                let t = t.clamp(-1.0, 1.0);
                -((1.0 - t * t).sqrt() - 1.0)
            }
            Curve::Power(power) => t.powf(power.max(0.0)),
            Curve::Elastic {
                oscillations,
                springiness,
            } => {
                let oscillations = oscillations.max(0) as f64;
                let modifier = if springiness.abs() <= EPSILON {
                    t
                } else {
                    ((E.powf(springiness * t)) - 1.0) / (E.powf(springiness) - 1.0)
                };
                modifier * (t * (2.0 * PI * oscillations + PI / 2.0)).sin()
            }
            Curve::Exponential { exponent } => {
                if exponent.abs() <= EPSILON {
                    t
                } else {
                    (E.powf(exponent * t) - 1.0) / (E.powf(exponent) - 1.0)
                }
            }
            Curve::Sine => 1.0 - ((1.0 - t) * (PI / 2.0)).sin(),
        }
    }
}

/// WinUI's `CBounceInterpolator::EaseInCore`.
fn bounce(t: f64, bounces: i32, bounciness: f64) -> f64 {
    let bounces = bounces.max(0) as f64;
    let bounciness = if bounciness <= 1.0 + EPSILON { 1.01 } else { bounciness };
    let pow = bounciness.powf(bounces);
    let one_minus_bounciness = 1.0 - bounciness;
    let sum_of_units = (1.0 - pow) / one_minus_bounciness + pow / 2.0;
    let unit_at_t = t * sum_of_units;
    let bounce_at_t = (-unit_at_t * (1.0 - bounciness) + 1.0).ln() / bounciness.ln();
    let start = bounce_at_t.floor();
    let end = start + 1.0;
    let start_time = (1.0 - bounciness.powf(start)) / (one_minus_bounciness * sum_of_units);
    let end_time = (1.0 - bounciness.powf(end)) / (one_minus_bounciness * sum_of_units);
    let mid_time = (start_time + end_time) / 2.0;
    let relative_to_peak = t - mid_time;
    let radius = mid_time - start_time;
    let amplitude = (1.0 / bounciness).powf(bounces - start);
    (-amplitude / (radius * radius)) * (relative_to_peak - radius) * (relative_to_peak + radius)
}

/// An easing function with its mode, as a value: what [`EasingFunctionBase::to_easing`] hands to
/// the transitions and animations of the framework.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EasingCurve {
    curve: Curve,
    mode: EasingMode,
}

impl EasingCurve {
    /// WinUI's `CEasingFunctionImpl::Ease`.
    pub(crate) fn ease(&self, t: f64) -> f64 {
        match self.mode {
            EasingMode::EaseIn => self.curve.ease_in(t),
            EasingMode::EaseOut => 1.0 - self.curve.ease_in(1.0 - t),
            EasingMode::EaseInOut => {
                if t < 0.5 {
                    self.curve.ease_in(t * 2.0) / 2.0
                } else {
                    (1.0 - self.curve.ease_in(2.0 - t * 2.0)) / 2.0 + 0.5
                }
            }
        }
    }
}

impl IEasing for EasingCurve {
    fn ease(&self, progress: f64) -> f64 {
        EasingCurve::ease(self, progress)
    }

    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(*self)
    }
}

/// The base class of WinUI's easing functions: a curve that maps the linear progress of an
/// animation, with its [`easing_mode`](Self::easing_mode).
///
/// An easing function of a storyboard animation; [`to_easing`](Self::to_easing) also gives it to
/// the transitions and key frame animations of the framework.
#[repr(C)]
pub struct EasingFunctionBase {
    base: FerroObject,
}

ferro_class!(EasingFunctionBase: FerroObject);
ferro_class_info!(EasingFunctionBase { markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(EasingFunctionBase: FerroObjectImpl);

ferro_properties! { impl EasingFunctionBase {
    /// Defines the `EasingMode` property. Default [`EasingMode::EaseOut`], as WinUI.
    pub fn easing_mode_property() -> StyledProperty<EasingMode> {
        FerroProperty::register::<EasingFunctionBase, _>("EasingMode", EasingMode::EaseOut)
    }
} }

impl EasingFunctionBase {
    /// Creates the class data of a derived function.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }

    pub fn easing_mode(&self) -> EasingMode {
        self.get_value(Self::easing_mode_property())
    }

    pub fn set_easing_mode(&self, value: EasingMode) {
        self.set_value(Self::easing_mode_property(), value);
    }

    /// The eased progress for the linear progress `normalized_time` (WinUI's `Ease`).
    pub fn ease(&self, normalized_time: f64) -> f64 {
        self.curve()
            .map_or(normalized_time, |curve| curve.ease(normalized_time))
    }

    /// The function as an easing of the framework, with the parameters it has now.
    pub fn to_easing(&self) -> Easing {
        match self.curve() {
            Some(curve) => Easing::new(curve),
            None => Easing::new(ferroui_base::animation::easings::LinearEasing::new()),
        }
    }

    /// The curve and the mode of the function; `None` for a class that is not one of WinUI's.
    pub(crate) fn curve(&self) -> Option<EasingCurve> {
        let curve = if let Some(back) = self.downcast_ref::<BackEase>() {
            Curve::Back {
                amplitude: back.amplitude(),
            }
        } else if let Some(bounce) = self.downcast_ref::<BounceEase>() {
            Curve::Bounce {
                bounces: bounce.bounces(),
                bounciness: bounce.bounciness(),
            }
        } else if self.downcast_ref::<CircleEase>().is_some() {
            Curve::Circle
        } else if self.downcast_ref::<CubicEase>().is_some() {
            Curve::Power(3.0)
        } else if let Some(elastic) = self.downcast_ref::<ElasticEase>() {
            Curve::Elastic {
                oscillations: elastic.oscillations(),
                springiness: elastic.springiness(),
            }
        } else if let Some(exponential) = self.downcast_ref::<ExponentialEase>() {
            Curve::Exponential {
                exponent: exponential.exponent(),
            }
        } else if let Some(power) = self.downcast_ref::<PowerEase>() {
            Curve::Power(power.power())
        } else if self.downcast_ref::<QuadraticEase>().is_some() {
            Curve::Power(2.0)
        } else if self.downcast_ref::<QuarticEase>().is_some() {
            Curve::Power(4.0)
        } else if self.downcast_ref::<QuinticEase>().is_some() {
            Curve::Power(5.0)
        } else if self.downcast_ref::<SineEase>().is_some() {
            Curve::Sine
        } else {
            return None;
        };
        Some(EasingCurve {
            curve,
            mode: self.easing_mode(),
        })
    }
}

/// A circular curve (WinUI's `CircleEase`).
#[repr(C)]
pub struct CircleEase {
    base: EasingFunctionBase,
}

ferro_class!(CircleEase: EasingFunctionBase);
ferro_class_info!(CircleEase { new: CircleEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(CircleEase: FerroObjectImpl);

impl CircleEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }
}

/// The curve `t^3` (WinUI's `CubicEase`).
#[repr(C)]
pub struct CubicEase {
    base: EasingFunctionBase,
}

ferro_class!(CubicEase: EasingFunctionBase);
ferro_class_info!(CubicEase { new: CubicEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(CubicEase: FerroObjectImpl);

impl CubicEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }
}

/// The curve `t^2` (WinUI's `QuadraticEase`).
#[repr(C)]
pub struct QuadraticEase {
    base: EasingFunctionBase,
}

ferro_class!(QuadraticEase: EasingFunctionBase);
ferro_class_info!(QuadraticEase { new: QuadraticEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(QuadraticEase: FerroObjectImpl);

impl QuadraticEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }
}

/// The curve `t^4` (WinUI's `QuarticEase`).
#[repr(C)]
pub struct QuarticEase {
    base: EasingFunctionBase,
}

ferro_class!(QuarticEase: EasingFunctionBase);
ferro_class_info!(QuarticEase { new: QuarticEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(QuarticEase: FerroObjectImpl);

impl QuarticEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }
}

/// The curve `t^5` (WinUI's `QuinticEase`).
#[repr(C)]
pub struct QuinticEase {
    base: EasingFunctionBase,
}

ferro_class!(QuinticEase: EasingFunctionBase);
ferro_class_info!(QuinticEase { new: QuinticEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(QuinticEase: FerroObjectImpl);

impl QuinticEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }
}

/// A sine curve (WinUI's `SineEase`).
#[repr(C)]
pub struct SineEase {
    base: EasingFunctionBase,
}

ferro_class!(SineEase: EasingFunctionBase);
ferro_class_info!(SineEase { new: SineEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(SineEase: FerroObjectImpl);

impl SineEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }
}

/// Pulls back before it moves (WinUI's `BackEase`).
#[repr(C)]
pub struct BackEase {
    base: EasingFunctionBase,
}

ferro_class!(BackEase: EasingFunctionBase);
ferro_class_info!(BackEase { new: BackEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(BackEase: FerroObjectImpl);

ferro_properties! { impl BackEase {
    /// Defines the `Amplitude` property: how far it pulls back. Default 1.
    pub fn amplitude_property() -> StyledProperty<f64> {
        FerroProperty::register::<BackEase, _>("Amplitude", 1.0)
    }
} }

impl BackEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }

    pub fn amplitude(&self) -> f64 {
        self.get_value(Self::amplitude_property())
    }

    pub fn set_amplitude(&self, value: f64) {
        self.set_value(Self::amplitude_property(), value);
    }
}

/// Bounces (WinUI's `BounceEase`).
#[repr(C)]
pub struct BounceEase {
    base: EasingFunctionBase,
}

ferro_class!(BounceEase: EasingFunctionBase);
ferro_class_info!(BounceEase { new: BounceEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(BounceEase: FerroObjectImpl);

ferro_properties! { impl BounceEase {
    /// Defines the `Bounces` property: the number of bounces. Default 3.
    pub fn bounces_property() -> StyledProperty<i32> {
        FerroProperty::register::<BounceEase, _>("Bounces", 3)
    }

    /// Defines the `Bounciness` property: how much each bounce shrinks (values up to 1 are 1.01).
    /// Default 2.
    pub fn bounciness_property() -> StyledProperty<f64> {
        FerroProperty::register::<BounceEase, _>("Bounciness", 2.0)
    }
} }

impl BounceEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }

    pub fn bounces(&self) -> i32 {
        self.get_value(Self::bounces_property())
    }

    pub fn set_bounces(&self, value: i32) {
        self.set_value(Self::bounces_property(), value);
    }

    pub fn bounciness(&self) -> f64 {
        self.get_value(Self::bounciness_property())
    }

    pub fn set_bounciness(&self, value: f64) {
        self.set_value(Self::bounciness_property(), value);
    }
}

/// Oscillates like a spring (WinUI's `ElasticEase`).
#[repr(C)]
pub struct ElasticEase {
    base: EasingFunctionBase,
}

ferro_class!(ElasticEase: EasingFunctionBase);
ferro_class_info!(ElasticEase { new: ElasticEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(ElasticEase: FerroObjectImpl);

ferro_properties! { impl ElasticEase {
    /// Defines the `Oscillations` property: the number of oscillations. Default 3.
    pub fn oscillations_property() -> StyledProperty<i32> {
        FerroProperty::register::<ElasticEase, _>("Oscillations", 3)
    }

    /// Defines the `Springiness` property: the stiffness of the spring. Default 3.
    pub fn springiness_property() -> StyledProperty<f64> {
        FerroProperty::register::<ElasticEase, _>("Springiness", 3.0)
    }
} }

impl ElasticEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }

    pub fn oscillations(&self) -> i32 {
        self.get_value(Self::oscillations_property())
    }

    pub fn set_oscillations(&self, value: i32) {
        self.set_value(Self::oscillations_property(), value);
    }

    pub fn springiness(&self) -> f64 {
        self.get_value(Self::springiness_property())
    }

    pub fn set_springiness(&self, value: f64) {
        self.set_value(Self::springiness_property(), value);
    }
}

/// An exponential curve (WinUI's `ExponentialEase`).
#[repr(C)]
pub struct ExponentialEase {
    base: EasingFunctionBase,
}

ferro_class!(ExponentialEase: EasingFunctionBase);
ferro_class_info!(ExponentialEase { new: ExponentialEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(ExponentialEase: FerroObjectImpl);

ferro_properties! { impl ExponentialEase {
    /// Defines the `Exponent` property. Default 2.
    pub fn exponent_property() -> StyledProperty<f64> {
        FerroProperty::register::<ExponentialEase, _>("Exponent", 2.0)
    }
} }

impl ExponentialEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }

    pub fn exponent(&self) -> f64 {
        self.get_value(Self::exponent_property())
    }

    pub fn set_exponent(&self, value: f64) {
        self.set_value(Self::exponent_property(), value);
    }
}

/// The curve `t^Power` (WinUI's `PowerEase`).
#[repr(C)]
pub struct PowerEase {
    base: EasingFunctionBase,
}

ferro_class!(PowerEase: EasingFunctionBase);
ferro_class_info!(PowerEase { new: PowerEase::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(PowerEase: FerroObjectImpl);

ferro_properties! { impl PowerEase {
    /// Defines the `Power` property (a negative power is 0). Default 2.
    pub fn power_property() -> StyledProperty<f64> {
        FerroProperty::register::<PowerEase, _>("Power", 2.0)
    }
} }

impl PowerEase {
    pub fn construct() -> Self {
        Self {
            base: EasingFunctionBase::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the function with a mode.
    pub fn with_mode(mode: EasingMode) -> Ref<Self> {
        let easing = Self::new();
        easing.set_easing_mode(mode);
        easing
    }

    pub fn power(&self) -> f64 {
        self.get_value(Self::power_property())
    }

    pub fn set_power(&self, value: f64) {
        self.set_value(Self::power_property(), value);
    }
}
