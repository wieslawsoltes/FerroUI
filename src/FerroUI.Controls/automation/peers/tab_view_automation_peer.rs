//! WinUI's `TabViewAutomationPeer` and `TabViewItemAutomationPeer`
//! (`controls/dev/TabView/TabViewAutomationPeer.cpp` and `TabViewItemAutomationPeer.cpp` of
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl, ListViewItemAutomationPeer,
};
use crate::automation::provider::{ISelectionProvider, ProviderAdapter};
use crate::{TabView, TabViewItem};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// The automation peer of a [`TabView`]: a Tab with the Selection pattern, one tab selected.
#[repr(C)]
pub struct TabViewAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TabViewAutomationPeer: ControlAutomationPeer);
ferro_class_info!(TabViewAutomationPeer {
    markup: { namespace: "FerroUI.Automation.Peers" },
    interfaces: [Rc<dyn ISelectionProvider> => ProviderAdapter::as_selection_provider],
});

impl FerroObjectImpl for TabViewAutomationPeer {}
impl ControlAutomationPeerImpl for TabViewAutomationPeer {}

impl AutomationPeerImpl for TabViewAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Tab
    }

    fn get_class_name_core(_this: &Self) -> String {
        "TabView".to_string()
    }
}

impl ISelectionProvider for ProviderAdapter<TabViewAutomationPeer> {
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
        let owner = self.0.owner();
        owner
            .container_from_index(owner.selected_index())
            .map(|tab| vec![ControlAutomationPeer::create_peer_for_element(&tab)])
            .unwrap_or_default()
    }
}

impl TabViewAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TabView) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    pub fn new(owner: &TabView) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// The tab view of the peer.
    pub fn owner(&self) -> Ref<TabView> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a tab view.")
    }
}

/// The automation peer of a [`TabViewItem`]: a TabItem, named by its header when it has no name.
#[repr(C)]
pub struct TabViewItemAutomationPeer {
    base: ListViewItemAutomationPeer,
}

ferro_class!(TabViewItemAutomationPeer: ListViewItemAutomationPeer);
ferro_class_info!(TabViewItemAutomationPeer { markup: { namespace: "FerroUI.Automation.Peers" } });

impl FerroObjectImpl for TabViewItemAutomationPeer {}
impl ControlAutomationPeerImpl for TabViewItemAutomationPeer {}

impl AutomationPeerImpl for TabViewItemAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TabItem
    }

    fn get_class_name_core(_this: &Self) -> String {
        "TabViewItem".to_string()
    }

    fn get_name_core(this: &Self) -> Option<String> {
        Self::parent_get_name_core(this).filter(|name| !name.is_empty()).or_else(|| {
            let owner = ControlAutomationPeer::owner(this).cast::<TabViewItem>()?;
            owner.header().map(|header| ValueTypes::to_display_string(Some(&header)))
        })
    }
}

impl TabViewItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TabViewItem) -> Self {
        Self { base: ListViewItemAutomationPeer::construct(owner) }
    }

    pub fn new(owner: &TabViewItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
