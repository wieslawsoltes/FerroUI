use super::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeerImpl, RangeBaseAutomationPeer,
    RangeBaseAutomationPeerImpl,
};
use crate::Slider;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`Slider`].
#[repr(C)]
pub struct SliderAutomationPeer {
    base: RangeBaseAutomationPeer,
}

ferro_class!(SliderAutomationPeer: RangeBaseAutomationPeer);
ferroui_base::ferro_class_info!(SliderAutomationPeer {});

ferroui_base::ferro_impl_classes!(SliderAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl, RangeBaseAutomationPeerImpl);

ferroui_base::ferro_overrides! { impl AutomationPeerImpl for SliderAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "Slider".to_owned()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Slider
    }
} }

impl SliderAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Slider) -> Self {
        Self { base: RangeBaseAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Slider) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
