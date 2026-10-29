//! The behavior, the triggers and the actions of a context dialog shown in a popup
//! (`ContextDialogs/`).

mod context_dialog_behavior;
mod context_dialog_closed_trigger;
mod context_dialog_opened_trigger;
mod hide_context_dialog_action;
mod show_context_dialog_action;

pub use context_dialog_behavior::ContextDialogBehavior;
pub use context_dialog_closed_trigger::ContextDialogClosedTrigger;
pub use context_dialog_opened_trigger::ContextDialogOpenedTrigger;
pub use hide_context_dialog_action::HideContextDialogAction;
pub use show_context_dialog_action::ShowContextDialogAction;
