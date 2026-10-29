//! The shared object boundary retains actual values and reference identities.
use crate::*;
use ferroui_base::BoxedValue;
use std::{any::Any, rc::Rc};
#[test]
fn native_values_round_trip_without_reboxing_or_losing_their_equality_metadata() {
    let value: BoxedValue = Rc::new("native context".to_owned());
    let source = to_source_value(value.clone());
    let native = from_source_value(&source);
    assert!(Rc::ptr_eq(&value, &native));
    assert_eq!(native.downcast_ref::<String>().unwrap(), "native context");
}
#[test]
fn registered_source_types_keep_the_exact_concrete_reference_in_both_directions() {
    #[derive(PartialEq)]
    struct Context(i32);
    register_value_type::<Context>();
    let source: Rc<dyn Any> = Rc::new(Context(7));
    let native = from_source_value(&source);
    assert_eq!(native.downcast_ref::<Context>().unwrap().0, 7);
    assert!(Rc::ptr_eq(&to_source_value(native), &source));
}
#[test]
fn unknown_source_values_and_raw_scalars_retain_reference_identity() {
    struct Context;
    let source: Rc<dyn Any> = Rc::new(Context);
    let native = from_source_value(&source);
    let again = from_source_value(&source);
    assert!(native == again);
    assert!(Rc::ptr_eq(&to_source_value(native), &source));
    let scalar = Rc::new(42_i32);
    let erased: Rc<dyn Any> = scalar.clone();
    let native = from_source_value(&erased);
    assert!(Rc::ptr_eq(&native, &(scalar as BoxedValue)));
}
