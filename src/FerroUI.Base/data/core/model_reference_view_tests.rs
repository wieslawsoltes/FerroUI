use super::{ValueType, ValueTypes};
use crate::{BoxedValue, data::core::WeakValue};
use std::{any::Any, cell::Cell, rc::Rc};

trait View: Any {
    fn number(&self) -> i32;
}
impl PartialEq for dyn View {
    fn eq(&self, other: &Self) -> bool {
        self.number() == other.number()
    }
}
struct Model(Cell<i32>);
impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.0.get() == other.0.get()
    }
}
impl View for Model {
    fn number(&self) -> i32 {
        self.0.get()
    }
}
fn register() {
    ValueTypes::register_reference::<Model>();
    ValueTypes::register_reference_view::<Rc<dyn View>>(|v| v.clone());
}
fn same(model: &Rc<Model>, value: BoxedValue) {
    let actual: Rc<dyn Any> = value;
    let expected: Rc<dyn Any> = model.clone();
    assert!(Rc::ptr_eq(&actual, &expected));
}
#[test]
fn model_views_and_nullable_forms_share_the_actual_nonclone_allocation() {
    register();
    let model = Rc::new(Model(Cell::new(3)));
    let view: Rc<dyn View> = model.clone();
    let forms: Vec<BoxedValue> = vec![
        Rc::new(model.clone()),
        Rc::new(Some(model.clone())),
        Rc::new(view.clone()),
        Rc::new(Some(view)),
    ];
    for form in forms {
        same(&model, ValueTypes::reference_object(&form).unwrap());
        let boxed = ValueTypes::try_cast(&form, ValueType::of::<Option<BoxedValue>>()).unwrap();
        same(
            &model,
            boxed
                .downcast_ref::<Option<BoxedValue>>()
                .unwrap()
                .clone()
                .unwrap(),
        );
    }
    let raw: BoxedValue = model.clone();
    assert!(ValueTypes::reference_object(&raw).is_none());
    model.0.set(8);
    let view: Rc<dyn View> = model;
    assert_eq!(view.number(), 8);
}
#[test]
fn weak_binding_handles_release_only_after_the_last_actual_owner_and_compare_identity() {
    register();
    let first = Rc::new(Model(Cell::new(1)));
    let second = Rc::new(Model(Cell::new(1)));
    let direct = WeakValue::new(&(first.clone() as BoxedValue));
    let concrete = WeakValue::new(&(Rc::new(first.clone()) as BoxedValue));
    let view = WeakValue::new(&(Rc::new(first.clone() as Rc<dyn View>) as BoxedValue));
    let nullable = WeakValue::new(&(Rc::new(Some(first.clone() as Rc<dyn View>)) as BoxedValue));
    let other = WeakValue::new(&(second.clone() as BoxedValue));
    assert!(direct == concrete && concrete == view && view == nullable);
    assert!(
        view != other,
        "model value equality must not replace binding source identity"
    );
    let weak = Rc::downgrade(&first);
    drop(first);
    assert!(weak.upgrade().is_none());
    for value in [direct, concrete, view, nullable] {
        assert!(value.upgrade().is_none());
    }
    assert!(other.upgrade().is_some());
    drop(second);
    assert!(other.upgrade().is_none());
}
#[test]
fn model_view_null_and_scalar_values_keep_their_original_kind() {
    register();
    for value in [
        Rc::new(None::<Rc<Model>>) as BoxedValue,
        Rc::new(None::<Rc<dyn View>>) as BoxedValue,
    ] {
        assert!(ValueTypes::reference_object(&value).is_none());
        let boxed = ValueTypes::try_cast(&value, ValueType::of::<Option<BoxedValue>>()).unwrap();
        assert!(
            boxed
                .downcast_ref::<Option<BoxedValue>>()
                .unwrap()
                .is_none()
        );
    }
    let scalar: BoxedValue = Rc::new(12i32);
    assert!(ValueTypes::reference_object(&scalar).is_none());
    assert!(matches!(WeakValue::new(&scalar), WeakValue::Value(_)));
}
#[test]
fn declared_model_view_with_unregistered_concrete_type_fails_instead_of_retaining_it() {
    struct Unknown;
    impl View for Unknown {
        fn number(&self) -> i32 {
            0
        }
    }
    register();
    let view: Rc<dyn View> = Rc::new(Unknown);
    let value: BoxedValue = Rc::new(view);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ValueTypes::reference_object(&value)
        }))
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| WeakValue::new(&value))).is_err()
    );
}
#[test]
fn reference_view_callback_can_register_types_without_a_registry_borrow() {
    ValueTypes::register_reference_view::<Rc<dyn View>>(|v| {
        ValueTypes::register_reference::<Model>();
        v.clone()
    });
    let model = Rc::new(Model(Cell::new(9)));
    let value: BoxedValue = Rc::new(model.clone() as Rc<dyn View>);
    same(&model, ValueTypes::reference_object(&value).unwrap());
}
