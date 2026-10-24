//! The automation peer of WinUI's `AutoSuggestBox`
//! (`dxaml/xcp/dxaml/lib/AutoSuggestBoxAutomationPeer_Partial.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::{AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::automation::provider::{IInvokeProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use crate::AutoSuggestBox;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents an [`AutoSuggestBox`]: a group whose invoke pattern
/// submits the query.
#[repr(C)]
pub struct AutoSuggestBoxAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(AutoSuggestBoxAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(AutoSuggestBoxAutomationPeer {
    interfaces: [Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider]
});
ferro_impl_classes!(AutoSuggestBoxAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);

impl AutomationPeerImpl for AutoSuggestBoxAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "AutoSuggestBox".to_owned()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }
}

impl AutoSuggestBoxAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &AutoSuggestBox) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &AutoSuggestBox) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning box.
    pub fn owner(&self) -> Ref<AutoSuggestBox> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is an auto-suggest box.")
    }
}

impl IInvokeProvider for ProviderAdapter<AutoSuggestBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    // AutoSuggestBoxAutomationPeer::Invoke: ProgrammaticSubmitQuery.
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.owner().programmatic_submit_query();
        Ok(())
    }
}
