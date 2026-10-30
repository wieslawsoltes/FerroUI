//! Port of `Core/EventTrigger.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{EventTriggerBase, EventTriggerBaseImpl, StyledElementBehaviorImpl};

/// A behavior that listens for a specified event on its source and executes its actions when that
/// event is fired.
#[repr(C)]
pub struct EventTrigger {
    base: EventTriggerBase,
}

ferro_class!(EventTrigger: EventTriggerBase);
ferro_class_info!(EventTrigger { new: EventTrigger::new });
ferroui_base::ferro_impl_classes!(
    EventTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    EventTriggerBaseImpl
);

impl EventTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: EventTriggerBase::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
