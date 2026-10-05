use crate::primitives::IPopupHost;
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// Diagnostics interface to retrieve an associated [`IPopupHost`] (internal
/// upstream).
pub trait IPopupHostProvider {
    /// The popup host.
    fn popup_host(&self) -> Option<Rc<dyn IPopupHost>>;

    /// Raised when the popup host changes. Disposing the returned handle
    /// unsubscribes.
    fn popup_host_changed(&self, handler: Rc<dyn Fn(Option<&Rc<dyn IPopupHost>>)>) -> Rc<dyn IDisposable>;
}
