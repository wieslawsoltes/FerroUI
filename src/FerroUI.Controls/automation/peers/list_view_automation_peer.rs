//! WinUI's `ListViewBaseAutomationPeer`, `ListViewAutomationPeer`, `ListViewBaseItemAutomationPeer`
//! and `ListViewItemAutomationPeer` (`dxaml/xcp/dxaml/lib/ListViewBaseAutomationPeer_Partial.cpp`
//! and the item peers of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md).
//!
//! The list is a List with the Selection pattern; its items are ListItems with SelectionItem, and
//! Invoke while the list raises item clicks. WinUI's data item peers
//! (`ListViewItemDataAutomationPeer`, ScrollItem) and the DropTarget and Drag patterns are left
//! out: FerroUI has no data item peers and no drag and drop of items.

use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeerImpl,
    ItemsControlAutomationPeerImpl, ListItemAutomationPeer, SelectingItemsControlAutomationPeer,
    SelectingItemsControlAutomationPeerImpl,
};
use crate::automation::provider::{IInvokeProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledError;
use crate::{ListView, ListViewBase, ListViewBaseItem, ListViewItem};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// The automation peer of a [`ListViewBase`]: a List with the Selection pattern.
#[repr(C)]
pub struct ListViewBaseAutomationPeer {
    base: SelectingItemsControlAutomationPeer,
}

ferro_class!(ListViewBaseAutomationPeer: SelectingItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(ListViewBaseAutomationPeer {});

impl FerroObjectImpl for ListViewBaseAutomationPeer {}
impl ControlAutomationPeerImpl for ListViewBaseAutomationPeer {}
impl ItemsControlAutomationPeerImpl for ListViewBaseAutomationPeer {}
impl SelectingItemsControlAutomationPeerImpl for ListViewBaseAutomationPeer {}

impl AutomationPeerImpl for ListViewBaseAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::List
    }
}

impl ListViewBaseAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ListViewBase) -> Self {
        Self { base: SelectingItemsControlAutomationPeer::construct(owner) }
    }

    pub fn new(owner: &ListViewBase) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}

/// The automation peer of a [`ListView`].
#[repr(C)]
pub struct ListViewAutomationPeer {
    base: ListViewBaseAutomationPeer,
}

ferro_class!(ListViewAutomationPeer: ListViewBaseAutomationPeer);
ferroui_base::ferro_class_info!(ListViewAutomationPeer {});

impl FerroObjectImpl for ListViewAutomationPeer {}
impl ControlAutomationPeerImpl for ListViewAutomationPeer {}
impl ItemsControlAutomationPeerImpl for ListViewAutomationPeer {}
impl SelectingItemsControlAutomationPeerImpl for ListViewAutomationPeer {}

impl AutomationPeerImpl for ListViewAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "ListView".to_owned()
    }
}

impl ListViewAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ListView) -> Self {
        Self { base: ListViewBaseAutomationPeer::construct(owner) }
    }

    pub fn new(owner: &ListView) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}

/// The automation peer of a [`ListViewBaseItem`]: a ListItem with SelectionItem, and Invoke while
/// its list raises item clicks.
#[repr(C)]
pub struct ListViewBaseItemAutomationPeer {
    base: ListItemAutomationPeer,
}

ferro_class!(ListViewBaseItemAutomationPeer: ListItemAutomationPeer);
ferroui_base::ferro_class_info!(ListViewBaseItemAutomationPeer {
    interfaces: [Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider]
});

impl FerroObjectImpl for ListViewBaseItemAutomationPeer {}
impl ControlAutomationPeerImpl for ListViewBaseItemAutomationPeer {}
impl AutomationPeerImpl for ListViewBaseItemAutomationPeer {}

impl IInvokeProvider for ProviderAdapter<ListViewBaseItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        self.0.invoke()
    }
}

impl ListViewBaseItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ListViewBaseItem) -> Self {
        Self { base: ListItemAutomationPeer::construct(owner) }
    }

    pub fn new(owner: &ListViewBaseItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn list(&self) -> Option<(Ref<ListViewBase>, i32)> {
        let owner = self.owner();
        let list =
            crate::ItemsControl::items_control_from_item_container(&owner.clone().upcast())?.cast::<ListViewBase>()?;
        let index = list.index_from_container(&owner.upcast());
        Some((list, index))
    }

    /// Whether the item can be invoked: its list raises item clicks (WinUI's Invoke pattern).
    pub fn is_invokable(&self) -> bool {
        self.list().is_some_and(|(list, _)| list.is_item_click_enabled())
    }

    /// Clicks the item (WinUI's `Invoke`, through `ItemClick`).
    pub fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        self.ensure_enabled()?;
        if let Some((list, index)) = self.list() {
            if index >= 0 {
                list.invoke_item(index);
            }
        }
        Ok(())
    }
}

/// The automation peer of a [`ListViewItem`].
#[repr(C)]
pub struct ListViewItemAutomationPeer {
    base: ListViewBaseItemAutomationPeer,
}

ferro_class!(ListViewItemAutomationPeer: ListViewBaseItemAutomationPeer);
ferroui_base::ferro_class_info!(ListViewItemAutomationPeer {});

impl FerroObjectImpl for ListViewItemAutomationPeer {}
impl ControlAutomationPeerImpl for ListViewItemAutomationPeer {}

impl AutomationPeerImpl for ListViewItemAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "ListViewItem".to_owned()
    }
}

impl ListViewItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ListViewItem) -> Self {
        Self { base: ListViewBaseItemAutomationPeer::construct(owner) }
    }

    pub fn new(owner: &ListViewItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
