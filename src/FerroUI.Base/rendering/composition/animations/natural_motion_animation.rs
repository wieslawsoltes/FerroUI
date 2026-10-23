//! WinUI's natural motion animations: `NaturalMotionAnimation`, the scalar and
//! vector natural motion animations, and the spring animations
//! (`SpringScalarNaturalMotionAnimation`, `SpringVector2NaturalMotionAnimation`,
//! `SpringVector3NaturalMotionAnimation`) with their `Compositor` factories.
//!
//! A natural motion animation has no duration and no key frames: it moves the
//! property by a physical model until the model comes to rest. The spring
//! animations move each component on a damped harmonic oscillator
//! ([`spring_state`](super::spring_state)) from the initial value (or the
//! current value of the property) with the initial velocity to the final value
//! (or the new value of the property when the animation is implicit). They end
//! by themselves, so a scoped batch waits for them; a spring without damping
//! never comes to rest and counts as an infinite animation.
//!
//! The classes "derive" from each other by embedding and dereferencing, as the
//! key frame animations do: `SpringScalarNaturalMotionAnimation` dereferences
//! to `ScalarNaturalMotionAnimation`, which dereferences to
//! `NaturalMotionAnimation`, which dereferences to `CompositionAnimation`.

use super::{
    AnimationDelayBehavior, AnimationInstanceFactory, AnimationStopBehavior, CompositionAnimation,
    ICompositionAnimation, ICompositionAnimationBase, SpringNaturalMotionAnimationInstance,
    SpringNaturalMotionParameters,
};
use crate::numerics::{Vector2, Vector3};
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::ServerObjectId;
use crate::rendering::composition::{AsCompositionObject, CompositionObject, Compositor};
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;
use std::time::Duration;

/// The damping ratio of a new spring animation: critically damped.
//
// Deviation (winui-composition.md): the defaults of `DampingRatio` and
// `Period` are not documented in the reference the port reads; it chose a
// critically damped spring with a period of 50 ms.
pub const DEFAULT_SPRING_DAMPING_RATIO: f32 = 1.0;

/// The period of a new spring animation.
pub const DEFAULT_SPRING_PERIOD: Duration = Duration::from_millis(50);

/// The base of the natural motion animations (WinUI `NaturalMotionAnimation`):
/// the delay and the stop behavior.
pub struct NaturalMotionAnimation {
    base: CompositionAnimation,
    delay_behavior: Cell<AnimationDelayBehavior>,
    delay_time: Cell<Duration>,
    stop_behavior: Cell<AnimationStopBehavior>,
}

impl Deref for NaturalMotionAnimation {
    type Target = CompositionAnimation;

    fn deref(&self) -> &CompositionAnimation {
        &self.base
    }
}

impl NaturalMotionAnimation {
    fn new(compositor: &Rc<Compositor>) -> NaturalMotionAnimation {
        NaturalMotionAnimation {
            base: CompositionAnimation::new(compositor),
            delay_behavior: Cell::new(AnimationDelayBehavior::default()),
            delay_time: Cell::new(Duration::ZERO),
            stop_behavior: Cell::new(AnimationStopBehavior::default()),
        }
    }

    /// Whether the property takes the initial value before the delay
    /// (`SetInitialValueBeforeDelay`) or when the motion starts.
    pub fn delay_behavior(&self) -> AnimationDelayBehavior {
        self.delay_behavior.get()
    }

    pub fn set_delay_behavior(&self, value: AnimationDelayBehavior) {
        self.delay_behavior.set(value)
    }

    /// The delay between the start of the animation and the start of the
    /// motion.
    pub fn delay_time(&self) -> Duration {
        self.delay_time.get()
    }

    pub fn set_delay_time(&self, value: Duration) {
        self.delay_time.set(value)
    }

    /// The value the property is set to when the animation is stopped with
    /// `StopAnimation`.
    pub fn stop_behavior(&self) -> AnimationStopBehavior {
        self.stop_behavior.get()
    }

    pub fn set_stop_behavior(&self, value: AnimationStopBehavior) {
        self.stop_behavior.set(value)
    }
}

/// Expands the natural motion animation of a value type and its spring
/// animation.
macro_rules! natural_motion_classes {
    ($(($motion:ident, $spring:ident, $create:ident, $ty:ty, $type_name:literal)),* $(,)?) => {$(
        #[doc = concat!(
            "A natural motion animation of ", $type_name, " values (WinUI `", stringify!($motion), "`): ",
            "where the motion starts and ends, and its initial velocity."
        )]
        pub struct $motion {
            base: NaturalMotionAnimation,
            final_value: Cell<Option<$ty>>,
            initial_value: Cell<Option<$ty>>,
            initial_velocity: Cell<$ty>,
        }

        impl Deref for $motion {
            type Target = NaturalMotionAnimation;

            fn deref(&self) -> &NaturalMotionAnimation {
                &self.base
            }
        }

        impl $motion {
            fn new(compositor: &Rc<Compositor>) -> $motion {
                $motion {
                    base: NaturalMotionAnimation::new(compositor),
                    final_value: Cell::new(None),
                    initial_value: Cell::new(None),
                    initial_velocity: Cell::new(<$ty>::default()),
                }
            }

            /// The value the motion comes to rest at. When `None`, an
            /// implicit animation moves to the new value of the property
            /// (`this.FinalValue`), and an explicit one to the current value.
            pub fn final_value(&self) -> Option<$ty> {
                self.final_value.get()
            }

            pub fn set_final_value(&self, value: Option<$ty>) {
                self.final_value.set(value)
            }

            /// The value the motion starts from; the current value of the
            /// property when `None`.
            pub fn initial_value(&self) -> Option<$ty> {
                self.initial_value.get()
            }

            pub fn set_initial_value(&self, value: Option<$ty>) {
                self.initial_value.set(value)
            }

            /// The velocity the motion starts with, in units of the property
            /// per second.
            pub fn initial_velocity(&self) -> $ty {
                self.initial_velocity.get()
            }

            pub fn set_initial_velocity(&self, value: $ty) {
                self.initial_velocity.set(value)
            }
        }

        #[doc = concat!(
            "A spring animation of ", $type_name, " values (WinUI `", stringify!($spring), "`): ",
            "each component moves on a damped harmonic oscillator of the given damping ratio and period."
        )]
        pub struct $spring {
            base: $motion,
            damping_ratio: Cell<f32>,
            period: Cell<Duration>,
        }

        impl Deref for $spring {
            type Target = $motion;

            fn deref(&self) -> &$motion {
                &self.base
            }
        }

        impl $spring {
            pub fn new(compositor: &Rc<Compositor>) -> Rc<$spring> {
                Rc::new($spring {
                    base: $motion::new(compositor),
                    damping_ratio: Cell::new(DEFAULT_SPRING_DAMPING_RATIO),
                    period: Cell::new(DEFAULT_SPRING_PERIOD),
                })
            }

            /// The damping ratio of the spring: 1 (the default) comes to rest
            /// as fast as possible without overshooting, less overshoots and
            /// oscillates, more comes to rest slower; 0 oscillates forever.
            pub fn damping_ratio(&self) -> f32 {
                self.damping_ratio.get()
            }

            /// Sets the damping ratio. Panics when it is negative or not a
            /// number.
            pub fn set_damping_ratio(&self, value: f32) {
                if value.is_nan() || value < 0.0 || value.is_infinite() {
                    panic!("The damping ratio must be a non-negative number");
                }
                self.damping_ratio.set(value)
            }

            /// The period of the oscillation of the spring without damping;
            /// a shorter period is a stiffer spring.
            pub fn period(&self) -> Duration {
                self.period.get()
            }

            /// Sets the period. Panics when it is zero.
            pub fn set_period(&self, value: Duration) {
                if value.is_zero() {
                    panic!("The period of a spring must be positive");
                }
                self.period.set(value)
            }
        }

        impl ICompositionAnimationBase for $spring {
            fn as_composition_animation(&self) -> Option<&dyn ICompositionAnimation> {
                Some(self)
            }
        }

        impl ICompositionAnimation for $spring {
            fn target(&self) -> Option<String> {
                self.base.target()
            }

            fn create_instance(
                &self,
                target_object: ServerObjectId,
                implicit_final_value: Option<ExpressionVariant>,
            ) -> AnimationInstanceFactory {
                let parameters = self.create_snapshot_source();
                // Parsed now: an expression that cannot be parsed fails the
                // start, as in WinUI.
                let initial_values = match self.initial_value_expressions().parse_all() {
                    Ok(initial_values) => initial_values,
                    Err(error) => panic!("{error}"),
                };
                let spring = SpringNaturalMotionParameters::<$ty> {
                    final_value: self
                        .base
                        .final_value()
                        .or_else(|| implicit_final_value.and_then(|value| value.try_cast::<$ty>())),
                    initial_value: self.base.initial_value(),
                    initial_velocity: self.base.initial_velocity(),
                    damping_ratio: self.damping_ratio(),
                    period: self.period(),
                    delay_behavior: self.delay_behavior(),
                    delay_time: self.delay_time(),
                    stop_behavior: self.stop_behavior(),
                };
                // The animation belongs to the batches that are open now; a
                // spring without damping never ends.
                let completion = self.compositor().acquire_animation_completion(spring.damping_ratio <= 0.0);
                AnimationInstanceFactory::new(move || {
                    let instance = SpringNaturalMotionAnimationInstance::<$ty>::new(
                        spring,
                        target_object,
                        Rc::new(parameters.build()),
                    );
                    instance.set_completion(completion);
                    instance.set_initial_value_expressions(initial_values);
                    instance
                })
            }
        }

        impl AsCompositionObject for $spring {
            fn as_composition_object(&self) -> &CompositionObject {
                self.object()
            }

            fn composition_type_name(&self) -> &'static str {
                stringify!($spring)
            }
        }

        impl Compositor {
            #[doc = concat!("Creates a `", stringify!($spring), "`.")]
            pub fn $create(&self) -> Rc<$spring> {
                $spring::new(&self.this_handle())
            }
        }
    )*};
}

natural_motion_classes! {
    (ScalarNaturalMotionAnimation, SpringScalarNaturalMotionAnimation, create_spring_scalar_animation, f32, "scalar"),
    (Vector2NaturalMotionAnimation, SpringVector2NaturalMotionAnimation, create_spring_vector2_animation, Vector2, "`Vector2`"),
    (Vector3NaturalMotionAnimation, SpringVector3NaturalMotionAnimation, create_spring_vector3_animation, Vector3, "`Vector3`"),
}
