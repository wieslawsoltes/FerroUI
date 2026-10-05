use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::CarouselPage;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`CarouselPage`].
#[repr(C)]
pub struct CarouselPageAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(CarouselPageAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(CarouselPageAutomationPeer {});

impl FerroObjectImpl for CarouselPageAutomationPeer {}
impl ControlAutomationPeerImpl for CarouselPageAutomationPeer {}

impl AutomationPeerImpl for CarouselPageAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let mut result = Self::parent_get_name_core(this);

        if result.as_deref().is_none_or(str::is_empty) {
            result = this.owner().header().map(|header| ValueTypes::to_display_string(Some(&header)));
        }

        result
    }
}

impl CarouselPageAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &CarouselPage) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &CarouselPage) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning carousel page.
    pub fn owner(&self) -> Ref<CarouselPage> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a carousel page.")
    }
}
