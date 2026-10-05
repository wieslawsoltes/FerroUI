use ferroui_base::media::text_formatting::unicode::{Codepoint, GeneralCategory};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CharClass {
    Unknown,
    Whitespace,
    AlphaNumeric,
}

/// Word and line navigation over UTF-16 text.
///
/// Every index is a UTF-16 code unit index, and every character test looks
/// at a single code unit, as the reference does.
pub struct StringUtils;

impl StringUtils {
    pub fn is_eol(c: u16) -> bool {
        c == '\r' as u16 || c == '\n' as u16
    }

    pub fn is_start_of_word(text: &[u16], index: i32) -> bool {
        if index >= text.len() as i32 {
            return false;
        }

        let codepoint = Codepoint::new(text[index as usize] as u32);

        // A 'word' starts with an AlphaNumeric or some punctuation symbols
        // immediately preceded by lwsp.
        if index > 0 {
            let previous_codepoint = Codepoint::new(text[index as usize - 1] as u32);

            if !previous_codepoint.is_white_space() {
                return false;
            }

            if previous_codepoint.is_break_char() {
                return true;
            }
        }

        Self::is_word_start_category(codepoint.general_category())
    }

    pub fn is_end_of_word(text: &[u16], index: i32) -> bool {
        if index >= text.len() as i32 {
            return true;
        }

        let codepoint = Codepoint::new(text[index as usize] as u32);

        if !codepoint.is_white_space() {
            return false;
        }

        // A 'word' starts with an AlphaNumeric or some punctuation symbols
        // immediately preceded by lwsp.
        if index > 0 {
            if index + 1 < text.len() as i32 {
                let next_codepoint = Codepoint::new(text[index as usize + 1] as u32);

                if next_codepoint.is_break_char() {
                    return true;
                }
            } else {
                return true;
            }
        }

        !Self::is_word_start_category(codepoint.general_category())
    }

    pub fn previous_word(text: &[u16], cursor: i32) -> i32 {
        if text.is_empty() {
            return 0;
        }

        let cursor = cursor.min(text.len() as i32);
        let at = |index: i32| text[index as usize];

        let lf = Self::line_begin(text, cursor) - 1;

        let cr = if lf > 0 && at(lf) == '\n' as u16 && at(lf - 1) == '\r' as u16 { lf - 1 } else { lf };

        // If the cursor is at the beginning of the line, return the end of
        // the previous line.
        if cursor - 1 == lf {
            return if cr > 0 { cr } else { 0 };
        }

        let mut cc = Self::get_char_class(at(cursor - 1));
        let begin = lf + 1;
        let mut i = cursor;

        // Skip over the word, punctuation, or run of whitespace.
        while i > begin && Self::get_char_class(at(i - 1)) == cc {
            i -= 1;
        }

        // If the cursor was at whitespace, skip back a word too.
        if cc == CharClass::Whitespace && i > begin {
            cc = Self::get_char_class(at(i - 1));
            while i > begin && Self::get_char_class(at(i - 1)) == cc {
                i -= 1;
            }
        }

        i
    }

    pub fn next_word(text: &[u16], cursor: i32) -> i32 {
        let length = text.len() as i32;
        let at = |index: i32| text[index as usize];

        let cr = Self::line_end(text, cursor, false);

        if cursor >= length {
            return cursor;
        }

        let lf = if cr < length && at(cr) == '\r' as u16 && cr + 1 < length && at(cr + 1) == '\n' as u16 {
            cr + 1
        } else {
            cr
        };

        // If the cursor is at the end of the line, return the starting
        // offset of the next line.
        if cursor == cr || cursor == lf {
            if lf < length {
                return lf + 1;
            }

            return cursor;
        }

        let mut i = cursor;

        // Skip any whitespace after the word/punct.
        while i < cr && Self::is_white_space(at(i)) {
            i += 1;
        }

        if i >= cr {
            return i;
        }

        let cc = Self::get_char_class(at(i));

        // Skip over the word, punctuation, or run of whitespace.
        while i < cr && Self::get_char_class(at(i)) == cc {
            i += 1;
        }

        i
    }

    /// The categories a word can start with.
    fn is_word_start_category(category: GeneralCategory) -> bool {
        matches!(
            category,
            GeneralCategory::LowercaseLetter
                | GeneralCategory::TitlecaseLetter
                | GeneralCategory::UppercaseLetter
                | GeneralCategory::DecimalNumber
                | GeneralCategory::LetterNumber
                | GeneralCategory::OtherNumber
                | GeneralCategory::DashPunctuation
                | GeneralCategory::InitialPunctuation
                | GeneralCategory::OpenPunctuation
                | GeneralCategory::CurrencySymbol
                | GeneralCategory::MathSymbol
        )
    }

    fn get_char_class(c: u16) -> CharClass {
        if Self::is_white_space(c) {
            CharClass::Whitespace
        } else if Self::is_letter_or_digit(c) {
            CharClass::AlphaNumeric
        } else {
            CharClass::Unknown
        }
    }

    /// The white space test of the platform the reference runs on, for one
    /// UTF-16 code unit: the separator categories and the control characters
    /// U+0009..U+000D and U+0085.
    fn is_white_space(c: u16) -> bool {
        match c {
            0x0009..=0x000D | 0x0020 | 0x0085 | 0x00A0 => true,
            0x0000..=0x00FF => false,
            _ => matches!(
                Codepoint::new(c as u32).general_category(),
                GeneralCategory::SpaceSeparator | GeneralCategory::LineSeparator | GeneralCategory::ParagraphSeparator
            ),
        }
    }

    /// Whether one UTF-16 code unit is a letter or a decimal digit.
    fn is_letter_or_digit(c: u16) -> bool {
        matches!(
            Codepoint::new(c as u32).general_category(),
            GeneralCategory::UppercaseLetter
                | GeneralCategory::LowercaseLetter
                | GeneralCategory::TitlecaseLetter
                | GeneralCategory::ModifierLetter
                | GeneralCategory::OtherLetter
                | GeneralCategory::DecimalNumber
        )
    }

    fn line_begin(text: &[u16], mut pos: i32) -> i32 {
        while pos > 0 && !Self::is_eol(text[pos as usize - 1]) {
            pos -= 1;
        }

        pos
    }

    fn line_end(text: &[u16], mut cursor: i32, include: bool) -> i32 {
        let length = text.len() as i32;

        while cursor < length && !Self::is_eol(text[cursor as usize]) {
            cursor += 1;
        }

        if include && cursor < length {
            if text[cursor as usize] == '\r' as u16 && text.get(cursor as usize + 1) == Some(&('\n' as u16)) {
                cursor += 2;
            } else {
                cursor += 1;
            }
        }

        cursor
    }
}

#[cfg(test)]
mod tests {
    use super::StringUtils;

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn is_eol_matches_carriage_return_and_line_feed() {
        assert!(StringUtils::is_eol('\r' as u16));
        assert!(StringUtils::is_eol('\n' as u16));
        assert!(!StringUtils::is_eol(' ' as u16));
        assert!(!StringUtils::is_eol(0x2028));
    }

    #[test]
    fn is_start_of_word_requires_preceding_white_space() {
        let text = utf16("First Second\nthird (x) .y");

        assert!(StringUtils::is_start_of_word(&text, 0));
        assert!(!StringUtils::is_start_of_word(&text, 1));
        assert!(!StringUtils::is_start_of_word(&text, 5)); // the space
        assert!(StringUtils::is_start_of_word(&text, 6));
        // After a break char anything starts a word.
        assert!(StringUtils::is_start_of_word(&text, 13));
        assert!(StringUtils::is_start_of_word(&text, 19)); // '('
        assert!(!StringUtils::is_start_of_word(&text, 20)); // 'x' after '('
        assert!(!StringUtils::is_start_of_word(&text, 23)); // '.' after a space
        assert!(!StringUtils::is_start_of_word(&text, text.len() as i32));
        assert!(!StringUtils::is_start_of_word(&text, text.len() as i32 + 5));
    }

    #[test]
    fn is_end_of_word_is_white_space_after_a_word() {
        let text = utf16("ab cd \nef");

        assert!(!StringUtils::is_end_of_word(&text, 0));
        assert!(!StringUtils::is_end_of_word(&text, 1));
        assert!(StringUtils::is_end_of_word(&text, 2));
        assert!(!StringUtils::is_end_of_word(&text, 3));
        assert!(StringUtils::is_end_of_word(&text, 5)); // followed by a break char
        assert!(StringUtils::is_end_of_word(&text, 6));
        assert!(StringUtils::is_end_of_word(&text, text.len() as i32));

        // White space at the very end and at the very start.
        assert!(StringUtils::is_end_of_word(&utf16("ab "), 2));
        assert!(StringUtils::is_end_of_word(&utf16(" ab"), 0));
    }

    #[test]
    fn previous_word_skips_the_word_and_the_white_space_before_the_cursor() {
        let text = utf16("one two  three");

        assert_eq!(StringUtils::previous_word(&[], 3), 0);
        assert_eq!(StringUtils::previous_word(&text, 0), 0);
        assert_eq!(StringUtils::previous_word(&text, 2), 0);
        assert_eq!(StringUtils::previous_word(&text, 3), 0);
        assert_eq!(StringUtils::previous_word(&text, 4), 0);
        assert_eq!(StringUtils::previous_word(&text, 6), 4);
        assert_eq!(StringUtils::previous_word(&text, 9), 4);
        assert_eq!(StringUtils::previous_word(&text, 100), 9);

        // Punctuation is a class of its own.
        assert_eq!(StringUtils::previous_word(&utf16("a.,b"), 3), 1);
    }

    #[test]
    fn previous_word_stops_at_line_boundaries() {
        let text = utf16("ab\r\ncd\nef");

        // At the beginning of a line: the end of the previous line.
        assert_eq!(StringUtils::previous_word(&text, 4), 2);
        assert_eq!(StringUtils::previous_word(&text, 7), 6);
        // Within a line the search does not leave it.
        assert_eq!(StringUtils::previous_word(&text, 6), 4);
        assert_eq!(StringUtils::previous_word(&text, 9), 7);
        // The first line ends at offset zero.
        assert_eq!(StringUtils::previous_word(&utf16("\nab"), 1), 0);
    }

    #[test]
    fn next_word_skips_white_space_then_one_class() {
        let text = utf16("one two  three");

        assert_eq!(StringUtils::next_word(&text, 0), 3);
        assert_eq!(StringUtils::next_word(&text, 1), 3);
        assert_eq!(StringUtils::next_word(&text, 3), 7);
        assert_eq!(StringUtils::next_word(&text, 7), 14);
        assert_eq!(StringUtils::next_word(&text, 14), 14);
        assert_eq!(StringUtils::next_word(&text, 20), 20);
        assert_eq!(StringUtils::next_word(&[], 0), 0);

        assert_eq!(StringUtils::next_word(&utf16("a.,b"), 1), 3);
        // Trailing white space runs to the end of the line.
        assert_eq!(StringUtils::next_word(&utf16("ab   "), 2), 5);
    }

    #[test]
    fn next_word_moves_to_the_next_line_at_a_line_end() {
        let text = utf16("ab\r\ncd\nef");

        assert_eq!(StringUtils::next_word(&text, 0), 2);
        assert_eq!(StringUtils::next_word(&text, 2), 4);
        assert_eq!(StringUtils::next_word(&text, 3), 4);
        assert_eq!(StringUtils::next_word(&text, 4), 6);
        assert_eq!(StringUtils::next_word(&text, 6), 7);
        assert_eq!(StringUtils::next_word(&text, 7), 9);
        assert_eq!(StringUtils::next_word(&text, 9), 9);
    }

    #[test]
    fn information_separators_are_not_white_space() {
        // U+001C..U+001F are control characters, not white space: they
        // belong to the run of punctuation they are in.
        for separator in 0x001C..=0x001F_u16 {
            let text = [u16::from(b'a'), separator, u16::from(b'.'), u16::from(b'b')];

            assert_eq!(StringUtils::next_word(&text, 1), 3);
            assert_eq!(StringUtils::previous_word(&text, 3), 1);
        }

        // The white space of the first 256 code units.
        for white_space in [0x0009_u16, 0x000A, 0x000B, 0x000C, 0x000D, 0x0020, 0x0085, 0x00A0] {
            let text = [u16::from(b'a'), white_space, white_space, u16::from(b'b')];

            if !StringUtils::is_eol(white_space) {
                assert_eq!(StringUtils::next_word(&text, 1), 4, "{white_space:#06x}");
            }
        }
    }

    #[test]
    fn surrogates_are_neither_white_space_nor_alphanumeric() {
        // "a😀b": the two halves of the pair form one run of their own.
        let text = utf16("a\u{1F600}b");

        assert_eq!(StringUtils::next_word(&text, 0), 1);
        assert_eq!(StringUtils::next_word(&text, 1), 3);
        assert_eq!(StringUtils::previous_word(&text, 3), 1);
    }
}
