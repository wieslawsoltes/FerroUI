//! `NullableSingleToSingleTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableSingleToSingleTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `Option` of `f32` (`float`) to the value it holds
/// (`NullableSingleToSingleTypeConverter`); `None` fails.
///
/// The type-erased conversion takes an `Option<f32>`, or a plain `f32` as upstream takes the boxed
/// value of a nullable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableSingleToSingleTypeConverter;

impl IBindingTypeConverter for NullableSingleToSingleTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<Option<f32>>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<f32>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let from = from?;
        let value = match from.downcast_ref::<Option<f32>>() {
            Some(value) => self.try_convert(Some(value), conversion_hint)?,
            None => *from.downcast_ref::<f32>()?,
        };
        Some(Rc::new(value))
    }
}

impl IBindingTypeConverterOf<Option<f32>, f32> for NullableSingleToSingleTypeConverter {
    fn try_convert(&self, from: Option<&Option<f32>>, _conversion_hint: Option<&dyn Any>) -> Option<f32> {
        from.copied().flatten()
    }
}
