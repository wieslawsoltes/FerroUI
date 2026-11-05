//! WinUI's `PagerControlAutomationPeer` (`controls/dev/PagerControl/PagerControlAutomationPeer.cpp`
//! and `.h` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md).

use super::{AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::automation::provider::{ISelectionProvider, ProviderAdapter};
use crate::automation::SelectionPatternIdentifiers;
use crate::PagerControl;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// The automation peer of a [`PagerControl`]: a menu with the selection pattern, as in WinUI.
///
/// The selection is empty, as WinUI's `GetSelection` returns it: the number panel mixes buttons
/// and ellipses, so the index of an element is not the index of a page. A change of the
/// selected page invalidates the selection pattern (WinUI's `SelectionPatternOnInvalidated`),
/// which FerroUI's automation reports as a change of its `Selection` property.
#[repr(C)]
pub struct PagerControlAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(PagerControlAutomationPeer: ControlAutomationPeer);
ferro_class_info!(PagerControlAutomationPeer {
    interfaces: [Rc<dyn ISelectionProvider> => ProviderAdapter::as_selection_provider],
    markup: { namespace: "FerroUI.Automation.Peers" },
});
ferroui_base::ferro_impl_classes!(PagerControlAutomationPeer: FerroObjectImpl);

impl ControlAutomationPeerImpl for PagerControlAutomationPeer {}

impl AutomationPeerImpl for PagerControlAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Menu
    }

    fn get_class_name_core(_this: &Self) -> String {
        "PagerControl".to_owned()
    }
}

impl ISelectionProvider for ProviderAdapter<PagerControlAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn can_select_multiple(&self) -> bool {
        false
    }

    fn is_selection_required(&self) -> bool {
        true
    }

    fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        Vec::new()
    }
}

impl PagerControlAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &PagerControl) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &PagerControl) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Reports that the selected page changed (WinUI's `RaiseSelectionChanged`).
    pub(crate) fn raise_selection_changed(&self) {
        self.raise_property_changed_event(SelectionPatternIdentifiers::selection_property(), None, None);
    }
}
