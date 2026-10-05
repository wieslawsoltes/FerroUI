use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ContentControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{ISelectionItemProvider, ISelectionProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use crate::primitives::SelectingItemsControl;
use crate::{ContentControl, Control, ItemsControl, ListBox, ListBoxItem};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a selectable item of a list (a
/// `ListBoxItem`, a `TabItem`).
#[repr(C)]
pub struct ListItemAutomationPeer {
    base: ContentControlAutomationPeer,
}

ferro_class!(ListItemAutomationPeer: ContentControlAutomationPeer);
ferroui_base::ferro_class_info!(ListItemAutomationPeer { interfaces: [Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider] });

impl FerroObjectImpl for ListItemAutomationPeer {}
impl ControlAutomationPeerImpl for ListItemAutomationPeer {}

impl AutomationPeerImpl for ListItemAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ListItem
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl ISelectionItemProvider for ProviderAdapter<ListItemAutomationPeer> {
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

impl ListItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ContentControl) -> Self {
        Self { base: ContentControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ContentControl) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets a value that indicates whether the item is selected (the
    /// selection item provider contract).
    pub fn is_selected(&self) -> bool {
        self.owner().get_value(ListBoxItem::is_selected_property())
    }

    /// Gets the selection provider of the container of the item (the
    /// selection item provider contract).
    pub fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        if let Some(parent) = self.owner().parent().and_then(|parent| parent.cast::<Control>()) {
            let parent_peer = self.get_or_create(&parent);
            return parent_peer.get_provider::<dyn ISelectionProvider>();
        }

        None
    }

    /// Clears any existing selection and then selects the item (the
    /// selection item provider contract).
    pub fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        let owner = self.owner();
        if let Some(parent) = owner.parent().and_then(|parent| parent.cast::<SelectingItemsControl>()) {
            let index = parent.index_from_container(&owner.upcast());

            if index != -1 {
                parent.set_selected_index(index);
            }
        }

        Ok(())
    }

    // The reference implements the two members below explicitly: they are
    // members of the selection item provider contract only.
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        let owner = self.owner();
        if let Some(parent) = owner.parent().and_then(|parent| parent.cast::<ItemsControl>()) {
            if let Some(selection_model) = parent.get_direct_value(ListBox::selection_property()) {
                let index = parent.index_from_container(&owner.upcast());

                if index != -1 {
                    selection_model.select(index);
                }
            }
        }

        Ok(())
    }

    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        let owner = self.owner();
        if let Some(parent) = owner.parent().and_then(|parent| parent.cast::<ItemsControl>()) {
            if let Some(selection_model) = parent.get_direct_value(ListBox::selection_property()) {
                let index = parent.index_from_container(&owner.upcast());

                if index != -1 {
                    selection_model.deselect(index);
                }
            }
        }

        Ok(())
    }
}
