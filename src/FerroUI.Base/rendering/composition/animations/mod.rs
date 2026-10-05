//! Composition animations.

mod animation_instance_base;
mod composition_animation;
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
mod property_set_snapshot;

pub use animation_instance_base::AnimationInstanceBase;
pub use composition_animation::CompositionAnimation;
pub use composition_animation_group::CompositionAnimationGroup;
pub use expression_animation::ExpressionAnimation;
pub use expression_animation_instance::ExpressionAnimationInstance;
pub use i_animation_instance::IAnimationInstance;
pub use i_composition_animation_base::{ICompositionAnimation, ICompositionAnimationBase};
pub use implicit_animation_collection::ImplicitAnimationCollection;
pub use interpolators::{
    BooleanInterpolator, ColorInterpolator, DoubleInterpolator, IInterpolator, QuaternionInterpolator,
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
pub use property_set_snapshot::{PropertySetSnapshot, PropertySetSnapshotObject, PropertySetSnapshotValue};

