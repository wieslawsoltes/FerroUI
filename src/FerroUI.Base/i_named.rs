//! Port of `INamed.cs`.

use crate::{ObjectType, Ref, StyledElement, Upcast};
use std::rc::Rc;

/// Objects implementing this contract and providing a value for
/// [`name`](INamed::name) are registered in the relevant name scope when
/// constructed in markup.
pub trait INamed {
    /// Gets the element name.
    fn name(&self) -> Option<String>;

    /// The identity of the object, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same object, whichever adapter they were made
/// from.
impl PartialEq for dyn INamed {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

/// Implements the contract for the styled element behind a class handle.
struct StyledElementNamed(Ref<StyledElement>);

impl INamed for StyledElementNamed {
    fn name(&self) -> Option<String> {
        self.0.name()
    }

    fn reference_id(&self) -> *const () {
        &*self.0 as *const StyledElement as *const ()
    }
}

impl<T: ObjectType + Upcast<StyledElement>> From<Ref<T>> for Rc<dyn INamed> {
    fn from(value: Ref<T>) -> Self {
        Rc::new(StyledElementNamed(value.upcast()))
    }
}

impl<T: ObjectType + Upcast<StyledElement>> From<&Ref<T>> for Rc<dyn INamed> {
    fn from(value: &Ref<T>) -> Self {
        Rc::new(StyledElementNamed(value.clone().upcast()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::core::{ValueType, ValueTypes};
    use crate::BoxedValue;

    #[test]
    fn styled_elements_are_named() {
        let element = StyledElement::new();
        element.set_name(Some("root".to_string()));
        let named: Rc<dyn INamed> = (&element).into();
        assert_eq!(named.name().as_deref(), Some("root"));

        let again: Rc<dyn INamed> = element.clone().into();
        let other: Rc<dyn INamed> = StyledElement::new().into();
        assert!(*named == *again);
        assert!(*named != *other);
    }

    #[test]
    fn the_handle_of_a_styled_element_casts_to_the_contract() {
        let element = StyledElement::new();
        assert!(StyledElement::TYPE.interfaces().iter().any(|i| i() == ValueType::of::<Rc<dyn INamed>>()));
        let boxed: BoxedValue = Rc::new(element.clone());
        let cast = ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn INamed>>()).expect("the cast is declared");
        assert!(cast.downcast_ref::<Rc<dyn INamed>>().is_some());
    }
}
