//! Composition animations.

mod animation_controller;
mod animation_instance_base;
mod composition_animation;
mod composition_easing_function;
mod composition_animation_group;
mod expression_animation;
mod expression_animation_instance;
mod i_animation_instance;
mod i_composition_animation_base;
mod implicit_animation_collection;
mod interpolators;
mod key_frame_animation;
mod key_frame_animation_instance;
mod key_frames;
mod natural_motion_animation;
mod natural_motion_animation_instance;
mod property_set_snapshot;
mod subchannel_animation_instance;

pub use animation_controller::{AnimationController, AnimationControllerProgressBehavior};
pub(crate) use animation_controller::PreparedAnimation;
pub use animation_instance_base::AnimationInstanceBase;
pub use composition_animation::{CompositionAnimation, InitialValueExpressionCollection};
pub use composition_animation_group::CompositionAnimationGroup;
pub use composition_easing_function::{
    BackEasingFunction, BounceEasingFunction, CircleEasingFunction, CompositionEasingFunction,
    CompositionEasingFunctionMode, CubicBezierEasingFunction, ElasticEasingFunction, ExponentialEasingFunction,
    LinearEasingFunction, PowerEasingFunction, SineEasingFunction, StepEasingFunction,
};
pub use expression_animation::ExpressionAnimation;
pub use expression_animation_instance::ExpressionAnimationInstance;
pub use i_animation_instance::{AnimationInstanceFactory, IAnimationInstance};
pub use i_composition_animation_base::{ICompositionAnimation, ICompositionAnimationBase};
pub use implicit_animation_collection::ImplicitAnimationCollection;
pub use interpolators::{
    BooleanInterpolator, ColorHslInterpolator, ColorHslLinearInterpolator, ColorInterpolator,
    ColorRgbLinearInterpolator, CompositionColorSpace, DoubleInterpolator, IInterpolator, QuaternionInterpolator,
    RelativePointInterpolator, RelativeScalarInterpolator, ScalarInterpolator, Vector2Interpolator,
    Vector3DInterpolator, Vector3Interpolator, Vector4Interpolator, VectorInterpolator,
};
pub use key_frame_animation::{
    AnimationDelayBehavior, AnimationIterationBehavior, AnimationStopBehavior, BooleanKeyFrameAnimation,
    ColorKeyFrameAnimation, DoubleKeyFrameAnimation, KeyFrameAnimation, QuaternionKeyFrameAnimation,
    RelativePointKeyFrameAnimation, RelativeScalarKeyFrameAnimation, ScalarKeyFrameAnimation,
    Vector2KeyFrameAnimation, Vector3DKeyFrameAnimation, Vector3KeyFrameAnimation, Vector4KeyFrameAnimation,
    VectorKeyFrameAnimation,
};
pub use key_frame_animation_instance::KeyFrameAnimationInstance;
pub use key_frames::{IKeyFrames, KeyFrame, KeyFrames, ServerKeyFrame};
pub use natural_motion_animation::{
    NaturalMotionAnimation, ScalarNaturalMotionAnimation, SpringScalarNaturalMotionAnimation,
    SpringVector2NaturalMotionAnimation, SpringVector3NaturalMotionAnimation, Vector2NaturalMotionAnimation,
    Vector3NaturalMotionAnimation, DEFAULT_SPRING_DAMPING_RATIO, DEFAULT_SPRING_PERIOD,
};
pub use natural_motion_animation_instance::{
    spring_state, SpringNaturalMotionAnimationInstance, SpringNaturalMotionParameters, SpringValue,
};
pub use property_set_snapshot::{
    PropertySetSnapshot, PropertySetSnapshotObject, PropertySetSnapshotSource, PropertySetSnapshotSourceValue,
    PropertySetSnapshotValue, SnapshotServerObject,
};
pub use subchannel_animation_instance::SubchannelAnimationInstance;
pub(crate) use subchannel_animation_instance::{extract_subchannel, parse_subchannel, SubchannelAnimation};

#[cfg(test)]
mod composition_animation_tests;
#[cfg(test)]
mod composition_easing_function_tests;
#[cfg(test)]
mod natural_motion_animation_tests;
#[cfg(test)]
mod animation_controller_tests;
