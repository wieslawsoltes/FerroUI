use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use crate::automation::ElementNotEnabledException;

/// Exposes methods and properties to support UI Automation client access to controls that
/// initiate or perform a single, unambiguous action and do not maintain state when
/// activated.
pub trait IInvokeProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Sends a request to activate a control and initiate its single, unambiguous action.
    ///
    /// Windows: `IInvokeProvider.Invoke`. macOS: `NSAccessibilityProtocol.accessibilityPerformPress`.
    fn invoke(&self) -> Result<(), ElementNotEnabledException>;
}

impl PartialEq for dyn IInvokeProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
