use super::{ExpressionNode, NodeState};
use crate::data::core::{ValueType, ValueTypes};
use crate::data::BindingError;
use crate::{BoxedValue, TypeInfo};
use std::rc::Rc;

/// What a [`TypeCastNode`] casts to.
#[derive(Clone, Copy)]
pub enum CastTarget {
    /// A class of the class hierarchy.
    Class(&'static TypeInfo),
    /// A value or model type.
    Value(ValueType),
}

impl CastTarget {
    /// Whether `value` is an instance of the type (the `is` test of the
    /// managed original): an object of the class or of a class deriving
    /// from it; a value of exactly the value type, or one that is
    /// assignable to it without conversion (a contract its type implements,
    /// a base type it has a registered cast to).
    pub fn is_instance(&self, value: &BoxedValue) -> bool {
        match *self {
            CastTarget::Class(t) => ValueTypes::as_object(&**value).is_some_and(|o| t.is_assignable_from(o.get_type())),
            CastTarget::Value(t) => ValueType::of_value(&**value) == t || ValueTypes::try_cast(value, t).is_some(),
        }
    }
}

/// A node that passes its source on if it is of the given type (a cast in a
/// binding path, `((Type)Property)`).
pub struct TypeCastNode {
    state: NodeState,
    target_type: CastTarget,
}

impl TypeCastNode {
    pub fn new(target_type: CastTarget) -> Rc<Self> {
        Rc::new(Self { state: NodeState::new(), target_type })
    }
}

impl ExpressionNode for TypeCastNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        builder.push('(');
        match self.target_type {
            CastTarget::Class(t) => builder.push_str(t.name()),
            CastTarget::Value(t) => builder.push_str(t.name()),
        }
        builder.push(')');
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        // Casting null produces null. Any error belongs to the member access
        // which follows, where a null-conditional operator gets the chance to
        // short-circuit it.
        let Some(source) = source else {
            self.state.set_value(None, None);
            return;
        };
        if self.target_type.is_instance(source) {
            self.state.set_value(Some(source.clone()), None);
        } else {
            self.state.clear_value();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StaticType, StyledElement};

    trait INamed {}

    impl PartialEq for dyn INamed {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::addr_eq(self as *const Self, other as *const Self)
        }
    }

    struct Named;

    impl PartialEq for Named {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    impl INamed for Named {}

    #[test]
    fn a_value_is_an_instance_of_what_it_is_assignable_to() {
        ValueTypes::register_reference::<Named>();
        ValueTypes::register_boxed_cast::<Named, Rc<dyn INamed>>(|object| {
            let any: Rc<dyn std::any::Any> = object.clone();
            any.downcast::<Named>().ok().map(|named| named as Rc<dyn INamed>)
        });
        let named = Rc::new(Named);
        let object: BoxedValue = named.clone();
        let handle: BoxedValue = Rc::new(named.clone());
        for value in [&object, &handle] {
            assert!(CastTarget::Value(ValueType::of::<Named>()).is_instance(value));
            assert!(CastTarget::Value(ValueType::of::<Rc<Named>>()).is_instance(value));
            // The contract it has a registered cast to.
            assert!(CastTarget::Value(ValueType::of::<Rc<dyn INamed>>()).is_instance(value));
            assert!(!CastTarget::Value(ValueType::of::<i32>()).is_instance(value));
            assert!(!CastTarget::Value(ValueType::of::<String>()).is_instance(value));
            assert!(!CastTarget::Class(StyledElement::TYPE).is_instance(value));
        }
        // Conversions are not instance tests.
        let number: BoxedValue = Rc::new(1i32);
        assert!(CastTarget::Value(ValueType::of::<i32>()).is_instance(&number));
        assert!(!CastTarget::Value(ValueType::of::<f64>()).is_instance(&number));
        assert!(!CastTarget::Value(ValueType::of::<String>()).is_instance(&number));

        let element: BoxedValue = Rc::new(StyledElement::new());
        assert!(CastTarget::Class(StyledElement::TYPE).is_instance(&element));
        assert!(CastTarget::Class(crate::FerroObject::TYPE).is_instance(&element));
        assert!(!CastTarget::Value(ValueType::of::<Rc<dyn INamed>>()).is_instance(&element));
    }
}
