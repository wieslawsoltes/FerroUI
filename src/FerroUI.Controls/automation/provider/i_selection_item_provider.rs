use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use super::ISelectionProvider;
use crate::automation::ElementNotEnabledException;
use std::rc::Rc;

/// Exposes methods and properties to support access by a UI Automation client to individual,
/// selectable child controls of containers that implement `ISelectionProvider`.
pub trait ISelectionItemProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets a value that indicates whether an item is selected.
    fn is_selected(&self) -> bool;

    /// Gets the UI Automation provider that implements `ISelectionProvider` and acts as the
    /// container for the calling object.
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>>;

    /// Adds the current element to the collection of selected items.
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException>;

    /// Removes the current element from the collection of selected items.
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException>;

    /// Clears any existing selection and then selects the current element.
    fn select(&self) -> Result<(), ElementNotEnabledException>;
}

impl PartialEq for dyn ISelectionItemProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
