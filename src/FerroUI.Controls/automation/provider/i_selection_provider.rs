use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;

/// Exposes methods and properties to support access by a UI Automation client to controls
/// that act as containers for a collection of individual, selectable child items.
pub trait ISelectionProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets a value that indicates whether the provider allows more than one child element
    /// to be selected concurrently.
    ///
    /// Windows: `ISelectionProvider.CanSelectMultiple`. macOS: no mapping.
    fn can_select_multiple(&self) -> bool;

    /// Gets a value that indicates whether the provider requires at least one child element
    /// to be selected.
    ///
    /// Windows: `ISelectionProvider.IsSelectionRequired`. macOS: no mapping.
    fn is_selection_required(&self) -> bool;

    /// Retrieves a provider for each child element that is selected.
    ///
    /// Windows: `ISelectionProvider.GetSelection`.
    /// macOS: `NSAccessibilityProtocol.accessibilitySelectedChildren`.
    fn get_selection(&self) -> Vec<Ref<AutomationPeer>>;
}

impl PartialEq for dyn ISelectionProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
