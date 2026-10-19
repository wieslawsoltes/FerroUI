//! `NullableIntegerToIntegerTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableIntegerToIntegerTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `Option` of `i32` (`int`) to the value it holds
/// (`NullableIntegerToIntegerTypeConverter`); `None` fails.
///
/// The type-erased conversion takes an `Option<i32>`, or a plain `i32` as upstream takes the boxed
/// value of a nullable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableIntegerToIntegerTypeConverter;

impl IBindingTypeConverter for NullableIntegerToIntegerTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<Option<i32>>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<i32>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let from = from?;
        let value = match from.downcast_ref::<Option<i32>>() {
            Some(value) => self.try_convert(Some(value), conversion_hint)?,
            None => *from.downcast_ref::<i32>()?,
        };
        Some(Rc::new(value))
    }
}

impl IBindingTypeConverterOf<Option<i32>, i32> for NullableIntegerToIntegerTypeConverter {
    fn try_convert(&self, from: Option<&Option<i32>>, _conversion_hint: Option<&dyn Any>) -> Option<i32> {
        from.copied().flatten()
    }
}
