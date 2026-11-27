//! Object-valued Core properties retain one payload and its virtual equality operation.
use std::{any::Any, rc::Rc};

/// Equality/representation protocol for an erased object reference. Implementations
/// retain a view of the original allocation, never a replacement payload. Native
/// adapters can recover their own view without introducing a Core UI dependency.
pub trait RibbonObjectEquality: Any {
    fn equals(&self, other: &RibbonObject) -> bool;
}

#[derive(Clone)]
pub struct RibbonObject {
    value: Rc<dyn Any>,
    equals: Rc<dyn RibbonObjectEquality>,
}
struct TypedEquality<T, F> {
    target: Rc<T>,
    equals: F,
}
impl<T: Any, F: Fn(&T, &dyn Any) -> bool + 'static> RibbonObjectEquality for TypedEquality<T, F> {
    fn equals(&self, other: &RibbonObject) -> bool {
        (self.equals)(self.target.as_ref(), other.value.as_ref())
    }
}
impl RibbonObject {
    pub fn reference<T: Any>(value: Rc<T>) -> Self {
        Self::with_equality(value, |old, other| {
            other
                .downcast_ref::<T>()
                .is_some_and(|other| std::ptr::eq(old, other))
        })
    }
    pub fn value<T: Any + PartialEq>(value: T) -> Self {
        Self::with_equality(Rc::new(value), |old, new| {
            new.downcast_ref::<T>()
                .is_some_and(|new| crate::observable_object::default_equals(old, new))
        })
    }
    /// Models a user override of object.Equals. Receives the original other payload.
    pub fn with_equality<T: Any>(
        value: Rc<T>,
        equals: impl Fn(&T, &dyn Any) -> bool + 'static,
    ) -> Self {
        let operation = Rc::new(TypedEquality {
            target: value.clone(),
            equals,
        });
        Self::with_erased_equality(value, operation)
    }
    /// Retains an existing allocation and its externally supplied equality view.
    /// The caller must describe this exact payload; no clone/downcast/proxy occurs.
    pub fn with_erased_equality(value: Rc<dyn Any>, equals: Rc<dyn RibbonObjectEquality>) -> Self {
        Self { value, equals }
    }
    /// The operation is separate from the payload. This is an adapter protocol,
    /// not runtime discovery of object members or an owning global registry.
    pub fn equality(&self) -> &dyn RibbonObjectEquality {
        self.equals.as_ref()
    }
    pub fn payload(&self) -> Rc<dyn Any> {
        self.value.clone()
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.value, &other.value)
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }
}
impl PartialEq for RibbonObject {
    fn eq(&self, other: &Self) -> bool {
        // EqualityComparer<object>.Default dispatches the old value's virtual
        // Equals even when both references point at the same allocation.
        self.equals.equals(other)
    }
}
