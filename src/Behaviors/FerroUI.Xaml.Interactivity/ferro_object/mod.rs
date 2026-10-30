//! The behaviors, triggers and actions that are plain objects of the class model
//! (`FerroObject/`): no logical parent, no data context, no styles.

mod action;
mod behavior;
pub(crate) mod behavior_of_t;
mod trigger;
mod trigger_of_t;

pub use action::{Action, ActionImpl, ActionImplExt, ActionVTable};
pub use behavior::{Behavior, BehaviorImpl, BehaviorImplExt, BehaviorVTable};
pub use behavior_of_t::BehaviorOfT;
pub use trigger::Trigger;
pub use trigger_of_t::TriggerOfT;
