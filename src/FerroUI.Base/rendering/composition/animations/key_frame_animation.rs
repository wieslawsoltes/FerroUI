use super::{CompositionAnimation, IKeyFrames};
use crate::animation::easings::{Easing, IEasing};
use crate::animation::PlaybackDirection;
use crate::rendering::composition::Compositor;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use std::time::Duration;

/// A time-based animation with one or more key frames.
/// These frames are markers, allowing developers to specify values at specific times for the animating property.
/// KeyFrame animations can be further customized by specifying how the animation interpolates between keyframes.
///
/// The classes for the value types (`ScalarKeyFrameAnimation`,
/// `Vector3KeyFrameAnimation`, ...) embed it and dereference to it; they are
/// expanded from the key frame animations of the composition schema at the
/// end of this module.
pub struct KeyFrameAnimation {
    base: CompositionAnimation,
    duration: Cell<Duration>,
    delay_behavior: Cell<AnimationDelayBehavior>,
    delay_time: Cell<Duration>,
    direction: Cell<PlaybackDirection>,
    iteration_behavior: Cell<AnimationIterationBehavior>,
    iteration_count: Cell<i32>,
    stop_behavior: Cell<AnimationStopBehavior>,
    /// `private protected abstract IKeyFrames KeyFrames`: the key frames of
    /// the class for the value type.
    key_frames: RefCell<Box<dyn IKeyFrames>>,
}

impl Deref for KeyFrameAnimation {
    type Target = CompositionAnimation;

    fn deref(&self) -> &CompositionAnimation {
        &self.base
    }
}

impl KeyFrameAnimation {
    pub(crate) fn new(compositor: &Rc<Compositor>, key_frames: Box<dyn IKeyFrames>) -> Self {
        Self {
            base: CompositionAnimation::new(compositor),
            duration: Cell::new(Duration::from_millis(1)),
            delay_behavior: Cell::new(AnimationDelayBehavior::default()),
            delay_time: Cell::new(Duration::ZERO),
            direction: Cell::new(PlaybackDirection::default()),
            iteration_behavior: Cell::new(AnimationIterationBehavior::default()),
            iteration_count: Cell::new(1),
            stop_behavior: Cell::new(AnimationStopBehavior::default()),
            key_frames: RefCell::new(key_frames),
        }
    }

    /// The delay behavior of the key frame animation.
    pub fn delay_behavior(&self) -> AnimationDelayBehavior {
        self.delay_behavior.get()
    }

    pub fn set_delay_behavior(&self, value: AnimationDelayBehavior) {
        self.delay_behavior.set(value)
    }

    /// Delay before the animation starts after `start_animation` is called.
    pub fn delay_time(&self) -> Duration {
        self.delay_time.get()
    }

    pub fn set_delay_time(&self, value: Duration) {
        self.delay_time.set(value)
    }

    /// The direction the animation is playing.
    /// The Direction property allows you to drive your animation from start to end or end to start or alternate
    /// between start and end or end to start if animation has an [`iteration_count`](Self::iteration_count) greater than one.
    /// This gives an easy way for customizing animation definitions.
    pub fn direction(&self) -> PlaybackDirection {
        self.direction.get()
    }

    pub fn set_direction(&self, value: PlaybackDirection) {
        self.direction.set(value)
    }

    /// The duration of the animation.
    /// Minimum allowed value is 1ms and maximum allowed value is 24 days.
    pub fn duration(&self) -> Duration {
        self.duration.get()
    }

    /// Sets the duration. As upstream, the range check is made on the
    /// current value rather than on the new one.
    pub fn set_duration(&self, value: Duration) {
        let duration = self.duration.get();
        if duration < Duration::from_millis(1) || duration > Duration::from_secs(24 * 60 * 60) {
            panic!("Minimum allowed value is 1ms and maximum allowed value is 24 days.");
        }
        self.duration.set(value)
    }

    /// The iteration behavior for the key frame animation.
    pub fn iteration_behavior(&self) -> AnimationIterationBehavior {
        self.iteration_behavior.get()
    }

    pub fn set_iteration_behavior(&self, value: AnimationIterationBehavior) {
        self.iteration_behavior.set(value)
    }

    /// The number of times to repeat the key frame animation.
    pub fn iteration_count(&self) -> i32 {
        self.iteration_count.get()
    }

    pub fn set_iteration_count(&self, value: i32) {
        self.iteration_count.set(value)
    }

    /// Specifies how to set the property value when animation is stopped
    pub fn stop_behavior(&self) -> AnimationStopBehavior {
        self.stop_behavior.get()
    }

    pub fn set_stop_behavior(&self, value: AnimationStopBehavior) {
        self.stop_behavior.set(value)
    }

    /// Inserts an expression keyframe.
    ///
    /// `normalized_progress_key` is the time the key frame should occur at,
    /// expressed as a percentage of the animation Duration. Allowed value
    /// is from 0.0 to 1.0. `value` is the expression used to calculate the
    /// value of the key frame. `easing_function` is the easing function to
    /// use when interpolating between frames; the default easing of the
    /// compositor when `None`.
    pub fn insert_expression_key_frame(&self, normalized_progress_key: f32, value: &str, easing_function: Option<Easing>) {
        let easing = match easing_function {
            Some(easing) => easing.as_rc().clone(),
            None => self.compositor().default_easing(),
        };
        self.key_frames.borrow_mut().insert_expression_key_frame(normalized_progress_key, value, easing)
    }

    /// The key frames of the class for value type `T`.
    pub(crate) fn with_key_frames<T: Copy + Default + 'static, R>(&self, f: impl FnOnce(&mut super::KeyFrames<T>) -> R) -> R {
        let mut key_frames = self.key_frames.borrow_mut();
        match key_frames.as_any_mut().downcast_mut::<super::KeyFrames<T>>() {
            Some(key_frames) => f(key_frames),
            None => unreachable!("the key frames of a key frame animation are of its value type"),
        }
    }
}

/// Specifies the animation delay behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnimationDelayBehavior {
    /// If a DelayTime is specified, it delays starting the animation according to delay time and after delay
    /// has expired it applies animation to the object property.
    #[default]
    SetInitialValueAfterDelay,
    /// Applies the initial value of the animation (i.e. the value at Keyframe 0) to the object before the delay time
    /// is elapsed (when there is a DelayTime specified), it then delays starting the animation according to the DelayTime.
    SetInitialValueBeforeDelay,
}

/// Specifies if the animation should loop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnimationIterationBehavior {
    /// The animation should loop the specified number of times.
    #[default]
    Count,
    /// The animation should loop forever.
    Forever,
}

/// Specifies the behavior of an animation when it stops.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnimationStopBehavior {
    /// Leave the animation at its current value.
    #[default]
    LeaveCurrentValue,
    /// Reset the animation to its initial value.
    SetToInitialValue,
    /// Set the animation to its final value.
    SetToFinalValue,
}

/// Expands the key frame animation classes of the composition schema (see
/// [`for_each_composition_key_frame_animation`](crate::rendering::composition::generated::for_each_composition_key_frame_animation)):
/// upstream's generated `{Name}KeyFrameAnimation` classes and their
/// `Compositor.Create{Name}KeyFrameAnimation` factories.
macro_rules! key_frame_animation_classes {
    ($(($name:ident, $create:ident, $ty:ty, $interpolator:ty)),* $(,)?) => {$(
        #[doc = concat!("A key frame animation of `", stringify!($ty), "` values.")]
        pub struct $name {
            base: KeyFrameAnimation,
        }

        impl Deref for $name {
            type Target = KeyFrameAnimation;

            fn deref(&self) -> &KeyFrameAnimation {
                &self.base
            }
        }

        impl $name {
            pub fn new(compositor: &Rc<Compositor>) -> Rc<$name> {
                Rc::new($name {
                    base: KeyFrameAnimation::new(compositor, Box::new(super::KeyFrames::<$ty>::new())),
                })
            }

            pub fn insert_key_frame_with_easing(
                &self,
                normalized_progress_key: f32,
                value: $ty,
                easing_function: Rc<dyn IEasing>,
            ) {
                self.base.with_key_frames::<$ty, _>(|key_frames| {
                    key_frames.insert(normalized_progress_key, value, easing_function)
                })
            }

            pub fn insert_key_frame(&self, normalized_progress_key: f32, value: $ty) {
                let easing = self.compositor().default_easing();
                self.base.with_key_frames::<$ty, _>(|key_frames| {
                    key_frames.insert(normalized_progress_key, value, easing)
                })
            }
        }

        impl super::ICompositionAnimationBase for $name {
            fn as_composition_animation(&self) -> Option<&dyn super::ICompositionAnimation> {
                Some(self)
            }
        }

        impl super::ICompositionAnimation for $name {
            fn target(&self) -> Option<String> {
                self.base.target()
            }

            fn create_instance(
                &self,
                target_object: crate::rendering::composition::server::ServerObjectId,
                final_value: Option<crate::rendering::composition::expressions::ExpressionVariant>,
            ) -> super::AnimationInstanceFactory {
                let interpolator: &'static (dyn super::IInterpolator<$ty> + Sync) = <$interpolator>::instance();
                let key_frames = self.base.with_key_frames::<$ty, _>(|key_frames| key_frames.snapshot());
                let parameters = self.create_snapshot_source();
                let final_value = final_value.map(|value| {
                    crate::rendering::composition::expressions::ExpressionVariant::create(
                        value.cast_or_default::<$ty>(),
                    )
                });
                let delay_behavior = self.delay_behavior();
                let delay_time = self.delay_time();
                let direction = self.direction();
                let duration = self.duration();
                let iteration_behavior = self.iteration_behavior();
                let iteration_count = self.iteration_count();
                let stop_behavior = self.stop_behavior();
                super::AnimationInstanceFactory::new(move || {
                    super::KeyFrameAnimationInstance::<$ty>::new(
                        interpolator,
                        key_frames,
                        Rc::new(parameters.build()),
                        final_value,
                        target_object,
                        delay_behavior,
                        delay_time,
                        direction,
                        duration,
                        iteration_behavior,
                        iteration_count,
                        stop_behavior,
                    )
                })
            }
        }

        impl crate::rendering::composition::AsCompositionObject for $name {
            fn as_composition_object(&self) -> &crate::rendering::composition::CompositionObject {
                self.base.object()
            }

            fn composition_type_name(&self) -> &'static str {
                stringify!($name)
            }
        }

        impl Compositor {
            pub fn $create(&self) -> Rc<$name> {
                $name::new(&self.this_handle())
            }
        }
    )*};
}

crate::rendering::composition::generated::for_each_composition_key_frame_animation!(key_frame_animation_classes);
