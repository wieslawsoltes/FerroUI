use super::{AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::primitives::Thumb;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`Thumb`].
#[repr(C)]
pub struct ThumbAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ThumbAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(ThumbAutomationPeer {});

impl FerroObjectImpl for ThumbAutomationPeer {}
impl ControlAutomationPeerImpl for ThumbAutomationPeer {}

impl AutomationPeerImpl for ThumbAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Thumb
    }

    fn is_content_element_core(_this: &Self) -> bool {
        false
    }
}

impl ThumbAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Thumb) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Thumb) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
