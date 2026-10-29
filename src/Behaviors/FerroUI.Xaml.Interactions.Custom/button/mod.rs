//! The behaviors, the trigger and the action of buttons (`Button/`).

mod button_click_event_trigger_behavior;
mod button_execute_command_on_key_down_behavior;
mod button_hide_flyout_behavior;
mod button_hide_flyout_on_click_behavior;
mod button_hide_popup_on_click_behavior;
mod invoke_button_click_action;

pub use button_click_event_trigger_behavior::ButtonClickEventTriggerBehavior;
pub use button_execute_command_on_key_down_behavior::ButtonExecuteCommandOnKeyDownBehavior;
pub use button_hide_flyout_behavior::ButtonHideFlyoutBehavior;
pub use button_hide_flyout_on_click_behavior::ButtonHideFlyoutOnClickBehavior;
pub use button_hide_popup_on_click_behavior::ButtonHidePopupOnClickBehavior;
pub use invoke_button_click_action::InvokeButtonClickAction;
