use super::{DocumentError, DocumentLine, ITextSource, SimpleSegment, TextDocument};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalDirection {
    Backward,
    Forward,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretPositioningMode {
    Normal,
    WordBorder,
    WordStart,
    WordStartOrSymbol,
    WordBorderOrSymbol,
    EveryCodepoint,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterClass {
    Other,
    Whitespace,
    IdentifierPart,
    LineTerminator,
    CombiningMark,
}
pub struct TextUtilities;

impl TextUtilities {
    pub fn get_control_character_name(value: u16) -> String {
        const C0: [&str; 32] = [
            "NUL", "SOH", "STX", "ETX", "EOT", "ENQ", "ACK", "BEL", "BS", "HT", "LF", "VT", "FF",
            "CR", "SO", "SI", "DLE", "DC1", "DC2", "DC3", "DC4", "NAK", "SYN", "ETB", "CAN", "EM",
            "SUB", "ESC", "FS", "GS", "RS", "US",
        ];
        const C1: [&str; 33] = [
            "DEL", "PAD", "HOP", "BPH", "NBH", "IND", "NEL", "SSA", "ESA", "HTS", "HTJ", "VTS",
            "PLD", "PLU", "RI", "SS2", "SS3", "DCS", "PU1", "PU2", "STS", "CCH", "MW", "SPA",
            "EPA", "SOS", "SGCI", "SCI", "CSI", "ST", "OSC", "PM", "APC",
        ];
        if value < 32 {
            C0[value as usize].to_owned()
        } else if (127..=159).contains(&value) {
            C1[value as usize - 127].to_owned()
        } else {
            format!("{value:04x}")
        }
    }
    fn check_offset(source: &dyn ITextSource, offset: usize) -> Result<(), DocumentError> {
        if offset > source.text_length() {
            Err(DocumentError::InvalidOffset {
                offset,
                text_length: source.text_length(),
            })
        } else {
            Ok(())
        }
    }
    pub fn get_whitespace_after(
        source: &dyn ITextSource,
        offset: usize,
    ) -> Result<SimpleSegment, DocumentError> {
        Self::check_offset(source, offset)?;
        let mut end = offset;
        while end < source.text_length() && matches!(source.get_char_at(end)?, 9 | 32) {
            end += 1;
        }
        Ok(SimpleSegment::new(offset, end - offset))
    }
    pub fn get_whitespace_before(
        source: &dyn ITextSource,
        offset: usize,
    ) -> Result<SimpleSegment, DocumentError> {
        Self::check_offset(source, offset)?;
        let mut start = offset;
        while start > 0 && matches!(source.get_char_at(start - 1)?, 9 | 32) {
            start -= 1;
        }
        Ok(SimpleSegment::new(start, offset - start))
    }
    pub fn get_leading_whitespace(
        document: &TextDocument,
        line: DocumentLine,
    ) -> Result<SimpleSegment, DocumentError> {
        Self::get_whitespace_after(document, line.offset)
    }
    pub fn get_trailing_whitespace(
        document: &TextDocument,
        line: DocumentLine,
    ) -> Result<SimpleSegment, DocumentError> {
        let segment = Self::get_whitespace_before(document, line.end_offset())?;
        Ok(if segment.offset == line.offset {
            SimpleSegment::new(line.end_offset(), 0)
        } else {
            segment
        })
    }
    pub fn get_single_indentation_segment(
        source: &dyn ITextSource,
        offset: usize,
        indentation_size: usize,
    ) -> Result<SimpleSegment, DocumentError> {
        Self::check_offset(source, offset)?;
        if offset < source.text_length() && source.get_char_at(offset)? == 9 {
            return Ok(SimpleSegment::new(offset, 1));
        }
        let mut length = 0;
        while length < indentation_size
            && offset + length < source.text_length()
            && source.get_char_at(offset + length)? == 32
        {
            length += 1;
        }
        Ok(SimpleSegment::new(offset, length))
    }
    pub fn find_next_new_line(
        source: &dyn ITextSource,
        offset: usize,
    ) -> Result<Option<(usize, &'static str)>, DocumentError> {
        Self::check_offset(source, offset)?;
        for position in offset..source.text_length() {
            match source.get_char_at(position)? {
                10 => return Ok(Some((position, "\n"))),
                13 => {
                    return Ok(Some((
                        position,
                        if position + 1 < source.text_length()
                            && source.get_char_at(position + 1)? == 10
                        {
                            "\r\n"
                        } else {
                            "\r"
                        },
                    )))
                }
                _ => {}
            }
        }
        Ok(None)
    }
    pub fn is_new_line(value: &str) -> bool {
        matches!(value, "\r" | "\n" | "\r\n")
    }
    pub fn normalize_new_lines(input: &str, new_line: &str) -> Result<String, DocumentError> {
        if !Self::is_new_line(new_line) {
            return Err(DocumentError::InvalidNewLine);
        }
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\r' {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                result.push_str(new_line);
            } else if c == '\n' {
                result.push_str(new_line);
            } else {
                result.push(c);
            }
        }
        Ok(result)
    }
    pub fn get_new_line_from_document(
        document: &TextDocument,
        line_number: usize,
    ) -> Result<String, DocumentError> {
        let mut line = document.get_line_by_number(line_number)?;
        if line.delimiter_length == 0 {
            if line.line_number == 1 {
                return Ok(if cfg!(windows) { "\r\n" } else { "\n" }.to_owned());
            }
            line = document.get_line_by_number(line.line_number - 1)?;
        }
        document.get_text(line.end_offset(), line.delimiter_length)
    }
    pub fn get_character_class(unit: u16) -> CharacterClass {
        char::from_u32(unit as u32).map_or(CharacterClass::Other, Self::classify)
    }
    fn classify(c: char) -> CharacterClass {
        if c == '\r' || c == '\n' {
            return CharacterClass::LineTerminator;
        }
        if c == '_' {
            return CharacterClass::IdentifierPart;
        }
        if c.is_control() || c.is_whitespace() {
            return CharacterClass::Whitespace;
        }
        // General categories match the reference's UnicodeCategory checks;
        // is_alphanumeric would also include non-decimal numeric characters.
        static IDENTIFIER: OnceLock<Regex> = OnceLock::new();
        static MARK: OnceLock<Regex> = OnceLock::new();
        let mut buffer = [0; 4];
        let text = c.encode_utf8(&mut buffer);
        if IDENTIFIER
            .get_or_init(|| {
                Regex::new(r"^[\p{L}\p{Nd}]$").expect("constant Unicode category regex")
            })
            .is_match(text)
        {
            CharacterClass::IdentifierPart
        } else if MARK
            .get_or_init(|| Regex::new(r"^\p{M}$").expect("constant Unicode category regex"))
            .is_match(text)
        {
            CharacterClass::CombiningMark
        } else {
            CharacterClass::Other
        }
    }
    fn classify_pair(first: u16, second: u16) -> CharacterClass {
        if (0xd800..=0xdbff).contains(&first) && (0xdc00..=0xdfff).contains(&second) {
            char::from_u32(0x10000 + (((first as u32 - 0xd800) << 10) | (second as u32 - 0xdc00)))
                .map_or(CharacterClass::Other, Self::classify)
        } else {
            CharacterClass::Other
        }
    }
    /// Reference caret-stop semantics: combining marks and surrogate pairs are
    /// atomic, with selectable word-border modes. Visual movement additionally
    /// uses extended-grapheme boundaries and the editor's line layout.
    pub fn get_next_caret_position(
        source: &dyn ITextSource,
        mut offset: isize,
        direction: LogicalDirection,
        mode: CaretPositioningMode,
    ) -> Result<Option<usize>, DocumentError> {
        let length = source.text_length();
        let normal = matches!(
            mode,
            CaretPositioningMode::Normal | CaretPositioningMode::EveryCodepoint
        );
        if length == 0 {
            return Ok((normal
                && (offset > 0 && direction == LogicalDirection::Backward
                    || offset < 0 && direction == LogicalDirection::Forward))
                .then_some(0));
        }
        loop {
            let next = match direction {
                LogicalDirection::Backward => offset.checked_sub(1),
                LogicalDirection::Forward => offset.checked_add(1),
            };
            let Some(next) = next.filter(|&p| p >= 0 && p as usize <= length) else {
                return Ok(None);
            };
            let position = next as usize;
            if position == 0 {
                let first = source.get_char_at(0)?;
                if normal || !char::from_u32(first as u32).is_some_and(char::is_whitespace) {
                    return Ok(Some(0));
                }
            } else if position == length {
                if !matches!(
                    mode,
                    CaretPositioningMode::WordStart | CaretPositioningMode::WordStartOrSymbol
                ) {
                    let last = source.get_char_at(length - 1)?;
                    if normal || !char::from_u32(last as u32).is_some_and(char::is_whitespace) {
                        return Ok(Some(length));
                    }
                }
            } else {
                let before = source.get_char_at(position - 1)?;
                let after = source.get_char_at(position)?;
                if !((0xd800..=0xdbff).contains(&before) && (0xdc00..=0xdfff).contains(&after)) {
                    let before = if (0xdc00..=0xdfff).contains(&before) && position >= 2 {
                        Self::classify_pair(source.get_char_at(position - 2)?, before)
                    } else {
                        Self::get_character_class(before)
                    };
                    let after = if (0xd800..=0xdbff).contains(&after) && position + 1 < length {
                        Self::classify_pair(after, source.get_char_at(position + 1)?)
                    } else {
                        Self::get_character_class(after)
                    };
                    if Self::stop_between(mode, before, after) {
                        return Ok(Some(position));
                    }
                }
            }
            offset = next;
        }
    }
    fn stop_between(
        mode: CaretPositioningMode,
        before: CharacterClass,
        after: CharacterClass,
    ) -> bool {
        if mode == CaretPositioningMode::EveryCodepoint {
            return true;
        }
        if after == CharacterClass::CombiningMark {
            return false;
        }
        if mode == CaretPositioningMode::Normal {
            return true;
        }
        if before == after {
            return before == CharacterClass::Other
                && matches!(
                    mode,
                    CaretPositioningMode::WordBorderOrSymbol
                        | CaretPositioningMode::WordStartOrSymbol
                );
        }
        !(matches!(
            mode,
            CaretPositioningMode::WordStart | CaretPositioningMode::WordStartOrSymbol
        ) && matches!(
            after,
            CharacterClass::Whitespace | CharacterClass::LineTerminator
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::TextSnapshot;
    #[test]
    fn reference_whitespace_examples() {
        assert_eq!(
            TextUtilities::get_whitespace_after(&TextSnapshot::new("a \t \tb"), 2).unwrap(),
            SimpleSegment::new(2, 3)
        );
        assert_eq!(
            TextUtilities::get_whitespace_before(&TextSnapshot::new("a\t \t b"), 4).unwrap(),
            SimpleSegment::new(1, 3)
        );
        assert_eq!(
            TextUtilities::get_whitespace_before(&TextSnapshot::new("a\n b"), 3).unwrap(),
            SimpleSegment::new(2, 1)
        );
        assert_eq!(
            TextUtilities::get_whitespace_before(&TextSnapshot::new(" a"), 0).unwrap(),
            SimpleSegment::new(0, 0)
        );
    }
    #[test]
    fn newline_normalization_and_trailing_blank_line() {
        assert_eq!(
            TextUtilities::normalize_new_lines("a\r\nb\rc\n", "\n").unwrap(),
            "a\nb\nc\n"
        );
        assert!(TextUtilities::normalize_new_lines("x", "bad").is_err());
        let doc = TextDocument::from_text("a\r\n");
        assert_eq!(
            TextUtilities::get_new_line_from_document(&doc, 2).unwrap(),
            "\r\n"
        );
        assert_eq!(
            TextUtilities::find_next_new_line(&doc, 0).unwrap(),
            Some((1, "\r\n"))
        );
    }
    #[test]
    fn reference_caret_modes_cover_combining_marks_and_surrogates() {
        let source = TextSnapshot::new("a\u{301}😀 word ++ ");
        assert_eq!(
            TextUtilities::get_next_caret_position(
                &source,
                0,
                LogicalDirection::Forward,
                CaretPositioningMode::Normal
            )
            .unwrap(),
            Some(2)
        );
        assert_eq!(
            TextUtilities::get_next_caret_position(
                &source,
                0,
                LogicalDirection::Forward,
                CaretPositioningMode::EveryCodepoint
            )
            .unwrap(),
            Some(1)
        );
        assert_eq!(
            TextUtilities::get_next_caret_position(
                &source,
                2,
                LogicalDirection::Forward,
                CaretPositioningMode::Normal
            )
            .unwrap(),
            Some(4)
        );
        assert_eq!(
            TextUtilities::get_next_caret_position(
                &source,
                5,
                LogicalDirection::Forward,
                CaretPositioningMode::WordStart
            )
            .unwrap(),
            Some(10)
        );
        assert_eq!(
            TextUtilities::get_character_class(0x0301),
            CharacterClass::CombiningMark
        );
        assert_eq!(
            TextUtilities::get_character_class(0x00b2),
            CharacterClass::Other
        );
    }
}
