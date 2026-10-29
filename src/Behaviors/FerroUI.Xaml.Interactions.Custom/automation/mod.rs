//! The actions, behaviors and triggers of automation and accessibility (`Automation/`).

mod actions;
mod behaviors;
mod triggers;

pub use self::actions::{ScreenReaderAnnounceAction, SetAutomationIdAction};
pub use self::behaviors::AutomationNameBehavior;
pub use self::triggers::AutomationNameChangedTrigger;
