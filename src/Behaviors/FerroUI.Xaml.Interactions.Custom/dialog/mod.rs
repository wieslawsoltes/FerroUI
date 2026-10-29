//! The triggers and actions of dialog windows (`Dialog/`).

mod dialog_closed_trigger;
mod dialog_opened_trigger;
mod show_dialog_action;

pub use dialog_closed_trigger::DialogClosedTrigger;
pub use dialog_opened_trigger::DialogOpenedTrigger;
pub use show_dialog_action::ShowDialogAction;
