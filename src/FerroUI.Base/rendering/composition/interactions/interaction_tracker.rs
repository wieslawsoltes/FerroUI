//! `InteractionTracker`: a position and a scale driven by input, inertia and
//! requests, which expressions read to move visuals.

use super::interaction_tracker_inertia_modifier::InteractionTrackerInertiaModifier;
use super::visual_interaction_source::VisualInteractionSource;
use crate::animation::{PlaybackDirection, TimeSpan};
use crate::media::MediaContext;
use crate::numerics::Vector3;
use crate::rendering::composition::animations::{
    AnimationIterationBehavior, ExpressionAnimation, KeyFrame, KeyFrameAnimation, PropertySetSnapshot,
    ScalarKeyFrameAnimation, Vector3KeyFrameAnimation,
};
use crate::rendering::composition::expressions::{
    BuiltInExpressionFfi, Expression, ExpressionEvaluationContext, ExpressionVariant, ExpressionVariantValue,
    IExpressionObject,
};
use crate::rendering::composition::{AsCompositionObject, CompositionObject, CompositionPropertySet, Compositor};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The decay rate of the position and of the scale while
/// `PositionInertiaDecayRate` and `ScaleInertiaDecayRate` are not set.
pub const DEFAULT_INERTIA_DECAY_RATE: f32 = 0.95;

/// How far, in pixels, the position can be pulled past a boundary while
/// interacting: the limit the rubber band approaches.
const OVERPAN_LIMIT: f32 = 120.0;

/// The distance (pixels, or scale units for the scale) under which an
/// inertia is considered at rest.
const REST_EPSILON: f32 = 0.01;

/// The states of an [`InteractionTracker`].
///
/// WinUI has no public state property; the state is known through the
/// owner callbacks. The port exposes it as [`InteractionTracker::state`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InteractionTrackerState {
    /// Nothing moves the tracker.
    #[default]
    Idle,
    /// A source drives the tracker with input.
    Interacting,
    /// The tracker coasts to its resting values.
    Inertia,
    /// A custom animation moves the tracker
    /// (`TryUpdatePositionWithAnimation`, `TryUpdateScaleWithAnimation`).
    CustomAnimation,
}

/// Whether a position request is clamped to the boundaries
/// (`InteractionTrackerClampingOption`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InteractionTrackerClampingOption {
    /// The value is clamped to `MinPosition` and `MaxPosition`.
    #[default]
    Auto,
    /// The value is used as given, even outside the boundaries.
    Disabled,
}

/// The arguments of [`IInteractionTrackerOwner::custom_animation_state_entered`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionTrackerCustomAnimationStateEnteredArgs {
    pub request_id: i32,
    /// Always `false`: tracker binding is not ported.
    pub is_from_binding: bool,
}

/// The arguments of [`IInteractionTrackerOwner::idle_state_entered`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionTrackerIdleStateEnteredArgs {
    pub request_id: i32,
    /// Always `false`: tracker binding is not ported.
    pub is_from_binding: bool,
}

/// The arguments of [`IInteractionTrackerOwner::inertia_state_entered`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionTrackerInertiaStateEnteredArgs {
    /// The resting position after the inertia modifiers and the boundaries.
    pub modified_resting_position: Option<Vector3>,
    /// The resting scale after the inertia modifiers and the boundaries.
    pub modified_resting_scale: Option<f32>,
    /// The resting position of the natural decay of the velocity.
    pub natural_resting_position: Vector3,
    /// The resting scale of the natural decay of the scale velocity.
    pub natural_resting_scale: f32,
    pub position_velocity_in_pixels_per_second: Vector3,
    pub scale_velocity_in_percent_per_second: f32,
    pub request_id: i32,
    /// Whether the inertia comes from a `TryUpdate*WithAdditionalVelocity`
    /// request rather than from the end of an interaction.
    pub is_inertia_from_impulse: bool,
    /// Always `false`: tracker binding is not ported.
    pub is_from_binding: bool,
}

/// The arguments of [`IInteractionTrackerOwner::interacting_state_entered`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionTrackerInteractingStateEnteredArgs {
    pub request_id: i32,
    /// Always `false`: tracker binding is not ported.
    pub is_from_binding: bool,
}

/// The arguments of [`IInteractionTrackerOwner::request_ignored`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionTrackerRequestIgnoredArgs {
    pub request_id: i32,
}

/// The arguments of [`IInteractionTrackerOwner::values_changed`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionTrackerValuesChangedArgs {
    pub position: Vector3,
    pub scale: f32,
    pub request_id: i32,
}

/// The callbacks of an [`InteractionTracker`] (`IInteractionTrackerOwner`).
///
/// Every callback does nothing by default. The tracker holds its owner
/// strongly, as WinUI does: an owner that keeps the tracker should keep it
/// weakly (each callback gets the tracker as `sender`), or the two keep each
/// other alive.
///
/// Callbacks run on the UI thread after the change they report, never
/// nested: a callback that makes a request on the tracker gets the
/// callbacks of that request after it returns.
pub trait IInteractionTrackerOwner {
    fn custom_animation_state_entered(
        &self,
        _sender: &Rc<InteractionTracker>,
        _args: &InteractionTrackerCustomAnimationStateEnteredArgs,
    ) {
    }

    fn idle_state_entered(&self, _sender: &Rc<InteractionTracker>, _args: &InteractionTrackerIdleStateEnteredArgs) {}

    fn inertia_state_entered(
        &self,
        _sender: &Rc<InteractionTracker>,
        _args: &InteractionTrackerInertiaStateEnteredArgs,
    ) {
    }

    fn interacting_state_entered(
        &self,
        _sender: &Rc<InteractionTracker>,
        _args: &InteractionTrackerInteractingStateEnteredArgs,
    ) {
    }

    fn request_ignored(&self, _sender: &Rc<InteractionTracker>, _args: &InteractionTrackerRequestIgnoredArgs) {}

    fn values_changed(&self, _sender: &Rc<InteractionTracker>, _args: &InteractionTrackerValuesChangedArgs) {}
}

/// A custom animation of the position or of the scale of a tracker: the
/// `CompositionAnimation` that `TryUpdatePositionWithAnimation` and
/// `TryUpdateScaleWithAnimation` take.
///
/// Deviation (winui-composition.md): the tracker runs on the UI thread, so
/// it evaluates the animation itself rather than on the server: key frames
/// (value and expression key frames, easing, delay, iterations, direction)
/// and expressions. Reference parameters of these animations that are
/// property sets (or trackers) are read; references to other composition
/// objects read as missing, because their values live on the server.
#[derive(Clone)]
pub enum InteractionTrackerAnimation {
    /// A key frame animation of the position.
    Vector3KeyFrame(Rc<Vector3KeyFrameAnimation>),
    /// A key frame animation of the scale.
    ScalarKeyFrame(Rc<ScalarKeyFrameAnimation>),
    /// An expression of the position (`Vector3`) or of the scale (scalar),
    /// evaluated every frame until another request or an interaction.
    Expression(Rc<ExpressionAnimation>),
}

impl From<Rc<Vector3KeyFrameAnimation>> for InteractionTrackerAnimation {
    fn from(value: Rc<Vector3KeyFrameAnimation>) -> Self {
        Self::Vector3KeyFrame(value)
    }
}

impl From<Rc<ScalarKeyFrameAnimation>> for InteractionTrackerAnimation {
    fn from(value: Rc<ScalarKeyFrameAnimation>) -> Self {
        Self::ScalarKeyFrame(value)
    }
}

impl From<Rc<ExpressionAnimation>> for InteractionTrackerAnimation {
    fn from(value: Rc<ExpressionAnimation>) -> Self {
        Self::Expression(value)
    }
}

/// The values of a tracker: what expressions read as `tracker.Name` and
/// `this.Target.Name`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TrackerValues {
    pub position: Vector3,
    pub position_velocity: Vector3,
    pub scale: f32,
    pub scale_velocity: f32,
    pub min_position: Vector3,
    pub max_position: Vector3,
    pub min_scale: f32,
    pub max_scale: f32,
    pub natural_resting_position: Vector3,
    pub natural_resting_scale: f32,
    pub position_inertia_decay_rate: Option<Vector3>,
    pub scale_inertia_decay_rate: Option<f32>,
    pub is_position_rounding_suggested: bool,
    pub is_inertia_from_impulse: bool,
}

impl Default for TrackerValues {
    fn default() -> Self {
        Self {
            position: Vector3::ZERO,
            position_velocity: Vector3::ZERO,
            scale: 1.0,
            scale_velocity: 0.0,
            min_position: Vector3::ZERO,
            max_position: Vector3::ZERO,
            min_scale: 1.0,
            max_scale: 1.0,
            natural_resting_position: Vector3::ZERO,
            natural_resting_scale: 1.0,
            position_inertia_decay_rate: None,
            scale_inertia_decay_rate: None,
            is_position_rounding_suggested: true,
            is_inertia_from_impulse: false,
        }
    }
}

impl TrackerValues {
    fn effective_position_decay_rate(&self) -> Vector3 {
        self.position_inertia_decay_rate.unwrap_or(Vector3::from_value(DEFAULT_INERTIA_DECAY_RATE))
    }

    fn effective_scale_decay_rate(&self) -> f32 {
        self.scale_inertia_decay_rate.unwrap_or(DEFAULT_INERTIA_DECAY_RATE)
    }

    /// The value of the property with the given WinUI name.
    pub fn get_property(&self, name: &str) -> ExpressionVariant {
        match name {
            "Position" => self.position.into(),
            "PositionVelocityInPixelsPerSecond" => self.position_velocity.into(),
            "Scale" => self.scale.into(),
            "ScaleVelocityInPercentPerSecond" => self.scale_velocity.into(),
            "MinPosition" => self.min_position.into(),
            "MaxPosition" => self.max_position.into(),
            "MinScale" => self.min_scale.into(),
            "MaxScale" => self.max_scale.into(),
            "NaturalRestingPosition" => self.natural_resting_position.into(),
            "NaturalRestingScale" => self.natural_resting_scale.into(),
            "PositionInertiaDecayRate" => self.effective_position_decay_rate().into(),
            "ScaleInertiaDecayRate" => self.effective_scale_decay_rate().into(),
            "IsPositionRoundingSuggested" => self.is_position_rounding_suggested.into(),
            "IsInertiaFromImpulse" => self.is_inertia_from_impulse.into(),
            _ => ExpressionVariant::Invalid,
        }
    }
}

/// The tracker as `this.Target` of the expressions it evaluates.
struct TrackerTarget(TrackerValues);

impl IExpressionObject for TrackerTarget {
    fn get_property(&self, name: &str) -> ExpressionVariant {
        self.0.get_property(name)
    }
}

/// Evaluates the expression of `animation` against the tracker values.
/// The parameters are taken from the animation at each evaluation, so a
/// referenced property set is read as it is now. An empty or invalid
/// expression gives an invalid variant.
fn evaluate_animation(animation: &ExpressionAnimation, values: TrackerValues, current: ExpressionVariant) -> ExpressionVariant {
    let Some(source) = animation.expression() else { return ExpressionVariant::Invalid };
    let Ok(expression) = Expression::parse(&source) else { return ExpressionVariant::Invalid };
    evaluate_parsed(&expression, animation.create_snapshot(), values, current, current)
}

fn evaluate_parsed(
    expression: &Expression,
    parameters: Rc<PropertySetSnapshot>,
    values: TrackerValues,
    starting: ExpressionVariant,
    current: ExpressionVariant,
) -> ExpressionVariant {
    let target = TrackerTarget(values);
    let ctx = ExpressionEvaluationContext {
        starting_value: starting,
        current_value: current,
        final_value: starting,
        target: Some(&target),
        parameters: Some(&*parameters),
        foreign_function_interface: Some(BuiltInExpressionFfi::instance()),
    };
    expression.evaluate(&ctx)
}

fn axis(v: Vector3, index: usize) -> f32 {
    match index {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn set_axis(v: &mut Vector3, index: usize, value: f32) {
    match index {
        0 => v.x = value,
        1 => v.y = value,
        _ => v.z = value,
    }
}

/// Clamps to `[min, max]`; when `min > max`, `min` wins, as for
/// `Math.Clamp` applied to a reversed range in WinUI's samples.
fn clamp_scalar(value: f32, min: f32, max: f32) -> f32 {
    let max = max.max(min);
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn clamp_vector(value: Vector3, min: Vector3, max: Vector3) -> Vector3 {
    Vector3::new(
        clamp_scalar(value.x, min.x, max.x),
        clamp_scalar(value.y, min.y, max.y),
        clamp_scalar(value.z, min.z, max.z),
    )
}

/// The rubber band of an overpan: the displayed distance past a boundary
/// for a pulled distance `x`, approaching [`OVERPAN_LIMIT`].
fn rubber_band(x: f32) -> f32 {
    OVERPAN_LIMIT * (1.0 - 1.0 / (1.0 + 0.55 * x / OVERPAN_LIMIT))
}

fn rubber_band_scalar(raw: f32, min: f32, max: f32) -> f32 {
    let max = max.max(min);
    if raw < min {
        min - rubber_band(min - raw)
    } else if raw > max {
        max + rubber_band(raw - max)
    } else {
        raw
    }
}

/// The share of the velocity kept after one second for a decay rate:
/// `1 - decay_rate`, kept below 1 so that the inertia ends.
///
/// Deviation (winui-composition.md): WinUI documents the decay rate as the
/// part of the velocity lost per second (0.95 keeps 5% after a second);
/// the port uses that reading: `v(t) = v0 * (1 - rate)^t`.
fn retention(decay_rate: f32) -> f32 {
    (1.0 - decay_rate.clamp(0.0, 1.0)).min(0.9999)
}

/// The closed form of the exponential decay: where a value `p` moving at
/// `v` per second comes to rest when the velocity keeps the share `r` of
/// itself each second. `p(t) = p + v * (r^t - 1) / ln r`, so
/// `p(inf) = p - v / ln r = p + v / ln(1 / r)`.
pub(crate) fn natural_resting_value(p: f32, v: f32, r: f32) -> f32 {
    if r <= 0.0 || v == 0.0 {
        p
    } else {
        p - v / r.ln()
    }
}

/// The value and velocity at `t` of the exponential motion from `start` to
/// `target`: `target + (start - target) * r^t`. With the natural resting
/// value as the target this is exactly the decay of the velocity; with a
/// modified target the motion keeps the same shape and ends there.
fn exponential_motion(start: f32, target: f32, r: f32, t: f32) -> (f32, f32, bool) {
    if r <= 0.0 {
        return (target, 0.0, true);
    }
    let k = r.powf(t);
    let remaining = (start - target) * k;
    let velocity = (target - start) * (-r.ln()) * k;
    (target + remaining, velocity, remaining.abs() < REST_EPSILON)
}

/// The value types a custom animation runs on.
trait TrackValue: ExpressionVariantValue {
    fn interpolate(from: Self, to: Self, progress: f32) -> Self;
}

impl TrackValue for Vector3 {
    fn interpolate(from: Self, to: Self, progress: f32) -> Self {
        from + (to - from) * progress
    }
}

impl TrackValue for f32 {
    fn interpolate(from: Self, to: Self, progress: f32) -> Self {
        from + (to - from) * progress
    }
}

/// A key frame animation, read from its animation when the request is
/// made, evaluated on the UI thread.
struct KeyFrameTrack<T> {
    frames: Vec<KeyFrame<T>>,
    duration: f32,
    delay: f32,
    iteration_count: i32,
    forever: bool,
    direction: PlaybackDirection,
    parameters: Rc<PropertySetSnapshot>,
}

impl<T: TrackValue> KeyFrameTrack<T> {
    fn new(animation: &KeyFrameAnimation) -> Self {
        let frames = animation.with_key_frames::<T, _>(|key_frames| key_frames.items().to_vec());
        KeyFrameTrack {
            frames,
            duration: animation.duration().as_secs_f32().max(0.001),
            delay: animation.delay_time().as_secs_f32(),
            iteration_count: animation.iteration_count().max(1),
            forever: animation.iteration_behavior() == AnimationIterationBehavior::Forever,
            direction: animation.direction(),
            parameters: animation.create_snapshot(),
        }
    }

    /// The value at `elapsed` seconds, and whether the animation is over.
    fn value_at(&self, elapsed: f32, starting: T, current: T, values: TrackerValues) -> (T, bool) {
        let t = elapsed - self.delay;
        if t < 0.0 {
            return (starting, false);
        }
        let total = self.duration * self.iteration_count as f32;
        let (iteration, local, finished) = if !self.forever && t >= total {
            (self.iteration_count - 1, 1.0, true)
        } else {
            let iteration = (t / self.duration).floor();
            (iteration as i32, (t - iteration * self.duration) / self.duration, false)
        };
        let even = iteration % 2 == 0;
        let progress = match self.direction {
            PlaybackDirection::Normal => local,
            PlaybackDirection::Reverse => 1.0 - local,
            PlaybackDirection::Alternate => {
                if even {
                    local
                } else {
                    1.0 - local
                }
            }
            PlaybackDirection::AlternateReverse => {
                if even {
                    1.0 - local
                } else {
                    local
                }
            }
        };
        (self.interpolate(progress, starting, current, values), finished)
    }

    fn frame_value(&self, frame: &KeyFrame<T>, starting: T, current: T, values: TrackerValues) -> T {
        match &frame.expression {
            Some(expression) => evaluate_parsed(
                expression,
                self.parameters.clone(),
                values,
                ExpressionVariant::create(starting),
                ExpressionVariant::create(current),
            )
            .try_cast::<T>()
            .unwrap_or(frame.value),
            None => frame.value,
        }
    }

    fn interpolate(&self, progress: f32, starting: T, current: T, values: TrackerValues) -> T {
        // The value before the first key frame is the starting value, as
        // the server instance adds a key frame for it at 0.
        let mut previous_key = 0.0f32;
        let mut previous_value = starting;
        for frame in &self.frames {
            let value = self.frame_value(frame, starting, current, values);
            if progress <= frame.normalized_progress_key {
                let span = frame.normalized_progress_key - previous_key;
                let local = if span <= 0.0 { 1.0 } else { (progress - previous_key) / span };
                let eased = frame.easing_function.ease(local as f64) as f32;
                return T::interpolate(previous_value, value, eased);
            }
            previous_key = frame.normalized_progress_key;
            previous_value = value;
        }
        previous_value
    }
}

/// What a custom animation evaluates.
enum Track<T> {
    KeyFrames(KeyFrameTrack<T>),
    Expression { expression: Rc<Expression>, animation: Rc<ExpressionAnimation> },
}

impl<T: TrackValue> Track<T> {
    fn value_at(&self, elapsed: f32, starting: T, current: T, values: TrackerValues) -> (T, bool) {
        match self {
            Track::KeyFrames(track) => track.value_at(elapsed, starting, current, values),
            Track::Expression { expression, animation } => {
                let value = evaluate_parsed(
                    expression,
                    animation.create_snapshot(),
                    values,
                    ExpressionVariant::create(starting),
                    ExpressionVariant::create(current),
                )
                .try_cast::<T>()
                .unwrap_or(current);
                (value, false)
            }
        }
    }
}

enum CustomKind {
    Position { track: Track<Vector3>, starting: Vector3 },
    Scale { track: Track<f32>, starting: f32, center: Vector3 },
}

struct CustomRun {
    request_id: i32,
    elapsed: f32,
    kind: CustomKind,
}

struct InertiaRun {
    request_id: i32,
    elapsed: f32,
    start_position: Vector3,
    target_position: Vector3,
    position_retention: Vector3,
    start_scale: f32,
    target_scale: f32,
    scale_retention: f32,
    scale_center: Vector3,
    /// The motion modifiers of the X, Y and scale axes.
    motions: [Option<Rc<ExpressionAnimation>>; 3],
}

enum Notification {
    CustomAnimation(InteractionTrackerCustomAnimationStateEnteredArgs),
    Idle(InteractionTrackerIdleStateEnteredArgs),
    Inertia(InteractionTrackerInertiaStateEnteredArgs),
    Interacting(InteractionTrackerInteractingStateEnteredArgs),
    RequestIgnored(InteractionTrackerRequestIgnoredArgs),
    ValuesChanged(InteractionTrackerValuesChangedArgs),
}

/// The sources of a tracker (`CompositionInteractionSourceCollection`).
pub struct CompositionInteractionSourceCollection {
    tracker: Weak<InteractionTracker>,
    items: RefCell<Vec<Rc<VisualInteractionSource>>>,
}

impl CompositionInteractionSourceCollection {
    /// Adds a source: its input drives the tracker from now on. Adding a
    /// source twice does nothing.
    pub fn add(&self, source: Rc<VisualInteractionSource>) {
        if self.items.borrow().iter().any(|s| Rc::ptr_eq(s, &source)) {
            return;
        }
        self.items.borrow_mut().push(source.clone());
        source.attach_tracker(&self.tracker);
    }

    /// Removes a source.
    pub fn remove(&self, source: &Rc<VisualInteractionSource>) {
        let removed = {
            let mut items = self.items.borrow_mut();
            let index = items.iter().position(|s| Rc::ptr_eq(s, source));
            index.map(|index| items.remove(index))
        };
        if let Some(removed) = removed {
            removed.detach_tracker(&self.tracker);
        }
    }

    /// Removes every source.
    pub fn remove_all(&self) {
        let removed = std::mem::take(&mut *self.items.borrow_mut());
        for source in removed {
            source.detach_tracker(&self.tracker);
        }
    }

    /// The number of sources.
    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// The sources, in the order they were added.
    pub fn items(&self) -> Vec<Rc<VisualInteractionSource>> {
        self.items.borrow().clone()
    }
}

/// A position and a scale driven by input, inertia and requests
/// (`Microsoft.UI.Composition.Interactions.InteractionTracker`).
///
/// The main use is to bind visuals to it: an expression animation that
/// references the tracker (`set_reference_parameter("tracker", tracker)`)
/// reads `tracker.Position`, `tracker.Scale` and the other values of the
/// tracker, e.g. `-tracker.Position` for the `Offset` of a scrolled visual.
///
/// Deviation (winui-composition.md): WinUI runs the tracker in the
/// compositor. Here it runs on the UI thread, ticking with the animation
/// frames of the media context while it is in inertia or in a custom
/// animation, and publishes its values into its property set
/// ([`properties`](Self::properties)) after every change; a reference to
/// the tracker in an expression is a reference to that property set.
pub struct InteractionTracker {
    this: Weak<InteractionTracker>,
    properties: Rc<CompositionPropertySet>,
    owner: Option<Rc<dyn IInteractionTrackerOwner>>,
    state: Cell<InteractionTrackerState>,
    values: Cell<TrackerValues>,
    interaction_sources: CompositionInteractionSourceCollection,
    position_x_modifiers: RefCell<Vec<InteractionTrackerInertiaModifier>>,
    position_y_modifiers: RefCell<Vec<InteractionTrackerInertiaModifier>>,
    scale_modifiers: RefCell<Vec<InteractionTrackerInertiaModifier>>,
    next_request_id: Cell<i32>,
    last_request_id: Cell<i32>,
    /// The number of sources that currently drive the tracker.
    active_interactions: Cell<u32>,
    /// The position the input asked for while interacting, before the
    /// rubber band of the boundaries.
    raw_position: Cell<Vector3>,
    inertia: RefCell<Option<InertiaRun>>,
    custom: RefCell<Option<CustomRun>>,
    frame_requested: Cell<bool>,
    frame_requests_enabled: Cell<bool>,
    last_frame_time: Cell<Option<f64>>,
    notifications: RefCell<VecDeque<Notification>>,
    notifying: Cell<bool>,
}

impl InteractionTracker {
    /// `InteractionTracker.Create(compositor)`.
    pub fn create(compositor: &Rc<Compositor>) -> Rc<InteractionTracker> {
        Self::create_core(compositor, None)
    }

    /// `InteractionTracker.CreateWithOwner(compositor, owner)`.
    pub fn create_with_owner(compositor: &Rc<Compositor>, owner: Rc<dyn IInteractionTrackerOwner>) -> Rc<InteractionTracker> {
        Self::create_core(compositor, Some(owner))
    }

    fn create_core(compositor: &Rc<Compositor>, owner: Option<Rc<dyn IInteractionTrackerOwner>>) -> Rc<InteractionTracker> {
        let tracker = Rc::new_cyclic(|this: &Weak<InteractionTracker>| InteractionTracker {
            this: this.clone(),
            properties: CompositionPropertySet::new(compositor),
            owner,
            state: Cell::new(InteractionTrackerState::Idle),
            values: Cell::new(TrackerValues::default()),
            interaction_sources: CompositionInteractionSourceCollection {
                tracker: this.clone(),
                items: RefCell::new(Vec::new()),
            },
            position_x_modifiers: RefCell::new(Vec::new()),
            position_y_modifiers: RefCell::new(Vec::new()),
            scale_modifiers: RefCell::new(Vec::new()),
            next_request_id: Cell::new(0),
            last_request_id: Cell::new(0),
            active_interactions: Cell::new(0),
            raw_position: Cell::new(Vector3::ZERO),
            inertia: RefCell::new(None),
            custom: RefCell::new(None),
            frame_requested: Cell::new(false),
            frame_requests_enabled: Cell::new(true),
            last_frame_time: Cell::new(None),
            notifications: RefCell::new(VecDeque::new()),
            notifying: Cell::new(false),
        });
        tracker.publish();
        tracker
    }

    /// The compositor of the tracker.
    pub fn compositor(&self) -> &Rc<Compositor> {
        self.properties.compositor()
    }

    /// The property set the tracker publishes its values into, under the
    /// WinUI names (`Position`, `Scale`, `PositionVelocityInPixelsPerSecond`,
    /// `ScaleVelocityInPercentPerSecond`, `MinPosition`, `MaxPosition`,
    /// `MinScale`, `MaxScale`, `NaturalRestingPosition`,
    /// `NaturalRestingScale`, `PositionInertiaDecayRate`,
    /// `ScaleInertiaDecayRate`, `IsPositionRoundingSuggested`,
    /// `IsInertiaFromImpulse`).
    ///
    /// Deviation (winui-composition.md): in WinUI, `Properties` of a tracker
    /// is a property set of the application and the values are properties
    /// of the tracker itself; here the values live in this set, so entries
    /// the application adds must not use those names.
    pub fn properties(&self) -> &Rc<CompositionPropertySet> {
        &self.properties
    }

    /// The owner given at creation.
    pub fn owner(&self) -> Option<Rc<dyn IInteractionTrackerOwner>> {
        self.owner.clone()
    }

    /// The current state (not a WinUI member, see
    /// [`InteractionTrackerState`]).
    pub fn state(&self) -> InteractionTrackerState {
        self.state.get()
    }

    /// The sources whose input drives the tracker.
    pub fn interaction_sources(&self) -> &CompositionInteractionSourceCollection {
        &self.interaction_sources
    }

    // --- values -------------------------------------------------------------

    fn update_values(&self, f: impl FnOnce(&mut TrackerValues)) {
        let mut values = self.values.get();
        f(&mut values);
        self.values.set(values);
        self.publish();
    }

    /// Writes the values into the property set.
    fn publish(&self) {
        let v = self.values.get();
        let p = &self.properties;
        p.insert_vector3("Position", v.position);
        p.insert_vector3("PositionVelocityInPixelsPerSecond", v.position_velocity);
        p.insert_scalar("Scale", v.scale);
        p.insert_scalar("ScaleVelocityInPercentPerSecond", v.scale_velocity);
        p.insert_vector3("MinPosition", v.min_position);
        p.insert_vector3("MaxPosition", v.max_position);
        p.insert_scalar("MinScale", v.min_scale);
        p.insert_scalar("MaxScale", v.max_scale);
        p.insert_vector3("NaturalRestingPosition", v.natural_resting_position);
        p.insert_scalar("NaturalRestingScale", v.natural_resting_scale);
        p.insert_vector3("PositionInertiaDecayRate", v.effective_position_decay_rate());
        p.insert_scalar("ScaleInertiaDecayRate", v.effective_scale_decay_rate());
        p.insert_boolean("IsPositionRoundingSuggested", v.is_position_rounding_suggested);
        p.insert_boolean("IsInertiaFromImpulse", v.is_inertia_from_impulse);
    }

    /// The position (`Position`).
    pub fn position(&self) -> Vector3 {
        self.values.get().position
    }

    /// The velocity of the position (`PositionVelocityInPixelsPerSecond`).
    pub fn position_velocity_in_pixels_per_second(&self) -> Vector3 {
        self.values.get().position_velocity
    }

    /// The scale (`Scale`).
    pub fn scale(&self) -> f32 {
        self.values.get().scale
    }

    /// The velocity of the scale (`ScaleVelocityInPercentPerSecond`), in
    /// scale units per second.
    pub fn scale_velocity_in_percent_per_second(&self) -> f32 {
        self.values.get().scale_velocity
    }

    /// The lower boundary of the position (`MinPosition`).
    pub fn min_position(&self) -> Vector3 {
        self.values.get().min_position
    }

    /// Sets the lower boundary. The position is not moved by the change; the
    /// next request or inertia honours it.
    pub fn set_min_position(&self, value: Vector3) {
        self.update_values(|v| v.min_position = value);
    }

    /// The upper boundary of the position (`MaxPosition`).
    pub fn max_position(&self) -> Vector3 {
        self.values.get().max_position
    }

    pub fn set_max_position(&self, value: Vector3) {
        self.update_values(|v| v.max_position = value);
    }

    /// The lower boundary of the scale (`MinScale`).
    pub fn min_scale(&self) -> f32 {
        self.values.get().min_scale
    }

    pub fn set_min_scale(&self, value: f32) {
        self.update_values(|v| v.min_scale = value);
    }

    /// The upper boundary of the scale (`MaxScale`).
    pub fn max_scale(&self) -> f32 {
        self.values.get().max_scale
    }

    pub fn set_max_scale(&self, value: f32) {
        self.update_values(|v| v.max_scale = value);
    }

    /// Where the natural decay of the velocity of the last inertia comes to
    /// rest, before the inertia modifiers and the boundaries
    /// (`NaturalRestingPosition`).
    pub fn natural_resting_position(&self) -> Vector3 {
        self.values.get().natural_resting_position
    }

    /// The natural resting scale of the last inertia (`NaturalRestingScale`).
    pub fn natural_resting_scale(&self) -> f32 {
        self.values.get().natural_resting_scale
    }

    /// The decay rate of the position velocity per axis
    /// (`PositionInertiaDecayRate`); `None` for the default
    /// ([`DEFAULT_INERTIA_DECAY_RATE`]).
    pub fn position_inertia_decay_rate(&self) -> Option<Vector3> {
        self.values.get().position_inertia_decay_rate
    }

    pub fn set_position_inertia_decay_rate(&self, value: Option<Vector3>) {
        self.update_values(|v| v.position_inertia_decay_rate = value);
    }

    /// The decay rate of the scale velocity (`ScaleInertiaDecayRate`).
    pub fn scale_inertia_decay_rate(&self) -> Option<f32> {
        self.values.get().scale_inertia_decay_rate
    }

    pub fn set_scale_inertia_decay_rate(&self, value: Option<f32>) {
        self.update_values(|v| v.scale_inertia_decay_rate = value);
    }

    /// Whether the position should be rounded to whole pixels: `true` while
    /// the tracker is idle (`IsPositionRoundingSuggested`).
    pub fn is_position_rounding_suggested(&self) -> bool {
        self.values.get().is_position_rounding_suggested
    }

    /// Whether the current or last inertia comes from an impulse
    /// (`IsInertiaFromImpulse`).
    pub fn is_inertia_from_impulse(&self) -> bool {
        self.values.get().is_inertia_from_impulse
    }

    // --- inertia modifiers --------------------------------------------------

    /// `ConfigurePositionXInertiaModifiers`: the modifiers of the inertia of
    /// the X axis; an empty list removes them.
    pub fn configure_position_x_inertia_modifiers(&self, modifiers: impl IntoIterator<Item = InteractionTrackerInertiaModifier>) {
        *self.position_x_modifiers.borrow_mut() = modifiers.into_iter().collect();
    }

    /// `ConfigurePositionYInertiaModifiers`.
    pub fn configure_position_y_inertia_modifiers(&self, modifiers: impl IntoIterator<Item = InteractionTrackerInertiaModifier>) {
        *self.position_y_modifiers.borrow_mut() = modifiers.into_iter().collect();
    }

    /// `ConfigureScaleInertiaModifiers`.
    pub fn configure_scale_inertia_modifiers(&self, modifiers: impl IntoIterator<Item = InteractionTrackerInertiaModifier>) {
        *self.scale_modifiers.borrow_mut() = modifiers.into_iter().collect();
    }

    // --- requests -----------------------------------------------------------

    fn new_request_id(&self) -> i32 {
        let id = self.next_request_id.get() + 1;
        self.next_request_id.set(id);
        self.last_request_id.set(id);
        id
    }

    /// Ignores a request while a source drives the tracker. Returns whether
    /// it was ignored.
    fn ignore_while_interacting(&self, request_id: i32) -> bool {
        if self.state.get() == InteractionTrackerState::Interacting {
            self.notify(Notification::RequestIgnored(InteractionTrackerRequestIgnoredArgs { request_id }));
            return true;
        }
        false
    }

    /// `TryUpdatePosition(value)`: moves to `value`, clamped to the
    /// boundaries, and stops any inertia or custom animation. Ignored while
    /// interacting. Returns the id of the request.
    pub fn try_update_position(&self, value: Vector3) -> i32 {
        self.try_update_position_with_option(value, InteractionTrackerClampingOption::Auto)
    }

    /// `TryUpdatePosition(value, option)`.
    pub fn try_update_position_with_option(&self, value: Vector3, option: InteractionTrackerClampingOption) -> i32 {
        let id = self.new_request_id();
        if !self.ignore_while_interacting(id) {
            self.jump_to(value, option, id);
        }
        self.flush();
        id
    }

    /// `TryUpdatePositionBy(amount)`: moves by `amount`, clamped.
    pub fn try_update_position_by(&self, amount: Vector3) -> i32 {
        let id = self.new_request_id();
        if !self.ignore_while_interacting(id) {
            self.jump_to(self.position() + amount, InteractionTrackerClampingOption::Auto, id);
        }
        self.flush();
        id
    }

    fn jump_to(&self, value: Vector3, option: InteractionTrackerClampingOption, request_id: i32) {
        self.stop_runs();
        let values = self.values.get();
        let position = match option {
            InteractionTrackerClampingOption::Auto => clamp_vector(value, values.min_position, values.max_position),
            InteractionTrackerClampingOption::Disabled => value,
        };
        self.update_values(|v| {
            v.position = position;
            v.position_velocity = Vector3::ZERO;
            v.scale_velocity = 0.0;
        });
        self.values_changed(request_id);
        self.enter_idle(request_id);
    }

    /// `TryUpdatePositionWithAnimation(animation)`: runs a custom animation
    /// of the position (a `Vector3` key frame animation, or an expression).
    /// Ignored while interacting, or for an animation of another type.
    pub fn try_update_position_with_animation(&self, animation: impl Into<InteractionTrackerAnimation>) -> i32 {
        let id = self.new_request_id();
        if !self.ignore_while_interacting(id) {
            let track = match animation.into() {
                InteractionTrackerAnimation::Vector3KeyFrame(a) => Some(Track::KeyFrames(KeyFrameTrack::<Vector3>::new(&a))),
                InteractionTrackerAnimation::Expression(a) => Self::expression_track::<Vector3>(a),
                InteractionTrackerAnimation::ScalarKeyFrame(_) => None,
            };
            match track {
                Some(track) => {
                    let starting = self.position();
                    self.start_custom(CustomRun {
                        request_id: id,
                        elapsed: 0.0,
                        kind: CustomKind::Position { track, starting },
                    });
                }
                None => self.notify(Notification::RequestIgnored(InteractionTrackerRequestIgnoredArgs { request_id: id })),
            }
        }
        self.flush();
        id
    }

    fn expression_track<T: TrackValue>(animation: Rc<ExpressionAnimation>) -> Option<Track<T>> {
        let source = animation.expression()?;
        let expression = Expression::parse(&source).ok()?;
        Some(Track::Expression { expression: Rc::new(expression), animation })
    }

    /// `TryUpdatePositionWithAdditionalVelocity(velocity)`: adds `velocity`
    /// (pixels per second) to the current velocity and coasts in inertia.
    /// Ignored while interacting.
    pub fn try_update_position_with_additional_velocity(&self, velocity_in_pixels_per_second: Vector3) -> i32 {
        let id = self.new_request_id();
        if !self.ignore_while_interacting(id) {
            let values = self.values.get();
            let base = match self.state.get() {
                InteractionTrackerState::Idle => Vector3::ZERO,
                _ => values.position_velocity,
            };
            let scale_velocity = match self.state.get() {
                InteractionTrackerState::Idle => 0.0,
                _ => values.scale_velocity,
            };
            self.enter_inertia(base + velocity_in_pixels_per_second, scale_velocity, Vector3::ZERO, id, true);
        }
        self.flush();
        id
    }

    /// `TryUpdateScale(value, centerPoint)`: scales to `value`, clamped to
    /// the scale boundaries, keeping `center_point` (in the coordinates of
    /// the viewport) in place. Ignored while interacting.
    pub fn try_update_scale(&self, value: f32, center_point: Vector3) -> i32 {
        let id = self.new_request_id();
        if !self.ignore_while_interacting(id) {
            self.stop_runs();
            self.apply_scale(value, center_point);
            self.update_values(|v| {
                v.position_velocity = Vector3::ZERO;
                v.scale_velocity = 0.0;
            });
            self.values_changed(id);
            self.enter_idle(id);
        }
        self.flush();
        id
    }

    /// Sets the scale (clamped) and moves the position so that
    /// `center_point` stays in place.
    fn apply_scale(&self, value: f32, center_point: Vector3) {
        let values = self.values.get();
        let new_scale = clamp_scalar(value, values.min_scale, values.max_scale);
        let old_scale = values.scale;
        let position = if old_scale != 0.0 {
            let c = Vector3::new(center_point.x, center_point.y, 0.0);
            clamp_vector((values.position + c) * (new_scale / old_scale) - c, values.min_position, values.max_position)
        } else {
            values.position
        };
        self.update_values(|v| {
            v.scale = new_scale;
            v.position = position;
        });
    }

    /// `TryUpdateScaleWithAnimation(animation, centerPoint)`: runs a custom
    /// animation of the scale (a scalar key frame animation, or an
    /// expression). Ignored while interacting, or for an animation of
    /// another type.
    pub fn try_update_scale_with_animation(&self, animation: impl Into<InteractionTrackerAnimation>, center_point: Vector3) -> i32 {
        let id = self.new_request_id();
        if !self.ignore_while_interacting(id) {
            let track = match animation.into() {
                InteractionTrackerAnimation::ScalarKeyFrame(a) => Some(Track::KeyFrames(KeyFrameTrack::<f32>::new(&a))),
                InteractionTrackerAnimation::Expression(a) => Self::expression_track::<f32>(a),
                InteractionTrackerAnimation::Vector3KeyFrame(_) => None,
            };
            match track {
                Some(track) => {
                    let starting = self.scale();
                    self.start_custom(CustomRun {
                        request_id: id,
                        elapsed: 0.0,
                        kind: CustomKind::Scale { track, starting, center: center_point },
                    });
                }
                None => self.notify(Notification::RequestIgnored(InteractionTrackerRequestIgnoredArgs { request_id: id })),
            }
        }
        self.flush();
        id
    }

    /// `TryUpdateScaleWithAdditionalVelocity(velocity, centerPoint)`: adds
    /// `velocity` (scale units per second) to the scale velocity and coasts
    /// in inertia around `center_point`. Ignored while interacting.
    pub fn try_update_scale_with_additional_velocity(&self, velocity_in_percent_per_second: f32, center_point: Vector3) -> i32 {
        let id = self.new_request_id();
        if !self.ignore_while_interacting(id) {
            let values = self.values.get();
            let (position_velocity, scale_velocity) = match self.state.get() {
                InteractionTrackerState::Idle => (Vector3::ZERO, 0.0),
                _ => (values.position_velocity, values.scale_velocity),
            };
            self.enter_inertia(position_velocity, scale_velocity + velocity_in_percent_per_second, center_point, id, true);
        }
        self.flush();
        id
    }

    // --- input from the sources ---------------------------------------------

    /// A source starts driving the tracker: the tracker enters the
    /// interacting state, interrupting any inertia or custom animation.
    pub(crate) fn begin_interaction(&self) {
        self.active_interactions.set(self.active_interactions.get() + 1);
        if self.state.get() == InteractionTrackerState::Interacting {
            return;
        }
        self.stop_runs();
        self.raw_position.set(self.position());
        self.state.set(InteractionTrackerState::Interacting);
        self.update_values(|v| {
            v.is_position_rounding_suggested = false;
            v.position_velocity = Vector3::ZERO;
            v.scale_velocity = 0.0;
        });
        self.notify(Notification::Interacting(InteractionTrackerInteractingStateEnteredArgs {
            request_id: self.last_request_id.get(),
            is_from_binding: false,
        }));
        self.flush();
    }

    /// A source moved the position by `delta` while interacting. Past a
    /// boundary the position follows with a rubber band.
    pub(crate) fn interaction_delta(&self, delta: Vector3, velocity: Vector3) {
        if self.state.get() != InteractionTrackerState::Interacting {
            return;
        }
        let raw = self.raw_position.get() + delta;
        self.raw_position.set(raw);
        let values = self.values.get();
        let position = Vector3::new(
            rubber_band_scalar(raw.x, values.min_position.x, values.max_position.x),
            rubber_band_scalar(raw.y, values.min_position.y, values.max_position.y),
            rubber_band_scalar(raw.z, values.min_position.z, values.max_position.z),
        );
        self.update_values(|v| {
            v.position = position;
            v.position_velocity = velocity;
        });
        self.values_changed(self.last_request_id.get());
        self.flush();
    }

    /// A source stopped driving the tracker, releasing with `velocity`
    /// (pixels per second). When no source drives it any more, the tracker
    /// coasts in inertia.
    pub(crate) fn end_interaction(&self, velocity: Vector3) {
        let count = self.active_interactions.get().saturating_sub(1);
        self.active_interactions.set(count);
        if count > 0 || self.state.get() != InteractionTrackerState::Interacting {
            return;
        }
        self.enter_inertia(velocity, 0.0, Vector3::ZERO, self.last_request_id.get(), false);
        self.flush();
    }

    /// The mouse wheel of a source scrolled by `delta`: the tracker coasts
    /// to the position `delta` away from where it is heading.
    pub(crate) fn wheel_delta(&self, delta: Vector3) {
        if self.state.get() == InteractionTrackerState::Interacting {
            return;
        }
        let values = self.values.get();
        let base = match (self.state.get(), &*self.inertia.borrow()) {
            (InteractionTrackerState::Inertia, Some(run)) => run.target_position,
            _ => values.position,
        };
        let target = clamp_vector(base + delta, values.min_position, values.max_position);
        let r = values.effective_position_decay_rate();
        let mut velocity = Vector3::ZERO;
        for i in 0..3 {
            let ri = retention(axis(r, i));
            let distance = axis(target, i) - axis(values.position, i);
            // The velocity whose natural resting value is the target.
            set_axis(&mut velocity, i, if ri > 0.0 { -distance * ri.ln() } else { 0.0 });
        }
        self.enter_inertia(velocity, values.scale_velocity, Vector3::ZERO, self.last_request_id.get(), false);
        self.flush();
    }

    /// The mouse wheel of a source scaled by `factor` around
    /// `center_point`: applied at once.
    pub(crate) fn wheel_scale(&self, factor: f32, center_point: Vector3) {
        if self.state.get() == InteractionTrackerState::Interacting {
            return;
        }
        self.apply_scale(self.scale() * factor, center_point);
        self.values_changed(self.last_request_id.get());
        self.flush();
    }

    // --- states -------------------------------------------------------------

    fn stop_runs(&self) {
        drop(self.inertia.borrow_mut().take());
        drop(self.custom.borrow_mut().take());
    }

    fn enter_idle(&self, request_id: i32) {
        self.stop_runs();
        self.update_values(|v| {
            v.position_velocity = Vector3::ZERO;
            v.scale_velocity = 0.0;
            v.is_position_rounding_suggested = true;
        });
        if self.state.get() == InteractionTrackerState::Idle {
            return;
        }
        self.state.set(InteractionTrackerState::Idle);
        self.last_frame_time.set(None);
        self.notify(Notification::Idle(InteractionTrackerIdleStateEnteredArgs { request_id, is_from_binding: false }));
    }

    fn start_custom(&self, run: CustomRun) {
        self.stop_runs();
        let request_id = run.request_id;
        *self.custom.borrow_mut() = Some(run);
        self.state.set(InteractionTrackerState::CustomAnimation);
        self.last_frame_time.set(None);
        self.update_values(|v| v.is_position_rounding_suggested = false);
        self.notify(Notification::CustomAnimation(InteractionTrackerCustomAnimationStateEnteredArgs {
            request_id,
            is_from_binding: false,
        }));
        self.request_frame();
    }

    /// The first modifier of `modifiers` whose condition holds.
    fn matching_modifier(
        modifiers: &[InteractionTrackerInertiaModifier],
        values: TrackerValues,
    ) -> Option<InteractionTrackerInertiaModifier> {
        modifiers
            .iter()
            .find(|modifier| {
                modifier.condition().is_some_and(|condition| {
                    evaluate_animation(&condition, values, ExpressionVariant::Invalid) == ExpressionVariant::Boolean(true)
                })
            })
            .cloned()
    }

    /// Enters inertia with the given velocities: computes the natural
    /// resting values, applies the inertia modifiers and the boundaries.
    fn enter_inertia(&self, velocity: Vector3, scale_velocity: f32, scale_center: Vector3, request_id: i32, from_impulse: bool) {
        self.stop_runs();
        let values = self.values.get();
        let decay = values.effective_position_decay_rate();
        let position_retention = Vector3::new(retention(decay.x), retention(decay.y), retention(decay.z));
        let scale_retention = retention(values.effective_scale_decay_rate());
        let natural_position = Vector3::new(
            natural_resting_value(values.position.x, velocity.x, position_retention.x),
            natural_resting_value(values.position.y, velocity.y, position_retention.y),
            natural_resting_value(values.position.z, velocity.z, position_retention.z),
        );
        let natural_scale = natural_resting_value(values.scale, scale_velocity, scale_retention);
        self.update_values(|v| {
            v.position_velocity = velocity;
            v.scale_velocity = scale_velocity;
            v.natural_resting_position = natural_position;
            v.natural_resting_scale = natural_scale;
            v.is_inertia_from_impulse = from_impulse;
            v.is_position_rounding_suggested = false;
        });
        let values = self.values.get();

        // The modifiers read the natural resting values.
        let mut target_position = natural_position;
        let mut target_scale = natural_scale;
        let mut motions: [Option<Rc<ExpressionAnimation>>; 3] = [None, None, None];
        let lists = [
            self.position_x_modifiers.borrow().clone(),
            self.position_y_modifiers.borrow().clone(),
            self.scale_modifiers.borrow().clone(),
        ];
        for (index, modifiers) in lists.iter().enumerate() {
            match Self::matching_modifier(modifiers, values) {
                Some(InteractionTrackerInertiaModifier::RestingValue(modifier)) => {
                    let resting = modifier
                        .resting_value()
                        .map(|expression| evaluate_animation(&expression, values, ExpressionVariant::Invalid))
                        .and_then(|value| value.try_cast::<f32>());
                    if let Some(resting) = resting {
                        if index < 2 {
                            set_axis(&mut target_position, index, resting);
                        } else {
                            target_scale = resting;
                        }
                    }
                }
                Some(InteractionTrackerInertiaModifier::Motion(modifier)) => motions[index] = modifier.motion(),
                None => {}
            }
        }
        let target_position = clamp_vector(target_position, values.min_position, values.max_position);
        let target_scale = clamp_scalar(target_scale, values.min_scale, values.max_scale);

        *self.inertia.borrow_mut() = Some(InertiaRun {
            request_id,
            elapsed: 0.0,
            start_position: values.position,
            target_position,
            position_retention,
            start_scale: values.scale,
            target_scale,
            scale_retention,
            scale_center,
            motions,
        });
        self.state.set(InteractionTrackerState::Inertia);
        self.last_frame_time.set(None);
        self.notify(Notification::Inertia(InteractionTrackerInertiaStateEnteredArgs {
            modified_resting_position: Some(target_position),
            modified_resting_scale: Some(target_scale),
            natural_resting_position: natural_position,
            natural_resting_scale: natural_scale,
            position_velocity_in_pixels_per_second: velocity,
            scale_velocity_in_percent_per_second: scale_velocity,
            request_id,
            is_inertia_from_impulse: from_impulse,
            is_from_binding: false,
        }));
        self.request_frame();
    }

    // --- frames -------------------------------------------------------------

    /// Stops or resumes the requests of animation frames: a test drives the
    /// tracker with [`advance`](Self::advance) instead.
    #[cfg(test)]
    pub(crate) fn set_frame_requests_enabled(&self, value: bool) {
        self.frame_requests_enabled.set(value);
    }

    fn request_frame(&self) {
        if self.frame_requested.get() || !self.frame_requests_enabled.get() {
            return;
        }
        if !matches!(self.state.get(), InteractionTrackerState::Inertia | InteractionTrackerState::CustomAnimation) {
            return;
        }
        self.frame_requested.set(true);
        let weak = self.this.clone();
        MediaContext::instance().request_animation_frame(move |now| {
            if let Some(this) = weak.upgrade() {
                this.on_animation_frame(now);
            }
        });
    }

    fn on_animation_frame(&self, now: TimeSpan) {
        self.frame_requested.set(false);
        let now = now.total_seconds();
        let elapsed = match self.last_frame_time.replace(Some(now)) {
            Some(last) => (now - last).max(0.0),
            None => 0.0,
        };
        self.advance(Duration::from_secs_f64(elapsed));
        self.request_frame();
    }

    /// Moves the inertia or the custom animation forward by `elapsed`. The
    /// animation frames call it with the time between two frames.
    pub(crate) fn advance(&self, elapsed: Duration) {
        let dt = elapsed.as_secs_f32();
        match self.state.get() {
            InteractionTrackerState::Inertia => self.advance_inertia(dt),
            InteractionTrackerState::CustomAnimation => self.advance_custom(dt),
            _ => {}
        }
        self.flush();
    }

    fn advance_inertia(&self, dt: f32) {
        let Some(mut run) = self.inertia.borrow_mut().take() else { return };
        run.elapsed += dt;
        let t = run.elapsed;
        let values = self.values.get();

        // The scale first: it moves the position around its center.
        let (scale, scale_velocity, scale_done) = match &run.motions[2] {
            Some(motion) => {
                let value = evaluate_animation(motion, values, values.scale.into()).try_cast::<f32>().unwrap_or(values.scale);
                let velocity = if dt > 0.0 { (value - values.scale) / dt } else { 0.0 };
                (value, velocity, dt > 0.0 && (value - values.scale).abs() < REST_EPSILON * 0.01)
            }
            None => exponential_motion(run.start_scale, run.target_scale, run.scale_retention, t),
        };
        if values.scale != 0.0 && scale != values.scale {
            let k = scale / values.scale;
            let c = Vector3::new(run.scale_center.x, run.scale_center.y, 0.0);
            run.start_position = (run.start_position + c) * k - c;
            run.target_position = (run.target_position + c) * k - c;
        }

        let mut position = values.position;
        let mut velocity = Vector3::ZERO;
        let mut done = scale_done;
        for i in 0..3 {
            // The third motion is the one of the scale; Z has none.
            let motion = if i < 2 { run.motions[i].clone() } else { None };
            let (value, v, axis_done) = match &motion {
                Some(motion) => {
                    let current = axis(values.position, i);
                    let value = evaluate_animation(motion, values, current.into()).try_cast::<f32>().unwrap_or(current);
                    let v = if dt > 0.0 { (value - current) / dt } else { 0.0 };
                    (value, v, dt > 0.0 && (value - current).abs() < REST_EPSILON)
                }
                None => exponential_motion(
                    axis(run.start_position, i),
                    axis(run.target_position, i),
                    axis(run.position_retention, i),
                    t,
                ),
            };
            set_axis(&mut position, i, value);
            set_axis(&mut velocity, i, v);
            done &= axis_done;
        }

        if done {
            let final_position = {
                let mut p = run.target_position;
                for i in 0..2 {
                    if run.motions[i].is_some() {
                        set_axis(&mut p, i, axis(position, i));
                    }
                }
                clamp_vector(p, values.min_position, values.max_position)
            };
            let final_scale = if run.motions[2].is_some() {
                clamp_scalar(scale, values.min_scale, values.max_scale)
            } else {
                run.target_scale
            };
            self.update_values(|v| {
                v.position = final_position;
                v.scale = final_scale;
            });
            self.values_changed(run.request_id);
            self.enter_idle(run.request_id);
        } else {
            let request_id = run.request_id;
            *self.inertia.borrow_mut() = Some(run);
            self.update_values(|v| {
                v.position = position;
                v.position_velocity = velocity;
                v.scale = scale;
                v.scale_velocity = scale_velocity;
            });
            self.values_changed(request_id);
        }
    }

    fn advance_custom(&self, dt: f32) {
        let Some(mut run) = self.custom.borrow_mut().take() else { return };
        run.elapsed += dt;
        let values = self.values.get();
        let finished = match &run.kind {
            CustomKind::Position { track, starting } => {
                let (value, finished) = track.value_at(run.elapsed, *starting, values.position, values);
                let velocity = if dt > 0.0 { (value - values.position) / dt } else { values.position_velocity };
                self.update_values(|v| {
                    v.position = value;
                    v.position_velocity = velocity;
                });
                finished
            }
            CustomKind::Scale { track, starting, center } => {
                let (value, finished) = track.value_at(run.elapsed, *starting, values.scale, values);
                let velocity = if dt > 0.0 { (value - values.scale) / dt } else { values.scale_velocity };
                // Deviation (winui-composition.md): the boundaries of the
                // scale are applied to a custom animation of the scale.
                self.apply_scale(value, *center);
                self.update_values(|v| v.scale_velocity = velocity);
                finished
            }
        };
        let request_id = run.request_id;
        self.values_changed(request_id);
        if !finished {
            *self.custom.borrow_mut() = Some(run);
            return;
        }
        // A custom animation that ends past a boundary returns to it in
        // inertia; otherwise the tracker is idle.
        let values = self.values.get();
        if clamp_vector(values.position, values.min_position, values.max_position) != values.position {
            self.enter_inertia(Vector3::ZERO, 0.0, Vector3::ZERO, request_id, false);
        } else {
            self.enter_idle(request_id);
        }
    }

    // --- notifications ------------------------------------------------------

    fn values_changed(&self, request_id: i32) {
        let values = self.values.get();
        self.notify(Notification::ValuesChanged(InteractionTrackerValuesChangedArgs {
            position: values.position,
            scale: values.scale,
            request_id,
        }));
    }

    fn notify(&self, notification: Notification) {
        if self.owner.is_some() {
            self.notifications.borrow_mut().push_back(notification);
        }
    }

    /// Delivers the queued notifications to the owner, unless they are
    /// being delivered already (a callback that makes a request).
    fn flush(&self) {
        if self.notifying.get() {
            return;
        }
        let (Some(owner), Some(this)) = (self.owner.clone(), self.this.upgrade()) else {
            self.notifications.borrow_mut().clear();
            return;
        };
        self.notifying.set(true);
        loop {
            let next = self.notifications.borrow_mut().pop_front();
            let Some(next) = next else { break };
            match next {
                Notification::CustomAnimation(args) => owner.custom_animation_state_entered(&this, &args),
                Notification::Idle(args) => owner.idle_state_entered(&this, &args),
                Notification::Inertia(args) => owner.inertia_state_entered(&this, &args),
                Notification::Interacting(args) => owner.interacting_state_entered(&this, &args),
                Notification::RequestIgnored(args) => owner.request_ignored(&this, &args),
                Notification::ValuesChanged(args) => owner.values_changed(&this, &args),
            }
        }
        self.notifying.set(false);
    }
}

impl AsCompositionObject for InteractionTracker {
    /// The composition object of the property set of the tracker: a
    /// reference to the tracker is a reference to its values.
    fn as_composition_object(&self) -> &CompositionObject {
        self.properties.as_composition_object()
    }

    fn composition_type_name(&self) -> &'static str {
        "InteractionTracker"
    }

    fn as_property_set(self: Rc<Self>) -> Option<Rc<CompositionPropertySet>> {
        Some(self.properties.clone())
    }
}
