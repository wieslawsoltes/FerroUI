use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;

/// The error of [`IValueProvider::set_value`]: what the setter throws in the
/// original, which is any exception (an [`ElementNotEnabledException`](crate::automation::ElementNotEnabledException)
/// for an element that is not enabled, a format error for text a peer cannot
/// parse, ...). Inspect it with `downcast_ref`.
pub type ProviderError = Box<dyn std::error::Error + Send + Sync>;

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
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError>;
}

impl PartialEq for dyn IValueProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
