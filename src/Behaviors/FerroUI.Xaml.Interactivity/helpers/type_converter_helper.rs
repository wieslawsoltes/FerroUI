//! Port of `Helpers/TypeConverterHelper.cs`.

use super::ParseHelper;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use std::num::IntErrorKind;
use std::rc::Rc;

/// Why a text could not be converted: the exceptions of the conversions of the reference that
/// its callers tell apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionError {
    /// The text is not in the format of the type (`FormatException`).
    Format(String),
    /// The text is a number outside the range of the type (`OverflowException`).
    Overflow(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Format(message) | Self::Overflow(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ConversionError {}

/// A helper class that enables converting values specified in markup (strings) to their object
/// representation.
pub struct TypeConverterHelper;

impl TypeConverterHelper {
    /// Converts string representation of a value to its object representation.
    ///
    /// The text, boolean, 32-bit integer and floating point types are parsed as the reference
    /// parses them (a malformed or overflowing text is an error); every other type converts with
    /// the conversions of the untyped values (enumerations, the registered conversions and the
    /// `parse:` of the markup metadata of the type, the type converters of the reference), and
    /// finally [`ParseHelper`]. `Ok(None)` when no conversion applies.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the reference's chain of `Enum.Parse`,
    /// `IConvertible` and `TypeDescriptor.GetConverter` is the conversion of the untyped values of
    /// the framework.
    pub fn convert(value: &str, destination_type: ValueType) -> Result<Option<BoxedValue>, ConversionError> {
        // Value types in the "System" namespace must be special cased due to a bug in the xaml
        // compiler.
        if destination_type.is::<String>() {
            return Ok(Some(Rc::new(value.to_string())));
        }

        if destination_type.is::<bool>() {
            return match value.trim() {
                text if text.eq_ignore_ascii_case("true") => Ok(Some(Rc::new(true))),
                text if text.eq_ignore_ascii_case("false") => Ok(Some(Rc::new(false))),
                _ => Err(ConversionError::Format(format!("String '{value}' was not recognized as a valid Boolean."))),
            };
        }

        if destination_type.is::<i32>() {
            return match value.trim().parse::<i32>() {
                Ok(number) => Ok(Some(Rc::new(number))),
                Err(error) => match error.kind() {
                    IntErrorKind::PosOverflow | IntErrorKind::NegOverflow => {
                        Err(ConversionError::Overflow("Value was either too large or too small for an Int32.".to_string()))
                    }
                    _ => Err(ConversionError::Format(format!("The input string '{value}' was not in a correct format."))),
                },
            };
        }

        if destination_type.is::<f64>() {
            return match value.trim().parse::<f64>() {
                Ok(number) => Ok(Some(Rc::new(number))),
                Err(_) => Err(ConversionError::Format(format!("The input string '{value}' was not in a correct format."))),
            };
        }

        let text: BoxedValue = Rc::new(value.to_string());
        if let Some(Some(converted)) = ValueTypes::try_convert(Some(&text), destination_type) {
            return Ok(Some(converted));
        }

        Ok(ParseHelper::invoke_parse(value, destination_type))
    }
}
