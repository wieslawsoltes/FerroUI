use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use crate::platform::ITopLevelImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Point;
use std::rc::Rc;

/// Exposes methods and properties to support UI Automation client access to the root of an
/// automation tree.
///
/// This contract is implemented by automation peers, and should only be implemented on true
/// root elements, such as windows. To embed an automation tree, use `IEmbeddedRootProvider`
/// instead.
///
/// Private API: for use by the platform backends.
pub trait IRootProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets the platform implementation of the TopLevel for the element.
    fn platform_impl(&self) -> Option<Rc<dyn ITopLevelImpl>>;

    /// Gets the currently focused element.
    ///
    /// Windows: `IRawElementProviderFragmentRoot.GetFocus`.
    /// macOS: `UIAccessibility.accessibilityFocusedUIElement`.
    fn get_focus(&self) -> Option<Ref<AutomationPeer>>;

    /// Gets the element at the specified point, expressed in top-level coordinates.
    ///
    /// Windows: `IRawElementProviderFragmentRoot.ElementProviderFromPoint`.
    /// macOS: `NSAccessibilityProtocol.accessibilityHitTest`.
    fn get_peer_from_point(&self, p: Point) -> Option<Ref<AutomationPeer>>;

    /// Raised by the automation peer when the focus changes.
    ///
    /// Windows: `IRawElementProviderAdviseEvents` (focus changed).
    /// macOS: `NSAccessibilityFocusedUIElementChangedNotification`.
    fn focus_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

impl PartialEq for dyn IRootProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
