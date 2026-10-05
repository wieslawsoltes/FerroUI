use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::input::KeyGesture;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts a [`KeyGesture`] to a string, formatting it according to the
/// current platform's style guidelines.
#[derive(Default)]
pub struct PlatformKeyGestureConverter;

impl PlatformKeyGestureConverter {
    pub fn new() -> Self {
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
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}
