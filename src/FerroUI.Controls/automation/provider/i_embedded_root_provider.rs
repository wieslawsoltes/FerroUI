use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Point;
use std::rc::Rc;

/// Exposes methods and properties to support UI Automation client access to the root of an
/// automation tree hosted by another UI framework.
///
/// This contract can be implemented on custom automation peers to allow them to be embedded
/// in another automation tree.
///
/// Private API: for use by the platform backends.
pub trait IEmbeddedRootProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Gets the currently focused element.
    fn get_focus(&self) -> Option<Ref<AutomationPeer>>;

    /// Gets the element at the specified point, expressed in top-level coordinates.
    fn get_peer_from_point(&self, p: Point) -> Option<Ref<AutomationPeer>>;

    /// Raised by the automation peer when the focus changes.
    fn focus_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

impl PartialEq for dyn IEmbeddedRootProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
