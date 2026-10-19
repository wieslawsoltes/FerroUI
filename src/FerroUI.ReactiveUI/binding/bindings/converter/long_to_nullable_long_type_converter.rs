//! `LongToNullableLongTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/LongToNullableLongTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `i64` (`long`) to an `Option` holding it (`LongToNullableLongTypeConverter`); it
/// always succeeds for a value of the type.
#[derive(Clone, Copy, Debug, Default)]
pub struct LongToNullableLongTypeConverter;

impl IBindingTypeConverter for LongToNullableLongTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<i64>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<Option<i64>>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let value = *from?.downcast_ref::<i64>()?;
        Some(Rc::new(Some(value)))
    }
}

impl IBindingTypeConverterOf<i64, Option<i64>> for LongToNullableLongTypeConverter {
    fn try_convert(&self, from: Option<&i64>, _conversion_hint: Option<&dyn Any>) -> Option<Option<i64>> {
        Some(Some(*from?))
    }
}
