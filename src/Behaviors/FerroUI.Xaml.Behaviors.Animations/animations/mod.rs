//! The `Animations/` folder of the reference: key-frame animation factories, the animation runner
//! and the fluid move animation.

mod animation_factory;
mod animation_runner;
mod fluid_move_animation;
mod i_animation_builder;

pub use animation_factory::AnimationFactory;
pub use animation_runner::AnimationRunner;
pub use fluid_move_animation::FluidMoveAnimation;
pub use i_animation_builder::IAnimationBuilder;
