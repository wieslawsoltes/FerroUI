//! Connected animations: elements that fly from one view to another during
//! navigation.

mod connected_animation;
mod connected_animation_configuration;
mod connected_animation_service;
#[cfg(test)]
mod connected_animation_tests;

pub use connected_animation::{ConnectedAnimation, ConnectedAnimationCompletedEventArgs};
pub(crate) use connected_animation::ConnectedAnimationProxy;
pub use connected_animation_configuration::{
    BasicConnectedAnimationConfiguration, ConnectedAnimationConfiguration, DirectConnectedAnimationConfiguration,
    GravityConnectedAnimationConfiguration,
};
pub use connected_animation_service::ConnectedAnimationService;
