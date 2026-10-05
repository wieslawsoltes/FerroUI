use super::{AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::NativeMenuBar;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`NativeMenuBar`].
#[repr(C)]
pub struct NativeMenuBarAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(NativeMenuBarAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(NativeMenuBarAutomationPeer {});

impl FerroObjectImpl for NativeMenuBarAutomationPeer {}
impl ControlAutomationPeerImpl for NativeMenuBarAutomationPeer {}

impl AutomationPeerImpl for NativeMenuBarAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::MenuBar
    }
}

impl NativeMenuBarAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &NativeMenuBar) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &NativeMenuBar) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
