//! Port of `Converters/TimeSpanTypeConverter.cs`.

use super::type_converter::{convert_to_text, text_of, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::XamlLoadError;
use ferroui_base::animation::SignedDuration;
use ferroui_base::data::core::ValueType;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use std::rc::Rc;
use std::time::Duration;

/// Converts text to a time span. Besides the time span format
/// (`[d.]hh:mm[:ss[.fffffff]]`) it accepts a number of seconds (`0.25`).
///
/// A time span has two Rust forms, `SignedDuration` and `Duration` (C5 of #423): the
/// converter converts text to a `SignedDuration`, and a time span or text to a `Duration`
/// ([`convert_to`](TypeConverter::convert_to)), where a negative time span is an error
/// (decision O2 of docs/idiomatic-rust-signatures.md).
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
    ) -> Result<Option<BoxedValue>, XamlLoadError> {
        let value_str = text_of(value)?;
        if !value_str.contains(':') {
            // shorthand seconds format (ie. "0.25")
            let secs: f64 = value_str.trim().parse().map_err(|_| {
                XamlLoadError::with_message(format!("The input string '{value_str}' was not in a correct format."))
            })?;
            return Ok(Some(Rc::new(SignedDuration::from_seconds(secs))));
        }

        let time_span = SignedDuration::parse(value_str).map_err(|e| XamlLoadError::with_message(e.to_string()))?;
        Ok(Some(Rc::new(time_span)))
    }

    fn can_convert_to(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, destination_type: ValueType) -> bool {
        destination_type.is_string() || destination_type.is::<Duration>() || destination_type.is::<Option<Duration>>()
    }

    /// Converts a time span (a `SignedDuration`, a `Duration`, or text that converts to a
    /// time span) to a `Duration` or to its nullable form; a negative time span is not a
    /// duration and fails the conversion. Every value converts to text.
    fn convert_to(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
        destination_type: ValueType,
    ) -> Result<Option<BoxedValue>, XamlLoadError> {
        let nullable = destination_type.is::<Option<Duration>>();
        if !nullable && !destination_type.is::<Duration>() {
            return convert_to_text(value, destination_type);
        }
        let boxed = |duration: Duration| -> BoxedValue {
            match nullable {
                true => Rc::new(Some(duration)) as BoxedValue,
                false => Rc::new(duration) as BoxedValue,
            }
        };
        let span = match value {
            None if nullable => return Ok(Some(Rc::new(Option::<Duration>::None))),
            Some(value) if value.is::<Duration>() => {
                return Ok(value.downcast_ref::<Duration>().map(|duration| boxed(*duration)));
            }
            Some(value) if value.is::<SignedDuration>() => value.downcast_ref::<SignedDuration>().copied(),
            _ => self.convert_from(context, culture, value)?.and_then(|converted| converted.downcast_ref::<SignedDuration>().copied()),
        };
        let span = span.ok_or_else(|| {
            XamlLoadError::with_message(format!(
                "'TimeSpanTypeConverter' is unable to convert '{}' to '{}'.",
                value.map_or("(null)", |v| (**v).type_name()),
                destination_type
            ))
        })?;
        let duration = Duration::try_from(span).map_err(|error| XamlLoadError::with_message(error.to_string()))?;
        Ok(Some(boxed(duration)))
    }
}

type_converter_markup!(TimeSpanTypeConverter);
