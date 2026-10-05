use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeerImpl, ItemsControlAutomationPeer,
    ItemsControlAutomationPeerImpl,
};
use crate::automation::provider::{ISelectionItemProvider, ISelectionProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use crate::TreeViewItem;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`TreeViewItem`].
#[repr(C)]
pub struct TreeViewItemAutomationPeer {
    base: ItemsControlAutomationPeer,
}

ferro_class!(TreeViewItemAutomationPeer: ItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(TreeViewItemAutomationPeer { interfaces: [Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider] });

impl FerroObjectImpl for TreeViewItemAutomationPeer {}
impl ControlAutomationPeerImpl for TreeViewItemAutomationPeer {}
impl ItemsControlAutomationPeerImpl for TreeViewItemAutomationPeer {}

impl AutomationPeerImpl for TreeViewItemAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TreeItem
    }
}

impl ISelectionItemProvider for ProviderAdapter<TreeViewItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_selected(&self) -> bool {
        self.0.is_selected()
    }

    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0.selection_container()
    }

    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.add_to_selection()
    }

    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.remove_from_selection()
    }

    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.select()
    }
}

impl TreeViewItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeViewItem) -> Self {
        Self { base: ItemsControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeViewItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets a value that indicates whether the item is selected (the
    /// selection item provider contract).
    pub fn is_selected(&self) -> bool {
        self.owner().get_value(TreeViewItem::is_selected_property())
    }

    /// Gets the selection provider of the tree view of the item (the
    /// selection item provider contract).
    pub fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        if let Some(tree_view) = self.owner().cast::<TreeViewItem>().and_then(|item| item.tree_view_owner()) {
            let parent_peer = self.get_or_create(&tree_view);
            return parent_peer.get_provider::<dyn ISelectionProvider>();
        }

        None
    }

    /// Clears any existing selection and then selects the item (the
    /// selection item provider contract).
    pub fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let Some(item) = self.owner().cast::<TreeViewItem>() {
            if let Some(tree_view) = item.tree_view_owner() {
                tree_view.selected_items().clear();
            }
            item.set_is_selected(true);
        }

        Ok(())
    }

    // The reference implements the two members below explicitly: they are
    // members of the selection item provider contract only.
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let Some(item) = self.owner().cast::<TreeViewItem>() {
            item.set_is_selected(true);
        }

        Ok(())
    }

    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let Some(item) = self.owner().cast::<TreeViewItem>() {
            item.set_is_selected(false);
        }

        Ok(())
    }
}
