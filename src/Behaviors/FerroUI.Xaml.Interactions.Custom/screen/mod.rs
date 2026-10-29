//! The behaviors, triggers and actions of the screens of a top level (`Screen/`).

mod active_screen_behavior;
mod request_screen_details_action;
mod screens_changed_trigger;
mod screens_ref;

pub use active_screen_behavior::ActiveScreenBehavior;
pub use request_screen_details_action::RequestScreenDetailsAction;
pub use screens_changed_trigger::ScreensChangedTrigger;
pub use screens_ref::ScreensRef;
