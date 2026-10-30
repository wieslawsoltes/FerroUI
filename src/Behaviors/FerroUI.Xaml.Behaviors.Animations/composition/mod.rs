//! The `Composition/` folder of the reference: the composition animation catalogs and effects,
//! and the selection indicator animation.

mod attention_animations;
pub(crate) mod composition_animation_definition;
pub(crate) mod composition_animation_helpers;
mod entrance_animations;
mod exit_animations;
mod fade_animation;
mod framer_motion_animations;
mod orbit_animation;
mod parallax_animation;
mod rotate_animation;
mod scale_animation;
mod selecting_items_control_behavior;
mod selection_indicator_animation;
mod sliding_animation;
mod special_animations;
mod tilt_animation;

pub use attention_animations::AttentionAnimations;
pub use entrance_animations::EntranceAnimations;
pub use exit_animations::ExitAnimations;
pub use fade_animation::FadeAnimation;
pub use framer_motion_animations::FramerMotionAnimations;
pub use orbit_animation::OrbitAnimation;
pub use parallax_animation::ParallaxAnimation;
pub use rotate_animation::RotateAnimation;
pub use scale_animation::ScaleAnimation;
pub use selecting_items_control_behavior::SelectingItemsControlBehavior;
pub use selection_indicator_animation::SelectionIndicatorAnimation;
pub use sliding_animation::SlidingAnimation;
pub use special_animations::SpecialAnimations;
pub use tilt_animation::TiltAnimation;
