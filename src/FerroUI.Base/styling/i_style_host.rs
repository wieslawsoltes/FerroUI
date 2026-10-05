use super::{IStyle, Styles};
use crate::controls::{IResourceHost, ResourceHostRef};
use crate::{ObjectType, Ref, StyledElement, Upcast};
use std::ops::Deref;
use std::rc::Rc;

/// Defines an element that has a [`Styles`] collection.
///
/// Implemented by [`StyledElement`] and by the application object, which
/// holds the global styles and is the root of the styling-parent chain. Every
/// style host is also a resource host.
pub trait IStyleHost: IResourceHost {
    /// Whether the styles collection has been created.
    ///
    /// The styles collection is created lazily; this lets a style lookup
    /// skip hosts without styles without creating the collection.
    fn is_styles_initialized(&self) -> bool;

    /// The styles for the element.
    fn styles(&self) -> Ref<Styles>;

    /// The parent style host element.
    fn styling_parent(&self) -> Option<StyleHostRef>;

    /// Called when styles are added to the styles collection of the host or
    /// a nested styles collection.
    fn styles_added(&self, styles: &[Rc<dyn IStyle>]);

    /// Called when styles are removed from the styles collection of the host
    /// or a nested styles collection.
    fn styles_removed(&self, styles: &[Rc<dyn IStyle>]);
}

/// A strong reference to a style host: a styled element, or another host
/// such as the application object.
#[derive(Clone)]
pub enum StyleHostRef {
    Element(Ref<StyledElement>),
    Other(Rc<dyn IStyleHost>),
}

impl StyleHostRef {
    /// The host as a styled element, if it is one.
    #[inline]
    pub fn as_element(&self) -> Option<&Ref<StyledElement>> {
        match self {
            StyleHostRef::Element(e) => Some(e),
            StyleHostRef::Other(_) => None,
        }
    }

    /// The host as a resource host.
    pub fn to_resource_host(&self) -> ResourceHostRef {
        match self {
            StyleHostRef::Element(e) => ResourceHostRef::Element(e.clone()),
            StyleHostRef::Other(h) => ResourceHostRef::Other(h.clone()),
        }
    }
}

impl Deref for StyleHostRef {
    type Target = dyn IStyleHost;

    #[inline]
    fn deref(&self) -> &(dyn IStyleHost + 'static) {
        match self {
            StyleHostRef::Element(e) => {
                let element: &StyledElement = e;
                element
            }
            StyleHostRef::Other(h) => &**h,
        }
    }
}

impl PartialEq for StyleHostRef {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (StyleHostRef::Element(a), StyleHostRef::Element(b)) => a.ptr_eq(b),
            (StyleHostRef::Other(a), StyleHostRef::Other(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            _ => false,
        }
    }
}

impl<T: ObjectType + Upcast<StyledElement>> From<Ref<T>> for StyleHostRef {
    fn from(value: Ref<T>) -> Self {
        StyleHostRef::Element(value.upcast())
    }
}

impl<T: ObjectType + Upcast<StyledElement>> From<&Ref<T>> for StyleHostRef {
    fn from(value: &Ref<T>) -> Self {
        StyleHostRef::Element(value.clone().upcast())
    }
}

impl From<Rc<dyn IStyleHost>> for StyleHostRef {
    fn from(value: Rc<dyn IStyleHost>) -> Self {
        StyleHostRef::Other(value)
    }
}

impl<T: IStyleHost + 'static> From<Rc<T>> for StyleHostRef {
    fn from(value: Rc<T>) -> Self {
        StyleHostRef::Other(value)
    }
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same host.
impl PartialEq for dyn IStyleHost {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}
