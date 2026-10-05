use super::{IResourceNode, ResourcesChangedEventArgs};
use crate::reactive::IDisposable;
use crate::styling::{IStyleHost, IThemeVariantHost};
use crate::{ObjectType, Ref, StyledElement, Upcast, WeakRef};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// Represents an element which hosts resources.
///
/// Implemented by [`StyledElement`] and by the application object, the root
/// of the styling-parent chain.
pub trait IResourceHost: IResourceNode {
    /// Raised when the resources change on the element or an ancestor of the
    /// element. Disposing the returned handle unsubscribes.
    fn resources_changed(&self, handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Notifies the resource host that one or more of its hosted resources
    /// has changed.
    ///
    /// This method will be called automatically by the framework, you should
    /// not need to call this method yourself. It is called when the resources
    /// hosted by this element have changed, and is usually called by a
    /// resource dictionary or style hosted by the element in response to a
    /// resource being added or removed.
    fn notify_hosted_resources_changed(&self, e: ResourcesChangedEventArgs);

    /// The host as a style host, if it is one.
    fn as_style_host(&self) -> Option<&dyn IStyleHost> {
        None
    }

    /// The host as a theme variant host, if it is one.
    fn as_theme_variant_host(&self) -> Option<&dyn IThemeVariantHost> {
        None
    }
}

/// A strong reference to a resource host: a styled element, or another host
/// such as the application object.
#[derive(Clone)]
pub enum ResourceHostRef {
    Element(Ref<StyledElement>),
    Other(Rc<dyn IResourceHost>),
}

impl ResourceHostRef {
    /// Creates a weak reference to the host.
    pub fn downgrade(&self) -> WeakResourceHost {
        match self {
            ResourceHostRef::Element(e) => WeakResourceHost::Element(e.downgrade()),
            ResourceHostRef::Other(h) => WeakResourceHost::Other(Rc::downgrade(h)),
        }
    }

    /// The host as a styled element, if it is one.
    pub fn as_element(&self) -> Option<&Ref<StyledElement>> {
        match self {
            ResourceHostRef::Element(e) => Some(e),
            ResourceHostRef::Other(_) => None,
        }
    }
}

impl Deref for ResourceHostRef {
    type Target = dyn IResourceHost;

    #[inline]
    fn deref(&self) -> &(dyn IResourceHost + 'static) {
        match self {
            ResourceHostRef::Element(e) => {
                let element: &StyledElement = e;
                element
            }
            ResourceHostRef::Other(h) => &**h,
        }
    }
}

impl PartialEq for ResourceHostRef {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ResourceHostRef::Element(a), ResourceHostRef::Element(b)) => a.ptr_eq(b),
            (ResourceHostRef::Other(a), ResourceHostRef::Other(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            _ => false,
        }
    }
}

impl<T: ObjectType + Upcast<StyledElement>> From<Ref<T>> for ResourceHostRef {
    fn from(value: Ref<T>) -> Self {
        ResourceHostRef::Element(value.upcast())
    }
}

impl<T: ObjectType + Upcast<StyledElement>> From<&Ref<T>> for ResourceHostRef {
    fn from(value: &Ref<T>) -> Self {
        ResourceHostRef::Element(value.clone().upcast())
    }
}

impl From<Rc<dyn IResourceHost>> for ResourceHostRef {
    fn from(value: Rc<dyn IResourceHost>) -> Self {
        ResourceHostRef::Other(value)
    }
}

impl<T: IResourceHost + 'static> From<Rc<T>> for ResourceHostRef {
    fn from(value: Rc<T>) -> Self {
        ResourceHostRef::Other(value)
    }
}

/// A weak reference to a resource host.
#[derive(Clone)]
pub enum WeakResourceHost {
    Element(WeakRef<StyledElement>),
    Other(Weak<dyn IResourceHost>),
}

impl WeakResourceHost {
    /// Returns a strong reference if the host is still alive.
    pub fn upgrade(&self) -> Option<ResourceHostRef> {
        match self {
            WeakResourceHost::Element(e) => e.upgrade().map(ResourceHostRef::Element),
            WeakResourceHost::Other(h) => h.upgrade().map(ResourceHostRef::Other),
        }
    }
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same host.
impl PartialEq for dyn IResourceHost {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}
