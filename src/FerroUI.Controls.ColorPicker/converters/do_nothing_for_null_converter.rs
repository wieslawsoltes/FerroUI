use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::BoxedValue;

/// Converter that will do nothing (not update bound values) when a null value is encountered.
/// This converter enables binding nullable with non-nullable properties in some scenarios.
#[derive(Debug, Default)]
pub struct DoNothingForNullConverter;

impl DoNothingForNullConverter {
    /// Initializes a new instance of the [`DoNothingForNullConverter`] class.
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for DoNothingForNullConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(value.cloned().unwrap_or_else(BindingOperations::do_nothing)))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(value.cloned().unwrap_or_else(BindingOperations::do_nothing)))
    }
}
