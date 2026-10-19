//! `NullableLongToLongTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableLongToLongTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `Option` of `i64` (`long`) to the value it holds
/// (`NullableLongToLongTypeConverter`); `None` fails.
///
/// The type-erased conversion takes an `Option<i64>`, or a plain `i64` as upstream takes the boxed
/// value of a nullable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableLongToLongTypeConverter;

impl IBindingTypeConverter for NullableLongToLongTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<Option<i64>>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<i64>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let from = from?;
        let value = match from.downcast_ref::<Option<i64>>() {
            Some(value) => self.try_convert(Some(value), conversion_hint)?,
            None => *from.downcast_ref::<i64>()?,
        };
        Some(Rc::new(value))
    }
}

impl IBindingTypeConverterOf<Option<i64>, i64> for NullableLongToLongTypeConverter {
    fn try_convert(&self, from: Option<&Option<i64>>, _conversion_hint: Option<&dyn Any>) -> Option<i64> {
        from.copied().flatten()
    }
}
