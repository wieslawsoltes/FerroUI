use crate::animation::{
    Animatable, Animation, AnimationInstance, AnimatorKeyFrame, Clock, DisposeAnimationInstanceSubject, IAnimator,
    IClock, KeySpline,
};
use crate::data::BindingPriority;
use crate::reactive::{CompositeDisposable, IDisposable, IObservable};
use crate::{AnyValue, FerroProperty, PropertyValue, Ref, StyledProperty, Visual};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The state every animator has: its key frames and its target property.
#[derive(Default)]
pub struct AnimatorBase {
    key_frames: RefCell<Vec<Ref<AnimatorKeyFrame>>>,
    property: Cell<Option<&'static FerroProperty>>,
}

impl AnimatorBase {
    pub fn new() -> Self {
        Self::default()
    }

    /// The target property.
    #[inline]
    pub fn property(&self) -> Option<&'static FerroProperty> {
        self.property.get()
    }

    pub fn set_property(&self, value: Option<&'static FerroProperty>) {
        self.property.set(value)
    }

    /// The number of key frames.
    #[inline]
    pub fn count(&self) -> usize {
        self.key_frames.borrow().len()
    }

    /// The key frame at `index`. Panics when out of range.
    pub fn get(&self, index: usize) -> Ref<AnimatorKeyFrame> {
        self.key_frames.borrow()[index].clone()
    }

    /// A copy of the key frames.
    pub fn to_vec(&self) -> Vec<Ref<AnimatorKeyFrame>> {
        self.key_frames.borrow().clone()
    }

    pub(crate) fn push(&self, item: Ref<AnimatorKeyFrame>) {
        self.key_frames.borrow_mut().push(item)
    }

    /// The target property as a styled property of value type `T`. Panics
    /// when the animator has no property or the property has another type.
    pub fn styled_property<T: PropertyValue>(&self) -> &'static StyledProperty<T> {
        let Some(property) = self.property.get() else {
            panic!("Animator has no property specified.");
        };
        match property.as_styled::<T>() {
            Some(property) => property,
            None => panic!(
                "Unable to animate property '{}' of type {} as {}.",
                property,
                property.property_type_name(),
                std::any::type_name::<T>()
            ),
        }
    }
}

/// Base class for animators: interpolates the key frames of one property of
/// an animation.
///
/// An animator implements [`interpolate`](Self::interpolate) for its value
/// type; everything else has an implementation that subclasses override
/// only for special targets (brushes, transforms).
pub trait Animator: Sized + 'static {
    /// The type of the values the animator produces, which is the value
    /// type of the properties it animates.
    type Value: PropertyValue + Default;

    /// The key frames and the target property.
    fn base(&self) -> &AnimatorBase;

    /// Interpolates between two values using the specified progress.
    fn interpolate(&self, progress: f64, old_value: &Self::Value, new_value: &Self::Value) -> Self::Value;

    /// Reads a key frame value; `None` when it is not a value this animator
    /// can animate, in which case the neutral value is used in its place.
    #[inline]
    fn typed_value(&self, untyped: &dyn AnyValue) -> Option<Self::Value> {
        untyped.downcast_ref::<Self::Value>().cloned()
    }

    /// Validates a key frame that is being added. Panics to reject it.
    fn validate(&self, _item: &Ref<AnimatorKeyFrame>) {}

    /// Binds the values a running animation produces to the target property
    /// of the control, at animation priority.
    fn bind_animation(&self, control: &Animatable, instance: Rc<dyn IObservable<Self::Value>>) -> Rc<dyn IDisposable> {
        control.bind(self.base().styled_property::<Self::Value>(), instance, BindingPriority::Animation)
    }

    /// Applies the animation to `control`: it runs while `match_` is true.
    fn apply(
        this: &Rc<Self>,
        animation: &Animation,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        match_: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        should_pause_on_invisible: bool,
    ) -> Option<Rc<dyn IDisposable>> {
        let visual_target = control.downcast_ref::<Visual>().map(Visual::to_ref);
        Some(apply_with_visual(
            this,
            animation,
            control,
            clock,
            match_,
            on_complete,
            should_pause_on_invisible,
            visual_target,
        ))
    }
}

/// Applies an animation to `control`, tying its lifetime and visibility
/// handling to `visual_target`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_with_visual<A: Animator>(
    animator: &Rc<A>,
    animation: &Animation,
    control: &Animatable,
    clock: Option<Rc<dyn IClock>>,
    match_: Rc<dyn IObservable<bool>>,
    on_complete: Option<Rc<dyn Fn()>>,
    should_pause_on_invisible: bool,
    visual_target: Option<Ref<Visual>>,
) -> Rc<dyn IDisposable> {
    let subject = Rc::new(DisposeAnimationInstanceSubject::new(
        animator.clone(),
        animation.to_ref(),
        control.to_ref(),
        clock,
        on_complete,
        should_pause_on_invisible,
        visual_target,
    ));
    let subscription = match_.subscribe(subject.clone());
    let subject: Rc<dyn IDisposable> = subject;
    Rc::new(CompositeDisposable::from_disposables([subscription, subject]))
}

/// Runs the animation once: creates the instance that produces the values
/// and binds it to the control.
pub(crate) fn run<A: Animator>(
    animator: &Rc<A>,
    animation: &Ref<Animation>,
    control: &Ref<Animatable>,
    clock: Option<Rc<dyn IClock>>,
    on_complete: Option<Rc<dyn Fn()>>,
    should_pause_on_invisible: bool,
    visual_target: Option<&Ref<Visual>>,
) -> Rc<dyn IDisposable> {
    let base_clock = clock.or_else(|| control.clock()).unwrap_or_else(Clock::global_clock);
    let instance = AnimationInstance::new(
        animation.clone(),
        control,
        animator.clone(),
        base_clock,
        on_complete,
        should_pause_on_invisible,
        visual_target,
    );
    animator.bind_animation(control, instance)
}

struct KeyFrameInfo<T> {
    time: f64,
    value: T,
    key_spline: Option<Ref<KeySpline>>,
}

impl<T: Clone> KeyFrameInfo<T> {
    fn from_key_frame<A: Animator<Value = T>>(animator: &A, source: &AnimatorKeyFrame, neutral_value: &T) -> Self {
        Self {
            time: source.cue().cue_value(),
            value: get_typed_value(animator, source, neutral_value),
            key_spline: source.key_spline().cloned(),
        }
    }
}

#[inline]
fn get_typed_value<A: Animator>(animator: &A, frame: &AnimatorKeyFrame, neutral_value: &A::Value) -> A::Value {
    frame
        .with_value(|value| value.and_then(|value| animator.typed_value(value)))
        .unwrap_or_else(|| neutral_value.clone())
}

/// Computes the value of the animation at `animation_time`, the eased
/// position within an iteration. `neutral_value` stands in for the parts of
/// the iteration that no key frame covers.
pub(crate) fn interpolation_handler<A: Animator>(animator: &A, animation_time: f64, neutral_value: &A::Value) -> A::Value {
    let (from, to) = {
        let key_frames = animator.base().key_frames.borrow();
        if key_frames.is_empty() {
            return neutral_value.clone();
        }
        get_key_frames(animator, &key_frames, animation_time, neutral_value)
    };

    let mut progress = (animation_time - from.time) / (to.time - from.time);

    if let Some(key_spline) = &to.key_spline {
        progress = key_spline.get_spline_progress(progress);
    }

    animator.interpolate(progress, &from.value, &to.value)
}

fn get_key_frames<A: Animator>(
    animator: &A,
    key_frames: &[Ref<AnimatorKeyFrame>],
    time: f64,
    neutral_value: &A::Value,
) -> (KeyFrameInfo<A::Value>, KeyFrameInfo<A::Value>) {
    debug_assert!(!key_frames.is_empty());
    let count = key_frames.len();

    // Before or right at the first frame which isn't at time 0.0: interpolate between 0.0 and the first frame.
    let first_frame = &key_frames[0];
    let first_time = first_frame.cue().cue_value();
    if time <= first_time && first_time > 0.0 {
        let before_value = if first_frame.fill_before() {
            get_typed_value(animator, first_frame, neutral_value)
        } else {
            neutral_value.clone()
        };
        return (
            KeyFrameInfo { time: 0.0, value: before_value, key_spline: first_frame.key_spline().cloned() },
            KeyFrameInfo::from_key_frame(animator, first_frame, neutral_value),
        );
    }

    // Between two frames: interpolate between the previous frame and the next frame.
    for i in 1..count {
        let frame = &key_frames[i];
        if time <= frame.cue().cue_value() {
            return (
                KeyFrameInfo::from_key_frame(animator, &key_frames[i - 1], neutral_value),
                KeyFrameInfo::from_key_frame(animator, frame, neutral_value),
            );
        }
    }

    // Past the last frame which is at time 1.0: interpolate between the last two frames.
    let last_frame = &key_frames[count - 1];
    if last_frame.cue().cue_value() >= 1.0 {
        if count == 1 {
            let before_value = if last_frame.fill_before() {
                get_typed_value(animator, last_frame, neutral_value)
            } else {
                neutral_value.clone()
            };
            return (
                KeyFrameInfo { time: 0.0, value: before_value, key_spline: last_frame.key_spline().cloned() },
                KeyFrameInfo::from_key_frame(animator, last_frame, neutral_value),
            );
        }

        return (
            KeyFrameInfo::from_key_frame(animator, &key_frames[count - 2], neutral_value),
            KeyFrameInfo::from_key_frame(animator, last_frame, neutral_value),
        );
    }

    // Past the last frame which isn't at time 1.0: interpolate between the last frame and 1.0.
    let after_value = if last_frame.fill_after() {
        get_typed_value(animator, last_frame, neutral_value)
    } else {
        neutral_value.clone()
    };
    (
        KeyFrameInfo::from_key_frame(animator, last_frame, neutral_value),
        KeyFrameInfo { time: 1.0, value: after_value, key_spline: last_frame.key_spline().cloned() },
    )
}

impl<A: Animator> IAnimator for A {
    fn property(&self) -> Option<&'static FerroProperty> {
        self.base().property()
    }

    fn set_property(&self, value: Option<&'static FerroProperty>) {
        self.base().set_property(value)
    }

    fn count(&self) -> usize {
        self.base().count()
    }

    fn get(&self, index: usize) -> Ref<AnimatorKeyFrame> {
        self.base().get(index)
    }

    fn add(&self, item: Ref<AnimatorKeyFrame>) {
        self.validate(&item);
        self.base().push(item)
    }

    fn animator_type(&self) -> TypeId {
        TypeId::of::<A>()
    }

    fn apply(
        self: Rc<Self>,
        animation: &Animation,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        match_: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        should_pause_on_invisible: bool,
    ) -> Option<Rc<dyn IDisposable>> {
        <A as Animator>::apply(&self, animation, control, clock, match_, on_complete, should_pause_on_invisible)
    }
}
