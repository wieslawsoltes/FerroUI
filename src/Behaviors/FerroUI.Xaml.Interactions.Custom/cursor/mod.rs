//! The behaviors and actions that set the cursor of an input element (`Cursor/`), with their
//! contract (`Cursor/Contract/`).

mod contract;
mod pointer_over_cursor_behavior;
mod set_cursor_action;
mod set_cursor_behavior;
mod set_cursor_from_provider_action;
mod set_cursor_from_provider_behavior;

pub use contract::ICursorProvider;
pub use pointer_over_cursor_behavior::PointerOverCursorBehavior;
pub use set_cursor_action::SetCursorAction;
pub use set_cursor_behavior::SetCursorBehavior;
pub use set_cursor_from_provider_action::SetCursorFromProviderAction;
pub use set_cursor_from_provider_behavior::SetCursorFromProviderBehavior;
