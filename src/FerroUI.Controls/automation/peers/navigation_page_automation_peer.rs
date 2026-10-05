use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::NavigationPage;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`NavigationPage`].
#[repr(C)]
pub struct NavigationPageAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(NavigationPageAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(NavigationPageAutomationPeer {});

impl FerroObjectImpl for NavigationPageAutomationPeer {}
impl ControlAutomationPeerImpl for NavigationPageAutomationPeer {}

impl AutomationPeerImpl for NavigationPageAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let mut result = Self::parent_get_name_core(this);

        if result.as_deref().is_none_or(str::is_empty) {
            result = this.owner().header().map(|header| ValueTypes::to_display_string(Some(&header)));
        }

        if result.as_deref().is_none_or(str::is_empty) {
            result = this
                .owner()
                .current_page()
                .and_then(|current_page| current_page.header())
                .map(|header| ValueTypes::to_display_string(Some(&header)));
        }

        result
    }
}

impl NavigationPageAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &NavigationPage) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &NavigationPage) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning navigation page.
    pub fn owner(&self) -> Ref<NavigationPage> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a navigation page.")
    }
}
