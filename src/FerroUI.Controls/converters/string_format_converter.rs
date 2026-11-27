use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::composite_format::format_values;
use ferroui_base::data::converters::{cast_value, IMultiValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

/// Calls a composite format on the passed in values, where the first element
/// in the list is the format string and the following items are the
/// arguments.
// Made only through `new` (`non_exhaustive`), which registers the casts of the converter.
#[non_exhaustive]
pub struct StringFormatConverter;

impl Default for StringFormatConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl StringFormatConverter {
    pub fn new() -> Self {
        register_casts!(Self);
        Self
    }
}

impl IMultiValueConverter for StringFormatConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(format) = values.first().and_then(|v| cast_value::<String>(v.as_ref())) {
            return match format_values(&format, &values[1..]) {
                Ok(text) => Ok(Some(Rc::new(text))),
                Err(_) => Ok(Some(FerroProperty::unset_value())),
            };
        }

        Ok(Some(FerroProperty::unset_value()))
    }
}
