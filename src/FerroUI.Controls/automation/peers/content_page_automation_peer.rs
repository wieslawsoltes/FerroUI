use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::ContentPage;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`ContentPage`].
#[repr(C)]
pub struct ContentPageAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ContentPageAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(ContentPageAutomationPeer {});

impl FerroObjectImpl for ContentPageAutomationPeer {}
impl ControlAutomationPeerImpl for ContentPageAutomationPeer {}

impl AutomationPeerImpl for ContentPageAutomationPeer {
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

impl ContentPageAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ContentPage) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ContentPage) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning content page.
    pub fn owner(&self) -> Ref<ContentPage> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a content page.")
    }
}
