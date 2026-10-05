use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use crate::automation::ElementNotEnabledException;

/// Exposes methods and properties to support access by a UI Automation client to controls
/// that have an intrinsic value not spanning a range and that can be represented as a string.
pub trait IValueProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets a value that indicates whether the value of a control is read-only.
    ///
    /// Windows: `IValueProvider.IsReadOnly`. macOS: no mapping.
    fn is_read_only(&self) -> bool;

    /// Gets the value of the control.
    ///
    /// Windows: `IValueProvider.Value`. macOS: `NSAccessibilityProtocol.accessibilityValue`.
    fn value(&self) -> Option<String>;

    /// Sets the value of a control.
    ///
    /// Windows: `IValueProvider.SetValue`. macOS: `NSAccessibilityProtocol.setAccessibilityValue`.
    fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException>;
}

impl PartialEq for dyn IValueProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
