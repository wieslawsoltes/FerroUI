//! `NullableByteToByteTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableByteToByteTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `Option` of `u8` (`byte`) to the value it holds (`NullableByteToByteTypeConverter`);
/// `None` fails.
///
/// The type-erased conversion takes an `Option<u8>`, or a plain `u8` as upstream takes the boxed
/// value of a nullable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableByteToByteTypeConverter;

impl IBindingTypeConverter for NullableByteToByteTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<Option<u8>>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<u8>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let from = from?;
        let value = match from.downcast_ref::<Option<u8>>() {
            Some(value) => self.try_convert(Some(value), conversion_hint)?,
            None => *from.downcast_ref::<u8>()?,
        };
        Some(Rc::new(value))
    }
}

impl IBindingTypeConverterOf<Option<u8>, u8> for NullableByteToByteTypeConverter {
    fn try_convert(&self, from: Option<&Option<u8>>, _conversion_hint: Option<&dyn Any>) -> Option<u8> {
        from.copied().flatten()
    }
}
