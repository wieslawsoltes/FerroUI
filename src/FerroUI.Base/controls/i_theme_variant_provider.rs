use super::IResourceProvider;
use crate::styling::ThemeVariant;

/// Resource provider with theme variant awareness.
///
/// Can be used with `ResourceDictionary::theme_dictionaries`. This is useful
/// for specifying multiple resource sets for different themes.
pub trait IThemeVariantProvider: IResourceProvider {
    /// The key that will be used in the theme dictionaries of the containing
    /// resource dictionary.
    fn key(&self) -> Option<ThemeVariant>;

    fn set_key(&self, value: Option<ThemeVariant>);
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same object, whichever adapter they were made
/// from.
impl PartialEq for dyn IThemeVariantProvider {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            (None, None) => std::ptr::addr_eq(self as *const Self, other as *const Self),
            _ => false,
        }
    }
}
