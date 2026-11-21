use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::input::KeyGesture;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts a [`KeyGesture`] to a string, formatting it according to the
/// current platform's style guidelines.
// Made only through `new` (`non_exhaustive`), which registers the casts of the converter.
#[non_exhaustive]
pub struct PlatformKeyGestureConverter;

impl Default for PlatformKeyGestureConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformKeyGestureConverter {
    pub fn new() -> Self {
        register_casts!(Self);
        Self
    }

    /// Converts a [`KeyGesture`] to a string, formatting it according to the
    /// current platform's style guidelines.
    pub fn to_platform_string(gesture: &KeyGesture) -> String {
        gesture.to_platform_string(None)
    }
}

impl IValueConverter for PlatformKeyGestureConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if value.cloned().and_then(ValueTypes::normalize).is_none() {
            return Ok(None);
        }

        match cast_value::<KeyGesture>(value) {
            Some(gesture) if target_type.is_string() => Ok(Some(Rc::new(gesture.to_platform_string(None)))),
            _ => Err(BindingError::message("Specified method is not supported.")),
        }
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}
