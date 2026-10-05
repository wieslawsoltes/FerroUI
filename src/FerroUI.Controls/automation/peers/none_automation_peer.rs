use super::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::Control;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents an element that is exposed to automation as non-
/// interactive or as not contributing to the logical structure of the application.
#[repr(C)]
pub struct NoneAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(NoneAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(NoneAutomationPeer {});

impl FerroObjectImpl for NoneAutomationPeer {}
impl ControlAutomationPeerImpl for NoneAutomationPeer {}

impl AutomationPeerImpl for NoneAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::None
    }

    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(_this: &Self) -> bool {
        false
    }
}

impl NoneAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Control) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Control) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
