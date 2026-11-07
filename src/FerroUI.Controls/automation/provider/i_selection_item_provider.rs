use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use super::ISelectionProvider;
use crate::automation::AutomationError;
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
    ///
    /// # Errors
    ///
    /// An [`AutomationError`] when the state of the element refuses the call
    /// (the exception the member throws in the original).
    fn add_to_selection(&self) -> Result<(), AutomationError>;

    /// Removes the current element from the collection of selected items.
    ///
    /// # Errors
    ///
    /// An [`AutomationError`] when the state of the element refuses the call
    /// (the exception the member throws in the original).
    fn remove_from_selection(&self) -> Result<(), AutomationError>;

    /// Clears any existing selection and then selects the current element.
    ///
    /// # Errors
    ///
    /// An [`AutomationError`] when the state of the element refuses the call
    /// (the exception the member throws in the original).
    fn select(&self) -> Result<(), AutomationError>;
}

impl PartialEq for dyn ISelectionItemProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
