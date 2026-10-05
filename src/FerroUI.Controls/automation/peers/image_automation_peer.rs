use super::{AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::Control;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents an image.
#[repr(C)]
pub struct ImageAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ImageAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(ImageAutomationPeer {});

impl FerroObjectImpl for ImageAutomationPeer {}
impl ControlAutomationPeerImpl for ImageAutomationPeer {}

impl AutomationPeerImpl for ImageAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "Image".to_owned()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Image
    }
}

impl ImageAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Control) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Control) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
