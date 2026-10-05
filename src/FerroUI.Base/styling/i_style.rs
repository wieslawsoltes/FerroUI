use crate::controls::{IResourceNode, IResourceProvider};
use crate::FerroObject;
use std::rc::Rc;

/// Defines the interface for styles.
///
/// Implemented by [`Style`](super::Style), [`ControlTheme`](super::ControlTheme),
/// [`ContainerQuery`](super::ContainerQuery), [`Styles`](super::Styles) and
/// style includes.
pub trait IStyle: IResourceNode {
    /// A snapshot of the collection of child styles.
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>>;

    /// The style as an object, when it is a class instance. Used for
    /// identity comparison and type tests.
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }

    /// The style as a resource provider, if it is one.
    fn as_resource_provider(&self) -> Option<&dyn IResourceProvider> {
        None
    }
}

/// Identity comparison of two style handles.
pub fn style_ptr_eq(a: &Rc<dyn IStyle>, b: &Rc<dyn IStyle>) -> bool {
    **a == **b
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same object, whichever adapter they were made
/// from.
impl PartialEq for dyn IStyle {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            (None, None) => std::ptr::addr_eq(self as *const Self, other as *const Self),
            _ => false,
        }
    }
}


thread_local! {
    static EMPTY: Rc<Vec<Rc<dyn IStyle>>> = Rc::new(Vec::new());
}

/// A shared empty list of styles.
pub(crate) fn empty_styles() -> Rc<Vec<Rc<dyn IStyle>>> {
    EMPTY.with(Rc::clone)
}
