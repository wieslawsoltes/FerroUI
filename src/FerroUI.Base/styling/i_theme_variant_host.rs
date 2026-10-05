use super::ThemeVariant;
use crate::controls::IResourceHost;
use crate::reactive::IDisposable;
use std::rc::Rc;

/// Interface for the host element with a theme variant.
pub trait IThemeVariantHost: IResourceHost {
    /// The UI theme that is currently used by the element, which might be
    /// different than the requested theme variant.
    ///
    /// `None` until a theme variant has been established for the element.
    fn actual_theme_variant(&self) -> Option<ThemeVariant>;

    /// Raised when the actual theme variant property value has changed.
    /// Disposing the returned handle unsubscribes.
    fn actual_theme_variant_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same host.
impl PartialEq for dyn IThemeVariantHost {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}
