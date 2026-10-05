use crate::animation::cancellation::register_local;
use crate::animation::easings::Easing;
use crate::animation::key_frame::KeyFrameTimingMode;
use crate::animation::{
    Animatable, AnimationError, AnimationTask, AnimatorFactory, AnimatorKeyFrame, Cue, FillMode, IAnimator, IClock,
    IterationCount, KeyFrames, PlaybackBehavior, PlaybackDirection, TimeSpan,
};
use crate::data::BindingMode;
use crate::reactive::{CompositeDisposable, IDisposable, IObservable, Observable};
use crate::animation::i_page_transition::continuation_context;
use crate::threading::CancellationToken;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, DirectProperty, DirectPropertyMetadata, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, Visual,
};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Tracks the progress of an animation.
#[repr(C)]
pub struct Animation {
    base: FerroObject,
    duration: Cell<TimeSpan>,
    iteration_count: Cell<IterationCount>,
    playback_direction: Cell<PlaybackDirection>,
    playback_behavior: Cell<PlaybackBehavior>,
    fill_mode: Cell<FillMode>,
    easing: RefCell<Easing>,
    delay: Cell<TimeSpan>,
    delay_between_iterations: Cell<TimeSpan>,
    speed_ratio: Cell<f64>,
    children: KeyFrames,
}

ferro_class!(Animation: FerroObject);
crate::ferro_class_info!(Animation { new: Animation::new, interfaces: [Rc<dyn crate::animation::IAnimation>] });
ferro_impl_classes!(Animation: FerroObjectImpl);

macro_rules! direct_cell_property {
    ($(#[$meta:meta])* $accessor:ident, $name:literal, $ty:ty, $getter:ident, $setter:ident, $field:ident, $unset:expr) => {
        ferro_property!(for Animation;
            $(#[$meta])*
            pub fn $accessor() -> DirectProperty<Animation, $ty> {
                FerroProperty::register_direct::<Animation, _>($name, |o| o.$getter(), Some(|o, v| o.$setter(v)), $unset)
            }
        );

        pub fn $getter(&self) -> $ty {
            self.$field.get()
        }

        pub fn $setter(&self, value: $ty) {
            self.set_and_raise_cell(Self::$accessor(), &self.$field, value);
        }
    };
}

crate::ferro_properties! {
    impl Animation, also [
        Animation::duration_property,
        Animation::iteration_count_property,
        Animation::playback_direction_property,
        Animation::playback_behavior_property,
        Animation::fill_mode_property,
        Animation::delay_property,
        Animation::delay_between_iterations_property,
        Animation::easing_property,
        Animation::speed_ratio_property,
    ] {}
}

impl Animation {
    direct_cell_property!(
        /// Defines the `Duration` property: the active time of one iteration.
        duration_property, "Duration", TimeSpan, duration, set_duration, duration, TimeSpan::ZERO
    );

    direct_cell_property!(
        /// Defines the `IterationCount` property: the repeat count.
        iteration_count_property, "IterationCount", IterationCount, iteration_count, set_iteration_count,
        iteration_count, IterationCount::new(1)
    );

    direct_cell_property!(
        /// Defines the `PlaybackDirection` property.
        playback_direction_property, "PlaybackDirection", PlaybackDirection, playback_direction,
        set_playback_direction, playback_direction, PlaybackDirection::Normal
    );

    direct_cell_property!(
        /// Defines the `PlaybackBehavior` property: whether the animation
        /// pauses when its target is not effectively visible.
        playback_behavior_property, "PlaybackBehavior", PlaybackBehavior, playback_behavior,
        set_playback_behavior, playback_behavior, PlaybackBehavior::Auto
    );

    direct_cell_property!(
        /// Defines the `FillMode` property: the value fill mode.
        fill_mode_property, "FillMode", FillMode, fill_mode, set_fill_mode, fill_mode, FillMode::None
    );

    direct_cell_property!(
        /// Defines the `Delay` property: the initial delay time.
        delay_property, "Delay", TimeSpan, delay, set_delay, delay, TimeSpan::ZERO
    );

    direct_cell_property!(
        /// Defines the `DelayBetweenIterations` property.
        delay_between_iterations_property, "DelayBetweenIterations", TimeSpan, delay_between_iterations,
        set_delay_between_iterations, delay_between_iterations, TimeSpan::ZERO
    );

    ferro_property!(for Animation;
        /// Defines the `Easing` property: the easing function to be used.
        pub fn easing_property() -> DirectProperty<Animation, Easing> {
            FerroProperty::register_direct::<Animation, _>(
                "Easing",
                |o| o.easing(),
                Some(|o, v| o.set_easing(v)),
                Easing::default(),
            )
        }
    );

    ferro_property!(for Animation;
        /// Defines the `SpeedRatio` property: the speed multiple.
        pub fn speed_ratio_property() -> DirectProperty<Animation, f64> {
            FerroProperty::register_direct_with::<Animation, _>(
                "SpeedRatio",
                |o| o.speed_ratio(),
                Some(|o, v| o.set_speed_ratio(v)),
                DirectPropertyMetadata::new(Some(1.0)).with_default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            duration: Cell::new(TimeSpan::ZERO),
            iteration_count: Cell::new(IterationCount::new(1)),
            playback_direction: Cell::new(PlaybackDirection::Normal),
            playback_behavior: Cell::new(PlaybackBehavior::Auto),
            fill_mode: Cell::new(FillMode::None),
            easing: RefCell::new(Easing::default()),
            delay: Cell::new(TimeSpan::ZERO),
            delay_between_iterations: Cell::new(TimeSpan::ZERO),
            speed_ratio: Cell::new(1.0),
            children: KeyFrames::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The easing function to be used for this animation.
    pub fn easing(&self) -> Easing {
        self.easing.borrow().clone()
    }

    pub fn set_easing(&self, value: impl Into<Easing>) {
        self.set_and_raise(Self::easing_property(), &self.easing, value.into());
    }

    /// The speed multiple for this animation.
    pub fn speed_ratio(&self) -> f64 {
        self.speed_ratio.get()
    }

    pub fn set_speed_ratio(&self, value: f64) {
        self.set_and_raise_cell(Self::speed_ratio_property(), &self.speed_ratio, value);
    }

    /// The children of the animation: its key frames.
    pub fn children(&self) -> KeyFrames {
        self.children.clone()
    }

    #[allow(clippy::type_complexity)]
    fn interpret_keyframes(&self, control: &Animatable) -> (Vec<Rc<dyn IAnimator>>, Vec<Rc<dyn IDisposable>>, bool) {
        let mut handler_list: Vec<((TypeId, &'static FerroProperty), AnimatorFactory)> = Vec::new();
        let mut animator_key_frames: Vec<Ref<AnimatorKeyFrame>> = Vec::new();
        let mut subscriptions: Vec<Rc<dyn IDisposable>> = Vec::new();
        let mut animates_visibility = false;

        for keyframe in self.children.snapshot().iter() {
            for setter in keyframe.setters().snapshot().iter() {
                let Some(property) = setter.property() else {
                    panic!("No Setter property assigned.");
                };

                if property == Visual::is_visible_property().as_property() {
                    animates_visibility = true;
                }

                let handler = Animation::get_animator(setter).or_else(|| Animation::get_animator_type(property));

                let Some((type_, factory)) = handler else {
                    panic!(
                        "No animator registered for the property {property}. Add an animator to the Animation.Animators collection that matches this property to animate it."
                    );
                };

                if !handler_list.iter().any(|((t, p), _)| *t == type_ && *p == property) {
                    handler_list.push(((type_, property), factory.clone()));
                }

                let mut cue = keyframe.cue();

                if keyframe.timing_mode() == KeyFrameTimingMode::TimeSpan {
                    cue = Cue::new(keyframe.key_time().total_seconds() / self.duration().total_seconds());
                }

                let new_kf = AnimatorKeyFrame::new(Some(type_), Some(factory), cue, keyframe.key_spline());

                subscriptions.push(new_kf.bind_setter(&**setter, control));

                animator_key_frames.push(new_kf);
            }
        }

        animator_key_frames.sort_by(|x, y| x.cue().cue_value().total_cmp(&y.cue().cue_value()));

        let mut new_animator_instances: Vec<Rc<dyn IAnimator>> = Vec::new();

        for ((_, property), factory) in &handler_list {
            let new_instance = factory();
            new_instance.set_property(Some(property));
            new_animator_instances.push(new_instance);
        }

        let fill_mode = self.fill_mode();
        for keyframe in animator_key_frames {
            let animator = new_animator_instances
                .iter()
                .find(|a| Some(a.animator_type()) == keyframe.animator_type() && a.property() == keyframe.property())
                .expect("Sequence contains no matching element");

            if animator.count() == 0 && matches!(fill_mode, FillMode::Backward | FillMode::Both) {
                keyframe.set_fill_before(true);
            }

            animator.add(keyframe);
        }

        if matches!(fill_mode, FillMode::Forward | FillMode::Both) {
            for new_animator_instance in &new_animator_instances {
                let count = new_animator_instance.count();
                if count > 0 {
                    new_animator_instance.get(count - 1).set_fill_after(true);
                }
            }
        }

        (new_animator_instances, subscriptions, animates_visibility)
    }

    /// Applies the animation to `control`: it runs while `match_` is true,
    /// on `clock` (or on the control's clock, or on the global clock).
    /// `on_complete` runs when the animation ends by itself.
    /// `is_manually_started` tells an animation started by code from one
    /// started by a style. Disposing the result removes the animation.
    pub fn apply(
        &self,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        match_: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        is_manually_started: bool,
    ) -> Rc<dyn IDisposable> {
        let (animators, mut subscriptions, animates_visibility) = self.interpret_keyframes(control);

        let should_pause_on_invisible = match self.playback_behavior.get() {
            PlaybackBehavior::Auto => !(animates_visibility || is_manually_started),
            PlaybackBehavior::Always => false,
            PlaybackBehavior::OnlyIfVisible => true,
        };

        if animators.len() == 1 {
            let subscription = animators[0].clone().apply(
                self,
                control,
                clock,
                match_,
                on_complete,
                should_pause_on_invisible,
            );

            if let Some(subscription) = subscription {
                subscriptions.push(subscription);
            }
        } else {
            // The completion action runs once every animator has completed,
            // through the synchronization context that is current now.
            let remaining = Rc::new(Cell::new(animators.len()));
            let schedule_on_complete = on_complete.map(|on_complete| {
                let context = continuation_context();
                Rc::new(move || {
                    let on_complete = on_complete.clone();
                    context.post_local(move || on_complete());
                })
            });

            for animator in &animators {
                let animator_on_complete: Option<Rc<dyn Fn()>> = schedule_on_complete.as_ref().map(|schedule| {
                    let schedule = schedule.clone();
                    let remaining = remaining.clone();
                    let completed = Cell::new(false);
                    let action: Rc<dyn Fn()> = Rc::new(move || {
                        if completed.replace(true) {
                            return;
                        }
                        remaining.set(remaining.get() - 1);
                        if remaining.get() == 0 {
                            schedule();
                        }
                    });
                    action
                });

                let subscription = animator.clone().apply(
                    self,
                    control,
                    clock.clone(),
                    match_.clone(),
                    animator_on_complete,
                    should_pause_on_invisible,
                );

                if let Some(subscription) = subscription {
                    subscriptions.push(subscription);
                }
            }

            if animators.is_empty() {
                if let Some(schedule) = &schedule_on_complete {
                    schedule();
                }
            }
        }

        Rc::new(CompositeDisposable::from_disposables(subscriptions))
    }

    /// Starts the animation on `control` and returns a task that completes
    /// when it has ended or `cancellation_token` was cancelled.
    pub fn run_async(&self, control: &Animatable, cancellation_token: CancellationToken) -> AnimationTask {
        self.run_async_with_clock(control, None, cancellation_token)
    }

    /// [`run_async`](Self::run_async) on an explicit clock.
    pub fn run_async_with_clock(
        &self,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        cancellation_token: CancellationToken,
    ) -> AnimationTask {
        if cancellation_token.is_cancellation_requested() {
            return AnimationTask::completed();
        }

        let run = AnimationTask::pending();

        if self.iteration_count() == IterationCount::INFINITE {
            run.try_set(Err(AnimationError("Looping animations must not use the Run method.".to_string())));
        }

        let subscriptions: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let cancellation = Rc::new(RefCell::new(None));

        let finish: Rc<dyn Fn()> = {
            let run = run.clone();
            let subscriptions = subscriptions.clone();
            let cancellation = cancellation.clone();
            Rc::new(move || {
                run.try_set_result();
                let subscriptions = subscriptions.borrow_mut().take();
                if let Some(subscriptions) = subscriptions {
                    subscriptions.dispose();
                }
                let cancellation: Option<crate::animation::cancellation::LocalCancellationRegistration> =
                    cancellation.borrow_mut().take();
                if let Some(cancellation) = cancellation {
                    cancellation.dispose();
                }
            })
        };

        let applied = self.apply(control, clock, Observable::return_(true), Some(finish.clone()), true);
        // The animation may have completed while it was being applied, in
        // which case there is nothing left to dispose on cancellation.
        *subscriptions.borrow_mut() = Some(applied);

        let registration = register_local(&cancellation_token, move || finish());
        if !run.is_completed() || !cancellation_token.is_cancellation_requested() {
            *cancellation.borrow_mut() = Some(registration);
        }

        run
    }
}
