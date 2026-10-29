//! The behaviors, triggers and actions of the split view (`SplitView/`).

mod split_view_pane_closed_trigger;
mod split_view_pane_closing_trigger;
mod split_view_pane_opened_trigger;
mod split_view_pane_opening_trigger;
mod split_view_state_behavior;
mod split_view_state_setter;
mod split_view_toggle_pane_action;

pub use split_view_pane_closed_trigger::SplitViewPaneClosedTrigger;
pub use split_view_pane_closing_trigger::SplitViewPaneClosingTrigger;
pub use split_view_pane_opened_trigger::SplitViewPaneOpenedTrigger;
pub use split_view_pane_opening_trigger::SplitViewPaneOpeningTrigger;
pub use split_view_state_behavior::{SplitViewStateBehavior, SplitViewStateSetters};
pub use split_view_state_setter::SplitViewStateSetter;
pub use split_view_toggle_pane_action::SplitViewTogglePaneAction;
