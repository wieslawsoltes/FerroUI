use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use crate::automation::ElementNotEnabledException;

/// The amount to scroll by, of a scroll request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollAmount {
    LargeDecrement,
    SmallDecrement,
    NoAmount,
    LargeIncrement,
    SmallIncrement,
}

/// Exposes methods and properties to support access by a UI Automation client to a control
/// that acts as a scrollable container for a collection of child objects.
pub trait IScrollProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets a value that indicates whether the control can scroll horizontally.
    ///
    /// Windows: `IScrollProvider.HorizontallyScrollable`. macOS: no mapping.
    fn horizontally_scrollable(&self) -> bool;

    /// Gets the current horizontal scroll position.
    ///
    /// Windows: `IScrollProvider.HorizontalScrollPercent`. macOS: no mapping.
    fn horizontal_scroll_percent(&self) -> f64;

    /// Gets the current horizontal view size.
    ///
    /// Windows: `IScrollProvider.HorizontalViewSize`. macOS: no mapping.
    fn horizontal_view_size(&self) -> f64;

    /// Gets a value that indicates whether the control can scroll vertically.
    ///
    /// Windows: `IScrollProvider.VerticallyScrollable`. macOS: no mapping.
    fn vertically_scrollable(&self) -> bool;

    /// Gets the current vertical scroll position.
    ///
    /// Windows: `IScrollProvider.VerticalScrollPercent`. macOS: no mapping.
    fn vertical_scroll_percent(&self) -> f64;

    /// Gets the vertical view size.
    ///
    /// Windows: `IScrollProvider.VerticalViewSize`. macOS: no mapping.
    fn vertical_view_size(&self) -> f64;

    /// Scrolls the visible region of the content area horizontally and vertically.
    ///
    /// Windows: `IScrollProvider.Scroll`.
    /// macOS: `NSAccessibilityProtocol.accessibilityPerformScroll*ByPage` (large increments
    /// and decrements only).
    fn scroll(&self, horizontal_amount: ScrollAmount, vertical_amount: ScrollAmount) -> Result<(), ElementNotEnabledException>;

    /// Sets the horizontal and vertical scroll position as a percentage of the total content
    /// area within the control.
    ///
    /// Windows: `IScrollProvider.SetScrollPercent`. macOS: no mapping.
    fn set_scroll_percent(&self, horizontal_percent: f64, vertical_percent: f64) -> Result<(), ElementNotEnabledException>;
}

impl PartialEq for dyn IScrollProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
