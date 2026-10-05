//! Allocation-free tokenizer used by the `parse` implementations of the value types.

use std::borrow::Cow;
use std::fmt;

use crate::utilities::span_helpers::{try_parse_double, try_parse_int, NumberStyles};

const DEFAULT_SEPARATOR_CHAR: char = ',';

const DEFAULT_FORMAT_MESSAGE: &str = "One of the identified items was in an invalid format.";

/// Error returned when a string is not in the expected format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatError {
    message: Cow<'static, str>,
}

impl FormatError {
    /// Creates an error with a static message.
    #[inline]
    pub const fn new(message: &'static str) -> Self {
        Self {
            message: Cow::Borrowed(message),
        }
    }

    /// Creates an error with a computed message.
    #[inline]
    pub fn from_string(message: String) -> Self {
        Self {
            message: Cow::Owned(message),
        }
    }

    /// Creates the error reported when a number cannot be parsed from `input`.
    pub fn invalid_input(input: &str) -> Self {
        Self::from_string(format!(
            "The input string '{input}' was not in a correct format."
        ))
    }

    /// Gets the error message.
    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Default for FormatError {
    fn default() -> Self {
        Self::new(DEFAULT_FORMAT_MESSAGE)
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FormatError {}

/// Splits a string into tokens separated by white space and/or a single separator
/// character, reading them as integers, doubles or raw slices.
///
/// Reading can fail in two distinct ways, mirrored by the `try_read_*` signatures:
/// `Ok(None)` means "no (valid) value here", `Err` means the input is malformed
/// (empty token, doubled separator, trailing separator, ...).
#[derive(Clone, Debug)]
pub struct SpanStringTokenizer<'a> {
    s: &'a str,
    separator: char,
    exception_message: Option<&'static str>,
    /// Byte offset of the read cursor.
    index: usize,
    /// Byte offset and byte length of the current token.
    token: Option<(usize, usize)>,
}

impl<'a> SpanStringTokenizer<'a> {
    /// Creates a tokenizer using `,` as separator and the default error message.
    #[inline]
    pub fn new(s: &'a str) -> Self {
        Self::with_separator(s, DEFAULT_SEPARATOR_CHAR, None)
    }

    /// Creates a tokenizer using `,` as separator (the invariant-culture separator)
    /// and the given error message.
    #[inline]
    pub fn with_message(s: &'a str, exception_message: &'static str) -> Self {
        Self::with_separator(s, DEFAULT_SEPARATOR_CHAR, Some(exception_message))
    }

    /// Creates a tokenizer with an explicit separator and optional error message.
    pub fn with_separator(
        s: &'a str,
        separator: char,
        exception_message: Option<&'static str>,
    ) -> Self {
        let trimmed = s.trim_start_matches(char::is_whitespace);
        Self {
            s,
            separator,
            exception_message,
            index: s.len() - trimmed.len(),
            token: None,
        }
    }

    /// Byte offset of the current token, or `None` when there is no current token.
    #[inline]
    pub fn current_token_index(&self) -> Option<usize> {
        self.token.map(|(index, _)| index)
    }

    /// The current token, or `None` when there is no current token.
    #[inline]
    pub fn current_token(&self) -> Option<&'a str> {
        self.token.map(|(index, len)| &self.s[index..index + len])
    }

    /// The current token, or an empty slice when there is no current token.
    #[inline]
    pub fn current_token_span(&self) -> &'a str {
        self.current_token().unwrap_or("")
    }

    /// Verifies that the whole input was consumed.
    ///
    /// This is the counterpart of disposing the tokenizer in the reference
    /// implementation and must be called once reading is done.
    #[inline]
    pub fn finish(&self) -> Result<(), FormatError> {
        if self.index != self.s.len() {
            return Err(self.format_error());
        }
        Ok(())
    }

    /// Runs `f` and then [`finish`](Self::finish). An error from `finish` takes
    /// precedence over an error from `f`, exactly like an exception thrown while
    /// leaving a scoped-disposal block replaces the one in flight.
    #[inline]
    pub fn scope<T>(
        mut self,
        f: impl FnOnce(&mut Self) -> Result<T, FormatError>,
    ) -> Result<T, FormatError> {
        let result = f(&mut self);
        self.finish()?;
        result
    }

    #[inline]
    pub fn try_read_int32(&mut self) -> Result<Option<i32>, FormatError> {
        self.try_read_int32_with(None)
    }

    pub fn try_read_int32_with(
        &mut self,
        separator: Option<char>,
    ) -> Result<Option<i32>, FormatError> {
        Ok(match self.try_read_span_with(separator)? {
            Some(token) => try_parse_int(token, NumberStyles::INTEGER),
            None => None,
        })
    }

    #[inline]
    pub fn read_int32(&mut self) -> Result<i32, FormatError> {
        self.read_int32_with(None)
    }

    pub fn read_int32_with(&mut self, separator: Option<char>) -> Result<i32, FormatError> {
        match self.try_read_int32_with(separator)? {
            Some(value) => Ok(value),
            None => Err(self.format_error()),
        }
    }

    #[inline]
    pub fn try_read_double(&mut self) -> Result<Option<f64>, FormatError> {
        self.try_read_double_with(None)
    }

    pub fn try_read_double_with(
        &mut self,
        separator: Option<char>,
    ) -> Result<Option<f64>, FormatError> {
        Ok(match self.try_read_span_with(separator)? {
            Some(token) => try_parse_double(token, NumberStyles::FLOAT),
            None => None,
        })
    }

    #[inline]
    pub fn read_double(&mut self) -> Result<f64, FormatError> {
        self.read_double_with(None)
    }

    pub fn read_double_with(&mut self, separator: Option<char>) -> Result<f64, FormatError> {
        match self.try_read_double_with(separator)? {
            Some(value) => Ok(value),
            None => Err(self.format_error()),
        }
    }

    /// Same as [`try_read_span`](Self::try_read_span); tokens are borrowed slices.
    #[inline]
    pub fn try_read_string(&mut self) -> Result<Option<&'a str>, FormatError> {
        self.try_read_span_with(None)
    }

    #[inline]
    pub fn try_read_string_with(
        &mut self,
        separator: Option<char>,
    ) -> Result<Option<&'a str>, FormatError> {
        self.try_read_span_with(separator)
    }

    /// Same as [`read_span`](Self::read_span); tokens are borrowed slices.
    #[inline]
    pub fn read_string(&mut self) -> Result<&'a str, FormatError> {
        self.read_span_with(None)
    }

    #[inline]
    pub fn read_string_with(&mut self, separator: Option<char>) -> Result<&'a str, FormatError> {
        self.read_span_with(separator)
    }

    #[inline]
    pub fn try_read_span(&mut self) -> Result<Option<&'a str>, FormatError> {
        self.try_read_span_with(None)
    }

    pub fn try_read_span_with(
        &mut self,
        separator: Option<char>,
    ) -> Result<Option<&'a str>, FormatError> {
        let success = self.try_read_token(separator.unwrap_or(self.separator))?;
        Ok(if success { self.current_token() } else { None })
    }

    #[inline]
    pub fn read_span(&mut self) -> Result<&'a str, FormatError> {
        self.read_span_with(None)
    }

    pub fn read_span_with(&mut self, separator: Option<char>) -> Result<&'a str, FormatError> {
        match self.try_read_span_with(separator)? {
            Some(token) => Ok(token),
            None => Err(self.format_error()),
        }
    }

    #[inline]
    fn peek(&self) -> Option<char> {
        self.s[self.index..].chars().next()
    }

    fn try_read_token(&mut self, separator: char) -> Result<bool, FormatError> {
        self.token = None;

        if self.index >= self.s.len() {
            return Ok(false);
        }

        let index = self.index;

        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == separator {
                break;
            }
            self.index += c.len_utf8();
        }

        let length = self.index - index;

        self.skip_to_next_token(separator)?;

        self.token = Some((index, length));

        if length < 1 {
            return Err(self.format_error());
        }

        Ok(true)
    }

    fn skip_to_next_token(&mut self, separator: char) -> Result<(), FormatError> {
        if let Some(c) = self.peek() {
            if c != separator && !c.is_whitespace() {
                return Err(self.format_error());
            }

            let mut length = 0;

            while let Some(c) = self.peek() {
                if c == separator {
                    length += 1;
                    self.index += c.len_utf8();

                    if length > 1 {
                        return Err(self.format_error());
                    }
                } else {
                    if !c.is_whitespace() {
                        break;
                    }

                    self.index += c.len_utf8();
                }
            }

            if length > 0 && self.index >= self.s.len() {
                return Err(self.format_error());
            }
        }

        Ok(())
    }

    fn format_error(&self) -> FormatError {
        match self.exception_message {
            Some(message) => FormatError::new(message),
            None => FormatError::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_int32_reads_values() {
        let mut target = SpanStringTokenizer::new("123,456");

        assert_eq!(123, target.read_int32().unwrap());
        assert_eq!(456, target.read_int32().unwrap());
        assert!(target.read_int32().is_err());
    }

    #[test]
    fn read_double_reads_values() {
        let mut target = SpanStringTokenizer::new("12.3,45.6");

        assert_eq!(12.3, target.read_double().unwrap());
        assert_eq!(45.6, target.read_double().unwrap());
        assert!(target.read_double().is_err());
    }

    #[test]
    fn try_read_int32_reads_values() {
        let mut target = SpanStringTokenizer::new("123,456");

        assert_eq!(Ok(Some(123)), target.try_read_int32());
        assert_eq!(Ok(Some(456)), target.try_read_int32());
        assert_eq!(Ok(None), target.try_read_int32());
    }

    #[test]
    fn try_read_int32_doesnt_throw() {
        let mut target = SpanStringTokenizer::new("abc");

        assert_eq!(Ok(None), target.try_read_int32());
    }

    #[test]
    fn try_read_double_reads_values() {
        let mut target = SpanStringTokenizer::new("12.3,45.6");

        assert_eq!(Ok(Some(12.3)), target.try_read_double());
        assert_eq!(Ok(Some(45.6)), target.try_read_double());
        assert_eq!(Ok(None), target.try_read_double());
    }

    #[test]
    fn try_read_double_doesnt_throw() {
        let mut target = SpanStringTokenizer::new("abc");

        assert_eq!(Ok(None), target.try_read_double());
    }

    #[test]
    fn read_span_and_read_string_reads_same() {
        let mut target1 = SpanStringTokenizer::new("abc,def");
        let mut target2 = SpanStringTokenizer::new("abc,def");

        assert_eq!(target1.read_string().unwrap(), target2.read_span().unwrap());
        assert_eq!(target1.read_span().unwrap(), target2.read_string().unwrap());
    }

    // Additional coverage of the malformed-input rules.

    #[test]
    fn rejects_empty_tokens_and_dangling_separators() {
        assert!(SpanStringTokenizer::new(",1").read_int32().is_err());
        assert!(SpanStringTokenizer::new("1,,2").read_int32().is_err());
        assert!(SpanStringTokenizer::new("1,").read_int32().is_err());

        let mut t = SpanStringTokenizer::with_message("1 , 2 3", "Invalid.");
        assert_eq!(Ok(1), t.read_int32());
        assert_eq!(Ok(2), t.read_int32());
        assert_eq!(Err(FormatError::new("Invalid.")), t.finish());
        assert_eq!(Ok(3), t.read_int32());
        assert_eq!(Ok(()), t.finish());
    }

    #[test]
    fn tracks_current_token() {
        let mut t = SpanStringTokenizer::new("  ab  cd");
        assert_eq!(None, t.current_token());
        assert_eq!("", t.current_token_span());
        assert_eq!(Ok("ab"), t.read_span());
        assert_eq!(Some(2), t.current_token_index());
        assert_eq!(Ok("cd"), t.read_span());
        assert_eq!(Some("cd"), t.current_token());
        assert_eq!(Ok(None), t.try_read_span());
        assert_eq!(None, t.current_token_index());
    }
}
