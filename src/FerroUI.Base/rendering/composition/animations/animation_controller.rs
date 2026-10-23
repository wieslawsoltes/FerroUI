//! WinUI's `AnimationController`: pausing, resuming, scrubbing and speeding up
//! running animations, and the global playback rate of the compositor
//! (`Compositor.GlobalPlaybackRate`).
//!
//! # Design
//!
//! A controller is a live [`CompositionPropertySet`] with the entries
//! `Progress`, `PlaybackRate`, `IsPaused`, `ProgressBehavior` and
//! `ProgressRevision`. The UI-thread members write the entries; the animation
//! instances the controller controls hold the id of the set and read the
//! entries on the server every frame through an [`AnimationControllerClock`],
//! which maps the time of the compositor to the time of the animation:
//!
//! - while not paused, the animation time advances by the elapsed compositor
//!   time multiplied by `PlaybackRate` (a negative rate plays backwards, down
//!   to the start);
//! - when `Progress` changes (set by the application, or animated: an
//!   expression animation of `Progress` scrubs the animation), the animation
//!   time jumps to that point of the animation;
//! - a paused animation does not end, so a scrubbed animation stays alive.
//!
//! Because the controller is a property set, its `Progress` and
//! `PlaybackRate` can be animated with `start_animation`, and an expression
//! can refer to the controller as a reference parameter.
//!
//! The global playback rate scales the clock of all the animations of the
//! compositor on the server ([`ServerCompositorAnimations::animation_time`](crate::rendering::composition::server::ServerCompositorAnimations::animation_time)).

use super::{AnimationInstanceFactory, ICompositionAnimation, ICompositionAnimationBase};
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::{
    CompositionProperty, ServerCompositionPropertySet, ServerCompositor, ServerObjectId,
};
use crate::rendering::composition::{
    AsCompositionObject, CompositionObject, CompositionPropertySet, Compositor, ICompositionObjectAnimations,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The entry of the progress (0..1).
const PROGRESS: &str = "Progress";
/// The entry of the playback rate.
const PLAYBACK_RATE: &str = "PlaybackRate";
/// The entry that tells whether the controlled animations are paused.
const IS_PAUSED: &str = "IsPaused";
/// The entry of the progress behavior: 0 for `Default`, 1 for
/// `IncludesDelayTime`.
const PROGRESS_BEHAVIOR: &str = "ProgressBehavior";
/// The entry counting the assignments of `Progress`, so that assigning the
/// same value again moves the animation back to it.
const PROGRESS_REVISION: &str = "ProgressRevision";

/// What the progress of an [`AnimationController`] covers (WinUI
/// `AnimationControllerProgressBehavior`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnimationControllerProgressBehavior {
    /// The progress covers the animation after its delay.
    #[default]
    Default,
    /// The progress covers the delay and the animation.
    IncludesDelayTime,
}

/// Controls running animations: pauses and resumes them, moves them to a
/// point (`Progress`) and changes their speed (`PlaybackRate`) (WinUI
/// `AnimationController`).
///
/// An animation comes under a controller when it is started with
/// [`start_animation_with_controller`](ICompositionObjectAnimations::start_animation_with_controller),
/// or through [`try_get_animation_controller`](ICompositionObjectAnimations::try_get_animation_controller)
/// of the object it animates. Key frame and natural motion animations can be
/// controlled; expression animations are not affected. One controller can
/// control several animations.
///
/// `Progress` and `PlaybackRate` can be animated
/// (`controller.start_animation("Progress", expression)`), and the controller
/// can be a reference parameter of an expression (`controller.Progress`).
pub struct AnimationController {
    properties: Rc<CompositionPropertySet>,
    progress_revision: Cell<f32>,
}

impl AnimationController {
    /// The highest playback rate (WinUI `MaxPlaybackRate`).
    pub const MAX_PLAYBACK_RATE: f32 = 1000.0;
    /// The lowest playback rate (WinUI `MinPlaybackRate`); a negative rate
    /// plays backwards.
    pub const MIN_PLAYBACK_RATE: f32 = -1000.0;

    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<AnimationController> {
        let properties = CompositionPropertySet::new(compositor);
        properties.insert_scalar(PROGRESS, 0.0);
        properties.insert_scalar(PLAYBACK_RATE, 1.0);
        properties.insert_boolean(IS_PAUSED, false);
        properties.insert_scalar(PROGRESS_BEHAVIOR, 0.0);
        properties.insert_scalar(PROGRESS_REVISION, 0.0);
        Rc::new(AnimationController { properties, progress_revision: Cell::new(0.0) })
    }

    /// The compositor of the controller.
    pub fn compositor(&self) -> &Rc<Compositor> {
        self.properties.compositor()
    }

    /// Pauses the controlled animations: their time stops, and they do not end
    /// while paused.
    pub fn pause(&self) {
        self.properties.insert_boolean(IS_PAUSED, true)
    }

    /// Resumes the controlled animations from where they were paused.
    pub fn resume(&self) {
        self.properties.insert_boolean(IS_PAUSED, false)
    }

    /// Whether the controller is paused (not a WinUI member).
    pub fn is_paused(&self) -> bool {
        self.properties.try_get_boolean(IS_PAUSED).1
    }

    /// The speed of the controlled animations: 1 is normal speed, 2 twice as
    /// fast, a negative rate plays backwards.
    pub fn playback_rate(&self) -> f32 {
        self.properties.try_get_scalar(PLAYBACK_RATE).1
    }

    /// Sets the playback rate, clamped to
    /// [`MIN_PLAYBACK_RATE`](Self::MIN_PLAYBACK_RATE)..[`MAX_PLAYBACK_RATE`](Self::MAX_PLAYBACK_RATE).
    pub fn set_playback_rate(&self, value: f32) {
        let value = if value.is_nan() { 1.0 } else { value.clamp(Self::MIN_PLAYBACK_RATE, Self::MAX_PLAYBACK_RATE) };
        self.properties.insert_scalar(PLAYBACK_RATE, value)
    }

    /// The progress last assigned (0..1).
    //
    // Deviation (winui-composition.md): WinUI reads the progress of the
    // running animation; the UI thread cannot read the server, so this is the
    // value last assigned.
    pub fn progress(&self) -> f32 {
        self.properties.try_get_scalar(PROGRESS).1
    }

    /// Moves the controlled animations to a point between their start (0)
    /// and their end (1), clamped to that range. A playing animation goes on
    /// from there; a paused one stays there.
    pub fn set_progress(&self, value: f32) {
        let value = if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) };
        self.properties.insert_scalar(PROGRESS, value);
        self.progress_revision.set(self.progress_revision.get() + 1.0);
        self.properties.insert_scalar(PROGRESS_REVISION, self.progress_revision.get());
    }

    /// What the progress covers.
    pub fn progress_behavior(&self) -> AnimationControllerProgressBehavior {
        match self.properties.try_get_scalar(PROGRESS_BEHAVIOR).1 {
            v if v >= 0.5 => AnimationControllerProgressBehavior::IncludesDelayTime,
            _ => AnimationControllerProgressBehavior::Default,
        }
    }

    pub fn set_progress_behavior(&self, value: AnimationControllerProgressBehavior) {
        let value = match value {
            AnimationControllerProgressBehavior::Default => 0.0,
            AnimationControllerProgressBehavior::IncludesDelayTime => 1.0,
        };
        self.properties.insert_scalar(PROGRESS_BEHAVIOR, value)
    }

    /// The id of the server counterpart (the property set of the controller).
    pub(crate) fn server(&self) -> ServerObjectId {
        self.properties.as_composition_object().required_server()
    }

    /// Wraps the factory of an animation instance so that the instance comes
    /// under this controller before it starts.
    pub(crate) fn wrap(&self, factory: AnimationInstanceFactory) -> AnimationInstanceFactory {
        let controller = self.server();
        AnimationInstanceFactory::new(move || {
            let instance = factory.create();
            instance.attach_controller(controller);
            instance
        })
    }
}

impl AsCompositionObject for AnimationController {
    fn as_composition_object(&self) -> &CompositionObject {
        self.properties.as_composition_object()
    }

    fn composition_type_name(&self) -> &'static str {
        "AnimationController"
    }

    /// An expression refers to the controller through its property set
    /// (`controller.Progress`).
    fn as_property_set(self: Rc<Self>) -> Option<Rc<CompositionPropertySet>> {
        Some(self.properties.clone())
    }
}

impl ICompositionObjectAnimations for AnimationController {
    /// `Progress` and `PlaybackRate` can be animated.
    fn try_start_animation(
        &self,
        property_name: &str,
        animation: &dyn ICompositionAnimation,
        final_value: Option<ExpressionVariant>,
    ) -> bool {
        if property_name != PROGRESS && property_name != PLAYBACK_RATE {
            return false;
        }
        self.properties.try_start_animation(property_name, animation, final_value)
    }

    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty> {
        if property_name != PROGRESS && property_name != PLAYBACK_RATE {
            return None;
        }
        self.properties.get_composition_property(property_name)
    }

    fn composition_object(&self) -> &CompositionObject {
        self.properties.as_composition_object()
    }
}

/// An animation whose instance factory is made already: what the start path
/// of a property is given for an animation started with a controller (the
/// factory of the animation wrapped by [`AnimationController::wrap`]).
pub(crate) struct PreparedAnimation {
    factory: RefCell<Option<AnimationInstanceFactory>>,
}

impl PreparedAnimation {
    pub(crate) fn new(factory: AnimationInstanceFactory) -> PreparedAnimation {
        PreparedAnimation { factory: RefCell::new(Some(factory)) }
    }
}

impl ICompositionAnimationBase for PreparedAnimation {
    fn as_composition_animation(&self) -> Option<&dyn ICompositionAnimation> {
        Some(self)
    }
}

impl ICompositionAnimation for PreparedAnimation {
    fn target(&self) -> Option<String> {
        None
    }

    /// The prepared factory; the arguments were applied when it was made.
    /// Called once by the start path.
    fn create_instance(&self, _target_object: ServerObjectId, _final_value: Option<ExpressionVariant>) -> AnimationInstanceFactory {
        match self.factory.borrow_mut().take() {
            Some(factory) => factory,
            None => panic!("the instance of a prepared animation is created once"),
        }
    }
}

/// The span of an animation that the progress of a controller covers, in
/// ticks (100 ns): the delay before the animation and the length of the
/// animation after it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProgressSpan {
    pub(crate) delay: i64,
    pub(crate) active: i64,
}

/// The time of a controlled animation instance, on the server: maps the time
/// of the compositor to the time of the animation through the controller the
/// instance is attached to. Without a controller the time of the animation is
/// the time since it started.
#[derive(Default)]
pub(crate) struct AnimationControllerClock {
    /// The property set of the controller.
    controller: Cell<Option<ServerObjectId>>,
    resolved: RefCell<Option<Weak<ServerCompositionPropertySet>>>,
    /// The compositor time (ticks) of the last evaluation under the
    /// controller.
    last_now: Cell<Option<i64>>,
    /// The time of the animation (ticks since it started).
    time: Cell<f64>,
    /// The progress and its revision seen last.
    progress_seen: Cell<(f64, f64)>,
    paused: Cell<bool>,
}

impl AnimationControllerClock {
    /// Attaches the controller whose property set has the id `controller`.
    pub(crate) fn attach(&self, controller: ServerObjectId) {
        self.controller.set(Some(controller));
        *self.resolved.borrow_mut() = None;
        self.last_now.set(None);
    }

    /// Whether the controller paused the animation at the last evaluation.
    pub(crate) fn is_paused(&self) -> bool {
        self.paused.get()
    }

    fn controller_set(&self, compositor: Option<&Rc<ServerCompositor>>) -> Option<Rc<ServerCompositionPropertySet>> {
        let id = self.controller.get()?;
        if let Some(set) = self.resolved.borrow().as_ref().and_then(Weak::upgrade) {
            return Some(set);
        }
        let set = compositor?.get::<ServerCompositionPropertySet>(id)?;
        *self.resolved.borrow_mut() = Some(Rc::downgrade(&set));
        Some(set)
    }

    /// The time of the animation at the compositor time `now` (ticks), for an
    /// animation started at `started_at` (ticks): the ticks since the start of
    /// the animation, negative before it. `span` is what the progress of the
    /// controller covers, if the animation can be moved to a progress.
    pub(crate) fn elapsed(
        &self,
        compositor: Option<&Rc<ServerCompositor>>,
        now: i64,
        started_at: i64,
        span: Option<ProgressSpan>,
    ) -> i64 {
        let raw = now - started_at;
        if self.controller.get().is_none() {
            return raw;
        }
        let Some(set) = self.controller_set(compositor) else {
            // The controller is gone: the animation plays on at normal speed
            // from where it was.
            self.paused.set(false);
            return match self.last_now.get() {
                Some(last) => {
                    self.time.set(self.time.get() + (now - last) as f64);
                    self.last_now.set(Some(now));
                    self.time.get() as i64
                }
                None => raw,
            };
        };
        let number = |name: &str, default: f64| set.get_value(name).try_cast::<f64>().unwrap_or(default);
        let paused = set.get_value(IS_PAUSED).try_cast::<bool>().unwrap_or(false);
        let rate = number(PLAYBACK_RATE, 1.0);
        let progress = number(PROGRESS, 0.0);
        let revision = number(PROGRESS_REVISION, 0.0);
        let includes_delay = number(PROGRESS_BEHAVIOR, 0.0) >= 0.5;

        match self.last_now.get() {
            None => {
                // The first evaluation under the controller: the animation
                // time goes on from the time since the start, and moves to
                // the progress of the controller if it was assigned.
                self.time.set(raw as f64);
                self.progress_seen.set((0.0, 0.0));
            }
            Some(last) => {
                if !paused && rate.is_finite() {
                    self.time.set(self.time.get() + (now - last) as f64 * rate);
                }
            }
        }
        self.last_now.set(Some(now));
        self.paused.set(paused);

        if self.progress_seen.get() != (progress, revision) {
            self.progress_seen.set((progress, revision));
            if let Some(span) = span {
                let (offset, length) =
                    if includes_delay { (0, span.delay + span.active) } else { (span.delay, span.active) };
                self.time.set(offset as f64 + progress.clamp(0.0, 1.0) * length as f64);
            }
        }
        let time = self.time.get();
        if time.is_nan() || time < 0.0 {
            self.time.set(0.0);
        }
        self.time.get() as i64
    }
}

crate::ferro_thread_local! {
    /// The global playback rates of the compositors of this thread, as the
    /// UI thread last set them (the server keeps its own copy).
    static GLOBAL_PLAYBACK_RATES: RefCell<Vec<(Weak<Compositor>, f32)>> = RefCell::new(Vec::new());
}

impl Compositor {
    /// The highest global playback rate (WinUI `MaxGlobalPlaybackRate`).
    pub const MAX_GLOBAL_PLAYBACK_RATE: f32 = 1000.0;
    /// The lowest global playback rate (WinUI `MinGlobalPlaybackRate`).
    pub const MIN_GLOBAL_PLAYBACK_RATE: f32 = 0.0;

    /// Creates an animation controller (WinUI `CreateAnimationController`).
    pub fn create_animation_controller(&self) -> Rc<AnimationController> {
        AnimationController::new(&self.this_handle())
    }

    /// The rate at which the time of every animation of the compositor runs
    /// (WinUI `GlobalPlaybackRate`): 1 by default, 0.5 for half speed.
    pub fn global_playback_rate(&self) -> f32 {
        let this = self as *const Compositor;
        GLOBAL_PLAYBACK_RATES
            .with(|rates| rates.borrow().iter().find(|(c, _)| std::ptr::eq(c.as_ptr(), this)).map(|(_, r)| *r))
            .unwrap_or(1.0)
    }

    /// Sets the global playback rate, clamped to
    /// [`MIN_GLOBAL_PLAYBACK_RATE`](Self::MIN_GLOBAL_PLAYBACK_RATE)..[`MAX_GLOBAL_PLAYBACK_RATE`](Self::MAX_GLOBAL_PLAYBACK_RATE).
    /// It applies from the next frame to running animations and to those
    /// started later: their time from then on runs at the new rate.
    //
    // Deviation (winui-composition.md): the range of WinUI is not documented
    // here; the port does not run time backwards globally (minimum 0).
    pub fn set_global_playback_rate(&self, value: f32) {
        let value = if value.is_nan() {
            1.0
        } else {
            value.clamp(Self::MIN_GLOBAL_PLAYBACK_RATE, Self::MAX_GLOBAL_PLAYBACK_RATE)
        };
        let this = self.this_handle();
        GLOBAL_PLAYBACK_RATES.with(|rates| {
            let mut rates = rates.borrow_mut();
            rates.retain(|(c, _)| c.strong_count() > 0);
            match rates.iter_mut().find(|(c, _)| std::ptr::eq(c.as_ptr(), Rc::as_ptr(&this))) {
                Some(entry) => entry.1 = value,
                None => rates.push((Rc::downgrade(&this), value)),
            }
        });
        self.post_server_job(
            move |server| server.animations().set_global_playback_rate(value as f64, server.server_now()),
            false,
        );
    }
}
