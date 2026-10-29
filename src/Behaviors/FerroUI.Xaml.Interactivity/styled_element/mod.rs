//! The behaviors, triggers and actions that are styled elements (`StyledElement/`): they join the
//! logical tree of their host, follow its data context and are styled.

mod styled_element_action;
mod styled_element_behavior;
mod styled_element_behavior_of_t;
mod styled_element_trigger;
mod styled_element_trigger_of_t;

pub use styled_element_action::{
    IActionLogicalTreeLifecycle, StyledElementAction, StyledElementActionImpl, StyledElementActionImplExt,
    StyledElementActionVTable,
};
pub use styled_element_behavior::{
    StyledElementBehavior, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorVTable,
};
pub use styled_element_behavior_of_t::StyledElementBehaviorOfT;
pub use styled_element_trigger::StyledElementTrigger;
pub use styled_element_trigger_of_t::StyledElementTriggerOfT;
