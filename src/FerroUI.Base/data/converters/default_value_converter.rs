use crate::utilities::CultureInfo;
use super::IValueConverter;
use crate::data::core::{ValueType, ValueTypes};
use crate::data::{BindingError, BindingErrorType, BindingNotification};
use crate::BoxedValue;
use std::rc::Rc;

/// Provides a default set of value conversions for bindings that do not
/// specify a value converter: the conversions known to
/// [`ValueTypes`].
pub struct DefaultValueConverter;

impl DefaultValueConverter {
    /// The shared instance.
    pub fn instance() -> Rc<dyn IValueConverter> {
        thread_local! {
            static INSTANCE: Rc<dyn IValueConverter> = Rc::new(DefaultValueConverter);
        }
        INSTANCE.with(Rc::clone)
    }

    fn convert_core(value: Option<&BoxedValue>, target_type: ValueType, culture: &CultureInfo) -> Option<BoxedValue> {
        if value.is_none() && ValueTypes::accepts_null(target_type) {
            return ValueTypes::try_convert(None, target_type).flatten();
        }
        // The delegate of a method becomes a command for a command target.
        if let Some(command) = value.and_then(|value| super::method_to_command(value, target_type)) {
            return Some(command);
        }
        if let Some(result) = ValueTypes::try_convert_with_culture(value, target_type, culture) {
            return result;
        }
        let message = match value {
            Some(v) => format!(
                "Could not convert '{}' to '{}'.",
                ValueTypes::to_display_string(Some(v)),
                target_type
            ),
            None => format!("Could not convert null to '{target_type}'."),
        };
        Some(Rc::new(BindingNotification::with_error(BindingError::message(message), BindingErrorType::Error)))
    }
}

impl IValueConverter for DefaultValueConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Self::convert_core(value, target_type, culture))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Self::convert_core(value, target_type, culture))
    }
}
