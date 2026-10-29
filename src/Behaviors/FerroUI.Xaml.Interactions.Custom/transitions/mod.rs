//! The behavior, the trigger and the actions of the transitions of an element (`Transitions/`),
//! on the transition operations of the animations crate.

mod add_transition_action;
mod clear_transitions_action;
mod remove_transition_action;
mod transitions_behavior;
mod transitions_changed_trigger;

pub use add_transition_action::AddTransitionAction;
pub use clear_transitions_action::ClearTransitionsAction;
pub use remove_transition_action::RemoveTransitionAction;
pub use transitions_behavior::TransitionsBehavior;
pub use transitions_changed_trigger::TransitionsChangedTrigger;
