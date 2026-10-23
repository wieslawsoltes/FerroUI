//! The easing functions of WinUI composition (`CompositionEasingFunction`
//! and its subclasses, Windows App SDK 1.4+), as [`IEasing`] implementations
//! that key frames take.
//!
//! The parametric curves (back, bounce, circle, elastic, exponential, power,
//! sine) are those of the easing functions of WPF and XAML
//! (`EasingFunctionBase`): each is defined by its ease-in curve `f(t)`, and
//! the mode makes the curve
//!
//! * `In`: `f(t)`;
//! * `Out`: `1 - f(1 - t)`;
//! * `InOut`: `f(2t) / 2` for `t < 0.5`, `(1 - f(2 - 2t)) / 2 + 0.5` after.
//!
//! The parameters of these functions are read-only, as in WinUI; the types
//! are plain values that are `Send + Sync`, so the render thread evaluates
//! the same object. The step function has settable members, as in WinUI, and
//! shares a copy of them.

use crate::animation::easings::{Easing, IEasing, SharedEasing};
use crate::animation::KeySplineSolver;
use crate::numerics::Vector2;
use crate::rendering::composition::Compositor;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// The mode of a parametric easing function (WinUI
/// `CompositionEasingFunctionMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CompositionEasingFunctionMode {
    /// The curve as defined (ease in).
    #[default]
    In,
    /// The curve mirrored in time and value (ease out).
    Out,
    /// Ease in for the first half, ease out for the second half.
    InOut,
}

/// Applies `mode` to the ease-in curve `ease_in` (`EasingFunctionBase.Ease`).
fn ease_with_mode(mode: CompositionEasingFunctionMode, t: f64, ease_in: impl Fn(f64) -> f64) -> f64 {
    match mode {
        CompositionEasingFunctionMode::In => ease_in(t),
        CompositionEasingFunctionMode::Out => 1.0 - ease_in(1.0 - t),
        CompositionEasingFunctionMode::InOut => {
            if t < 0.5 {
                ease_in(t * 2.0) * 0.5
            } else {
                (1.0 - ease_in((1.0 - t) * 2.0)) * 0.5 + 0.5
            }
        }
    }
}

/// Implements [`IEasing`] for an easing that is a plain `Send + Sync`
/// value, and the conversions into [`Easing`].
macro_rules! value_easing {
    ($ty:ty) => {
        impl IEasing for $ty {
            fn to_shared(&self) -> Arc<SharedEasing> {
                Arc::new(*self)
            }

            fn ease(&self, progress: f64) -> f64 {
                self.ease_core(progress)
            }
        }

        easing_conversions!($ty);
    };
}

/// The conversions of an easing function into the handle key frames take.
macro_rules! easing_conversions {
    ($ty:ty) => {
        impl From<$ty> for Easing {
            fn from(value: $ty) -> Easing {
                Easing::new(value)
            }
        }

        impl From<Rc<$ty>> for Easing {
            fn from(value: Rc<$ty>) -> Easing {
                Easing::from_rc(value)
            }
        }
    };
}

/// A cubic Bezier curve from `(0, 0)` to `(1, 1)` with two control points
/// (WinUI `CubicBezierEasingFunction`); the curve of
/// [`SplineEasing`](crate::animation::easings::SplineEasing).
pub struct CubicBezierEasingFunction {
    control_point1: Vector2,
    control_point2: Vector2,
    solver: Mutex<KeySplineSolver>,
}

impl CubicBezierEasingFunction {
    /// Creates the curve. Panics when the X coordinate of a control point is
    /// outside `[0, 1]` (WinUI throws).
    pub fn new(control_point1: Vector2, control_point2: Vector2) -> CubicBezierEasingFunction {
        for x in [control_point1.x, control_point2.x] {
            if !(0.0..=1.0).contains(&x) {
                panic!("The X coordinate of a control point must be between 0 and 1");
            }
        }
        CubicBezierEasingFunction {
            control_point1,
            control_point2,
            solver: Mutex::new(KeySplineSolver::new(
                control_point1.x as f64,
                control_point1.y as f64,
                control_point2.x as f64,
                control_point2.y as f64,
            )),
        }
    }

    /// The first control point.
    pub fn control_point1(&self) -> Vector2 {
        self.control_point1
    }

    /// The second control point.
    pub fn control_point2(&self) -> Vector2 {
        self.control_point2
    }
}

impl IEasing for CubicBezierEasingFunction {
    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(CubicBezierEasingFunction::new(self.control_point1, self.control_point2))
    }

    fn ease(&self, progress: f64) -> f64 {
        self.solver.lock().unwrap_or_else(|e| e.into_inner()).get_spline_progress(progress)
    }
}

easing_conversions!(CubicBezierEasingFunction);

/// The identity curve (WinUI `LinearEasingFunction`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinearEasingFunction;

impl LinearEasingFunction {
    pub fn new() -> LinearEasingFunction {
        LinearEasingFunction
    }

    fn ease_core(&self, progress: f64) -> f64 {
        progress
    }
}

value_easing!(LinearEasingFunction);

/// The members of a step function, as the render thread evaluates them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StepParameters {
    step_count: i32,
    initial_step: i32,
    final_step: i32,
    is_initial_step_single_frame: bool,
    is_final_step_single_frame: bool,
}

impl StepParameters {
    /// The output is `step / StepCount`, the step running from
    /// `InitialStep` to `FinalStep`. The steps share the duration equally;
    /// a step that is "single frame" is shown only at the very start
    /// (`t = 0`, the initial step) or at the very end (`t = 1`, the final
    /// step) and gets no share.
    // Deviation (winui-composition.md): an easing function does not know the
    // frames of the animation, so a single-frame step lasts no time rather
    // than one frame; the division of the duration between the steps is
    // this port's reading of the WinUI members.
    fn ease_core(&self, t: f64) -> f64 {
        let count = self.step_count.max(1);
        let initial = self.initial_step.clamp(0, count);
        let final_step = self.final_step.clamp(initial, count);
        let step = if t <= 0.0 {
            initial
        } else if t >= 1.0 {
            final_step
        } else {
            let first = initial + i32::from(self.is_initial_step_single_frame);
            let last = final_step - i32::from(self.is_final_step_single_frame);
            if last < first {
                first.min(final_step)
            } else {
                let steps = (last - first + 1) as f64;
                (first + (t * steps).floor() as i32).min(last)
            }
        };
        step as f64 / count as f64
    }
}

impl IEasing for StepParameters {
    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(*self)
    }

    fn ease(&self, progress: f64) -> f64 {
        self.ease_core(progress)
    }
}

/// A function that advances in steps (WinUI `StepEasingFunction`). The
/// output is `step / step_count` for a step between
/// [`initial_step`](Self::initial_step) and [`final_step`](Self::final_step).
pub struct StepEasingFunction {
    step_count: Cell<i32>,
    initial_step: Cell<i32>,
    /// `None`: the step count.
    final_step: Cell<Option<i32>>,
    is_initial_step_single_frame: Cell<bool>,
    is_final_step_single_frame: Cell<bool>,
}

impl StepEasingFunction {
    /// A function with one step.
    pub fn new() -> StepEasingFunction {
        StepEasingFunction::with_step_count(1)
    }

    /// A function with `step_count` steps.
    pub fn with_step_count(step_count: i32) -> StepEasingFunction {
        StepEasingFunction {
            step_count: Cell::new(step_count),
            initial_step: Cell::new(0),
            final_step: Cell::new(None),
            is_initial_step_single_frame: Cell::new(false),
            is_final_step_single_frame: Cell::new(false),
        }
    }

    /// The number of steps between the starting value and the end value.
    pub fn step_count(&self) -> i32 {
        self.step_count.get()
    }

    pub fn set_step_count(&self, value: i32) {
        self.step_count.set(value)
    }

    /// The step to start at (0 by default).
    pub fn initial_step(&self) -> i32 {
        self.initial_step.get()
    }

    pub fn set_initial_step(&self, value: i32) {
        self.initial_step.set(value)
    }

    /// The step to end at: the step count until it is set.
    pub fn final_step(&self) -> i32 {
        self.final_step.get().unwrap_or(self.step_count.get())
    }

    pub fn set_final_step(&self, value: i32) {
        self.final_step.set(Some(value))
    }

    /// Whether the initial step lasts the smallest possible duration.
    pub fn is_initial_step_single_frame(&self) -> bool {
        self.is_initial_step_single_frame.get()
    }

    pub fn set_is_initial_step_single_frame(&self, value: bool) {
        self.is_initial_step_single_frame.set(value)
    }

    /// Whether the final step lasts the smallest possible duration.
    pub fn is_final_step_single_frame(&self) -> bool {
        self.is_final_step_single_frame.get()
    }

    pub fn set_is_final_step_single_frame(&self, value: bool) {
        self.is_final_step_single_frame.set(value)
    }

    fn parameters(&self) -> StepParameters {
        StepParameters {
            step_count: self.step_count(),
            initial_step: self.initial_step(),
            final_step: self.final_step(),
            is_initial_step_single_frame: self.is_initial_step_single_frame(),
            is_final_step_single_frame: self.is_final_step_single_frame(),
        }
    }
}

impl Default for StepEasingFunction {
    fn default() -> Self {
        StepEasingFunction::new()
    }
}

impl IEasing for StepEasingFunction {
    /// A copy with the members the function has now.
    fn to_shared(&self) -> Arc<SharedEasing> {
        Arc::new(self.parameters())
    }

    fn ease(&self, progress: f64) -> f64 {
        self.parameters().ease_core(progress)
    }
}

easing_conversions!(StepEasingFunction);

/// A curve that retracts before it moves (WinUI `BackEasingFunction`):
/// `f(t) = t^3 - t * a * sin(pi t)`, `a` the amplitude (at least 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackEasingFunction {
    mode: CompositionEasingFunctionMode,
    amplitude: f32,
}

impl BackEasingFunction {
    pub fn new(mode: CompositionEasingFunctionMode, amplitude: f32) -> BackEasingFunction {
        BackEasingFunction { mode, amplitude }
    }

    /// The mode of the curve.
    pub fn mode(&self) -> CompositionEasingFunctionMode {
        self.mode
    }

    /// How far the curve retracts.
    pub fn amplitude(&self) -> f32 {
        self.amplitude
    }

    fn ease_core(&self, t: f64) -> f64 {
        let amplitude = (self.amplitude as f64).max(0.0);
        ease_with_mode(self.mode, t, |t| t * t * t - t * amplitude * (PI * t).sin())
    }
}

value_easing!(BackEasingFunction);

/// A bouncing curve (WinUI `BounceEasingFunction`): `bounces` bounces, each
/// `bounciness` times smaller than the one before.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BounceEasingFunction {
    mode: CompositionEasingFunctionMode,
    bounces: i32,
    bounciness: f32,
}

impl BounceEasingFunction {
    pub fn new(mode: CompositionEasingFunctionMode, bounces: i32, bounciness: f32) -> BounceEasingFunction {
        BounceEasingFunction { mode, bounces, bounciness }
    }

    /// The mode of the curve.
    pub fn mode(&self) -> CompositionEasingFunctionMode {
        self.mode
    }

    /// The number of bounces.
    pub fn bounces(&self) -> i32 {
        self.bounces
    }

    /// The bounciness of the curve: a larger value makes the bounces shrink
    /// faster.
    pub fn bounciness(&self) -> f32 {
        self.bounciness
    }

    /// `BounceEase.EaseInCore`.
    fn ease_in(t: f64, bounces: i32, bounciness: f32) -> f64 {
        let bounces = (bounces as f64).max(0.0);
        let mut bounciness = bounciness as f64;
        // Just over one avoids a division by zero.
        if bounciness < 1.0 || (bounciness - 1.0).abs() < 1e-9 {
            bounciness = 1.001;
        }
        let pow = bounciness.powf(bounces);
        let one_minus_bounciness = 1.0 - bounciness;
        // The bounces grow geometrically in time: the total in units of the
        // first bounce, with only half of the last one.
        let sum_of_units = (1.0 - pow) / one_minus_bounciness + pow * 0.5;
        let unit_at_t = t * sum_of_units;
        // The bounce `t` is in.
        let bounce_at_t = (-unit_at_t * (1.0 - bounciness) + 1.0).log(bounciness);
        let start = bounce_at_t.floor();
        let end = start + 1.0;
        let start_time = (1.0 - bounciness.powf(start)) / (one_minus_bounciness * sum_of_units);
        let end_time = (1.0 - bounciness.powf(end)) / (one_minus_bounciness * sum_of_units);
        // A parabola through (start, 0) and (end, 0) that peaks at the
        // amplitude of the bounce.
        let mid_time = (start_time + end_time) * 0.5;
        let time_relative_to_peak = t - mid_time;
        let radius = mid_time - start_time;
        let amplitude = (1.0 / bounciness).powf(bounces - start);
        (-amplitude / (radius * radius)) * (time_relative_to_peak - radius) * (time_relative_to_peak + radius)
    }

    fn ease_core(&self, t: f64) -> f64 {
        ease_with_mode(self.mode, t, |t| Self::ease_in(t, self.bounces, self.bounciness))
    }
}

value_easing!(BounceEasingFunction);

/// A circular curve (WinUI `CircleEasingFunction`): `f(t) = 1 - sqrt(1 - t^2)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CircleEasingFunction {
    mode: CompositionEasingFunctionMode,
}

impl CircleEasingFunction {
    pub fn new(mode: CompositionEasingFunctionMode) -> CircleEasingFunction {
        CircleEasingFunction { mode }
    }

    /// The mode of the curve.
    pub fn mode(&self) -> CompositionEasingFunctionMode {
        self.mode
    }

    fn ease_core(&self, t: f64) -> f64 {
        ease_with_mode(self.mode, t, |t| {
            let t = t.clamp(0.0, 1.0);
            1.0 - (1.0 - t * t).sqrt()
        })
    }
}

value_easing!(CircleEasingFunction);

/// An oscillating curve (WinUI `ElasticEasingFunction`): `oscillations`
/// oscillations whose amplitude grows exponentially with `springiness`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElasticEasingFunction {
    mode: CompositionEasingFunctionMode,
    oscillations: i32,
    springiness: f32,
}

impl ElasticEasingFunction {
    pub fn new(mode: CompositionEasingFunctionMode, oscillations: i32, springiness: f32) -> ElasticEasingFunction {
        ElasticEasingFunction { mode, oscillations, springiness }
    }

    /// The mode of the curve.
    pub fn mode(&self) -> CompositionEasingFunctionMode {
        self.mode
    }

    /// The number of oscillations.
    pub fn oscillations(&self) -> i32 {
        self.oscillations
    }

    /// The stiffness of the spring: a larger value damps the oscillations
    /// faster.
    pub fn springiness(&self) -> f32 {
        self.springiness
    }

    fn ease_core(&self, t: f64) -> f64 {
        let oscillations = (self.oscillations as f64).max(0.0);
        let springiness = (self.springiness as f64).max(0.0);
        ease_with_mode(self.mode, t, |t| {
            let expo = if springiness == 0.0 { t } else { ((springiness * t).exp() - 1.0) / (springiness.exp() - 1.0) };
            expo * ((PI * 2.0 * oscillations + PI * 0.5) * t).sin()
        })
    }
}

value_easing!(ElasticEasingFunction);

/// An exponential curve (WinUI `ExponentialEasingFunction`):
/// `f(t) = (e^(k t) - 1) / (e^k - 1)`, `k` the exponent (linear for 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExponentialEasingFunction {
    mode: CompositionEasingFunctionMode,
    exponent: f32,
}

impl ExponentialEasingFunction {
    pub fn new(mode: CompositionEasingFunctionMode, exponent: f32) -> ExponentialEasingFunction {
        ExponentialEasingFunction { mode, exponent }
    }

    /// The mode of the curve.
    pub fn mode(&self) -> CompositionEasingFunctionMode {
        self.mode
    }

    /// The exponent of the curve.
    pub fn exponent(&self) -> f32 {
        self.exponent
    }

    fn ease_core(&self, t: f64) -> f64 {
        let factor = self.exponent as f64;
        ease_with_mode(self.mode, t, |t| if factor == 0.0 { t } else { ((factor * t).exp() - 1.0) / (factor.exp() - 1.0) })
    }
}

value_easing!(ExponentialEasingFunction);

/// A power curve (WinUI `PowerEasingFunction`): `f(t) = t^p`, `p` the power
/// (at least 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerEasingFunction {
    mode: CompositionEasingFunctionMode,
    power: f32,
}

impl PowerEasingFunction {
    pub fn new(mode: CompositionEasingFunctionMode, power: f32) -> PowerEasingFunction {
        PowerEasingFunction { mode, power }
    }

    /// The mode of the curve.
    pub fn mode(&self) -> CompositionEasingFunctionMode {
        self.mode
    }

    /// The power of the curve.
    pub fn power(&self) -> f32 {
        self.power
    }

    fn ease_core(&self, t: f64) -> f64 {
        let power = (self.power as f64).max(0.0);
        ease_with_mode(self.mode, t, |t| t.powf(power))
    }
}

value_easing!(PowerEasingFunction);

/// A sine curve (WinUI `SineEasingFunction`): `f(t) = 1 - sin(pi/2 (1 - t))`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SineEasingFunction {
    mode: CompositionEasingFunctionMode,
}

impl SineEasingFunction {
    pub fn new(mode: CompositionEasingFunctionMode) -> SineEasingFunction {
        SineEasingFunction { mode }
    }

    /// The mode of the curve.
    pub fn mode(&self) -> CompositionEasingFunctionMode {
        self.mode
    }

    fn ease_core(&self, t: f64) -> f64 {
        ease_with_mode(self.mode, t, |t| 1.0 - (PI * 0.5 * (1.0 - t)).sin())
    }
}

value_easing!(SineEasingFunction);

/// The factories of the easing functions (the statics of WinUI
/// `CompositionEasingFunction`, Windows App SDK 1.4+). Each returns the
/// [`Easing`] handle key frames take; create the typed function
/// (`BackEasingFunction::new`) to read its members.
// Deviation (winui-composition.md): the easing functions are easings of
// FerroUI, not composition objects; the owner compositor is not needed.
pub struct CompositionEasingFunction;

impl CompositionEasingFunction {
    /// A cubic Bezier curve with the given control points.
    pub fn create_cubic_bezier_easing_function(
        _owner: &Compositor,
        control_point1: Vector2,
        control_point2: Vector2,
    ) -> Easing {
        CubicBezierEasingFunction::new(control_point1, control_point2).into()
    }

    /// The linear curve.
    pub fn create_linear_easing_function(_owner: &Compositor) -> Easing {
        LinearEasingFunction::new().into()
    }

    /// A step function with one step.
    pub fn create_step_easing_function(_owner: &Compositor) -> Easing {
        StepEasingFunction::new().into()
    }

    /// A step function with `step_count` steps.
    pub fn create_step_easing_function_with_step_count(_owner: &Compositor, step_count: i32) -> Easing {
        StepEasingFunction::with_step_count(step_count).into()
    }

    /// A back curve.
    pub fn create_back_easing_function(
        _owner: &Compositor,
        mode: CompositionEasingFunctionMode,
        amplitude: f32,
    ) -> Easing {
        BackEasingFunction::new(mode, amplitude).into()
    }

    /// A bounce curve.
    pub fn create_bounce_easing_function(
        _owner: &Compositor,
        mode: CompositionEasingFunctionMode,
        bounces: i32,
        bounciness: f32,
    ) -> Easing {
        BounceEasingFunction::new(mode, bounces, bounciness).into()
    }

    /// A circle curve.
    pub fn create_circle_easing_function(_owner: &Compositor, mode: CompositionEasingFunctionMode) -> Easing {
        CircleEasingFunction::new(mode).into()
    }

    /// An elastic curve.
    pub fn create_elastic_easing_function(
        _owner: &Compositor,
        mode: CompositionEasingFunctionMode,
        oscillations: i32,
        springiness: f32,
    ) -> Easing {
        ElasticEasingFunction::new(mode, oscillations, springiness).into()
    }

    /// An exponential curve.
    pub fn create_exponential_easing_function(
        _owner: &Compositor,
        mode: CompositionEasingFunctionMode,
        exponent: f32,
    ) -> Easing {
        ExponentialEasingFunction::new(mode, exponent).into()
    }

    /// A power curve.
    pub fn create_power_easing_function(_owner: &Compositor, mode: CompositionEasingFunctionMode, power: f32) -> Easing {
        PowerEasingFunction::new(mode, power).into()
    }

    /// A sine curve.
    pub fn create_sine_easing_function(_owner: &Compositor, mode: CompositionEasingFunctionMode) -> Easing {
        SineEasingFunction::new(mode).into()
    }
}

/// The easing factories of WinUI `Compositor`. They return the typed
/// function (`Rc`), which key frames take as it is
/// (`insert_key_frame_with_easing`) or as an [`Easing`] (`.into()`).
impl Compositor {
    /// A cubic Bezier curve with the given control points (WinUI
    /// `CreateCubicBezierEasingFunction`).
    pub fn create_cubic_bezier_easing_function(
        &self,
        control_point1: Vector2,
        control_point2: Vector2,
    ) -> Rc<CubicBezierEasingFunction> {
        Rc::new(CubicBezierEasingFunction::new(control_point1, control_point2))
    }

    /// The linear curve (WinUI `CreateLinearEasingFunction`).
    pub fn create_linear_easing_function(&self) -> Rc<LinearEasingFunction> {
        Rc::new(LinearEasingFunction::new())
    }

    /// A step function with one step (WinUI `CreateStepEasingFunction()`).
    pub fn create_step_easing_function(&self) -> Rc<StepEasingFunction> {
        Rc::new(StepEasingFunction::new())
    }

    /// A step function with `step_count` steps (WinUI
    /// `CreateStepEasingFunction(stepCount)`).
    pub fn create_step_easing_function_with_step_count(&self, step_count: i32) -> Rc<StepEasingFunction> {
        Rc::new(StepEasingFunction::with_step_count(step_count))
    }
}
