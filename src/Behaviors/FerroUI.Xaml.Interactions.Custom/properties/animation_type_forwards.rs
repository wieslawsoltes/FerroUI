//! Port of `Properties/AnimationTypeForwards.cs`.
//!
//! The reference forwards the animation types, which moved to the animations project, from this
//! assembly (`[assembly: TypeForwardedTo(..)]`), so code built against the earlier assembly finds
//! them here. The port re-exports them: `ferroui_xaml_interactions_custom::FadeAnimation` is the
//! type of `ferroui_xaml_behaviors_animations`. Markup needs no forward: the animations crate
//! declares its types in the namespace `FerroUI.Xaml.Interactions.Custom` already.

pub use ferroui_xaml_behaviors_animations::{
    AttentionAnimations, EntranceAnimations, ExitAnimations, FadeAnimation, FramerMotionAnimations, IAnimationBuilder,
    RotateAnimation, ScaleAnimation, SelectingItemsControlBehavior, SlidingAnimation, SpecialAnimations,
};
