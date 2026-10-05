use super::{IResourceNode, ResourceHostRef};
use crate::reactive::IDisposable;
use crate::FerroObject;
use std::rc::Rc;

/// Represents an object that can be queried for resources but does not appear
/// in the logical tree.
///
/// The interface represented by this trait is implemented by resource
/// dictionaries, styles and the like.
pub trait IResourceProvider: IResourceNode {
    /// The owner of the resource provider.
    ///
    /// If the resource provider is, or is a child of, a resource dictionary or
    /// style that is hosted by an element, this is that element.
    fn owner(&self) -> Option<ResourceHostRef>;

    /// Raised when the owner of the resource provider changes.
    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;

    /// Adds an owner to the resource provider.
    fn add_owner(&self, owner: &ResourceHostRef);

    /// Removes a resource provider owner.
    fn remove_owner(&self, owner: &ResourceHostRef);

    /// The provider as an object, when it is a class instance. Used for
    /// identity comparison and type tests.
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }
}

/// Identity comparison of two resource provider handles.
pub fn resource_provider_ptr_eq(a: &Rc<dyn IResourceProvider>, b: &Rc<dyn IResourceProvider>) -> bool {
    **a == **b
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same object, whichever adapter they were made
/// from.
impl PartialEq for dyn IResourceProvider {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            (None, None) => std::ptr::addr_eq(self as *const Self, other as *const Self),
            _ => false,
        }
    }
}

