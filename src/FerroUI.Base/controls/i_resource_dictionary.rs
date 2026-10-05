use super::{IResourceProvider, IThemeVariantProvider, ResourceKey, ResourceValue};
use crate::styling::ThemeVariant;
use std::rc::Rc;

/// An indexed dictionary of resources.
pub trait IResourceDictionary: IResourceProvider {
    /// A snapshot of the collection of child resource dictionaries.
    fn merged_dictionaries_snapshot(&self) -> Rc<Vec<Rc<dyn IResourceProvider>>>;

    /// A snapshot of the collection of resource providers used for specific
    /// theme variants.
    fn theme_dictionaries_snapshot(&self) -> Vec<(ThemeVariant, Rc<dyn IThemeVariantProvider>)>;

    /// The number of resources in the dictionary itself.
    fn count(&self) -> usize;

    /// Adds a resource. Panics if the key is already present.
    fn add(&self, key: ResourceKey, value: ResourceValue);

    /// Adds or replaces a resource.
    fn set(&self, key: ResourceKey, value: ResourceValue);

    /// Whether the dictionary itself contains the key.
    fn contains_key(&self, key: &ResourceKey) -> bool;

    /// Removes a resource. Returns whether it was present.
    fn remove(&self, key: &ResourceKey) -> bool;

    /// Gets a resource of the dictionary itself.
    fn try_get_value(&self, key: &ResourceKey) -> Option<ResourceValue>;

    /// Removes all resources of the dictionary itself.
    fn clear(&self);
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same object, whichever adapter they were made
/// from.
impl PartialEq for dyn IResourceDictionary {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            (None, None) => std::ptr::addr_eq(self as *const Self, other as *const Self),
            _ => false,
        }
    }
}
