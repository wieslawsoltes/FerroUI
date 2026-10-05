use super::ResourceKey;
use crate::styling::ThemeVariant;
use crate::BoxedValue;

/// The value of a resource. `None` is a resource that is present with a null
/// value.
pub type ResourceValue = Option<BoxedValue>;

/// Represents a resource provider in a tree.
pub trait IResourceNode {
    /// Whether the resource node has resources.
    fn has_resources(&self) -> bool;

    /// Tries to find a resource within the node.
    ///
    /// Returns `Some` with the resource if it was found. See the theme
    /// dictionaries of a resource dictionary for how `theme` is used.
    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue>;

    /// The identity of the node, used for reference equality: the address
    /// of the object the node is, whichever adapter it is seen through.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same object, whichever adapter they were made
/// from.
impl PartialEq for dyn IResourceNode {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}
