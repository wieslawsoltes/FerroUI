// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../../NOTICE.md.
// Adapted from NavigationViewAutomationPeer at microsoft/microsoft-ui-xaml a71c42bd.

use crate::NavigationView;
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{ISelectionProvider, ProviderAdapter};
use ferroui_base::{
    FerroObjectImpl, Ref, ferro_class, ferro_class_info, ferro_impl_classes, instantiate,
};
use std::rc::Rc;

/// Native selection-provider adapter for the realized NavigationView containers.
/// The underlying ControlAutomationPeer retains only a weak reference to its owner.
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
    pub fn new(owner: &NavigationView) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    pub fn owner(&self) -> Ref<NavigationView> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("NavigationView owner")
    }
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
    pub fn can_select_multiple(&self) -> bool {
        false
    }
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
