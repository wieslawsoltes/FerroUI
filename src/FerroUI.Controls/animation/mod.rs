//! Connected animations: elements that fly from one view to another during
//! navigation.

mod connected_animation;
mod connected_animation_flight;
mod connected_animation_configuration;
mod connected_animation_service;
mod storyboard;
#[cfg(test)]
mod connected_animation_flight_tests;
#[cfg(test)]
mod connected_animation_tests;

pub use connected_animation::{ConnectedAnimation, ConnectedAnimationCompletedEventArgs};
pub use connected_animation_flight::ConnectedAnimationComponent;
pub(crate) use connected_animation::ConnectedAnimationProxy;
pub use connected_animation_configuration::{
    BasicConnectedAnimationConfiguration, ConnectedAnimationConfiguration, DirectConnectedAnimationConfiguration,
    GravityConnectedAnimationConfiguration,
};
pub use connected_animation_service::ConnectedAnimationService;
pub use storyboard::{
    BackEase, BounceEase, CircleEase, ColorAnimation, ColorAnimationUsingKeyFrames, ColorKeyFrame, CubicEase,
    DiscreteColorKeyFrame, DiscreteDoubleKeyFrame, DiscreteObjectKeyFrame, DiscretePointKeyFrame, DoubleAnimation,
    DoubleAnimationUsingKeyFrames, DoubleKeyFrame, EasingColorKeyFrame, EasingDoubleKeyFrame, EasingFunctionBase,
    EasingMode, EasingPointKeyFrame, ElasticEase, ExponentialEase, LinearColorKeyFrame, LinearDoubleKeyFrame,
    LinearPointKeyFrame, ObjectAnimationUsingKeyFrames, ObjectKeyFrame, PointAnimation, PointAnimationUsingKeyFrames,
    PointKeyFrame, PowerEase, QuadraticEase, QuarticEase, QuinticEase, SineEase, SplineColorKeyFrame,
    SplineDoubleKeyFrame, SplinePointKeyFrame, Storyboard, Timeline,
};
pub(crate) use storyboard::ResolvedAnimation;
