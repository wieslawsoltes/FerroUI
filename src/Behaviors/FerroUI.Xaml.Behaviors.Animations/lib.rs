//! ferroui-xaml-behaviors-animations
//!
//! Reusable animations of the FerroUI behaviors, ported from the `Xaml.Behaviors.Animations`
//! project of Xaml.Behaviors, with no dependency on the behaviors themselves:
//!
//! - key-frame animations and their runners ([`AnimationFactory`], [`AnimationRunner`],
//!   [`IAnimationBuilder`], [`FluidMoveAnimation`]);
//! - composition animation catalogs ([`EntranceAnimations`], [`ExitAnimations`],
//!   [`AttentionAnimations`], [`FramerMotionAnimations`], [`SpecialAnimations`]) and the
//!   primitive effects ([`FadeAnimation`], [`ScaleAnimation`], [`RotateAnimation`],
//!   [`SlidingAnimation`]), started when their element is loaded;
//! - pointer and scroll effects ([`TiltAnimation`], [`OrbitAnimation`], [`ParallaxAnimation`]);
//! - the selection indicator animation ([`SelectionIndicatorAnimation`]) and the attached
//!   property that enables it on a selecting items control ([`SelectingItemsControlBehavior`]);
//! - operations on the transitions of an element ([`TransitionOperations`]).
//!
//! Every type is in the markup namespace `FerroUI.Xaml.Interactions.Custom`, mapped into the
//! default XML namespace of FerroUI; [`register_types`] registers them.

mod animations;
mod argument_out_of_range;
mod composition;
mod markup_types;
mod register_types;
mod rust_paths;
mod transitions;

pub use animations::{AnimationFactory, AnimationRunner, FluidMoveAnimation, IAnimationBuilder};
pub use composition::{
    AttentionAnimations, EntranceAnimations, ExitAnimations, FadeAnimation, FramerMotionAnimations, OrbitAnimation,
    ParallaxAnimation, RotateAnimation, ScaleAnimation, SelectingItemsControlBehavior, SelectionIndicatorAnimation,
    SlidingAnimation, SpecialAnimations, TiltAnimation,
};
pub use register_types::{register_types, ASSEMBLY};
pub use transitions::TransitionOperations;

#[cfg(test)]
mod tests;
