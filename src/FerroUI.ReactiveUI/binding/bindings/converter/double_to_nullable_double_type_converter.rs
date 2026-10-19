//! `DoubleToNullableDoubleTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/DoubleToNullableDoubleTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `f64` (`double`) to an `Option` holding it (`DoubleToNullableDoubleTypeConverter`);
/// it always succeeds for a value of the type.
#[derive(Clone, Copy, Debug, Default)]
pub struct DoubleToNullableDoubleTypeConverter;

impl IBindingTypeConverter for DoubleToNullableDoubleTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<f64>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<Option<f64>>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let value = *from?.downcast_ref::<f64>()?;
        Some(Rc::new(Some(value)))
    }
}

impl IBindingTypeConverterOf<f64, Option<f64>> for DoubleToNullableDoubleTypeConverter {
    fn try_convert(&self, from: Option<&f64>, _conversion_hint: Option<&dyn Any>) -> Option<Option<f64>> {
        Some(Some(*from?))
    }
}
