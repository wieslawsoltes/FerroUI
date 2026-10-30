//! The behaviors, the trigger and the actions of the window of the associated control
//! (`Window/`).

mod center_window_behavior;
mod close_window_action;
mod show_window_action;
mod window_action;
mod window_drag_move_behavior;
mod window_state_trigger;

pub use center_window_behavior::CenterWindowBehavior;
pub use close_window_action::CloseWindowAction;
pub use show_window_action::ShowWindowAction;
pub use window_action::{WindowAction, WindowActionType};
pub use window_drag_move_behavior::WindowDragMoveBehavior;
pub use window_state_trigger::WindowStateTrigger;
