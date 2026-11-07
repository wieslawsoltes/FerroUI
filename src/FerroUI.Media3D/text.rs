//! The text forms of the values: WPF's `Parse` and `ConvertToString` in the invariant culture.
//!
//! The numbers of a value are separated by a comma, white space, or both (`"1,2,3"`, `"1 2 3"`,
//! `"1, 2, 3"`), as WPF's tokenizer reads them; a value is written with commas and no spaces, each
//! number in its shortest round-trip form. Some values have a keyword for a distinguished value
//! (`"Identity"`, `"Empty"`), compared after trimming white space and case-sensitively, as WPF does.

use std::fmt;

use ferroui_base::utilities::span_helpers::{try_parse_double, try_parse_int, InvariantF64, NumberStyles};
use ferroui_base::utilities::{FormatError, SpanStringTokenizer};

/// Reads exactly `N` numbers from `text`; anything else is an error with `message`.
pub(crate) fn parse_numbers<const N: usize>(text: &str, message: &'static str) -> Result<[f64; N], FormatError> {
    SpanStringTokenizer::with_message(text, message).scope(|tokenizer| {
        let mut values = [0.0; N];
        for value in &mut values {
            *value = tokenizer.read_double()?;
        }
        Ok(values)
    })
}

/// Reads every number of `text` (a list of any length, as the collections write theirs); a
/// malformed number or separator is an error with `message`.
pub(crate) fn parse_number_list(text: &str, message: &'static str) -> Result<Vec<f64>, FormatError> {
    SpanStringTokenizer::with_message(text, message).scope(|tokenizer| {
        let mut values = Vec::new();
        while let Some(token) = tokenizer.try_read_span()? {
            values.push(try_parse_double(token, NumberStyles::FLOAT).ok_or(FormatError::new(message))?);
        }
        Ok(values)
    })
}

/// Reads every integer of `text`, as [`parse_number_list`].
pub(crate) fn parse_integer_list(text: &str, message: &'static str) -> Result<Vec<i32>, FormatError> {
    SpanStringTokenizer::with_message(text, message).scope(|tokenizer| {
        let mut values = Vec::new();
        while let Some(token) = tokenizer.try_read_span()? {
            values.push(try_parse_int(token, NumberStyles::INTEGER).ok_or(FormatError::new(message))?);
        }
        Ok(values)
    })
}

/// Whether `text` is the keyword alone, with white space around it.
pub(crate) fn is_keyword(text: &str, keyword: &str) -> bool {
    text.trim() == keyword
}

/// Writes the numbers separated by commas.
pub(crate) fn write_numbers(f: &mut fmt::Formatter<'_>, values: &[f64]) -> fmt::Result {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            f.write_str(",")?;
        }
        write!(f, "{}", InvariantF64(*value))?;
    }
    Ok(())
}

/// WPF's `DoubleUtil.IsZero`: closer to zero than ten machine epsilons.
#[inline]
pub(crate) fn is_zero(value: f64) -> bool {
    value.abs() < 10.0 * f64::EPSILON
}

/// WPF's `DoubleUtil.IsOne`: closer to one than ten machine epsilons.
#[inline]
pub(crate) fn is_one(value: f64) -> bool {
    (value - 1.0).abs() < 10.0 * f64::EPSILON
}
