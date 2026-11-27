//! WinUI's `PivotAutomationPeer` and `PivotItemAutomationPeer`
//! (`dxaml/phone/lib/PivotAutomationPeer_Partial.cpp`, `PivotItemAutomationPeer_Partial.cpp` of
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft
//! Corporation, see NOTICE.md).
//!
//! WinUI's pivot exposes a Tab with the Selection and Scroll patterns, its items as TabItems, and
//! a data item peer (`PivotItemDataAutomationPeer`) with SelectionItem, ScrollItem and
//! VirtualizedItem. FerroUI has no data item peers: the SelectionItem pattern is on the peer of
//! the item itself (a recorded deviation), the scroll patterns are left out.

use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ContentControlAutomationPeer,
    ControlAutomationPeerImpl, ItemsControlAutomationPeerImpl, SelectingItemsControlAutomationPeer,
    SelectingItemsControlAutomationPeerImpl,
};
use crate::automation::provider::{ISelectionItemProvider, ISelectionProvider, ProviderAdapter};
use crate::automation::AutomationError;
use crate::{Control, Pivot, PivotItem};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// The automation peer of a [`Pivot`]: a Tab with the Selection pattern.
#[repr(C)]
pub struct PivotAutomationPeer {
    base: SelectingItemsControlAutomationPeer,
}

ferro_class!(PivotAutomationPeer: SelectingItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(PivotAutomationPeer {});

impl FerroObjectImpl for PivotAutomationPeer {}
impl ControlAutomationPeerImpl for PivotAutomationPeer {}
impl ItemsControlAutomationPeerImpl for PivotAutomationPeer {}
impl SelectingItemsControlAutomationPeerImpl for PivotAutomationPeer {}

impl AutomationPeerImpl for PivotAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Tab
    }

    fn get_class_name_core(_this: &Self) -> String {
        "Pivot".to_owned()
    }
}

impl PivotAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Pivot) -> Self {
        Self { base: SelectingItemsControlAutomationPeer::construct(owner) }
    }

    pub fn new(owner: &Pivot) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}

/// The automation peer of a [`PivotItem`]: a TabItem with the SelectionItem pattern.
#[repr(C)]
pub struct PivotItemAutomationPeer {
    base: ContentControlAutomationPeer,
}

ferro_class!(PivotItemAutomationPeer: ContentControlAutomationPeer);
ferroui_base::ferro_class_info!(PivotItemAutomationPeer {
    interfaces: [Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider]
});

impl FerroObjectImpl for PivotItemAutomationPeer {}
impl ControlAutomationPeerImpl for PivotItemAutomationPeer {}

impl AutomationPeerImpl for PivotItemAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TabItem
    }

    fn get_class_name_core(_this: &Self) -> String {
        "PivotItem".to_owned()
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let header = this.item().and_then(|item| item.header());
        match header {
            Some(header) => Some(ferroui_base::data::core::ValueTypes::to_display_string(Some(&header))),
            None => Self::parent_get_name_core(this),
        }
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl ISelectionItemProvider for ProviderAdapter<PivotItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_selected(&self) -> bool {
        self.0.is_selected()
    }

    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        let pivot = self.0.item()?.parent_pivot()?;
        self.0.get_or_create(&pivot.upcast::<Control>()).get_provider::<dyn ISelectionProvider>()
    }

    fn add_to_selection(&self) -> Result<(), AutomationError> {
        // One item is selected at a time: adding is selecting.
        self.0.select()
    }

    fn remove_from_selection(&self) -> Result<(), AutomationError> {
        // A pivot always has its selected item.
        Ok(())
    }

    fn select(&self) -> Result<(), AutomationError> {
        self.0.select()
    }
}

impl PivotItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &PivotItem) -> Self {
        Self { base: ContentControlAutomationPeer::construct(owner) }
    }

    pub fn new(owner: &PivotItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn item(&self) -> Option<Ref<PivotItem>> {
        self.owner().cast::<PivotItem>()
    }

    /// Whether the item is the selected item of its pivot.
    pub fn is_selected(&self) -> bool {
        let Some(item) = self.item() else { return false };
        let Some(pivot) = item.parent_pivot() else { return false };
        let index = pivot.index_from_container(&item.upcast());
        index >= 0 && index == pivot.selected_index()
    }

    /// Selects the item in its pivot (WinUI's `Select`, which moves to the item).
    pub fn select(&self) -> Result<(), AutomationError> {
        self.ensure_enabled()?;
        if let Some(item) = self.item() {
            if let Some(pivot) = item.parent_pivot() {
                let index = pivot.index_from_container(&item.clone().upcast());
                if index >= 0 {
                    pivot.set_selected_index(index);
                }
            }
        }
        Ok(())
    }
}
