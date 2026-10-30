//! Ported from `Animations/AnimationFactory.cs`.

use crate::argument_out_of_range::{throw_if_greater_than, throw_if_less_than};
use ferroui_base::animation::{Animation, IAnimationSetter, KeyFrame, TimeSpan};
use ferroui_base::styling::Setter;
use ferroui_base::{Ref, Visual};
use std::rc::Rc;

/// Creates commonly used key-frame animations.
pub struct AnimationFactory;

impl AnimationFactory {
    /// Creates an opacity animation that holds at zero during the initial delay and then fades to
    /// one.
    ///
    /// `initial_delay` is the delay before the fade begins, `duration` the duration of the fade.
    ///
    /// # Panics
    ///
    /// When `initial_delay` or `duration` is negative (the `ArgumentOutOfRangeException` of the
    /// reference).
    #[track_caller]
    pub fn create_fade_in(initial_delay: TimeSpan, duration: TimeSpan) -> Ref<Animation> {
        throw_if_less_than(initial_delay, TimeSpan::ZERO, "initialDelay");
        throw_if_less_than(duration, TimeSpan::ZERO, "duration");

        let total_duration = initial_delay + duration;
        Self::create_fade_in_timeline(initial_delay, total_duration, total_duration)
    }

    /// Creates an opacity animation with explicit hold, completion, and total times.
    ///
    /// `initial_delay` is the key time through which opacity remains zero, `fade_completion_time`
    /// the key time at which opacity becomes one and `total_duration` the total animation
    /// duration.
    ///
    /// # Panics
    ///
    /// When any supplied time is negative or the key times are not chronological (the
    /// `ArgumentOutOfRangeException` of the reference).
    #[track_caller]
    pub fn create_fade_in_timeline(
        initial_delay: TimeSpan,
        fade_completion_time: TimeSpan,
        total_duration: TimeSpan,
    ) -> Ref<Animation> {
        throw_if_less_than(initial_delay, TimeSpan::ZERO, "initialDelay");
        throw_if_less_than(fade_completion_time, TimeSpan::ZERO, "fadeCompletionTime");
        throw_if_less_than(total_duration, TimeSpan::ZERO, "totalDuration");
        throw_if_greater_than(initial_delay, fade_completion_time, "initialDelay");
        throw_if_greater_than(fade_completion_time, total_duration, "fadeCompletionTime");

        let animation = Animation::new();
        animation.set_duration(total_duration);
        let children = animation.children();
        children.add(KeyFrame::with_key_time(TimeSpan::ZERO, [opacity_setter(0.0)]));
        children.add(KeyFrame::with_key_time(initial_delay, [opacity_setter(0.0)]));
        children.add(KeyFrame::with_key_time(fade_completion_time, [opacity_setter(1.0)]));
        animation
    }
}

/// `new Setter(Visual.OpacityProperty, value)`.
fn opacity_setter(value: f64) -> Rc<dyn IAnimationSetter> {
    Setter::new(Visual::opacity_property(), value)
}
