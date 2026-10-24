//! Port of `Diagnostics/Converters/GetTypeNameConverter.cs`.

use crate::diagnostics::TypeExtesnions;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue, TypeInfo};
use std::rc::Rc;

/// Converts a type (`value is Type`: a [`ValueType`], or the [`TypeInfo`] of a class) to the
/// name DevTools shows for it ([`TypeExtesnions::get_type_name`]).
#[derive(Default)]
pub struct GetTypeNameConverter;

impl PartialEq for GetTypeNameConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl GetTypeNameConverter {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
}

impl IValueConverter for GetTypeNameConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(type_) = value.and_then(|value| value.downcast_ref::<ValueType>()) {
            return Ok(Some(Rc::new(type_.get_type_name())));
        }
        if let Some(type_) = value.and_then(|value| value.downcast_ref::<&'static TypeInfo>()) {
            return Ok(Some(Rc::new(type_.get_type_name())));
        }
        Ok(Some(BindingOperations::do_nothing()))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(BindingOperations::do_nothing()))
    }
}

ferro_markup_type!(class GetTypeNameConverter {
    handles: [Rc<GetTypeNameConverter>, Option<Rc<GetTypeNameConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => GetTypeNameConverter::new],
});
