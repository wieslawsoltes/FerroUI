//! `SingleToNullableSingleTypeConverter`
//! (`ReactiveUI.Binding/Bindings/Converter/SingleToNullableSingleTypeConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter, IBindingTypeConverterOf};
use std::any::Any;
use std::rc::Rc;

/// Converts an `f32` (`float`) to an `Option` holding it (`SingleToNullableSingleTypeConverter`);
/// it always succeeds for a value of the type.
#[derive(Clone, Copy, Debug, Default)]
pub struct SingleToNullableSingleTypeConverter;

impl IBindingTypeConverter for SingleToNullableSingleTypeConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<f32>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<Option<f32>>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let value = *from?.downcast_ref::<f32>()?;
        Some(Rc::new(Some(value)))
    }
}

impl IBindingTypeConverterOf<f32, Option<f32>> for SingleToNullableSingleTypeConverter {
    fn try_convert(&self, from: Option<&f32>, _conversion_hint: Option<&dyn Any>) -> Option<Option<f32>> {
        Some(Some(*from?))
    }
}
