//! WinUI's `BreadcrumbBarItemAutomationPeer` (`controls/dev/Breadcrumb/BreadcrumbBarItemAutomationPeer.cpp`,
//! `.h` and `BreadcrumbBarAutomationPeer.idl` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IInvokeProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledError;
use crate::BreadcrumbBarItem;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// The localized control type of a breadcrumb bar item (`SR_BreadcrumbBarItemLocalizedControlType`,
/// the English text of WinUI's resources).
const BREADCRUMB_BAR_ITEM_LOCALIZED_CONTROL_TYPE: &str = "breadcrumb bar item";

/// An automation peer which represents a [`BreadcrumbBarItem`]: a button whose invoke clicks the
/// item.
#[repr(C)]
pub struct BreadcrumbBarItemAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(BreadcrumbBarItemAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(BreadcrumbBarItemAutomationPeer {
    interfaces: [Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider]
});

impl FerroObjectImpl for BreadcrumbBarItemAutomationPeer {}

impl ControlAutomationPeerImpl for BreadcrumbBarItemAutomationPeer {}

impl AutomationPeerImpl for BreadcrumbBarItemAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Button
    }

    fn get_class_name_core(_this: &Self) -> String {
        "BreadcrumbBarItem".to_owned()
    }

    fn get_localized_control_type_core(_this: &Self) -> String {
        BREADCRUMB_BAR_ITEM_LOCALIZED_CONTROL_TYPE.to_owned()
    }
}

impl IInvokeProvider for ProviderAdapter<BreadcrumbBarItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        self.0.invoke()
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

    /// Gets the owning breadcrumb bar item.
    pub fn owner(&self) -> Ref<BreadcrumbBarItem> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a breadcrumb bar item.")
    }

    /// Clicks the item (the invoke provider contract).
    pub fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        self.ensure_enabled()?;
        self.owner().on_click_event();
        Ok(())
    }
}
