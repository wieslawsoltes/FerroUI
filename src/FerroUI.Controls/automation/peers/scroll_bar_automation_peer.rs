use super::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeerImpl, RangeBaseAutomationPeer,
    RangeBaseAutomationPeerImpl,
};
use crate::primitives::ScrollBar;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`ScrollBar`].
#[repr(C)]
pub struct ScrollBarAutomationPeer {
    base: RangeBaseAutomationPeer,
}

ferro_class!(ScrollBarAutomationPeer: RangeBaseAutomationPeer);
ferroui_base::ferro_class_info!(ScrollBarAutomationPeer {});

impl FerroObjectImpl for ScrollBarAutomationPeer {}
impl ControlAutomationPeerImpl for ScrollBarAutomationPeer {}
impl RangeBaseAutomationPeerImpl for ScrollBarAutomationPeer {}

impl AutomationPeerImpl for ScrollBarAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "ScrollBar".to_owned()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ScrollBar
    }

    // AutomationControlType::ScrollBar must return IsContentElement false.
    // See http://msdn.microsoft.com/en-us/library/ms743712.aspx
    fn is_content_element_core(_this: &Self) -> bool {
        false
    }
}

impl ScrollBarAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ScrollBar) -> Self {
        Self { base: RangeBaseAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ScrollBar) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
