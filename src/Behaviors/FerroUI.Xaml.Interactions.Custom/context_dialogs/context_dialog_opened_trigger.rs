//! Port of `ContextDialogs/ContextDialogOpenedTrigger.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{EventTriggerBase, EventTriggerBaseImpl, StyledElementBehaviorImpl};

/// Trigger that listens for the `Opened` event of [`ContextDialogBehavior`](crate::ContextDialogBehavior).
///
/// The event trigger finds the event by name in the markup metadata of its source (usually the
/// behavior, given as its source object).
#[repr(C)]
pub struct ContextDialogOpenedTrigger {
    base: EventTriggerBase,
}

ferro_class!(ContextDialogOpenedTrigger: EventTriggerBase);
ferro_class_info!(ContextDialogOpenedTrigger { new: ContextDialogOpenedTrigger::new });
ferroui_base::ferro_impl_classes!(
    ContextDialogOpenedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    EventTriggerBaseImpl,
);

impl ContextDialogOpenedTrigger {
    /// The static constructor: the default event name of the class.
    fn static_constructor() {
        EventTriggerBase::event_name_property().override_default_value::<ContextDialogOpenedTrigger>(Some("Opened".to_string()));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: EventTriggerBase::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
