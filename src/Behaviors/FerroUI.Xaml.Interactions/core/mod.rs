//! The core triggers and actions (`Core/`, namespace `FerroUI.Xaml.Interactions.Core`).

mod async_action_group;
mod call_method_action;
mod change_property_action;
mod data_trigger;
mod data_trigger_behavior;
mod debounce_action;
mod delay_action;
mod event_trigger;
mod event_trigger_behavior;
mod invoke_command_action;
mod multi_data_trigger;
mod multi_data_trigger_behavior;
mod observable_stream_behavior;
mod reversible_action_execution;
mod task_completed_trigger;
mod throttle_action;
mod timer_trigger;

pub use async_action_group::{AsyncActionGroup, AsyncActionMode};
pub use call_method_action::CallMethodAction;
pub use change_property_action::ChangePropertyAction;
pub use data_trigger::DataTrigger;
pub use data_trigger_behavior::DataTriggerBehavior;
pub use debounce_action::DebounceAction;
pub use delay_action::DelayAction;
pub use event_trigger::EventTrigger;
pub use event_trigger_behavior::EventTriggerBehavior;
pub use invoke_command_action::InvokeCommandAction;
pub use multi_data_trigger::MultiDataTrigger;
pub use multi_data_trigger_behavior::MultiDataTriggerBehavior;
pub use observable_stream_behavior::ObservableStreamBehavior;
pub use reversible_action_execution::ReversibleActionExecution;
pub use task_completed_trigger::TaskCompletedTrigger;
pub use throttle_action::ThrottleAction;
pub use timer_trigger::TimerTrigger;

use ferroui_base::StyledElement;
use ferroui_xaml_interactivity::{ActionCollection, StyledElementAction};

/// Makes the styled element actions of `actions` logical children of `parent` (the private
/// `AttachActionsToLogicalTree` of each action container of the reference).
pub(crate) fn attach_actions_to_logical_tree(actions: Option<&ActionCollection>, parent: &StyledElement) {
    let Some(actions) = actions else {
        return;
    };

    for action in actions.snapshot().iter() {
        if let Some(styled_element_action) = action.downcast_ref::<StyledElementAction>() {
            styled_element_action.attach_action_to_logical_tree(parent);
        }
    }
}

/// Removes the styled element actions of `actions` from the logical tree of `parent` (the private
/// `DetachActionsFromLogicalTree` of each action container of the reference).
pub(crate) fn detach_actions_from_logical_tree(actions: Option<&ActionCollection>, parent: &StyledElement) {
    let Some(actions) = actions else {
        return;
    };

    for action in actions.snapshot().iter() {
        if let Some(styled_element_action) = action.downcast_ref::<StyledElementAction>() {
            styled_element_action.detach_action_from_logical_tree(parent);
        }
    }
}
