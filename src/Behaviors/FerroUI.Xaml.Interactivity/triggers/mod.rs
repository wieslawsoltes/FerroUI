//! The base classes of triggers (`Triggers/`).

mod disposing_trigger;
mod event_trigger_base;
mod interactive_trigger_base;

pub use disposing_trigger::{DisposingTrigger, DisposingTriggerImpl, DisposingTriggerImplExt, DisposingTriggerVTable};
pub use event_trigger_base::{EventTriggerBase, EventTriggerBaseImpl, EventTriggerBaseImplExt, EventTriggerBaseVTable};
pub use interactive_trigger_base::InteractiveTriggerBase;
