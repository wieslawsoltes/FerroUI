use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converter to convert an enum value to bool by comparing to the given
/// parameter. Both value and parameter must be of the same enum type.
///
/// This converter is useful to enable binding of radio buttons with a
/// selected enum value.
#[derive(Default)]
pub struct EnumToBoolConverter;

impl EnumToBoolConverter {
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for EnumToBoolConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let value_is_null = value.cloned().and_then(ValueTypes::normalize).is_none();
        let parameter_is_null = parameter.cloned().and_then(ValueTypes::normalize).is_none();

        let result = if value_is_null && parameter_is_null {
            true
        } else if value_is_null || parameter_is_null {
            false
        } else {
            let value = value.cloned().and_then(ValueTypes::normalize);
            let parameter = parameter.cloned().and_then(ValueTypes::normalize);
            ValueTypes::identity_equals(value.as_ref(), parameter.as_ref())
        };

        Ok(Some(Rc::new(result)))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if cast_value::<bool>(value) == Some(true) {
            return Ok(parameter.cloned());
        }

        Ok(Some(BindingOperations::do_nothing()))
    }
}
