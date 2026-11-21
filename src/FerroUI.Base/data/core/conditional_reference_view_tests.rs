//! Framework-only capability probe.
use crate::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
};
use std::{any::Any, cell::Cell, rc::Rc};
struct Model(Cell<i32>);
impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
#[derive(Clone)]
enum View {
    Model(Rc<Model>),
    Opaque(Rc<Cell<i32>>),
}
impl PartialEq for View {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Model(a), Self::Model(b)) => Rc::ptr_eq(a, b),
            (Self::Opaque(a), Self::Opaque(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}
#[test]
fn conditional_reference_view_preserves_object_and_nullable_identity_and_declines_unknown() {
    ValueTypes::register_reference::<Model>();
    ValueTypes::register_reference_view_if::<View>(|view| match view {
        View::Model(model) => Some(model.clone()),
        View::Opaque(_) => None,
    });
    let model = Rc::new(Model(Cell::new(3)));
    let weak = Rc::downgrade(&model);
    let view = Rc::new(View::Model(model.clone())) as BoxedValue;
    let object = ValueTypes::reference_object(&view).unwrap();
    let actual: Rc<dyn Any> = object.clone();
    let expected: Rc<dyn Any> = model.clone();
    assert!(Rc::ptr_eq(&actual, &expected));
    object.downcast_ref::<Model>().unwrap().0.set(7);
    assert_eq!(model.0.get(), 7);
    let nullable = Rc::new(Some(View::Model(model.clone()))) as BoxedValue;
    assert!(Rc::ptr_eq(
        &ValueTypes::reference_object(&nullable).unwrap(),
        &object
    ));
    let opaque = Rc::new(View::Opaque(Rc::new(Cell::new(9)))) as BoxedValue;
    assert!(ValueTypes::reference_object(&opaque).is_none());
    let converted = ValueTypes::try_cast(&opaque, ValueType::of::<Option<BoxedValue>>()).unwrap();
    assert!(Rc::ptr_eq(
        converted
            .downcast_ref::<Option<BoxedValue>>()
            .unwrap()
            .as_ref()
            .unwrap(),
        &opaque
    ));
    drop(model);
    drop(view);
    drop(nullable);
    drop(actual);
    drop(expected);
    assert!(weak.upgrade().is_some());
    drop(object);
    assert!(weak.upgrade().is_none());
}
#[test]
fn conditional_some_unregistered_still_fails_explicitly() {
    #[derive(Clone, PartialEq)]
    struct Bad(Rc<Cell<i32>>);
    ValueTypes::register_reference_view_if::<Bad>(|v| Some(v.0.clone()));
    let value = Rc::new(Bad(Rc::new(Cell::new(1)))) as BoxedValue;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ValueTypes::reference_object(&value)
        }))
        .is_err()
    );
}

#[test]
fn conditional_callback_can_register_its_concrete_model_reentrantly() {
    #[derive(Clone, PartialEq)]
    struct Reentrant(Rc<Cell<i64>>);
    ValueTypes::register_reference_view_if::<Reentrant>(|v| {
        ValueTypes::register_reference::<Cell<i64>>();
        Some(v.0.clone())
    });
    let model = Rc::new(Cell::new(4i64));
    let view = Rc::new(Reentrant(model.clone())) as BoxedValue;
    let object = ValueTypes::reference_object(&view).unwrap();
    let erased: Rc<dyn Any> = object;
    let expected: Rc<dyn Any> = model;
    assert!(Rc::ptr_eq(&erased, &expected));
}
