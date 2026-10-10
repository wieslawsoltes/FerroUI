//! The port of `AtSpiTextHandler.cs`: `org.a11y.atspi.Text` over the
//! value of a value provider.
//!
//! Offsets are counted in UTF-16 code units, as the reference counts
//! them (the indices of its strings).

use super::node_of;
use crate::at_spi::at_spi_constants::TEXT_VERSION;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::dbus::descriptions::TEXT;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use crate::at_spi::dbus::types::{AtSpiAttributeSet, AtSpiTextRange};
use ferroui_controls::automation::provider::IValueProvider;
use ferroui_controls::utils::StringUtils;
use std::rc::Weak;
use zbus::zvariant::Value;

// `TextGranularity`
const GRANULARITY_CHAR: u32 = 0;
const GRANULARITY_WORD: u32 = 1;

// `TextBoundaryType`
const BOUNDARY_CHAR: u32 = 0;
const BOUNDARY_WORD_START: u32 = 1;
const BOUNDARY_WORD_END: u32 = 2;

/// A piece of text with the offsets of its start and its end.
pub(crate) type TextSegment = (String, i32, i32);

fn length(text: &[u16]) -> i32 {
    i32::try_from(text.len()).unwrap_or(i32::MAX)
}

/// `text.Substring(start, end - start)`.
fn substring(text: &[u16], start: i32, end: i32) -> String {
    let start = usize::try_from(start).unwrap_or(0).min(text.len());
    let end = usize::try_from(end).unwrap_or(0).clamp(start, text.len());
    String::from_utf16_lossy(&text[start..end])
}

fn empty() -> TextSegment {
    (String::new(), 0, 0)
}

pub(crate) fn get_string_at_offset(text: &[u16], offset: i32, granularity: u32) -> TextSegment {
    if text.is_empty() {
        return empty();
    }

    let offset = offset.min(length(text) - 1).max(0);

    // For CHAR granularity, return single character
    if granularity == GRANULARITY_CHAR {
        return (substring(text, offset, offset + 1), offset, offset + 1);
    }

    // For WORD granularity, find word boundaries
    if granularity == GRANULARITY_WORD {
        let start = StringUtils::previous_word(text, offset + 1);
        if start >= length(text) || !StringUtils::is_start_of_word(text, start) {
            return empty();
        }

        let end = StringUtils::next_word(text, start).min(length(text));
        if end <= start {
            return empty();
        }

        return (substring(text, start, end), start, end);
    }

    // For SENTENCE, LINE, PARAGRAPH - return full text
    (String::from_utf16_lossy(text), 0, length(text))
}

pub(crate) fn get_text(text: &[u16], start_offset: i32, end_offset: i32) -> String {
    if text.is_empty() {
        return String::new();
    }

    let start_offset = start_offset.max(0);
    let end_offset = if end_offset < 0 || end_offset > length(text) { length(text) } else { end_offset };

    if start_offset >= end_offset {
        return String::new();
    }

    substring(text, start_offset, end_offset)
}

pub(crate) fn get_text_before_offset(text: &[u16], offset: i32, boundary_type: u32) -> TextSegment {
    if offset <= 0 || text.is_empty() {
        return empty();
    }

    let offset = offset.min(length(text));

    // CHAR boundary
    if boundary_type == BOUNDARY_CHAR {
        let char_offset = offset - 1;
        return (substring(text, char_offset, char_offset + 1), char_offset, char_offset + 1);
    }

    // WORD_START or WORD_END boundary
    if boundary_type == BOUNDARY_WORD_START || boundary_type == BOUNDARY_WORD_END {
        let mut end = offset;
        let mut start = StringUtils::previous_word(text, end);

        if start >= end {
            start = StringUtils::previous_word(text, start);
        }

        if start < 0 || start >= length(text) || !StringUtils::is_start_of_word(text, start) {
            return empty();
        }

        end = StringUtils::next_word(text, start).min(end);
        if end <= start {
            return empty();
        }

        return (substring(text, start, end), start, end);
    }

    // SENTENCE/LINE/PARAGRAPH - return all text before offset
    (substring(text, 0, offset), 0, offset)
}

pub(crate) fn get_text_after_offset(text: &[u16], offset: i32, boundary_type: u32) -> TextSegment {
    let len = length(text);
    let at_end = || (String::new(), len, len);
    if offset >= len - 1 || text.is_empty() {
        return at_end();
    }

    // CHAR boundary
    if boundary_type == BOUNDARY_CHAR {
        let char_offset = offset.saturating_add(1);
        if char_offset >= len || char_offset < 0 {
            return at_end();
        }
        return (substring(text, char_offset, char_offset + 1), char_offset, char_offset + 1);
    }

    // WORD_START or WORD_END boundary
    if boundary_type == BOUNDARY_WORD_START || boundary_type == BOUNDARY_WORD_END {
        // An offset before the text starts the search at the text: the
        // reference indexes the string with it.
        let mut start = offset.saturating_add(1).max(0);
        while start < len && StringUtils::is_end_of_word(text, start) && !StringUtils::is_start_of_word(text, start) {
            start += 1;
        }

        if start >= len {
            return at_end();
        }

        let end = StringUtils::next_word(text, start).min(len);
        if end <= start {
            return at_end();
        }

        return (substring(text, start, end), start, end);
    }

    // SENTENCE/LINE/PARAGRAPH - return all text after offset
    let after_offset = offset.saturating_add(1).max(0);
    (substring(text, after_offset, len), after_offset, len)
}

/// The code unit at an offset, or `0xFFFFFFFF` (as a signed number) for
/// an offset outside the text.
pub(crate) fn get_character_at_offset(text: &[u16], offset: i32) -> i32 {
    match usize::try_from(offset).ok().and_then(|offset| text.get(offset)) {
        Some(unit) => i32::from(*unit),
        None => -1,
    }
}

pub(crate) struct AtSpiTextHandler {
    node: Weak<AtSpiNode>,
}

impl AtSpiTextHandler {
    pub(crate) fn new(node: Weak<AtSpiNode>) -> Self {
        Self { node }
    }

    fn text_of(node: &AtSpiNode) -> Vec<u16> {
        node.peer()
            .get_provider::<dyn IValueProvider>()
            .and_then(|provider| provider.value())
            .unwrap_or_default()
            .encode_utf16()
            .collect()
    }
}

impl DBusInterface for AtSpiTextHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &TEXT
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let node = node_of(&self.node)?;
        let text = Self::text_of(&node);
        match member {
            "GetStringAtOffset" | "GetTextAtOffset" => {
                let (offset, granularity) = args::<(i32, u32)>(body)?;
                reply(get_string_at_offset(&text, offset, granularity))
            }
            "GetText" => {
                let (start_offset, end_offset) = args::<(i32, i32)>(body)?;
                reply((get_text(&text, start_offset, end_offset),))
            }
            "SetCaretOffset" => {
                args::<i32>(body)?;
                reply((false,))
            }
            "GetTextBeforeOffset" => {
                let (offset, boundary_type) = args::<(i32, u32)>(body)?;
                reply(get_text_before_offset(&text, offset, boundary_type))
            }
            "GetTextAfterOffset" => {
                let (offset, boundary_type) = args::<(i32, u32)>(body)?;
                reply(get_text_after_offset(&text, offset, boundary_type))
            }
            "GetCharacterAtOffset" => reply((get_character_at_offset(&text, args::<i32>(body)?),)),
            "GetAttributeValue" => {
                args::<(i32, String)>(body)?;
                reply((String::new(),))
            }
            "GetAttributes" => {
                args::<i32>(body)?;
                reply((AtSpiAttributeSet::new(), 0i32, length(&text)))
            }
            "GetAttributeRun" => {
                args::<(i32, bool)>(body)?;
                reply((AtSpiAttributeSet::new(), 0i32, length(&text)))
            }
            "GetDefaultAttributes" | "GetDefaultAttributeSet" => reply((AtSpiAttributeSet::new(),)),
            "GetCharacterExtents" => {
                args::<(i32, u32)>(body)?;
                reply((0i32, 0i32, 0i32, 0i32))
            }
            "GetOffsetAtPoint" => {
                args::<(i32, i32, u32)>(body)?;
                reply((-1i32,))
            }
            "GetNSelections" => reply((0i32,)),
            "GetSelection" => {
                args::<i32>(body)?;
                reply((0i32, 0i32))
            }
            "AddSelection" => {
                args::<(i32, i32)>(body)?;
                reply((false,))
            }
            "RemoveSelection" => {
                args::<i32>(body)?;
                reply((false,))
            }
            "SetSelection" => {
                args::<(i32, i32, i32)>(body)?;
                reply((false,))
            }
            "GetRangeExtents" => {
                args::<(i32, i32, u32)>(body)?;
                reply((0i32, 0i32, 0i32, 0i32))
            }
            "GetBoundedRanges" => {
                args::<(i32, i32, i32, i32, u32, u32, u32)>(body)?;
                reply((Vec::<AtSpiTextRange>::new(),))
            }
            "ScrollSubstringTo" => {
                args::<(i32, i32, u32)>(body)?;
                reply((false,))
            }
            "ScrollSubstringToPoint" => {
                args::<(i32, i32, u32, i32, i32)>(body)?;
                reply((false,))
            }
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        let node = self.node.upgrade()?;
        Some(match name {
            "version" => Value::from(TEXT_VERSION),
            "CharacterCount" => Value::from(length(&Self::text_of(&node))),
            "CaretOffset" => Value::from(0i32),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn segment(text: &str, start: i32, end: i32) -> TextSegment {
        (text.to_string(), start, end)
    }

    #[test]
    fn a_range_of_the_text() {
        let text = units("Hello world");
        assert_eq!(get_text(&text, 0, -1), "Hello world");
        assert_eq!(get_text(&text, 6, 11), "world");
        assert_eq!(get_text(&text, 6, 99), "world");
        assert_eq!(get_text(&text, -5, 5), "Hello");
        assert_eq!(get_text(&text, 5, 5), "");
        assert_eq!(get_text(&text, 7, 3), "");
        assert_eq!(get_text(&[], 0, -1), "");
    }

    #[test]
    fn the_text_at_an_offset() {
        let text = units("Hello world");
        assert_eq!(get_string_at_offset(&text, 1, GRANULARITY_CHAR), segment("e", 1, 2));
        // An offset outside the text is the nearest character.
        assert_eq!(get_string_at_offset(&text, 99, GRANULARITY_CHAR), segment("d", 10, 11));
        assert_eq!(get_string_at_offset(&text, -3, GRANULARITY_CHAR), segment("H", 0, 1));
        assert_eq!(get_string_at_offset(&text, 7, GRANULARITY_WORD), segment("world", 6, 11));
        // A word ends where its letters end.
        assert_eq!(get_string_at_offset(&text, 0, GRANULARITY_WORD), segment("Hello", 0, 5));
        // Sentences, lines and paragraphs are the whole text.
        assert_eq!(get_string_at_offset(&text, 3, 2), segment("Hello world", 0, 11));
        assert_eq!(get_string_at_offset(&text, 3, 4), segment("Hello world", 0, 11));
        assert_eq!(get_string_at_offset(&[], 0, GRANULARITY_CHAR), segment("", 0, 0));
    }

    #[test]
    fn the_text_before_an_offset() {
        let text = units("Hello world");
        assert_eq!(get_text_before_offset(&text, 0, BOUNDARY_CHAR), segment("", 0, 0));
        assert_eq!(get_text_before_offset(&text, 1, BOUNDARY_CHAR), segment("H", 0, 1));
        assert_eq!(get_text_before_offset(&text, 99, BOUNDARY_CHAR), segment("d", 10, 11));
        assert_eq!(get_text_before_offset(&text, 8, BOUNDARY_WORD_START), segment("wo", 6, 8));
        assert_eq!(get_text_before_offset(&text, 6, BOUNDARY_WORD_END), segment("Hello", 0, 5));
        assert_eq!(get_text_before_offset(&text, 5, 5), segment("Hello", 0, 5));
    }

    #[test]
    fn the_text_after_an_offset() {
        let text = units("Hello world");
        assert_eq!(get_text_after_offset(&text, 0, BOUNDARY_CHAR), segment("e", 1, 2));
        assert_eq!(get_text_after_offset(&text, 10, BOUNDARY_CHAR), segment("", 11, 11));
        assert_eq!(get_text_after_offset(&text, 0, BOUNDARY_WORD_START), segment("ello", 1, 5));
        assert_eq!(get_text_after_offset(&text, 4, BOUNDARY_WORD_START), segment("world", 6, 11));
        assert_eq!(get_text_after_offset(&text, 4, 6), segment(" world", 5, 11));
        assert_eq!(get_text_after_offset(&[], 0, BOUNDARY_CHAR), segment("", 0, 0));
    }

    #[test]
    fn the_character_at_an_offset_is_a_code_unit() {
        let text = units("a\u{e9}\u{1f600}");
        assert_eq!(text.len(), 4);
        assert_eq!(get_character_at_offset(&text, 0), 0x61);
        assert_eq!(get_character_at_offset(&text, 1), 0xe9);
        assert_eq!(get_character_at_offset(&text, 2), 0xd83d);
        assert_eq!(get_character_at_offset(&text, 4), -1);
        assert_eq!(get_character_at_offset(&text, -1), -1);
    }
}
