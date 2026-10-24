//! The base classes of behaviors (`Behaviors/`).

mod disposing_behavior;
mod interactive_behavior_base;
mod invoke_command_behavior_base;

pub use disposing_behavior::{DisposingBehavior, DisposingBehaviorImpl, DisposingBehaviorImplExt, DisposingBehaviorVTable};
pub use interactive_behavior_base::InteractiveBehaviorBase;
pub use invoke_command_behavior_base::InvokeCommandBehaviorBase;
