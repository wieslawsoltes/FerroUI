//! Forward-only character reader used by the hand-written grammars (binding
//! expressions, selectors, property paths, container queries).
//!
//! The reader works on Unicode scalar values (`char`) over a UTF-8 `&str`.
//!
//! Differences from the upstream reader, which works on UTF-16 code units:
//!
//! * [`CharacterReader::position`] and the `count` arguments are measured in
//!   `char`s. For text in the Basic Multilingual Plane this is identical to
//!   upstream; a supplementary-plane character counts as one instead of two.
//! * Reading at the end of the input never fails: [`CharacterReader::peek`] and
//!   [`CharacterReader::take`] return `None` and the `take_if*` methods return
//!   `false`, where upstream raises an index-out-of-range error. Grammars turn
//!   the resulting state into a parse error.
//!
//! The module also hosts the character classification helpers the grammars
//! share ([`is_white_space`], [`is_letter`], [`is_digit`]).

/// Returns `true` when `c` is in the Basic Multilingual Plane.
///
/// Upstream classifies UTF-16 code units, so a supplementary-plane character
/// (a surrogate pair there) is never a letter, digit or mark.
#[inline]
fn is_bmp(c: char) -> bool {
    (c as u32) < 0x1_0000
}

/// White space as defined by the upstream runtime: the Unicode `White_Space`
/// property plus the information separators U+001C..U+001F.
#[inline]
pub fn is_white_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1C}'..='\u{1F}')
}

/// Letter test (Unicode general categories Lu, Ll, Lt, Lm, Lo).
///
/// The standard library exposes the `Alphabetic` property rather than general
/// categories. Letter numbers (Nl) and the circled Latin letters (So) are
/// removed from it here; the remaining deviation is that combining marks
/// carrying the `Other_Alphabetic` property (mostly dependent vowel signs)
/// are reported as letters.
#[inline]
pub fn is_letter(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_alphabetic();
    }
    is_bmp(c) && c.is_alphabetic() && !c.is_numeric() && !matches!(c, '\u{24B6}'..='\u{24E9}')
}

/// Decimal digit test (Unicode general category Nd).
///
/// Deviation: the standard library cannot tell Nd from No, so non-ASCII
/// "other number" characters (superscripts, fractions, circled numbers) are
/// reported as digits too.
#[inline]
pub fn is_digit(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_digit();
    }
    is_bmp(c) && c.is_numeric() && !c.is_alphabetic()
}

/// Connector punctuation (Unicode general category Pc), exact for the BMP.
#[inline]
fn is_connector_punctuation(c: char) -> bool {
    matches!(
        c,
        '_' | '\u{203F}'
            | '\u{2040}'
            | '\u{2054}'
            | '\u{FE33}'
            | '\u{FE34}'
            | '\u{FE4D}'..='\u{FE4F}'
            | '\u{FF3F}'
    )
}

/// Format characters (Unicode general category Cf), exact for the BMP.
#[inline]
fn is_format(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{0600}'..='\u{0605}'
            | '\u{061C}'
            | '\u{06DD}'
            | '\u{070F}'
            | '\u{0890}'..='\u{0891}'
            | '\u{08E2}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
    )
}

/// Tests the categories that may continue (but not start) an identifier:
/// non-spacing mark, spacing combining mark, connector punctuation, format
/// and decimal digit.
///
/// Deviation: of the marks (Mn, Mc) only the combining diacritical marks block
/// U+0300..U+036F is recognised here; marks with the `Other_Alphabetic`
/// property are accepted through [`is_letter`], other marks are not accepted.
#[inline]
pub(crate) fn is_identifier_part_category(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_digit() || c == '_';
    }
    matches!(c, '\u{0300}'..='\u{036F}')
        || is_connector_punctuation(c)
        || is_format(c)
        || is_digit(c)
}

/// A forward-only reader over a string.
#[derive(Clone, Debug)]
pub struct CharacterReader<'a> {
    s: &'a str,
    position: i32,
}

impl<'a> CharacterReader<'a> {
    /// Creates a reader positioned at the start of `s`.
    #[inline]
    pub fn new(s: &'a str) -> Self {
        Self { s, position: 0 }
    }

    /// Gets a value indicating whether the whole input has been consumed.
    #[inline]
    pub fn end(&self) -> bool {
        self.s.is_empty()
    }

    /// Gets the next character without consuming it, or `None` at the end.
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.s.chars().next()
    }

    /// Gets the number of characters consumed so far.
    ///
    /// As upstream, [`Self::skip`] does not advance the position.
    #[inline]
    pub fn position(&self) -> i32 {
        self.position
    }

    /// Consumes and returns the next character, or returns `None` at the end.
    #[inline]
    pub fn take(&mut self) -> Option<char> {
        let taken = self.peek()?;
        self.position += 1;
        self.s = &self.s[taken.len_utf8()..];
        Some(taken)
    }

    /// Consumes leading white space.
    pub fn skip_whitespace(&mut self) {
        let len = self.peek_whitespace().len();
        self.advance(len);
    }

    /// Consumes the next character if it equals `c`.
    #[inline]
    pub fn take_if(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.take();
            true
        } else {
            false
        }
    }

    /// Consumes `s` if the remaining input starts with it.
    pub fn take_if_str(&mut self, s: &str) -> bool {
        if self.s.starts_with(s) {
            self.advance(s.len());
            true
        } else {
            false
        }
    }

    /// Consumes the next character if it satisfies `condition`.
    #[inline]
    pub fn take_if_fn(&mut self, condition: impl Fn(char) -> bool) -> bool {
        match self.peek() {
            Some(c) if condition(c) => {
                self.take();
                true
            }
            _ => false,
        }
    }

    /// Consumes and returns everything up to (not including) the first `c`,
    /// or the rest of the input when `c` does not occur.
    pub fn take_until(&mut self, c: char) -> &'a str {
        let len = self.s.find(c).unwrap_or(self.s.len());
        self.advance(len)
    }

    /// Consumes and returns the longest prefix whose characters all satisfy
    /// `condition`.
    pub fn take_while(&mut self, condition: impl Fn(char) -> bool) -> &'a str {
        let len = self
            .s
            .char_indices()
            .find(|&(_, c)| !condition(c))
            .map_or(self.s.len(), |(i, _)| i);
        self.advance(len)
    }

    /// Returns the next `count` characters without consuming them, or an empty
    /// string when fewer than `count` characters remain.
    pub fn try_peek(&self, count: usize) -> &'a str {
        match self.byte_offset(count) {
            Some(len) => &self.s[..len],
            None => "",
        }
    }

    /// Returns the leading white space without consuming it.
    pub fn peek_whitespace(&self) -> &'a str {
        let trimmed = self.s.trim_start_matches(is_white_space);
        &self.s[..self.s.len() - trimmed.len()]
    }

    /// Consumes `count` characters without advancing [`Self::position`]
    /// (as upstream).
    ///
    /// # Panics
    ///
    /// Panics when fewer than `count` characters remain; callers skip only
    /// what they have peeked.
    pub fn skip(&mut self, count: usize) {
        match self.byte_offset(count) {
            Some(len) => self.s = &self.s[len..],
            None => panic!("CharacterReader::skip: count is past the end of the input"),
        }
    }

    /// Byte offset of the character with index `count`, `None` when the
    /// remaining input is shorter than `count` characters.
    fn byte_offset(&self, count: usize) -> Option<usize> {
        if count == 0 {
            return Some(0);
        }
        self.s
            .char_indices()
            .nth(count - 1)
            .map(|(i, c)| i + c.len_utf8())
    }

    /// Consumes `len` bytes (a char boundary) and advances the position by the
    /// number of characters consumed.
    fn advance(&mut self, len: usize) -> &'a str {
        let (taken, rest) = self.s.split_at(len);
        self.position += taken.chars().count() as i32;
        self.s = rest;
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_take_and_position() {
        let mut r = CharacterReader::new("aé𝒜");
        assert!(!r.end());
        assert_eq!(r.peek(), Some('a'));
        assert_eq!(r.take(), Some('a'));
        assert_eq!(r.position(), 1);
        assert_eq!(r.take(), Some('é'));
        assert_eq!(r.take(), Some('𝒜'));
        assert_eq!(r.position(), 3);
        assert!(r.end());
        assert_eq!(r.peek(), None);
        assert_eq!(r.take(), None);
        assert_eq!(r.position(), 3);
    }

    #[test]
    fn take_if_variants() {
        let mut r = CharacterReader::new("?.ab");
        assert!(!r.take_if('.'));
        assert!(!r.take_if_str("?!"));
        assert!(r.take_if_str("?."));
        assert_eq!(r.position(), 2);
        assert!(r.take_if('a'));
        assert!(!r.take_if_fn(|c| c.is_ascii_digit()));
        assert!(r.take_if_fn(|c| c == 'b'));
        assert!(r.end());
        assert!(!r.take_if('a'));
        assert!(!r.take_if_fn(|_| true));
        assert!(!r.take_if_str("a"));
        assert!(r.take_if_str(""));
    }

    #[test]
    fn take_until_and_take_while() {
        let mut r = CharacterReader::new("żółw]rest");
        assert_eq!(r.take_until(']'), "żółw");
        assert_eq!(r.position(), 4);
        assert_eq!(r.take(), Some(']'));
        assert_eq!(r.take_while(|c| c != 's'), "re");
        assert_eq!(r.take_until('#'), "st");
        assert!(r.end());
        assert_eq!(r.take_until('#'), "");
        assert_eq!(r.take_while(|_| true), "");
        assert_eq!(r.position(), 9);
    }

    #[test]
    fn whitespace() {
        let mut r = CharacterReader::new(" \t\u{00A0}\u{1F}x ");
        assert_eq!(r.peek_whitespace(), " \t\u{00A0}\u{1F}");
        assert_eq!(r.position(), 0);
        r.skip_whitespace();
        assert_eq!(r.position(), 4);
        assert_eq!(r.peek(), Some('x'));
        assert_eq!(r.peek_whitespace(), "");
        r.skip_whitespace();
        assert_eq!(r.position(), 4);
    }

    #[test]
    fn try_peek_and_skip() {
        let mut r = CharacterReader::new("aéb");
        assert_eq!(r.try_peek(0), "");
        assert_eq!(r.try_peek(2), "aé");
        assert_eq!(r.try_peek(3), "aéb");
        assert_eq!(r.try_peek(4), "");
        r.skip(2);
        assert_eq!(r.peek(), Some('b'));
        // Upstream quirk: skipping does not move the position.
        assert_eq!(r.position(), 0);
    }

    #[test]
    #[should_panic]
    fn skip_past_end_panics() {
        CharacterReader::new("a").skip(2);
    }

    #[test]
    fn classification() {
        assert!(is_letter('a') && is_letter('Z') && is_letter('ż') && is_letter('λ') && is_letter('中'));
        assert!(!is_letter('_') && !is_letter('1') && !is_letter('Ⅷ') && !is_letter('Ⓐ'));
        assert!(!is_letter('𝒜'));
        assert!(is_digit('7') && is_digit('٣') && !is_digit('a') && !is_digit('Ⅷ'));
        assert!(is_white_space(' ') && is_white_space('\u{2003}') && !is_white_space('x'));
        assert!(is_identifier_part_category('\u{0301}'));
        assert!(is_identifier_part_category('\u{203F}'));
        assert!(is_identifier_part_category('\u{200D}'));
        assert!(!is_identifier_part_category('-') && !is_identifier_part_category('a'));
    }
}
