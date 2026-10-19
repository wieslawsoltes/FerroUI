//! `NullableDoubleToDoubleTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/NullableDoubleToDoubleTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `Option` of `f64` (`double`) to the value it holds
/// (`NullableDoubleToDoubleTypeConverter`); `None` fails.
///
/// The type-erased conversion takes an `Option<f64>`, or a plain `f64` as upstream takes the boxed
/// value of a nullable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullableDoubleToDoubleTypeConverter;

impl IBindingTypeConverter for NullableDoubleToDoubleTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<Option<f64>>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<f64>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let from = from?;
        let value = match from.downcast_ref::<Option<f64>>() {
            Some(value) => self.try_convert(Some(value), conversion_hint)?,
            None => *from.downcast_ref::<f64>()?,
        };
        Some(Rc::new(value))
    }
}

impl IBindingTypeConverterOf<Option<f64>, f64> for NullableDoubleToDoubleTypeConverter {
    fn try_convert(&self, from: Option<&Option<f64>>, _conversion_hint: Option<&dyn Any>) -> Option<f64> {
        from.copied().flatten()
    }
}
