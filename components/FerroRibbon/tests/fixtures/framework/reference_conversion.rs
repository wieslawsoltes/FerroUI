//! Framework-only conversion/assignability mismatch; no FerroRibbon imports.
#![cfg(test)]
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
};
use std::{any::Any, cell::Cell, rc::Rc};
trait BaseView: Any {
    fn number(&self) -> i32;
}
impl PartialEq for dyn BaseView {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const _, other as *const _)
    }
}
struct Model(Cell<i32>);
impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl BaseView for Model {
    fn number(&self) -> i32 {
        self.0.get()
    }
}
fn register() {
    ValueTypes::register_reference::<Model>();
    ValueTypes::register_nullable::<Rc<dyn BaseView>>();
    ValueTypes::register_cast::<Rc<Model>, Rc<dyn BaseView>>(|v| v.clone());
}
fn convert(value: &BoxedValue, target: ValueType) -> BoxedValue {
    ValueTypes::try_convert(Some(value), target)
        .flatten()
        .expect("TryConvert must accept an existing assignability cast")
}
#[test]
fn explicitly_registered_handle_cast_is_a_conversion_control() {
    register();
    let model = Rc::new(Model(Cell::new(1)));
    let value: BoxedValue = Rc::new(model.clone());
    let actual = convert(&value, ValueType::of::<Rc<dyn BaseView>>());
    let view = actual.downcast_ref::<Rc<dyn BaseView>>().unwrap();
    let object: Rc<dyn Any> = view.clone();
    assert!(Rc::ptr_eq(&(model as Rc<dyn Any>), &object));
    assert_eq!(view.number(), 1);
}
#[test]
fn raw_reference_object_existing_assignability_cast_is_also_a_conversion() {
    register();
    let model = Rc::new(Model(Cell::new(2)));
    let value: BoxedValue = model.clone();
    let target = ValueType::of::<Rc<dyn BaseView>>();
    assert!(ValueTypes::is_assignable(ValueType::of::<Model>(), target));
    assert!(
        ValueTypes::try_cast(&value, target).is_some(),
        "control proves the cast is registered"
    );
    let actual = convert(&value, target);
    let view = actual.downcast_ref::<Rc<dyn BaseView>>().unwrap();
    model.0.set(7);
    assert_eq!(view.number(), 7);
    assert!(std::ptr::addr_eq(Rc::as_ptr(&model), Rc::as_ptr(view)));
}
#[test]
fn raw_reference_object_nullable_base_assignability_is_also_a_conversion() {
    register();
    let model = Rc::new(Model(Cell::new(3)));
    let value: BoxedValue = model.clone();
    let target = ValueType::of::<Option<Rc<dyn BaseView>>>();
    assert!(ValueTypes::is_assignable(ValueType::of::<Model>(), target));
    assert!(ValueTypes::try_cast(&value, target).is_some());
    let actual = convert(&value, target);
    let view = actual
        .downcast_ref::<Option<Rc<dyn BaseView>>>()
        .unwrap()
        .as_ref()
        .unwrap();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&model), Rc::as_ptr(view)));
}
#[test]
fn null_values_and_unrelated_types_do_not_gain_assignability() {
    register();
    let target = ValueType::of::<Option<Rc<dyn BaseView>>>();
    let null = ValueTypes::try_convert(None, target).flatten().unwrap();
    assert!(
        null.downcast_ref::<Option<Rc<dyn BaseView>>>()
            .unwrap()
            .is_none()
    );
    let number: BoxedValue = Rc::new(4_i32);
    assert!(ValueTypes::try_cast(&number, target).is_none());
    assert!(ValueTypes::try_convert(Some(&number), target).is_none());
}
