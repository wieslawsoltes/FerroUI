//! The server side of the spring natural motion animations: a damped harmonic
//! oscillator per component.

use super::animation_controller::AnimationControllerClock;
use super::{AnimationDelayBehavior, AnimationInstanceBase, AnimationStopBehavior, IAnimationInstance, PropertySetSnapshot};
use crate::numerics::{Vector2, Vector3};
use crate::rendering::composition::expressions::{Expression, ExpressionVariant, ExpressionVariantValue};
use crate::rendering::composition::server::{CompositionProperty, IServerClockItem, ServerCompositor, ServerObjectId};
use crate::rendering::composition::AnimationCompletion;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The ticks (100 ns units) of a time span.
fn ticks(d: Duration) -> i64 {
    (d.as_nanos() / 100) as i64
}

/// A value a spring animation moves: its components move independently,
/// each on the same spring.
pub trait SpringValue: ExpressionVariantValue {
    /// The number of components (1 to 3).
    const COMPONENTS: usize;

    /// The components, unused ones zero.
    fn to_components(self) -> [f64; 3];

    /// The value with the given components.
    fn from_components(components: [f64; 3]) -> Self;
}

impl SpringValue for f32 {
    const COMPONENTS: usize = 1;

    fn to_components(self) -> [f64; 3] {
        [self as f64, 0.0, 0.0]
    }

    fn from_components(components: [f64; 3]) -> f32 {
        components[0] as f32
    }
}

impl SpringValue for Vector2 {
    const COMPONENTS: usize = 2;

    fn to_components(self) -> [f64; 3] {
        [self.x as f64, self.y as f64, 0.0]
    }

    fn from_components(components: [f64; 3]) -> Vector2 {
        Vector2::new(components[0] as f32, components[1] as f32)
    }
}

impl SpringValue for Vector3 {
    const COMPONENTS: usize = 3;

    fn to_components(self) -> [f64; 3] {
        [self.x as f64, self.y as f64, self.z as f64]
    }

    fn from_components(components: [f64; 3]) -> Vector3 {
        Vector3::new(components[0] as f32, components[1] as f32, components[2] as f32)
    }
}

/// How close to 1 a damping ratio is taken as critical damping.
const CRITICAL_DAMPING_TOLERANCE: f64 = 1e-6;

/// The state of a damped harmonic oscillator at time `t` (seconds): the
/// displacement from the rest point and the velocity, for an initial
/// displacement `x0`, an initial velocity `v0`, the undamped angular
/// frequency `omega` (radians per second) and the damping ratio `zeta`.
///
/// The closed forms of the equation `x'' + 2 zeta omega x' + omega^2 x = 0`:
/// under-damped (`zeta < 1`) the spring oscillates in a decaying envelope;
/// critically damped (`zeta = 1`) and over-damped (`zeta > 1`) it returns
/// without crossing the rest point when it starts at rest.
pub fn spring_state(x0: f64, v0: f64, omega: f64, zeta: f64, t: f64) -> (f64, f64) {
    if t <= 0.0 {
        return (x0, v0);
    }
    if zeta < 1.0 - CRITICAL_DAMPING_TOLERANCE {
        let damped = omega * (1.0 - zeta * zeta).sqrt();
        let decay = (-zeta * omega * t).exp();
        let b = (v0 + zeta * omega * x0) / damped;
        let (sin, cos) = (damped * t).sin_cos();
        let x = decay * (x0 * cos + b * sin);
        let v = decay * ((b * damped - zeta * omega * x0) * cos - (x0 * damped + zeta * omega * b) * sin);
        (x, v)
    } else if zeta <= 1.0 + CRITICAL_DAMPING_TOLERANCE {
        let decay = (-omega * t).exp();
        let b = v0 + omega * x0;
        let x = decay * (x0 + b * t);
        let v = decay * (b - omega * (x0 + b * t));
        (x, v)
    } else {
        let root = (zeta * zeta - 1.0).sqrt();
        let r1 = -omega * (zeta - root);
        let r2 = -omega * (zeta + root);
        let c2 = (v0 - r1 * x0) / (r2 - r1);
        let c1 = x0 - c2;
        let e1 = (r1 * t).exp();
        let e2 = (r2 * t).exp();
        (c1 * e1 + c2 * e2, c1 * r1 * e1 + c2 * r2 * e2)
    }
}

/// `value` in the representation of `like` where both name the same vector:
/// a `Vector3` spring on a property of `Vector3D` values (`Offset` of a
/// visual) evaluates to a `Vector3D`, so expressions that read the property
/// while it is animated see its type.
fn shaped(value: ExpressionVariant, like: ExpressionVariant) -> ExpressionVariant {
    match (like, value) {
        (ExpressionVariant::Vector3D(_), ExpressionVariant::Vector3(v)) => ExpressionVariant::Vector3D(v.into()),
        (ExpressionVariant::Vector(_), ExpressionVariant::Vector2(v)) => ExpressionVariant::Vector(v.into()),
        _ => value,
    }
}

/// What a spring natural motion animation is made of when it starts (the
/// members of `SpringScalarNaturalMotionAnimation` and its vector siblings),
/// in the form that crosses to the server.
#[derive(Clone, Copy)]
pub struct SpringNaturalMotionParameters<T> {
    /// The value the spring comes to rest at: `FinalValue`, or the new value
    /// of an implicit start; the current value of the property when `None`.
    pub final_value: Option<T>,
    /// The value the spring starts from; the current value of the property
    /// when `None`.
    pub initial_value: Option<T>,
    /// The velocity the spring starts with, per second.
    pub initial_velocity: T,
    /// The damping ratio: 1 is critically damped, less oscillates, more
    /// returns slower; 0 never comes to rest.
    pub damping_ratio: f32,
    /// The period of the undamped oscillation.
    pub period: Duration,
    pub delay_behavior: AnimationDelayBehavior,
    pub delay_time: Duration,
    pub stop_behavior: AnimationStopBehavior,
}

/// Server-side counterpart of the spring natural motion animations
/// (`SpringScalarNaturalMotionAnimation`, `SpringVector2NaturalMotionAnimation`,
/// `SpringVector3NaturalMotionAnimation`).
///
/// Each component of the value moves on a damped harmonic oscillator from the
/// initial value with the initial velocity to the final value. The animation
/// ends when every component is close enough to the final value and nearly at
/// rest: it then sets the final value, leaves the clock and releases the
/// batches it belongs to, as a key frame animation that finishes.
pub struct SpringNaturalMotionAnimationInstance<T: SpringValue> {
    base: AnimationInstanceBase,
    spring: SpringNaturalMotionParameters<T>,
    delay_time: i64,
    started_at: Cell<i64>,
    start: Cell<T>,
    end: Cell<T>,
    /// The value of the property when the animation started, which gives the
    /// representation of the values the animation evaluates to.
    starting_variant: Cell<ExpressionVariant>,
    finished: Cell<bool>,
    completion: RefCell<AnimationCompletion>,
    initial_values: RefCell<Vec<(String, Expression)>>,
    clock: AnimationControllerClock,
}

impl<T: SpringValue> SpringNaturalMotionAnimationInstance<T> {
    pub fn new(
        spring: SpringNaturalMotionParameters<T>,
        target: ServerObjectId,
        snapshot: Rc<PropertySetSnapshot>,
    ) -> Rc<SpringNaturalMotionAnimationInstance<T>> {
        Rc::new_cyclic(|this: &Weak<SpringNaturalMotionAnimationInstance<T>>| SpringNaturalMotionAnimationInstance {
            base: AnimationInstanceBase::new(this.clone(), target, snapshot),
            delay_time: ticks(spring.delay_time),
            spring,
            started_at: Cell::new(0),
            start: Cell::new(T::default()),
            end: Cell::new(T::default()),
            starting_variant: Cell::new(ExpressionVariant::Invalid),
            finished: Cell::new(false),
            completion: RefCell::new(AnimationCompletion::none()),
            initial_values: RefCell::new(Vec::new()),
            clock: AnimationControllerClock::default(),
        })
    }

    /// Sets the batches the animation belongs to.
    pub fn set_completion(&self, completion: AnimationCompletion) {
        *self.completion.borrow_mut() = completion;
    }

    /// Sets the initial value expressions of the animation, applied to the
    /// target when the instance is initialized.
    pub fn set_initial_value_expressions(&self, expressions: Vec<(String, Expression)>) {
        *self.initial_values.borrow_mut() = expressions;
    }

    fn release_completion_now(&self) {
        // Taken out first: releasing may post the completion of a batch.
        let mut completion = std::mem::take(&mut *self.completion.borrow_mut());
        completion.release();
    }

    fn leave_clock(&self) {
        if let (Some(compositor), Some(this)) = (self.base.compositor(), self.base.this_clock_item()) {
            compositor.animations().remove_from_clock(&this);
        }
    }

    fn variant(&self, value: T) -> ExpressionVariant {
        shaped(ExpressionVariant::create(value), self.starting_variant.get())
    }

    /// The value `t` seconds after the delay, and whether the spring is at
    /// rest at the final value.
    fn state_at(&self, t: f64) -> (T, bool) {
        let start = self.start.get().to_components();
        let end = self.end.get().to_components();
        let velocity = self.spring.initial_velocity.to_components();
        let period = self.spring.period.as_secs_f64();
        let omega = std::f64::consts::TAU / period;
        let zeta = (self.spring.damping_ratio as f64).max(0.0);
        // At rest within a ten-thousandth of the size of the motion (at least
        // of one unit), moving slower than that amplitude would at the
        // frequency of the spring.
        let mut scale: f64 = 1.0;
        for i in 0..T::COMPONENTS {
            scale = scale.max((start[i] - end[i]).abs()).max(velocity[i].abs() / omega);
        }
        let position_epsilon = 1e-4 * scale;
        let velocity_epsilon = position_epsilon * omega;
        let mut value = [0.0; 3];
        let mut at_rest = true;
        for i in 0..T::COMPONENTS {
            let (x, v) = spring_state(start[i] - end[i], velocity[i], omega, zeta, t);
            value[i] = end[i] + x;
            if x.abs() > position_epsilon || v.abs() > velocity_epsilon {
                at_rest = false;
            }
        }
        (T::from_components(value), at_rest)
    }

    fn evaluate_core(&self, now: Duration, current_value: ExpressionVariant) -> ExpressionVariant {
        let current = if current_value == ExpressionVariant::Invalid { self.starting_variant.get() } else { current_value };
        if self.finished.get() {
            return self.variant(self.end.get());
        }
        let elapsed = self.clock.elapsed(self.base.compositor().as_ref(), ticks(now), self.started_at.get(), None);
        if elapsed < self.delay_time {
            return match self.spring.delay_behavior {
                AnimationDelayBehavior::SetInitialValueBeforeDelay => self.variant(self.start.get()),
                AnimationDelayBehavior::SetInitialValueAfterDelay => current,
            };
        }
        let t = (elapsed - self.delay_time) as f64 / 10_000_000.0;
        let (value, at_rest) = self.state_at(t);
        // A paused animation does not end: its controller may resume it.
        if at_rest && !self.clock.is_paused() {
            self.leave_clock();
            self.finished.set(true);
            // The animation is over: the batches it belongs to may complete.
            self.release_completion_now();
            return self.variant(self.end.get());
        }
        self.variant(value)
    }
}

impl<T: SpringValue> IServerClockItem for SpringNaturalMotionAnimationInstance<T> {
    fn on_tick(&self) {
        // A target that is gone never evaluates the animation again.
        if self.base.target().is_none() {
            self.leave_clock();
            self.release_completion_now();
            return;
        }
        self.base.on_tick()
    }
}

impl<T: SpringValue> IAnimationInstance for SpringNaturalMotionAnimationInstance<T> {
    fn target_object(&self) -> ServerObjectId {
        self.base.target_object()
    }

    fn resolve(&self, compositor: &Rc<ServerCompositor>) {
        self.base.resolve(compositor)
    }

    fn evaluate(&self, now: Duration, current_value: ExpressionVariant) -> ExpressionVariant {
        self.base.begin_evaluate();
        self.evaluate_core(now, current_value)
    }

    fn initialize(&self, started_at: Duration, starting_value: ExpressionVariant, property: &'static CompositionProperty) {
        let initial_values = std::mem::take(&mut *self.initial_values.borrow_mut());
        let starting_value = self.base.apply_initial_values(&initial_values, property, starting_value);
        self.starting_variant.set(starting_value);
        let current = starting_value.try_cast::<T>().unwrap_or_default();
        self.start.set(self.spring.initial_value.unwrap_or(current));
        self.end.set(self.spring.final_value.unwrap_or(current));
        self.started_at.set(ticks(started_at));
        self.base.initialize(property, &HashSet::new());
    }

    fn activate(&self) {
        if self.finished.get() {
            return;
        }
        if let (Some(compositor), Some(this)) = (self.base.compositor(), self.base.this_clock_item()) {
            compositor.animations().add_to_clock(this);
        }
        self.base.activate();
    }

    fn deactivate(&self) {
        self.leave_clock();
        self.base.deactivate();
    }

    fn invalidate(&self) {
        self.base.invalidate()
    }

    fn value_on_stop(&self, _current_value: ExpressionVariant) -> Option<ExpressionVariant> {
        match self.spring.stop_behavior {
            AnimationStopBehavior::LeaveCurrentValue => None,
            AnimationStopBehavior::SetToInitialValue => Some(self.variant(self.start.get())),
            AnimationStopBehavior::SetToFinalValue => Some(self.variant(self.end.get())),
        }
    }

    fn release_completion(&self) {
        self.release_completion_now()
    }

    fn attach_controller(&self, controller: ServerObjectId) {
        self.clock.attach(controller)
    }
}
