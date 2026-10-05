use crate::animation::animators::{interpolation_handler, Animator};
use crate::animation::easings::Easing;
use crate::animation::{
    Animatable, Animation, Clock, FillMode, IClock, IterationType, PlayState, PlaybackDirection, TimeSpan,
};
use crate::data::BindingPriority;
use crate::reactive::{SingleSubscriberObservableBase, IDisposable, IObservable, IObserver, ObservableError};
use crate::{Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

const PRECISION_IN_TICKS: i64 = 10_000;

/// An error that ends a running animation: an animation setting that is
/// invalid at the time of a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationError(pub String);

impl fmt::Display for AnimationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AnimationError {}

/// Handles interpolation and time-related functions for keyframe
/// animations: one run of one animator of an animation on one control. It
/// is the observable that is bound to the animated property.
pub(crate) struct AnimationInstance<A: Animator> {
    this: Weak<AnimationInstance<A>>,
    core: SingleSubscriberObservableBase<A::Value>,
    animator: Rc<A>,
    animation: Ref<Animation>,
    // Weak: the control's value store owns the binding that owns this
    // instance.
    target_control: WeakRef<Animatable>,
    visual_target: Option<WeakRef<Visual>>,
    on_complete_action: Option<Rc<dyn Fn()>>,
    timer_sub: RefCell<Option<Rc<dyn IDisposable>>>,
    property_changed_sub: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_changed_sub: RefCell<Option<Rc<dyn IDisposable>>>,
    detached_sub: RefCell<Option<Rc<dyn IDisposable>>>,
    base_clock: Rc<dyn IClock>,
    clock: RefCell<Option<Rc<Clock>>>,
    ease_func: RefCell<Easing>,
    neutral_value: RefCell<A::Value>,
    fill_mode: Cell<FillMode>,
    is_first_frame: Cell<bool>,
    is_in_first_initial_delay: Cell<bool>,
    last_interp_value: RefCell<A::Value>,
    initial_kf_value: RefCell<A::Value>,
    iteration_count: Cell<Option<i64>>,
    initial_delay: Cell<TimeSpan>,
    iteration_delay: Cell<TimeSpan>,
    duration: Cell<TimeSpan>,
    time_prev: Cell<TimeSpan>,
    anim_time_prev: Cell<f64>,
    playback_direction: Cell<PlaybackDirection>,
    playback_direction_prev: Cell<PlaybackDirection>,
    speed_ratio: Cell<f64>,
    speed_ratio_prev: Cell<f64>,
    time_moves_backwards: Cell<bool>,
    time_of_last_change: Cell<TimeSpan>,
    anim_time_of_last_change: Cell<f64>,
    should_pause_on_invisible: bool,
}

/// Holds the instance strongly: a running animation stays alive until it
/// completes or is disposed, whether or not its subscription is kept.
struct StepObserver<A: Animator>(Rc<AnimationInstance<A>>);

impl<A: Animator> IObserver<TimeSpan> for StepObserver<A> {
    fn on_next(&self, value: TimeSpan) {
        self.0.step(value);
    }
}

impl<A: Animator> AnimationInstance<A> {
    /// Creates the instance. Panics when a setting of the animation is
    /// invalid.
    pub(crate) fn new(
        animation: Ref<Animation>,
        control: &Ref<Animatable>,
        animator: Rc<A>,
        base_clock: Rc<dyn IClock>,
        on_complete: Option<Rc<dyn Fn()>>,
        should_pause_on_invisible: bool,
        visual_target: Option<&Ref<Visual>>,
    ) -> Rc<Self> {
        let ease_func = animation.easing();
        let instance = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            core: SingleSubscriberObservableBase::new(),
            animator,
            animation,
            target_control: control.downgrade(),
            visual_target: visual_target.map(Ref::downgrade),
            on_complete_action: on_complete,
            timer_sub: RefCell::new(None),
            property_changed_sub: RefCell::new(None),
            visibility_changed_sub: RefCell::new(None),
            detached_sub: RefCell::new(None),
            base_clock,
            clock: RefCell::new(None),
            ease_func: RefCell::new(ease_func),
            neutral_value: RefCell::new(A::Value::default()),
            fill_mode: Cell::new(FillMode::None),
            is_first_frame: Cell::new(true),
            is_in_first_initial_delay: Cell::new(true),
            last_interp_value: RefCell::new(A::Value::default()),
            initial_kf_value: RefCell::new(A::Value::default()),
            iteration_count: Cell::new(None),
            initial_delay: Cell::new(TimeSpan::ZERO),
            iteration_delay: Cell::new(TimeSpan::ZERO),
            duration: Cell::new(TimeSpan::ZERO),
            time_prev: Cell::new(TimeSpan::ZERO),
            anim_time_prev: Cell::new(0.0),
            playback_direction: Cell::new(PlaybackDirection::Normal),
            playback_direction_prev: Cell::new(PlaybackDirection::Normal),
            speed_ratio: Cell::new(1.0),
            speed_ratio_prev: Cell::new(1.0),
            time_moves_backwards: Cell::new(false),
            time_of_last_change: Cell::new(TimeSpan::ZERO),
            anim_time_of_last_change: Cell::new(0.0),
            should_pause_on_invisible,
        });
        if let Err(error) = instance.fetch_properties() {
            panic!("{error}");
        }
        instance
    }

    fn fetch_properties(&self) -> Result<(), AnimationError> {
        let animation = &self.animation;
        *self.ease_func.borrow_mut() = animation.easing();

        self.speed_ratio_prev.set(self.speed_ratio.get());
        self.speed_ratio.set(animation.speed_ratio());

        if self.speed_ratio.get() < 0.0 {
            return Err(AnimationError("SpeedRatio value cannot be negative.".to_string()));
        }

        self.initial_delay.set(animation.delay());

        if self.initial_delay.get() < TimeSpan::ZERO {
            return Err(AnimationError("Delay value cannot be negative.".to_string()));
        }

        self.duration.set(animation.duration());

        if self.duration.get() < TimeSpan::ZERO {
            return Err(AnimationError("Duration value cannot be negative.".to_string()));
        }

        self.iteration_delay.set(animation.delay_between_iterations());

        if self.iteration_delay.get() < TimeSpan::ZERO {
            return Err(AnimationError("DelayBetweenIterations value cannot be negative.".to_string()));
        }

        let iteration_count = animation.iteration_count();
        if iteration_count.repeat_type() == IterationType::Many {
            if iteration_count.value() > i64::MAX as u64 {
                return Err(AnimationError(
                    "IterationCount value cannot be larger than long.MaxValue.".to_string(),
                ));
            }
            self.iteration_count.set(Some(iteration_count.value() as i64));
        } else {
            self.iteration_count.set(None);
        }

        self.playback_direction_prev.set(self.playback_direction.get());
        self.playback_direction.set(animation.playback_direction());
        self.fill_mode.set(animation.fill_mode());
        Ok(())
    }

    fn unsubscribed(&self) {
        // Guard against reentrancy: `do_complete` can trigger `unsubscribed`
        // via the completion action's disposal chain, and then the completion
        // itself calls it again.
        let timer_sub = self.timer_sub.borrow_mut().take();
        let Some(timer_sub) = timer_sub else { return };

        // Animation may have been stopped before it has finished.
        if self.can_apply_final_fill() {
            let value = self.last_interp_value.borrow().clone();
            self.apply_final_fill(value);
        }

        let property_changed_sub = self.property_changed_sub.borrow_mut().take();
        if let Some(subscription) = property_changed_sub {
            subscription.dispose();
        }
        timer_sub.dispose();

        let visibility_changed_sub = self.visibility_changed_sub.borrow_mut().take();
        if let Some(subscription) = visibility_changed_sub {
            subscription.dispose();
        }
        let detached_sub = self.detached_sub.borrow_mut().take();
        if let Some(subscription) = detached_sub {
            subscription.dispose();
        }

        let clock = self.clock.borrow().clone();
        if let Some(clock) = clock {
            clock.set_play_state(PlayState::Stop);
        }
    }

    fn subscribed(&self) {
        let clock = Clock::with_parent(&self.base_clock);
        *self.clock.borrow_mut() = Some(clock.clone());
        let Some(this) = self.this.upgrade() else { return };
        let timer_sub = clock.subscribe(Rc::new(StepObserver(this)));
        *self.timer_sub.borrow_mut() = Some(timer_sub);

        if let Some(visual) = self.visual_target.as_ref().and_then(WeakRef::upgrade) {
            if self.should_pause_on_invisible {
                let weak_clock = Rc::downgrade(&clock);
                let weak_visual = visual.downgrade();
                let subscription = visual.is_effectively_visible_changed(move || {
                    let (Some(clock), Some(visual)) = (weak_clock.upgrade(), weak_visual.upgrade()) else { return };
                    if clock.play_state() == PlayState::Stop {
                        return;
                    }
                    if visual.is_effectively_visible() {
                        if clock.play_state() == PlayState::Pause {
                            clock.set_play_state(PlayState::Run);
                        }
                    } else if clock.play_state() == PlayState::Run {
                        clock.set_play_state(PlayState::Pause);
                    }
                });
                *self.visibility_changed_sub.borrow_mut() = Some(subscription);

                // If already invisible when animation starts, pause immediately.
                if !visual.is_effectively_visible() {
                    clock.set_play_state(PlayState::Pause);
                }
            }

            // Stop and dispose the animation when detached from the visual tree.
            let this = self.this.clone();
            let subscription = visual.detached_from_visual_tree(move |_| {
                if let Some(this) = this.upgrade() {
                    this.do_complete(false);
                }
            });
            *self.detached_sub.borrow_mut() = Some(subscription);
        }

        if let Some(control) = self.target_control.upgrade() {
            let this = self.this.clone();
            let property = self.animator.base().property();
            let subscription = control.property_changed(move |e| {
                if Some(e.property()) == property && e.priority() > BindingPriority::Animation {
                    if let Some(this) = this.upgrade() {
                        this.update_neutral_value();
                    }
                }
            });
            *self.property_changed_sub.borrow_mut() = Some(subscription);
        }
        self.update_neutral_value();
    }

    pub(crate) fn step(&self, frame_tick: TimeSpan) {
        let paused = self.clock.borrow().as_ref().is_some_and(|clock| clock.play_state() == PlayState::Pause);
        if paused {
            return;
        }

        // The animated object is gone without the animation having been
        // disposed: end it, which releases the clock.
        if self.target_control.upgrade().is_none() {
            self.do_complete(true);
            return;
        }

        if let Err(error) = self.internal_step(frame_tick) {
            let error: ObservableError = Rc::new(error);
            self.core.publish_error(error, || self.unsubscribed());
        }
    }

    fn can_apply_final_fill(&self) -> bool {
        matches!(self.fill_mode.get(), FillMode::Forward | FillMode::Both)
    }

    fn apply_final_fill(&self, interpolated_value: A::Value) {
        let property = self.animator.base().styled_property::<A::Value>();
        if let Some(control) = self.target_control.upgrade() {
            control.set_value(property, interpolated_value);
        }
    }

    fn do_complete(&self, stop_as_is: bool) {
        if !stop_as_is {
            // Update the last value because the completion might perform the
            // final fill, and the filled value has to be snapped to the last
            // value that the animation would reach if it was left running in
            // its current state.
            let is_iteration_reversed = Self::is_iteration_reversed(
                self.is_alternating_playback_direction(),
                self.iteration_count.get().map(|count| count + 1),
            );
            let value = self.find_edge_value(self.is_anim_time_going_backwards(), is_iteration_reversed);
            *self.last_interp_value.borrow_mut() = value;
        }

        if let Some(on_complete) = &self.on_complete_action {
            on_complete();
        }
        self.core.publish_completed(|| self.unsubscribed());
    }

    fn do_initial_delay(&self) {
        if self.is_in_first_initial_delay.get() && !matches!(self.fill_mode.get(), FillMode::Backward | FillMode::Both)
        {
            return;
        }

        let value = self.initial_kf_value.borrow().clone();
        self.core.publish_next(value);
    }

    fn do_iteration_delay(&self, is_iteration_reversed: bool) {
        let value = self.find_edge_value(false, is_iteration_reversed);
        *self.last_interp_value.borrow_mut() = value.clone();
        self.core.publish_next(value);
    }

    fn do_play_states(&self) {
        let clock_stopped = self.clock.borrow().as_ref().is_some_and(|clock| clock.play_state() == PlayState::Stop);
        if clock_stopped || self.base_clock.play_state() == PlayState::Stop {
            self.do_complete(true);
        }

        if self.is_first_frame.get() {
            // In first frame of animation we determine the expected direction of time.
            let direction = self.animation.playback_direction();
            self.time_moves_backwards
                .set(matches!(direction, PlaybackDirection::Reverse | PlaybackDirection::AlternateReverse));

            let base = self.animator.base();
            let count = base.count();
            assert!(count != 0, "Sequence contains no elements");
            let initial_key_frame = if self.time_moves_backwards.get() { base.get(count - 1) } else { base.get(0) };
            let initial_value = initial_key_frame
                .with_value(|value| value.and_then(|value| self.animator.typed_value(value)))
                .unwrap_or_else(|| self.neutral_value.borrow().clone());
            *self.initial_kf_value.borrow_mut() = initial_value;

            self.is_first_frame.set(false);
        }
    }

    fn is_iteration_reversed(is_alternating_playback_direction: bool, iteration_index: Option<i64>) -> bool {
        if is_alternating_playback_direction {
            // An unknown index (an infinite animation) compares unequal to
            // zero.
            iteration_index.is_none_or(|index| (index & 1) != 0)
        } else {
            false
        }
    }

    fn find_edge_value(&self, is_anim_time_going_backwards: bool, is_iteration_reversed: bool) -> A::Value {
        let final_ease_value = if is_anim_time_going_backwards ^ is_iteration_reversed { 0.0 } else { 1.0 };
        self.interpolate(final_ease_value)
    }

    /// Eases `normalized` and interpolates the key frames at the result.
    fn interpolate(&self, normalized: f64) -> A::Value {
        let ease_func = self.ease_func.borrow().clone();
        let eased_time = ease_func.ease(normalized);
        let neutral_value = self.neutral_value.borrow().clone();
        interpolation_handler(&*self.animator, eased_time, &neutral_value)
    }

    fn is_anim_time_going_backwards(&self) -> bool {
        matches!(self.playback_direction.get(), PlaybackDirection::Reverse | PlaybackDirection::AlternateReverse)
    }

    fn is_alternating_playback_direction(&self) -> bool {
        matches!(self.playback_direction.get(), PlaybackDirection::Alternate | PlaybackDirection::AlternateReverse)
    }

    /// Handles all possible cases of applying the initial delay and
    /// clamping. Returns whether the time is inside the delay.
    fn apply_initial_delay(&self, anim_time: &mut i64) -> bool {
        let delay = self.initial_delay.get().ticks() / PRECISION_IN_TICKS;
        if self.iteration_count.get().is_some() {
            *anim_time -= delay;
            if *anim_time <= 0 {
                if *anim_time < -delay {
                    *anim_time = -delay;
                }
                return true;
            }
        } else {
            // Determine the interval of initial delay.
            let low = -delay;
            let high = 0;

            // Handle all 3 location cases based on above interval.
            if *anim_time < low {
                *anim_time += delay;
                if *anim_time > 0 {
                    *anim_time = 0;
                    return true;
                }
            } else if *anim_time > high {
                *anim_time -= delay;
                if *anim_time < 0 {
                    *anim_time = 0;
                    return true;
                }
            } else {
                *anim_time = 0;
                return true;
            }
        }
        false
    }

    fn apply_limited_clamp(&self, anim_time: &mut f64) {
        if self.time_moves_backwards.get() {
            if *anim_time > 0.0 {
                *anim_time = 0.0;
            }
        } else if *anim_time < 0.0 {
            *anim_time = 0.0;
        }
    }

    fn internal_step(&self, time: TimeSpan) -> Result<(), AnimationError> {
        self.do_play_states();

        self.fetch_properties()?;

        if self.speed_ratio.get() != self.speed_ratio_prev.get()
            || self.playback_direction.get() != self.playback_direction_prev.get()
        {
            // Remember the time when speed changed.
            // All we can know is that it changed some time between current and prev frame.
            // We assume that this happened exactly at prev frame.
            self.time_of_last_change.set(self.time_prev.get());
            self.anim_time_of_last_change.set(self.anim_time_prev.get());
        }

        let is_anim_time_going_backwards = self.is_anim_time_going_backwards();

        // Combine SpeedRatio and PlaybackDirection into a single signed speed ratio.
        let speed_ratio = if is_anim_time_going_backwards { -self.speed_ratio.get() } else { self.speed_ratio.get() };

        // Calculate animation time. That's time that has passed inside
        // the animation since its beginning.
        let time_since_last_change = time - self.time_of_last_change.get();
        let anim_time_since_last_change =
            time_since_last_change.ticks() as f64 * speed_ratio / PRECISION_IN_TICKS as f64;
        let mut anim_time_exact = self.anim_time_of_last_change.get() + anim_time_since_last_change;

        let iteration_count = self.iteration_count.get();
        if iteration_count.is_some() {
            // Make sure animation time is inside a valid interval.
            self.apply_limited_clamp(&mut anim_time_exact);
        }

        self.time_prev.set(time);
        self.anim_time_prev.set(anim_time_exact);
        let anim_time = anim_time_exact as i64;

        // Get animation time oriented in the direction of time. Animation running in
        // same direction as it was on first frame will always increase this variable.
        let mut anim_time_positive = if self.time_moves_backwards.get() { -anim_time } else { anim_time };

        if self.initial_delay.get() > TimeSpan::ZERO {
            let is_currently_inside_delay = self.apply_initial_delay(&mut anim_time_positive);
            if is_currently_inside_delay {
                self.do_initial_delay();
                return Ok(());
            }
            self.is_in_first_initial_delay.set(false);
        }

        let iter_duration = self.duration.get().ticks() / PRECISION_IN_TICKS;
        let iter_delay = self.iteration_delay.get().ticks() / PRECISION_IN_TICKS;
        let iter_duration_total = iter_duration + iter_delay;

        // An iteration with no duration can't be interpolated, no matter how long the
        // delay between iterations is: end the animation and snap to its final value.
        if iter_duration <= 0 {
            self.do_complete(false);
            return Ok(());
        }

        // Calculate current iteration info.
        let mut iter_index = anim_time_positive / iter_duration_total;
        let mut iter_time = anim_time_positive % iter_duration_total;

        let mut playback_reversed = anim_time_positive < 0;

        if playback_reversed {
            // Animation time is behind the starting point of animation.

            // First negative iteration has index -1, first positive iteration has index 0.
            iter_index -= 1;

            // Move iteration delay to the front of iteration, which is (when moving
            // backwards through animation time) effectively at the end of iteration.
            iter_time = -iter_time;
        }

        playback_reversed ^= self.time_moves_backwards.get()
            ^ Self::is_iteration_reversed(self.is_alternating_playback_direction(), Some(iter_index));

        let iters_until_end = iteration_count.map(|count| count - iter_index);

        // End animation when limit is reached.
        if iters_until_end.is_some_and(|iters| iters <= 0) {
            self.do_complete(false);
            return Ok(());
        }

        if iter_time > iter_duration && iter_time <= iter_duration_total && iter_delay > 0 {
            // The last iteration's trailing delay should be skipped.
            if iters_until_end.is_some_and(|iters| iters <= 1) {
                self.do_complete(false);
                return Ok(());
            }

            self.do_iteration_delay(playback_reversed);
        } else if iter_time <= iter_duration {
            // Ease and interpolate.
            let mut normalized = iter_time as f64 / iter_duration as f64;
            if playback_reversed {
                normalized = 1.0 - normalized;
            }

            let value = self.interpolate(normalized);
            *self.last_interp_value.borrow_mut() = value.clone();
            self.core.publish_next(value);
        }
        Ok(())
    }

    fn update_neutral_value(&self) {
        let property = self.animator.base().styled_property::<A::Value>();
        let Some(control) = self.target_control.upgrade() else { return };

        let value = match control.get_base_value(property) {
            Some(base_value) => base_value,
            None => control.get_value(property),
        };
        *self.neutral_value.borrow_mut() = value;
    }
}

impl<A: Animator> IObservable<A::Value> for AnimationInstance<A> {
    fn subscribe(&self, observer: Rc<dyn IObserver<A::Value>>) -> Rc<dyn IDisposable> {
        self.core.subscribe(observer, || self.subscribed());
        self.this.upgrade().expect("the instance is alive while it is being subscribed to")
    }
}

impl<A: Animator> IDisposable for AnimationInstance<A> {
    fn dispose(&self) {
        self.core.dispose(|| self.unsubscribed());
    }
}
