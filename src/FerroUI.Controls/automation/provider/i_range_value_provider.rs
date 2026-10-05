use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use crate::automation::ElementNotEnabledException;

/// Exposes methods and properties to support access by a UI Automation client to controls
/// that can be set to a value within a range.
pub trait IRangeValueProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets a value that indicates whether the value of a control is read-only.
    ///
    /// Windows: `IRangeValueProvider.IsReadOnly`. macOS: no mapping.
    fn is_read_only(&self) -> bool;

    /// Gets the minimum range value that is supported by the control.
    ///
    /// Windows: `IRangeValueProvider.Minimum`. macOS: `NSAccessibilityProtocol.accessibilityMinValue`.
    fn minimum(&self) -> f64;

    /// Gets the maximum range value that is supported by the control.
    ///
    /// Windows: `IRangeValueProvider.Maximum`. macOS: `NSAccessibilityProtocol.accessibilityMaxValue`.
    fn maximum(&self) -> f64;

    /// Gets the value of the control.
    ///
    /// Windows: `IRangeValueProvider.Value`. macOS: `NSAccessibilityProtocol.accessibilityValue`.
    fn value(&self) -> f64;

    /// Gets the value that is added to or subtracted from the Value property when a large
    /// change is made, such as with the PAGE DOWN key.
    ///
    /// Windows: `IRangeValueProvider.LargeChange`. macOS: no mapping.
    fn large_change(&self) -> f64;

    /// Gets the value that is added to or subtracted from the Value property when a small
    /// change is made, such as with an arrow key.
    ///
    /// Windows: `IRangeValueProvider.SmallChange`. macOS: no mapping.
    fn small_change(&self) -> f64;

    /// Sets the value of the control.
    ///
    /// Windows: `IRangeValueProvider.SetValue`. macOS: `NSAccessibilityProtocol.setAccessibilityValue`.
    fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException>;
}

impl PartialEq for dyn IRangeValueProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
