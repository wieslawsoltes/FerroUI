// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../../NOTICE.md.

use crate::AutoSuggestBox;
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{IInvokeProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use std::rc::Rc;

#[repr(C)]
pub struct AutoSuggestBoxAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(AutoSuggestBoxAutomationPeer: ControlAutomationPeer);
ferro_class_info!(AutoSuggestBoxAutomationPeer { interfaces: [Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider] });
ferro_impl_classes!(AutoSuggestBoxAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);
impl AutomationPeerImpl for AutoSuggestBoxAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "AutoSuggestBox".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }
}
impl AutoSuggestBoxAutomationPeer {
    pub fn new(owner: &AutoSuggestBox) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    pub fn owner(&self) -> Ref<AutoSuggestBox> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("AutoSuggestBox owner")
    }
}
impl IInvokeProvider for ProviderAdapter<AutoSuggestBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.owner().submit_query();
        Ok(())
    }
}
