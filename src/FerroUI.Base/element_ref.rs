use crate::{ObjectType, Ref, Upcast, WeakRef};
use std::fmt;

/// A reference to an element that the holder does not own: the value type
/// of properties such as a placement target, the target of a label or the
/// element an adorner adorns.
///
/// Such a property points sideways or up the tree — often at an ancestor of
/// the element it is set on — so holding the element strongly would make
/// the two keep each other alive. An `ElementRef` holds it weakly and
/// compares by identity. Classes expose these properties with getters that
/// return `Option<Ref<T>>` and setters that take `impl Into<Nullable<T>>`;
/// untyped access and bindings convert to and from `Ref<T>` (see
/// `ValueTypes::register_element_ref`).
///
/// When the referenced element is dropped the reference reads as nothing;
/// no change notification is raised for that.
pub struct ElementRef<T: ObjectType>(WeakRef<T>);

impl<T: ObjectType> ElementRef<T> {
    /// Creates a reference to `element`.
    #[inline]
    pub fn new<U: ObjectType + Upcast<T>>(element: &Ref<U>) -> Self {
        ElementRef(element.downgrade().upcast())
    }

    /// Creates a reference to an element of exactly class `T`.
    #[inline]
    pub fn of(element: &Ref<T>) -> Self {
        ElementRef(element.downgrade())
    }

    /// The property value for an optional element.
    #[inline]
    pub fn from_nullable(element: Option<Ref<T>>) -> Option<Self> {
        element.map(|element| ElementRef(element.downgrade()))
    }

    /// The referenced element, if it is still alive.
    #[inline]
    pub fn get(&self) -> Option<Ref<T>> {
        self.0.upgrade()
    }

    /// The element referenced by a property value.
    #[inline]
    pub fn resolve(value: &Option<Self>) -> Option<Ref<T>> {
        value.as_ref().and_then(ElementRef::get)
    }

    /// Whether the reference points at `element`.
    #[inline]
    pub fn points_to<U: ObjectType>(&self, element: &Ref<U>) -> bool {
        self.0.points_to(element)
    }
}

impl<T: ObjectType> Clone for ElementRef<T> {
    #[inline]
    fn clone(&self) -> Self {
        ElementRef(self.0.clone())
    }
}

/// Identity: two references are equal when they point at the same element,
/// whether or not it is still alive.
impl<T: ObjectType> PartialEq for ElementRef<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl<T: ObjectType> fmt::Debug for ElementRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get() {
            Some(_) => write!(f, "ElementRef<{}>", T::TYPE.name()),
            None => write!(f, "ElementRef<{}>(dropped)", T::TYPE.name()),
        }
    }
}

impl<T: ObjectType + Upcast<U>, U: ObjectType> From<&Ref<T>> for ElementRef<U> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        ElementRef::new(value)
    }
}
