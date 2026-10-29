//! The conditional and switch actions and triggers (`Logic/`).

mod case;
mod conditional_action;
mod conditional_behavior;
mod switch_case_action;
mod switch_case_behavior;

pub use case::{Case, CaseCollection};
pub use conditional_action::ConditionalAction;
pub use conditional_behavior::ConditionalBehavior;
pub use switch_case_action::SwitchCaseAction;
pub use switch_case_behavior::SwitchCaseBehavior;
