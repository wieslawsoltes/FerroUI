//! Convert application data at the native view/Core model boundary.
//!
//! Use this for values that must travel through native bindings, command
//! parameters, galleries or backstage content. The existing native allocation
//! and its equality implementation travel together; no application value is
//! cloned and no per-object registry is installed. Core remains independent of
//! the control layer.
//! A Core-only `with_equality` callback remains field-local when exporting its
//! bare payload. Use an intrinsic native `AnyValue` comparison for data that
//! crosses this boundary; no registry transports per-reference policies.
//!
//! ```
//! use std::rc::Rc;
//! use ferroribbon::mvvm::model_values;
//! use ferroribbon_core::model::{RibbonButtonModel, RibbonNodeModelExt};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #[derive(PartialEq)]
//! struct Document { id: u64 } // No framework dependency or Clone implementation.
//! let document = Rc::new(Document { id: 7 });
//! let model = RibbonButtonModel::new(None, None);
//! model.set_tag(model_values::from_native(Some(document.clone())));
//! let native = model_values::to_native(model.tag())?.unwrap();
//! assert!(std::ptr::eq(native.downcast_ref::<Document>().unwrap(), document.as_ref()));
//! # Ok(())
//! # }
//! ```
use ferroribbon_core::model::{
    RibbonBackstageModelImpl, RibbonModelImpl, RibbonNodeModelImpl, RibbonObject,
    RibbonObjectEquality, is_registered_model_type, try_cast_model,
};
use ferroui_base::{BoxedValue, data::core::ValueTypes};
use std::{any::Any, rc::Rc};

struct NativeObjectEquality {
    value: BoxedValue,
}
fn core_view<T: ?Sized + ferroribbon_core::model::RibbonModelView>(
    value: &BoxedValue,
) -> Option<Rc<T>> {
    try_cast_model::<T>(Some(value.clone()))
        .expect("known concrete Core model and declared protocol")
}
impl RibbonObjectEquality for NativeObjectEquality {
    fn equals(&self, other: &RibbonObject) -> bool {
        let other = to_native(Some(other.clone()))
            .unwrap_or_else(|error| {
                panic!("Object equality cannot expose the other Core payload: {error}")
            })
            .expect("non-null RibbonObject payload");
        // A native class handle can have different static types for one object.
        if let (Some(left), Some(right)) = (
            ValueTypes::as_object(&*self.value),
            ValueTypes::as_object(&*other),
        ) {
            return left.ptr_eq(&right);
        }
        // Plain Core models preserve their inherited virtual equality across
        // different concrete classes, not only through exact AnyValue types.
        if is_registered_model_type(self.value.as_ref().value_type_id())
            && is_registered_model_type(other.as_ref().value_type_id())
        {
            if let (Some(left), Some(right)) = (
                core_view::<dyn RibbonNodeModelImpl>(&self.value),
                core_view::<dyn RibbonNodeModelImpl>(&other),
            ) {
                return left.equals_node(right.as_ref());
            }
            if let (Some(left), Some(right)) = (
                core_view::<dyn RibbonModelImpl>(&self.value),
                core_view::<dyn RibbonModelImpl>(&other),
            ) {
                return left.equals_model(right.as_ref());
            }
            if let (Some(left), Some(right)) = (
                core_view::<dyn RibbonBackstageModelImpl>(&self.value),
                core_view::<dyn RibbonBackstageModelImpl>(&other),
            ) {
                return left.equals_backstage(right.as_ref());
            }
        }
        // Do not use identity_equals: its pointer shortcut would skip a
        // non-reflexive virtual Equals override on the SAME boxed allocation.
        self.value.as_ref().value_eq(other.as_ref())
    }
}
fn normalize(value: Option<BoxedValue>) -> Option<BoxedValue> {
    value
        .and_then(ValueTypes::normalize)
        .map(|value| ValueTypes::reference_object(&value).unwrap_or(value))
}
fn core(value: BoxedValue) -> RibbonObject {
    let payload: Rc<dyn Any> = value.clone();
    RibbonObject::with_erased_equality(payload, Rc::new(NativeObjectEquality { value }))
}
/// Convert an object argument without requiring its application type to be
/// registered or cloneable. Both references point at the same allocation.
pub fn from_native(value: Option<BoxedValue>) -> Option<RibbonObject> {
    normalize(value).map(core)
}
/// Preserve an existing Core operation on a round-trip of the same payload.
/// The actual setter still executes its comparison, even for non-reflexive Equals.
pub(crate) fn from_native_replacing(
    value: Option<BoxedValue>,
    previous: impl FnOnce() -> Option<RibbonObject>,
) -> Option<RibbonObject> {
    let value = normalize(value)?;
    if let Some(previous) = previous() {
        let native: Rc<dyn Any> = value.clone();
        if Rc::ptr_eq(&previous.payload(), &native) {
            return Some(previous);
        }
    }
    Some(core(value))
}

fn box_model_value(value: &RibbonObject) -> Result<BoxedValue, Rc<dyn Any>> {
    if let Some(native) = (value.equality() as &dyn Any).downcast_ref::<NativeObjectEquality>() {
        return Ok(native.value.clone());
    }
    ValueTypes::box_erased(value.payload())
}

/// Keeps a Core-only argument intact for the command adapter.
pub(super) fn box_command_parameter(value: RibbonObject) -> BoxedValue {
    box_model_value(&value).unwrap_or_else(|_| Rc::new(value))
}

/// Recover the existing native view first. Core-created payloads can opt into
/// the framework's public erased boxer; unknown types return an explicit error.
pub fn to_native(value: Option<RibbonObject>) -> Result<Option<BoxedValue>, RibbonModelValueError> {
    value
        .map(|value| {
            box_model_value(&value).map_err(|payload| {
                let type_id = payload.as_ref().type_id();
                RibbonModelValueError {
                    message: format!("Unregistered native model payload {type_id:?}; register its concrete AnyValue type with ValueTypes::register_erased_value::<T>()"),
                }
            })
        })
        .transpose()
}

/// The selected native framework cannot expose this Core-only payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonModelValueError {
    message: String,
}
impl std::fmt::Display for RibbonModelValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RibbonModelValueError {}
