//! Port of `Converters/TimeSpanTypeConverter.cs`.

use super::type_converter::{text_of, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::XamlLoadException;
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::core::ValueType;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts text to a time span. Besides the time span format
/// (`[d.]hh:mm[:ss[.fffffff]]`) it accepts a number of seconds (`0.25`).
pub struct TimeSpanTypeConverter;

impl TypeConverter for TimeSpanTypeConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        _context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let value_str = text_of(value)?;
        if !value_str.contains(':') {
            // shorthand seconds format (ie. "0.25")
            let secs: f64 = value_str.trim().parse().map_err(|_| {
                XamlLoadException::with_message(format!("The input string '{value_str}' was not in a correct format."))
            })?;
            return Ok(Some(Rc::new(TimeSpan::from_seconds(secs))));
        }

        let time_span = TimeSpan::parse(value_str).map_err(|e| XamlLoadException::with_message(e.to_string()))?;
        Ok(Some(Rc::new(time_span)))
    }
}

type_converter_markup!(TimeSpanTypeConverter);
