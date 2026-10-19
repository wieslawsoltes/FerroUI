//! `ShortToNullableShortTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/ShortToNullableShortTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `i16` (`short`) to an `Option` holding it (`ShortToNullableShortTypeConverter`); it
/// always succeeds for a value of the type.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShortToNullableShortTypeConverter;

impl IBindingTypeConverter for ShortToNullableShortTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<i16>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<Option<i16>>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let value = *from?.downcast_ref::<i16>()?;
        Some(Rc::new(Some(value)))
    }
}

impl IBindingTypeConverterOf<i16, Option<i16>> for ShortToNullableShortTypeConverter {
    fn try_convert(&self, from: Option<&i16>, _conversion_hint: Option<&dyn Any>) -> Option<Option<i16>> {
        Some(Some(*from?))
    }
}
