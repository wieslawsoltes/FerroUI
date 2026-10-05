use super::{AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::UserControl;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`UserControl`].
#[repr(C)]
pub struct UserControlAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(UserControlAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(UserControlAutomationPeer {});

impl FerroObjectImpl for UserControlAutomationPeer {}
impl ControlAutomationPeerImpl for UserControlAutomationPeer {}

impl AutomationPeerImpl for UserControlAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }
}

impl UserControlAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &UserControl) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &UserControl) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
