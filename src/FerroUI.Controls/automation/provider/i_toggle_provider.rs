use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use crate::automation::ElementNotEnabledException;

/// Contains values that specify the toggle state of a UI Automation element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ToggleState {
    /// The UI Automation element is not selected, checked, marked or otherwise activated.
    Off,

    /// The UI Automation element is selected, checked, marked or otherwise activated.
    On,

    /// The UI Automation element is in an indeterminate state.
    Indeterminate,
}

/// Exposes methods and properties to support UI Automation client access to controls that can
/// cycle through a set of states and maintain a particular state.
pub trait IToggleProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets the toggle state of the control.
    ///
    /// Windows: `IToggleProvider.ToggleState`. macOS: `NSAccessibilityProtocol.accessibilityValue`.
    fn toggle_state(&self) -> ToggleState;

    /// Cycles through the toggle states of a control.
    ///
    /// Windows: `IToggleProvider.Toggle`. macOS: `NSAccessibilityProtocol.accessibilityPerformPress`.
    fn toggle(&self) -> Result<(), ElementNotEnabledException>;
}

impl PartialEq for dyn IToggleProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
