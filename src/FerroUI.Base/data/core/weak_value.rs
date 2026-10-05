use super::value_type::value_type_id;
use super::ValueTypes;
use crate::{AnyValue, BoxedValue, FerroObject, WeakRef};
use std::any::TypeId;
use std::rc::{Rc, Weak};

/// A reference to a binding source that does not keep it alive.
///
/// Objects of the class hierarchy and registered reference (model) types are
/// held weakly, as the managed implementation holds every source. Plain
/// values (numbers, strings, value structs) own nothing that could form a
/// cycle and exist only as the transient box that carries them, so they are
/// held by value.
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
        if let Some(object) = ValueTypes::as_object(&**value) {
            WeakValue::Object(object.downgrade(), id)
        } else if ValueTypes::is_reference(id) {
            WeakValue::Reference(Rc::downgrade(value))
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
