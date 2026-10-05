use super::{
    AnimationDelayBehavior, AnimationInstanceBase, AnimationIterationBehavior, AnimationStopBehavior,
    IAnimationInstance, IInterpolator, PropertySetSnapshot, ServerKeyFrame,
};
use crate::animation::PlaybackDirection;
use crate::rendering::composition::expressions::{
    BuiltInExpressionFfi, ExpressionEvaluationContext, ExpressionVariant, ExpressionVariantValue,
};
use crate::rendering::composition::server::{CompositionProperty, IServerClockItem, ServerCompositor, ServerObjectId};
use std::cell::Cell;
use std::collections::HashSet;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The ticks (100 ns units) of a time span.
fn ticks(d: Duration) -> i64 {
    (d.as_nanos() / 100) as i64
}

/// `TimeSpan.TotalSeconds` of a number of ticks.
fn total_seconds(ticks: i64) -> f64 {
    ticks as f64 / 10_000_000.0
}

/// `Math.Min` and `Math.Max` of .NET, which return NaN when an argument is
/// NaN.
fn net_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

fn net_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// Server-side counterpart of KeyFrameAnimation with values baked-in
///
/// Times are kept in ticks, as upstream computes with `TimeSpan` ticks; the
/// elapsed time of an evaluation before the start of the animation is
/// negative.
pub struct KeyFrameAnimationInstance<T: ExpressionVariantValue> {
    base: AnimationInstanceBase,
    interpolator: &'static dyn IInterpolator<T>,
    key_frames: Vec<ServerKeyFrame<T>>,
    final_value: Option<ExpressionVariant>,
    delay_behavior: AnimationDelayBehavior,
    delay_time: i64,
    direction: PlaybackDirection,
    duration: i64,
    iteration_behavior: AnimationIterationBehavior,
    iteration_count: i32,
    // Stored as upstream stores it; upstream does not read it either.
    #[allow(dead_code)]
    stop_behavior: AnimationStopBehavior,
    started_at: Cell<i64>,
    starting_value: Cell<T>,
    total_duration: i64,
    finished: Cell<bool>,
}

impl<T: ExpressionVariantValue> KeyFrameAnimationInstance<T> {
    /// Panics when there are no key frames or the duration is not positive
    /// (`InvalidOperationException` upstream).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        interpolator: &'static dyn IInterpolator<T>,
        key_frames: Vec<ServerKeyFrame<T>>,
        snapshot: Rc<PropertySetSnapshot>,
        final_value: Option<ExpressionVariant>,
        target: ServerObjectId,
        delay_behavior: AnimationDelayBehavior,
        delay_time: Duration,
        direction: PlaybackDirection,
        duration: Duration,
        iteration_behavior: AnimationIterationBehavior,
        iteration_count: i32,
        stop_behavior: AnimationStopBehavior,
    ) -> Rc<KeyFrameAnimationInstance<T>> {
        let duration = ticks(duration);
        let delay_time = ticks(delay_time);
        let mut total_duration = 0;
        if iteration_behavior == AnimationIterationBehavior::Count {
            total_duration = delay_time + iteration_count as i64 * duration;
        }
        if key_frames.is_empty() {
            panic!("Animation has no key frames");
        }
        if duration <= 0 {
            panic!("Invalid animation duration");
        }
        Rc::new_cyclic(|this: &Weak<KeyFrameAnimationInstance<T>>| KeyFrameAnimationInstance {
            base: AnimationInstanceBase::new(this.clone(), target, snapshot),
            interpolator,
            key_frames,
            final_value,
            delay_behavior,
            delay_time,
            direction,
            duration,
            iteration_behavior,
            iteration_count,
            stop_behavior,
            started_at: Cell::new(0),
            starting_value: Cell::new(T::default()),
            total_duration,
            finished: Cell::new(false),
        })
    }

    fn evaluate_core(&self, now: Duration, current_value: ExpressionVariant) -> ExpressionVariant {
        let starting = ExpressionVariant::create(self.starting_value.get());
        let elapsed = ticks(now) - self.started_at.get();
        let res = self.base.with_target_expression_object(|target| {
            let ctx = ExpressionEvaluationContext {
                parameters: Some(&**self.base.parameters()),
                target,
                current_value,
                final_value: self.final_value.unwrap_or(starting),
                starting_value: starting,
                foreign_function_interface: Some(BuiltInExpressionFfi::instance()),
            };
            self.evaluate_impl(elapsed, current_value, &ctx)
        });

        if self.iteration_behavior == AnimationIterationBehavior::Count
            && !self.finished.get()
            && elapsed > self.total_duration
        {
            // Active check?
            if let (Some(compositor), Some(this)) = (self.base.compositor(), self.base.this_clock_item()) {
                compositor.animations().remove_from_clock(&this);
            }
            self.finished.set(true);
        }
        res
    }

    fn evaluate_impl(
        &self,
        mut elapsed: i64,
        current_value: ExpressionVariant,
        ctx: &ExpressionEvaluationContext<'_>,
    ) -> ExpressionVariant {
        if elapsed < self.delay_time {
            if self.delay_behavior == AnimationDelayBehavior::SetInitialValueBeforeDelay {
                return ExpressionVariant::create(Self::get_key_frame(ctx, &self.key_frames[0]));
            }
            return current_value;
        }

        elapsed -= self.delay_time;
        let iteration_number = elapsed / self.duration;
        if self.iteration_behavior == AnimationIterationBehavior::Count
            && iteration_number >= self.iteration_count as i64
        {
            return ExpressionVariant::create(Self::get_key_frame(ctx, &self.key_frames[self.key_frames.len() - 1]));
        }

        let even_iteration_number = iteration_number % 2 == 0;
        elapsed %= self.duration;

        let reverse = match self.direction {
            PlaybackDirection::Alternate => !even_iteration_number,
            PlaybackDirection::AlternateReverse => even_iteration_number,
            direction => direction == PlaybackDirection::Reverse,
        };

        let mut iteration_progress = total_seconds(elapsed) / total_seconds(self.duration);
        if reverse {
            iteration_progress = 1.0 - iteration_progress;
        }

        let starting_frame = ServerKeyFrame { value: self.starting_value.get(), ..ServerKeyFrame::default() };
        let mut left = &starting_frame;
        let mut right = &self.key_frames[self.key_frames.len() - 1];
        for c in 0..self.key_frames.len() {
            let kf = &self.key_frames[c];
            if (kf.key as f64) < iteration_progress {
                // this is the last frame
                if c == self.key_frames.len() - 1 {
                    return ExpressionVariant::create(Self::get_key_frame(ctx, kf));
                }

                left = kf;
                right = &self.key_frames[c + 1];
            } else if c == 0 {
                // The current progress is before the first frame, we implicitly use the starting value
                // as the first frame in this case
                right = &self.key_frames[c];
                break;
            } else {
                break;
            }
        }

        let key_progress = net_max(
            0.0,
            net_min(1.0, (iteration_progress - left.key as f64) / (right.key as f64 - left.key as f64)),
        );

        let eased = right.easing_function.as_ref().map_or(key_progress, |easing| easing.ease(key_progress));
        let eased_key_progress = eased as f32;
        if eased_key_progress.is_nan() || eased_key_progress.is_infinite() {
            return current_value;
        }

        ExpressionVariant::create(self.interpolator.interpolate(
            Self::get_key_frame(ctx, left),
            Self::get_key_frame(ctx, right),
            eased_key_progress,
        ))
    }

    fn get_key_frame(ctx: &ExpressionEvaluationContext<'_>, f: &ServerKeyFrame<T>) -> T {
        match &f.expression {
            Some(expression) => expression.evaluate(ctx).cast_or_default::<T>(),
            None => f.value,
        }
    }
}

impl<T: ExpressionVariantValue> IServerClockItem for KeyFrameAnimationInstance<T> {
    fn on_tick(&self) {
        self.base.on_tick()
    }
}

impl<T: ExpressionVariantValue> IAnimationInstance for KeyFrameAnimationInstance<T> {
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
        self.started_at.set(ticks(started_at));
        self.starting_value.set(starting_value.cast_or_default::<T>());
        let mut hs = HashSet::new();

        // As upstream, the instance subscribes to the references of all key
        // frames rather than to those of the current key frame only.
        for frame in &self.key_frames {
            if let Some(expression) = &frame.expression {
                expression.collect_references(&mut hs);
            }
        }
        self.base.initialize(property, &hs);
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
        if let (Some(compositor), Some(this)) = (self.base.compositor(), self.base.this_clock_item()) {
            compositor.animations().remove_from_clock(&this);
        }
        self.base.deactivate();
    }

    fn invalidate(&self) {
        self.base.invalidate()
    }
}
