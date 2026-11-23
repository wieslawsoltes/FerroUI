use crate::automation::peers::AutomationPeer;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Ref;
use std::rc::Rc;

/// Exposes the part of a focused control that has the keyboard focus, when that part is not a
/// control of its own: an element drawn by the control and reached with the keyboard.
///
/// The root of the automation tree reports this peer as the focused one, so that a screen reader
/// follows the focus inside the control.
pub trait IFocusedDescendantProvider {
    /// The automation peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// The peer of the focused part, or `None` when the control itself has the focus.
    fn focused_descendant(&self) -> Option<Ref<AutomationPeer>>;

    /// Calls `handler` when the focused part changes, until the result is disposed of.
    fn focused_descendant_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

impl PartialEq for dyn IFocusedDescendantProvider {
    /// Providers are equal if they are provided by the same automation peer.
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}

/// The peer a root reports as focused: the focused part of `peer`, or `peer`.
pub(crate) fn focused_descendant_or_self(peer: Ref<AutomationPeer>) -> Ref<AutomationPeer> {
    peer.get_provider::<dyn IFocusedDescendantProvider>()
        .and_then(|provider| provider.focused_descendant())
        .unwrap_or(peer)
}
