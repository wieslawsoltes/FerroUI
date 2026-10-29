//! Port of `Core/EventTriggerBehavior.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{EventTriggerBase, EventTriggerBaseImpl, StyledElementBehaviorImpl};

/// A behavior that listens for a specified event on its source and executes its actions when that
/// event is fired.
///
/// The event is found by name on the source (see [`EventTriggerBase`]): a routed event of its
/// class, a plain event its markup metadata declares, or an event of the registry of event
/// handlers. The default event, `AttachedToVisualTree`, fires when a control without a parent is
/// attached to the visual tree.
#[repr(C)]
pub struct EventTriggerBehavior {
    base: EventTriggerBase,
}

ferro_class!(EventTriggerBehavior: EventTriggerBase);
ferro_class_info!(EventTriggerBehavior { new: EventTriggerBehavior::new });
ferroui_base::ferro_impl_classes!(
    EventTriggerBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    EventTriggerBaseImpl
);

impl EventTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: EventTriggerBase::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
