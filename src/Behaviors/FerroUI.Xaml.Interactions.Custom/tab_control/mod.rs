//! The behaviors, triggers and actions of the tab control (`TabControl/`).

mod tab_control_key_navigation_behavior;
mod tab_control_next_action;
mod tab_control_previous_action;
mod tab_control_selection_changed_trigger;

pub use tab_control_key_navigation_behavior::TabControlKeyNavigationBehavior;
pub use tab_control_next_action::TabControlNextAction;
pub use tab_control_previous_action::TabControlPreviousAction;
pub use tab_control_selection_changed_trigger::TabControlSelectionChangedTrigger;
