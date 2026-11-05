//! WinUI's `BreadcrumbBarItemAutomationPeer` (`controls/dev/Breadcrumb/BreadcrumbBarItemAutomationPeer.cpp`
//! and `.h` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright
//! (c) Microsoft Corporation, see NOTICE.md).

use super::{AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::automation::provider::{IInvokeProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledError;
use crate::BreadcrumbBarItem;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// The automation peer of a [`BreadcrumbBarItem`]: a button ("breadcrumb bar item") with the invoke
/// pattern, which clicks the item, as in WinUI.
#[repr(C)]
pub struct BreadcrumbBarItemAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(BreadcrumbBarItemAutomationPeer: ControlAutomationPeer);
ferro_class_info!(BreadcrumbBarItemAutomationPeer {
    interfaces: [Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider],
    markup: { namespace: "FerroUI.Automation.Peers" },
});
ferroui_base::ferro_impl_classes!(BreadcrumbBarItemAutomationPeer: FerroObjectImpl);

impl ControlAutomationPeerImpl for BreadcrumbBarItemAutomationPeer {}

impl AutomationPeerImpl for BreadcrumbBarItemAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Button
    }

    fn get_class_name_core(_this: &Self) -> String {
        "BreadcrumbBarItem".to_owned()
    }

    fn get_localized_control_type_core(_this: &Self) -> String {
        "breadcrumb bar item".to_owned()
    }
}

impl IInvokeProvider for ProviderAdapter<BreadcrumbBarItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        self.0.ensure_enabled()?;
        let owner: Ref<BreadcrumbBarItem> = ControlAutomationPeer::owner(&self.0).cast().expect("The owner of the peer is a breadcrumb bar item.");
        owner.on_click_event();
        Ok(())
    }
}

impl BreadcrumbBarItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &BreadcrumbBarItem) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &BreadcrumbBarItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
