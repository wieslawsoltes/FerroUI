use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use crate::automation::{ElementNotEnabledException, ExpandCollapseState};

/// Exposes methods and properties to support UI Automation client access to controls that
/// visually expand to display content and collapse to hide content.
pub trait IExpandCollapseProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets the state, expanded or collapsed, of the control.
    ///
    /// Windows: `IExpandCollapseProvider.ExpandCollapseState`.
    /// macOS: `NSAccessibilityProtocol.isAccessibilityExpanded`.
    fn expand_collapse_state(&self) -> ExpandCollapseState;

    /// Gets a value indicating whether expanding the element shows a menu of items to the user,
    /// such as drop-down list.
    ///
    /// Used in OSX to enable the "Show Menu" action on the element.
    fn shows_menu(&self) -> bool;

    /// Displays all child nodes, controls, or content of the control.
    ///
    /// Windows: `IExpandCollapseProvider.Expand`.
    /// macOS: `NSAccessibilityProtocol.accessibilityPerformShowMenu`.
    fn expand(&self) -> Result<(), ElementNotEnabledException>;

    /// Hides all nodes, controls, or content that are descendants of the control.
    ///
    /// Windows: `IExpandCollapseProvider.Collapse`. macOS: no mapping.
    fn collapse(&self) -> Result<(), ElementNotEnabledException>;
}

impl PartialEq for dyn IExpandCollapseProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
