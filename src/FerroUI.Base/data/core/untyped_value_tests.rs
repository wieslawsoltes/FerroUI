use super::{ValueType, ValueTypes};
use crate::{metadata::into_markup_value, BoxedValue};
use std::rc::Rc;

#[derive(PartialEq)]
struct Token(u8); // deliberately not Clone: normalisation must retain the allocation

fn boxed(value: BoxedValue) -> BoxedValue {
    Rc::new(value)
}
fn some(value: BoxedValue) -> BoxedValue {
    Rc::new(Some(value))
}

#[test]
fn an_untyped_handle_in_a_typed_box_normalizes_to_the_original_allocation() {
    let original: BoxedValue = Rc::new(Token(7));
    for input in [
        boxed(original.clone()),
        some(boxed(original.clone())),
        boxed(some(original.clone())),
        boxed(boxed(some(boxed(original.clone())))),
    ] {
        let actual = ValueTypes::normalize(input).expect("non-null object");
        assert!(Rc::ptr_eq(&actual, &original));
        assert_eq!(actual.downcast_ref::<Token>().unwrap().0, 7);
    }
}

#[test]
fn boxed_null_untyped_handles_stay_null_without_a_payload_proxy() {
    for input in [
        boxed(Rc::new(Option::<BoxedValue>::None)),
        some(boxed(Rc::new(Option::<BoxedValue>::None))),
    ] {
        assert!(ValueTypes::normalize(input.clone()).is_none());
        // An exact typed cast is intentionally unchanged. Typed compiled code
        // adds a BoxedValue layer before requesting the canonical object form.
        if input.is::<Option<BoxedValue>>() {
            assert!(Rc::ptr_eq(
                &ValueTypes::try_cast(&input, ValueType::object()).unwrap(),
                &input
            ));
        }
        let value = ValueTypes::try_cast(&boxed(input), ValueType::object()).unwrap();
        assert!(value.downcast_ref::<Option<BoxedValue>>().unwrap().is_none());
    }
}

#[test]
fn object_assignment_of_an_untyped_static_property_keeps_raw_identity_and_lifetime() {
    let original: BoxedValue = Rc::new(Token(9));
    let weak = Rc::downgrade(&original);
    // This is the typed x:Static result passed through the emitted to_value path.
    let typed_result: BoxedValue = boxed(original.clone());
    let object = ValueTypes::try_cast(&typed_result, ValueType::object()).unwrap();
    let actual = object.downcast_ref::<Option<BoxedValue>>().unwrap().clone().unwrap();
    assert!(Rc::ptr_eq(&actual, &original));
    let direct = into_markup_value(original.clone()).unwrap();
    assert!(Rc::ptr_eq(&direct, &actual));
    drop(original);
    drop(typed_result);
    drop(object);
    drop(direct);
    assert!(weak.upgrade().is_some());
    drop(actual);
    assert!(weak.upgrade().is_none());
}

#[test]
fn untyped_boxes_preserve_existing_nullable_scalar_and_unregistered_value_behavior() {
    ValueTypes::register_nullable::<i32>();
    let scalar: BoxedValue = Rc::new(Some(31_i32));
    let result = ValueTypes::normalize(boxed(scalar)).unwrap();
    assert_eq!(result.downcast_ref::<i32>(), Some(&31));
    let raw: BoxedValue = Rc::new(Token(3));
    assert!(Rc::ptr_eq(&ValueTypes::normalize(raw.clone()).unwrap(), &raw));
}
