//! The boxing, conversion and text of the values of cells: what the reference's cells do with
//! `object`, `Convert.ChangeType` and `ToString()`.
//!
//! Not a file of the reference: in C# a value of any type is an `object`; here a typed value is
//! boxed into the framework's untyped value, with a nullable value (`Option<T>`) unwrapped as the
//! framework's bindings unwrap it.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use std::any::Any;
use std::rc::Rc;

/// The untyped form of a cell value (`(object?)value`): `None` for a null value.
pub fn box_cell_value<T: Clone + PartialEq + 'static>(value: &T) -> Option<BoxedValue> {
    let any: &dyn Any = value;
    // A value that is already untyped is not boxed again.
    if let Some(boxed) = any.downcast_ref::<BoxedValue>() {
        return Some(boxed.clone());
    }
    if let Some(boxed) = any.downcast_ref::<Option<BoxedValue>>() {
        return boxed.clone();
    }
    ValueTypes::normalize(Rc::new(value.clone()) as BoxedValue)
}

/// The typed form of an untyped value (`(T)Convert.ChangeType(value, typeof(T))`); `None` when
/// the value cannot be converted.
pub fn unbox_cell_value<T: Clone + 'static>(value: Option<&BoxedValue>) -> Option<T> {
    if let Some(value) = value {
        if let Some(typed) = value.downcast_ref::<T>() {
            return Some(typed.clone());
        }
    }
    // An untyped target takes the value as it is.
    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<BoxedValue>() {
        let boxed: Box<dyn Any> = Box::new(value?.clone());
        return boxed.downcast::<T>().ok().map(|b| *b);
    }
    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<Option<BoxedValue>>() {
        let boxed: Box<dyn Any> = Box::new(value.cloned());
        return boxed.downcast::<T>().ok().map(|b| *b);
    }
    match ValueTypes::try_convert(value, ValueType::of::<T>()) {
        Some(Some(converted)) => converted.downcast_ref::<T>().cloned(),
        _ => None,
    }
}

/// The value of a cell converted from text (`Convert.ChangeType(text, typeof(T))`); `None` when
/// the text is not a value of `T`.
pub fn cell_value_from_text<T: Clone + 'static>(text: Option<String>) -> Option<T> {
    let boxed = text.map(|text| Rc::new(text) as BoxedValue);
    unbox_cell_value::<T>(boxed.as_ref())
}

/// The text of an untyped value (`value?.ToString()`).
pub fn cell_value_text(value: Option<&BoxedValue>) -> Option<String> {
    value.map(|value| ValueTypes::to_display_string(Some(value)))
}
