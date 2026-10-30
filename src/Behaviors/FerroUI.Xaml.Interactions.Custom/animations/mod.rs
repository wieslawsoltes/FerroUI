//! The behaviors, triggers and actions that run key-frame animations (`Animations/`), on the
//! animation runner of the animations crate.

mod animate_on_attached_behavior;
mod animation_builder_ref;
mod animation_completed_trigger;
mod begin_animation_action;
mod fade_in_behavior;
mod play_animation_behavior;
mod run_animation_trigger;
mod start_animation_action;
mod start_built_animation_action;

pub use animate_on_attached_behavior::AnimateOnAttachedBehavior;
pub use animation_builder_ref::AnimationBuilderRef;
pub use animation_completed_trigger::AnimationCompletedTrigger;
pub use begin_animation_action::BeginAnimationAction;
pub use fade_in_behavior::FadeInBehavior;
pub use play_animation_behavior::PlayAnimationBehavior;
pub use run_animation_trigger::RunAnimationTrigger;
pub use start_animation_action::StartAnimationAction;
pub use start_built_animation_action::StartBuiltAnimationAction;
