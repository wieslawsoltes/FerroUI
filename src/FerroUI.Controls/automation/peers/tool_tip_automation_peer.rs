use super::{AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::ToolTip;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`ToolTip`].
#[repr(C)]
pub struct ToolTipAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ToolTipAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(ToolTipAutomationPeer {});

impl FerroObjectImpl for ToolTipAutomationPeer {}
impl ControlAutomationPeerImpl for ToolTipAutomationPeer {}

impl AutomationPeerImpl for ToolTipAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ToolTip
    }

    fn get_class_name_core(_this: &Self) -> String {
        "ToolTip".to_owned()
    }
}

impl ToolTipAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ToolTip) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ToolTip) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning tool tip.
    pub fn owner(&self) -> Ref<ToolTip> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a tool tip.")
    }
}
