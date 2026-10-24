use crate::{Error, Grammar, Result, StateStack, TokenizeLineResult, TokenizeOptions};
use std::{borrow::Cow, fmt, ops::Range};

/// Borrowed or owned Unicode line input. Length and slice positions count
/// UTF-16 code units; as_str exposes the underlying idiomatic Rust UTF-8 view.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LineText<'a>(Cow<'a, str>);
impl<'a> LineText<'a> {
    pub fn new(text: &'a str) -> Self {
        Self(Cow::Borrowed(text))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.encode_utf16().count()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn into_owned(self) -> LineText<'static> {
        LineText(Cow::Owned(self.0.into_owned()))
    }
    pub fn without_line_ending(&self) -> &str {
        self.0
            .strip_suffix("\r\n")
            .or_else(|| self.0.strip_suffix('\n'))
            .or_else(|| self.0.strip_suffix('\r'))
            .unwrap_or(&self.0)
    }
    pub fn slice(&self, range: Range<usize>) -> Result<LineText<'_>> {
        if range.start > range.end {
            return Err(Error::InvalidOffset(range.start));
        }
        let start = byte_at_utf16(&self.0, range.start)?;
        let end = byte_at_utf16(&self.0, range.end)?;
        Ok(LineText::new(&self.0[start..end]))
    }
}
impl<'a> From<&'a str> for LineText<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}
impl From<String> for LineText<'static> {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}
impl AsRef<str> for LineText<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Display for LineText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn byte_at_utf16(text: &str, offset: usize) -> Result<usize> {
    let mut index = 0;
    for (byte, character) in text.char_indices() {
        if index == offset {
            return Ok(byte);
        }
        index += character.len_utf16();
        if index > offset {
            return Err(Error::InvalidOffset(offset));
        }
    }
    if index == offset {
        Ok(text.len())
    } else {
        Err(Error::InvalidOffset(offset))
    }
}

/// Tokenizer abstraction for hosts that do not own a concrete grammar.
pub trait ITokenizationSupport {
    fn initial_state(&self) -> StateStack;
    fn tokenize(
        &self,
        line: LineText<'_>,
        previous: Option<&StateStack>,
        options: TokenizeOptions,
    ) -> Result<TokenizeLineResult>;
}
impl ITokenizationSupport for Grammar {
    fn initial_state(&self) -> StateStack {
        self.initial_state()
    }
    fn tokenize(
        &self,
        line: LineText<'_>,
        previous: Option<&StateStack>,
        options: TokenizeOptions,
    ) -> Result<TokenizeLineResult> {
        self.tokenize_line_with_options(line.without_line_ending(), previous, options, None)
    }
}
