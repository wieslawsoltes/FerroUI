//! `StringConverter` (`ReactiveUI.Binding/Bindings/Converter/StringConverter.cs`).

use crate::{BindingAffinity, ConverterType, IBindingTypeConverter};
use std::any::Any;
use std::rc::Rc;

/// The identity converter of strings (`StringConverter`): a `String` converts to itself, and no
/// value (`None`) or a value of another type fails. The conversion hint is ignored.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringConverter;

impl IBindingTypeConverter for StringConverter {
    fn from_type(&self) -> ConverterType {
        ConverterType::of::<String>()
    }

    fn to_type(&self) -> ConverterType {
        ConverterType::of::<String>()
    }

    fn get_affinity_for_objects(&self) -> i32 {
        BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER
    }

    fn try_convert_typed(&self, from: Option<&dyn Any>, _conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>> {
        let text = from?.downcast_ref::<String>()?;
        Some(Rc::new(text.clone()))
    }
}
