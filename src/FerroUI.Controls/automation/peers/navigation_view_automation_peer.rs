//! WinUI's `NavigationViewAutomationPeer` (`controls/dev/NavigationView/NavigationViewAutomationPeer.cpp`
//! and `.h` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright
//! (c) Microsoft Corporation, see NOTICE.md).

use crate::NavigationView;
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{ISelectionProvider, ProviderAdapter};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// The automation peer of a [`NavigationView`]: the selection pattern over its realized
/// containers, as WinUI's peer. The peer holds its owner weakly.
#[repr(C)]
pub struct NavigationViewAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(NavigationViewAutomationPeer: ControlAutomationPeer);
ferro_class_info!(NavigationViewAutomationPeer {
    markup: { namespace: "FerroUI.Automation.Peers" },
    interfaces: [Rc<dyn ISelectionProvider> => ProviderAdapter::as_selection_provider]
});
ferro_impl_classes!(NavigationViewAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);
impl AutomationPeerImpl for NavigationViewAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "NavigationView".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }
}
impl NavigationViewAutomationPeer {
    /// Creates the peer of `owner`.
    pub fn new(owner: &NavigationView) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    /// Gets the navigation view of the peer.
    pub fn owner(&self) -> Ref<NavigationView> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("NavigationView owner")
    }
    /// Gets the peer of the container of the selected item (`GetSelection`).
    pub fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        let Some(owner) =
            ControlAutomationPeer::try_owner(self).and_then(|owner| owner.cast::<NavigationView>())
        else {
            return Vec::new();
        };
        owner
            .container_from_menu_item(&owner.selected_item())
            .map(|container| vec![self.get_or_create(&container)])
            .unwrap_or_default()
    }
    /// One item at most is selected.
    pub fn can_select_multiple(&self) -> bool {
        false
    }
    /// No selection is required.
    pub fn is_selection_required(&self) -> bool {
        false
    }
}
impl ISelectionProvider for ProviderAdapter<NavigationViewAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn can_select_multiple(&self) -> bool {
        self.0.can_select_multiple()
    }
    fn is_selection_required(&self) -> bool {
        self.0.is_selection_required()
    }
    fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        self.0.get_selection()
    }
}
