//! `NullableShortToShortTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableShortToShortTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `Option` of `i16` (`short`) to the value it holds
/// (`NullableShortToShortTypeConverter`); `None` fails.
///
/// The type-erased conversion takes an `Option<i16>`, or a plain `i16` as upstream takes the boxed
/// value of a nullable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableShortToShortTypeConverter;

impl IBindingTypeConverter for NullableShortToShortTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<Option<i16>>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<i16>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let from = from?;
        let value = match from.downcast_ref::<Option<i16>>() {
            Some(value) => self.try_convert(Some(value), conversion_hint)?,
            None => *from.downcast_ref::<i16>()?,
        };
        Some(Rc::new(value))
    }
}

impl IBindingTypeConverterOf<Option<i16>, i16> for NullableShortToShortTypeConverter {
    fn try_convert(&self, from: Option<&Option<i16>>, _conversion_hint: Option<&dyn Any>) -> Option<i16> {
        from.copied().flatten()
    }
}
