//! Transitions and animations in the form of the element builders:
//! [`transition`], [`transitions`], [`animation`] and the hook
//! [`use_animation`].

use crate::builder::{IntoValue, TypedProperty};
use crate::control_flow::NodeRef;
use crate::hooks::use_effect_with;
use crate::reactive::on_cleanup;
use ferroui_base::animation::{
    Animatable, Animation, BoolTransition, BrushTransition, ColorTransition, CornerRadiusTransition, Cue,
    DoubleTransition, FillMode, FloatTransition, ITransition, IntegerTransition, IterationCount, KeyFrame,
    PointTransition, SizeTransition, ThicknessTransition, TimeSpan, Transitions, VectorTransition,
};
use ferroui_base::animation::easings::Easing;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationTokenSource;
use ferroui_base::{BoxedValue, CornerRadius, FerroProperty, ObjectType, Point, Ref, Size, Thickness, Upcast, Vector};
use std::rc::Rc;
use std::time::Duration;

/// A value type that a transition animates.
pub trait Transitionable {
    /// A transition of `property`, of values of this type, that takes
    /// `duration`, with an easing.
    fn transition(property: &'static FerroProperty, duration: TimeSpan, easing: Option<Easing>) -> Rc<dyn ITransition>;
}

macro_rules! transitionable {
    ($($value:ty => $transition:ty),* $(,)?) => {
        $(impl Transitionable for $value {
            fn transition(property: &'static FerroProperty, duration: TimeSpan, easing: Option<Easing>) -> Rc<dyn ITransition> {
                let transition = <$transition>::with_property(property, duration);
                if let Some(easing) = easing {
                    transition.set_easing(easing);
                }
                transition.into()
            }
        })*
    };
}

transitionable! {
    f64 => DoubleTransition,
    f32 => FloatTransition,
    i32 => IntegerTransition,
    bool => BoolTransition,
    Thickness => ThicknessTransition,
    CornerRadius => CornerRadiusTransition,
    Color => ColorTransition,
    Option<Rc<dyn IBrush>> => BrushTransition,
    Size => SizeTransition,
    Point => PointTransition,
    Vector => VectorTransition,
}

/// A transition of `property` that takes `duration`: when the property
/// changes, the control moves to the new value over that time
/// (`transitions: transitions([transition(Visual::opacity_property(), ms(200))])`).
pub fn transition<P>(property: &'static P, duration: Duration) -> Rc<dyn ITransition>
where
    P: TypedProperty + ?Sized,
    P::Value: Transitionable,
{
    <P::Value as Transitionable>::transition(property.untyped(), TimeSpan::from(duration), None)
}

/// A transition with an easing (`"CubicEaseOut"`, an easing of the port).
pub fn transition_eased<P>(property: &'static P, duration: Duration, easing: impl Into<Easing>) -> Rc<dyn ITransition>
where
    P: TypedProperty + ?Sized,
    P::Value: Transitionable,
{
    <P::Value as Transitionable>::transition(property.untyped(), TimeSpan::from(duration), Some(easing.into()))
}

/// The `Transitions` of a control, for its `transitions` setter.
pub fn transitions(items: impl IntoIterator<Item = Rc<dyn ITransition>>) -> Transitions {
    Transitions::from_items(items)
}

/// `Duration::from_millis`, short.
pub fn ms(milliseconds: u64) -> Duration {
    Duration::from_millis(milliseconds)
}

/// An animation being built: key frames of property values at cues, and
/// its duration, iterations, easing and fill mode.
///
/// ```ignore
/// animation(ms(600))
///     .key_frame(0.0, |frame| frame.set(Visual::opacity_property(), 0.0))
///     .key_frame(1.0, |frame| frame.set(Visual::opacity_property(), 1.0))
///     .fill_mode(FillMode::Forward)
/// ```
pub struct AnimationBuilder {
    animation: Ref<Animation>,
}

/// The setters of a key frame.
pub struct KeyFrameBuilder {
    frame: Ref<KeyFrame>,
}

impl KeyFrameBuilder {
    /// Sets `property` to `value` at the cue of the frame.
    pub fn set<P: TypedProperty + ?Sized>(self, property: &'static P, value: impl IntoValue<P::Value>) -> Self {
        let value: BoxedValue = Rc::new(value.into_value());
        let setter: Rc<dyn ferroui_base::animation::IAnimationSetter> = Setter::new_untyped(property.untyped(), value);
        self.frame.setters().add(setter);
        self
    }
}

/// An animation that takes `duration`.
pub fn animation(duration: Duration) -> AnimationBuilder {
    let animation = Animation::new();
    animation.set_duration(TimeSpan::from(duration));
    AnimationBuilder { animation }
}

impl AnimationBuilder {
    /// Adds a key frame at `cue` (0.0 the start, 1.0 the end).
    pub fn key_frame(self, cue: f64, frame: impl FnOnce(KeyFrameBuilder) -> KeyFrameBuilder) -> Self {
        let key_frame = KeyFrame::new();
        key_frame.set_cue(Cue::new(cue));
        let key_frame = frame(KeyFrameBuilder { frame: key_frame }).frame;
        self.animation.children().add(key_frame);
        self
    }

    /// How often it runs.
    pub fn iterations(self, count: IterationCount) -> Self {
        self.animation.set_iteration_count(count);
        self
    }

    /// Its easing.
    pub fn easing(self, easing: impl Into<Easing>) -> Self {
        self.animation.set_easing(easing);
        self
    }

    /// Whether the values of its ends are kept before and after it runs.
    pub fn fill_mode(self, fill_mode: FillMode) -> Self {
        self.animation.set_fill_mode(fill_mode);
        self
    }

    /// Its delay.
    pub fn delay(self, delay: Duration) -> Self {
        self.animation.set_delay(TimeSpan::from(delay));
        self
    }

    /// The animation.
    pub fn build(self) -> Ref<Animation> {
        self.animation
    }
}

/// Runs `animation` on the control of `target` once the controls of the run
/// are in place, and again whenever `trigger` differs from the last run's;
/// a run still in progress is cancelled first, and when the component is
/// taken out.
pub fn use_animation<C, D>(target: NodeRef<C>, trigger: D, animation: impl FnOnce() -> AnimationBuilder + 'static)
where
    C: ObjectType + Upcast<Animatable>,
    D: PartialEq + 'static,
{
    use_effect_with(trigger, move |_| {
        let Some(control) = target.get_untracked() else { return };
        let cancellation = CancellationTokenSource::new();
        let animatable: &Animatable = (*control).upcast();
        let _task = animation().build().run_async(animatable, cancellation.token());
        on_cleanup(move || cancellation.cancel());
    });
}
