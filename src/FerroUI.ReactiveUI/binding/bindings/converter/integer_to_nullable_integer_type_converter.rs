//! `IntegerToNullableIntegerTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/IntegerToNullableIntegerTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `i32` (`int`) to an `Option` holding it (`IntegerToNullableIntegerTypeConverter`);
/// it always succeeds for a value of the type.
#[derive(Clone, Copy, Debug, Default)]
pub struct IntegerToNullableIntegerTypeConverter;

impl IBindingTypeConverter for IntegerToNullableIntegerTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<i32>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<Option<i32>>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let value = *from?.downcast_ref::<i32>()?;
        Some(Rc::new(Some(value)))
    }
}

impl IBindingTypeConverterOf<i32, Option<i32>> for IntegerToNullableIntegerTypeConverter {
    fn try_convert(&self, from: Option<&i32>, _conversion_hint: Option<&dyn Any>) -> Option<Option<i32>> {
        Some(Some(*from?))
    }
}
