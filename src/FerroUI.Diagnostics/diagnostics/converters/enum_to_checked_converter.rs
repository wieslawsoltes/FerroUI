//! Port of `Diagnostics/Converters/EnumToCheckedConverter.cs`.

use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::Rc;

/// Converts a value to whether it equals the parameter (a radio button checked for one value of
/// an enumeration), and a check back to the parameter.
#[derive(Default)]
pub struct EnumToCheckedConverter;

impl PartialEq for EnumToCheckedConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl EnumToCheckedConverter {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
}

impl IValueConverter for EnumToCheckedConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // `Equals(value, parameter)`: nulls are equal; a nullable value is null or its contents.
        let value = value.cloned().and_then(ValueTypes::normalize);
        let parameter = parameter.cloned().and_then(ValueTypes::normalize);
        Ok(Some(Rc::new(ValueTypes::identity_equals(value.as_ref(), parameter.as_ref()))))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if cast_value::<bool>(value) == Some(true) {
            return Ok(parameter.cloned());
        }

        Ok(Some(BindingOperations::do_nothing()))
    }
}

ferro_markup_type!(class EnumToCheckedConverter {
    handles: [Rc<EnumToCheckedConverter>, Option<Rc<EnumToCheckedConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => EnumToCheckedConverter::new],
});
