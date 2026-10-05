//! Composition animations.

mod i_animation_instance;
mod i_composition_animation_base;
mod interpolators;

pub use i_animation_instance::IAnimationInstance;
pub use i_composition_animation_base::{ICompositionAnimation, ICompositionAnimationBase};
pub use interpolators::{
    BooleanInterpolator, ColorInterpolator, DoubleInterpolator, IInterpolator, QuaternionInterpolator,
    RelativePointInterpolator, RelativeScalarInterpolator, ScalarInterpolator, Vector2Interpolator,
    Vector3DInterpolator, Vector3Interpolator, Vector4Interpolator, VectorInterpolator,
};
