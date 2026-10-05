use super::{IStyle, IStyleHost};
use crate::reactive::IDisposable;
use std::rc::Rc;

/// The signature of the handlers of global style changes.
pub type GlobalStylesHandler = dyn Fn(&[Rc<dyn IStyle>]);

/// Defines the style host that provides styles global to the application.
///
/// The application object implements this; the root elements of logical trees
/// return it as their styling parent, which makes the global styles and
/// resources part of every style and resource lookup.
pub trait IGlobalStyles: IStyleHost {
    /// Raised when styles are added to the global styles or a nested styles
    /// collection. Disposing the returned handle unsubscribes.
    fn global_styles_added(&self, handler: Rc<GlobalStylesHandler>) -> Rc<dyn IDisposable>;

    /// Raised when styles are removed from the global styles or a nested
    /// styles collection. Disposing the returned handle unsubscribes.
    fn global_styles_removed(&self, handler: Rc<GlobalStylesHandler>) -> Rc<dyn IDisposable>;
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same host.
impl PartialEq for dyn IGlobalStyles {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}
