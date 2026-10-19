//! `ByteToNullableByteTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/ByteToNullableByteTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts a `u8` (`byte`) to an `Option` holding it (`ByteToNullableByteTypeConverter`); it
/// always succeeds for a value of the type.
#[derive(Clone, Copy, Debug, Default)]
pub struct ByteToNullableByteTypeConverter;

impl IBindingTypeConverter for ByteToNullableByteTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<u8>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<Option<u8>>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let value = *from?.downcast_ref::<u8>()?;
        Some(Rc::new(Some(value)))
    }
}

impl IBindingTypeConverterOf<u8, Option<u8>> for ByteToNullableByteTypeConverter {
    fn try_convert(&self, from: Option<&u8>, _conversion_hint: Option<&dyn Any>) -> Option<Option<u8>> {
        Some(Some(*from?))
    }
}
