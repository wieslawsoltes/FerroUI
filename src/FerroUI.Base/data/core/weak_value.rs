use super::value_type::{is_primitive_value, value_type_id};
use super::ValueTypes;
use crate::{AnyValue, BoxedValue, FerroObject, WeakRef};
use std::any::TypeId;
use std::rc::{Rc, Weak};

/// A reference to a binding source that does not keep it alive.
///
/// Objects of the class hierarchy and registered reference (model) types are
/// held weakly, as the managed implementation holds every source. So are the
/// handles that view a model: a declared reference view, and a non-null nullable
/// handle of a model (`Option<Rc<Model>>`), which is held as weakly as `Rc<Model>`
/// itself. Plain values (numbers, strings, value structs) own nothing that could
/// form a cycle and exist only as the transient box that carries them, so they are
/// held by value; a primitive is known to be one without a lookup.
#[derive(Clone)]
pub enum WeakValue {
    /// A shared model object.
    Reference(Weak<dyn AnyValue>),
    /// An object of the class hierarchy and the handle type it was seen as.
    Object(WeakRef<FerroObject>, TypeId),
    /// A plain value.
    Value(BoxedValue),
}

/// Two references are equal when they refer to the same source: the same
/// object for the weakly held ones, equal values for plain values.
impl PartialEq for WeakValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (WeakValue::Reference(a), WeakValue::Reference(b)) => std::ptr::addr_eq(a.as_ptr(), b.as_ptr()),
            (WeakValue::Object(a, a_id), WeakValue::Object(b, b_id)) => a.ptr_eq(b) && a_id == b_id,
            (WeakValue::Value(a), WeakValue::Value(b)) => **a == **b,
            _ => false,
        }
    }
}

impl WeakValue {
    pub fn new(value: &BoxedValue) -> Self {
        let id = value_type_id(&**value);
        if is_primitive_value(id) {
            WeakValue::Value(value.clone())
        } else if let Some(object) = ValueTypes::as_object(&**value) {
            WeakValue::Object(object.downgrade(), id)
        } else if ValueTypes::is_reference(id) {
            WeakValue::Reference(Rc::downgrade(value))
        } else if let Some(object) = ValueTypes::reference_object(value) {
            WeakValue::Reference(Rc::downgrade(&object))
        } else {
            WeakValue::Value(value.clone())
        }
    }

    /// The source, if it is still alive.
    pub fn upgrade(&self) -> Option<BoxedValue> {
        match self {
            WeakValue::Reference(weak) => weak.upgrade(),
            WeakValue::Object(weak, id) => weak.upgrade().and_then(|o| ValueTypes::from_object(*id, o)),
            WeakValue::Value(value) => Some(value.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Model(i32);
    impl PartialEq for Model {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    #[test]
    fn primitives_and_plain_nullables_are_held_by_value() {
        for value in [Rc::new(3_i32) as BoxedValue, Rc::new(String::from("text")), Rc::new(true), Rc::new(Some(2.5_f64))] {
            assert!(matches!(WeakValue::new(&value), WeakValue::Value(_)));
        }
    }

    /// A non-null nullable handle of a registered model is held as weakly as the model handle: the
    /// binding does not keep the model alive.
    #[test]
    fn a_nullable_handle_of_a_model_is_held_weakly() {
        ValueTypes::register_reference::<Model>();
        let model = Rc::new(Model(7));
        let handle: BoxedValue = Rc::new(Some(model.clone()));
        let weak = WeakValue::new(&handle);
        assert!(matches!(weak, WeakValue::Reference(_)));
        let alive = weak.upgrade().expect("the model is alive");
        assert_eq!(alive.downcast_ref::<Model>().map(|model| model.0), Some(7));
        drop(alive);
        drop(handle);
        drop(model);
        assert!(weak.upgrade().is_none());
        let none: BoxedValue = Rc::new(Option::<Rc<Model>>::None);
        assert!(matches!(WeakValue::new(&none), WeakValue::Value(_)));
    }
}
