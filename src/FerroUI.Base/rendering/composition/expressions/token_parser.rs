//! Helper for the composition expression parser.

/// Whether a character is white space as the reference runtime defines it
/// (the Unicode `White_Space` set plus the information separators
/// U+001C..U+001F).
#[inline]
fn is_white_space(ch: char) -> bool {
    ch.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&ch)
}

#[inline]
fn is_alpha_numeric(ch: u8) -> bool {
    ch.is_ascii_alphanumeric()
}

/// Culture-invariant lower-casing of a single character (a character whose
/// lower-case form is not a single character is left unchanged).
#[inline]
fn to_lower_invariant(ch: char) -> char {
    if ch.is_ascii() {
        return ch.to_ascii_lowercase();
    }
    let mut lower = ch.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(single), None) => single,
        _ => ch,
    }
}

/// Helper for the composition expression parser: a cursor over the text
/// that consumes tokens from its start.
///
/// [`TokenParser::position`] and [`TokenParser::length`] count UTF-16 code
/// units, the unit of the reference implementation.
#[derive(Clone, Copy, Debug)]
pub struct TokenParser<'a> {
    s: &'a str,
    position: i32,
}

impl<'a> TokenParser<'a> {
    /// Creates a parser over the given text.
    pub fn new(s: &'a str) -> Self {
        Self { s, position: 0 }
    }

    /// The number of UTF-16 code units consumed so far.
    pub fn position(&self) -> i32 {
        self.position
    }

    /// Consumes `bytes` bytes that are `units` UTF-16 code units long.
    #[inline]
    fn advance_raw(&mut self, bytes: usize, units: usize) {
        self.s = &self.s[bytes..];
        self.position += units as i32;
    }

    #[inline]
    fn first_byte(&self) -> Option<u8> {
        self.s.as_bytes().first().copied()
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(ch) = self.s.chars().next() {
            if is_white_space(ch) {
                self.advance_raw(ch.len_utf8(), ch.len_utf16());
            } else {
                return;
            }
        }
    }

    pub fn next_is_whitespace(&self) -> bool {
        self.s.chars().next().is_some_and(is_white_space)
    }

    /// Consumes the given character if it is next (after white space).
    pub fn try_consume(&mut self, c: char) -> bool {
        self.skip_whitespace();
        if !self.s.starts_with(c) {
            return false;
        }

        self.advance_raw(c.len_utf8(), c.len_utf16());
        true
    }

    /// Consumes the given text if it is next (after white space).
    pub fn try_consume_str(&mut self, s: &str) -> bool {
        self.skip_whitespace();
        if !self.s.starts_with(s) {
            return false;
        }

        self.advance_raw(s.len(), s.encode_utf16().count());
        true
    }

    /// Consumes whichever of the given characters is next (after white space).
    pub fn try_consume_any(&mut self, chars: &str) -> Option<char> {
        self.skip_whitespace();
        let first = self.s.chars().next()?;

        for c in chars.chars() {
            if c == first {
                self.advance_raw(c.len_utf8(), c.len_utf16());
                return Some(c);
            }
        }

        None
    }

    /// Consumes the given keyword if it is next (after white space) and is
    /// not followed by a letter or a digit.
    pub fn try_parse_keyword(&mut self, keyword: &str) -> bool {
        self.skip_whitespace();
        if !self.s.starts_with(keyword) {
            return false;
        }

        if self.s.as_bytes().get(keyword.len()).copied().is_some_and(is_alpha_numeric) {
            return false;
        }

        self.advance_raw(keyword.len(), keyword.encode_utf16().count());
        true
    }

    /// Like [`TokenParser::try_parse_keyword`], ignoring the case of the
    /// text; the keyword must be given in lower case.
    pub fn try_parse_keyword_lower_case(&mut self, keyword_in_lower_case: &str) -> bool {
        self.skip_whitespace();
        let mut bytes = 0;
        let mut units = 0;
        let mut text = self.s.chars();
        for expected in keyword_in_lower_case.chars() {
            match text.next() {
                Some(ch) if to_lower_invariant(ch) == expected => {
                    bytes += ch.len_utf8();
                    units += ch.len_utf16();
                }
                _ => return false,
            }
        }

        if self.s.as_bytes().get(bytes).copied().is_some_and(is_alpha_numeric) {
            return false;
        }

        self.advance_raw(bytes, units);
        true
    }

    /// Consumes `c` UTF-16 code units.
    pub fn advance(&mut self, c: i32) {
        let mut bytes = 0;
        let mut units = 0;
        for ch in self.s.chars() {
            if units >= c.max(0) as usize {
                break;
            }
            bytes += ch.len_utf8();
            units += ch.len_utf16();
        }
        self.advance_raw(bytes, units);
    }

    /// The number of UTF-16 code units left.
    pub fn length(&self) -> i32 {
        if self.s.is_ascii() {
            self.s.len() as i32
        } else {
            self.s.encode_utf16().count() as i32
        }
    }

    /// Consumes an identifier: an ASCII letter followed by ASCII letters,
    /// digits and any of the extra valid characters.
    pub fn try_parse_identifier_with(&mut self, extra_valid_chars: &str) -> Option<&'a str> {
        self.skip_whitespace();
        let first = self.first_byte()?;
        if !first.is_ascii_alphabetic() {
            return None;
        }
        let mut len = 1;
        for ch in self.s[1..].chars() {
            if (ch.is_ascii() && is_alpha_numeric(ch as u8)) || extra_valid_chars.contains(ch) {
                len += ch.len_utf8();
            } else {
                break;
            }
        }

        let res = &self.s[..len];
        self.advance_raw(len, res.encode_utf16().count());
        Some(res)
    }

    /// Consumes an identifier: an ASCII letter followed by ASCII letters and digits.
    pub fn try_parse_identifier(&mut self) -> Option<&'a str> {
        self.skip_whitespace();
        let first = self.first_byte()?;
        if !first.is_ascii_alphabetic() {
            return None;
        }
        let bytes = self.s.as_bytes();
        let mut len = 1;
        while len < bytes.len() && is_alpha_numeric(bytes[len]) {
            len += 1;
        }

        let res = &self.s[..len];
        self.advance_raw(len, len);
        Some(res)
    }

    /// Consumes a function name (an identifier that may contain dots)
    /// together with the opening parenthesis that follows it. Nothing is
    /// consumed when the name is not followed by a parenthesis.
    pub fn try_parse_call(&mut self) -> Option<&'a str> {
        self.skip_whitespace();
        let first = self.first_byte()?;
        if !first.is_ascii_alphabetic() {
            return None;
        }
        let bytes = self.s.as_bytes();
        let mut len = 1;
        while len < bytes.len() && (is_alpha_numeric(bytes[len]) || bytes[len] == b'.') {
            len += 1;
        }

        let res = &self.s[..len];

        // Find '('
        let mut consumed_bytes = len;
        let mut consumed_units = len;
        for ch in self.s[len..].chars() {
            consumed_bytes += ch.len_utf8();
            consumed_units += ch.len_utf16();
            if is_white_space(ch) {
                continue;
            }
            if ch == '(' {
                self.advance_raw(consumed_bytes, consumed_units);
                return Some(res);
            }

            return None;
        }

        None
    }

    /// The length of the number at the start of the text: digits, at most
    /// one decimal point, and a leading minus sign. `None` when a minus sign
    /// follows the number and `fail_on_inner_minus` is set.
    ///
    /// The scanned text is handed to the standard float parser, which agrees
    /// with the reference runtime's invariant number parsing on every text
    /// this scanner can produce (at least one digit is required; a value out
    /// of range is infinite).
    fn scan_number(&self, fail_on_inner_minus: bool) -> Option<usize> {
        let mut len = 0;
        let mut dot_count = 0;
        for (c, &ch) in self.s.as_bytes().iter().enumerate() {
            if ch.is_ascii_digit() {
                len = c + 1;
            } else if ch == b'.' && dot_count == 0 {
                len = c + 1;
                dot_count += 1;
            } else if ch == b'-' {
                if len != 0 {
                    if fail_on_inner_minus {
                        return None;
                    }
                    break;
                }
                len = c + 1;
            } else {
                break;
            }
        }
        Some(len)
    }

    /// Consumes a single-precision number.
    pub fn try_parse_float(&mut self) -> Option<f32> {
        self.skip_whitespace();
        if self.s.is_empty() {
            return None;
        }

        let len = self.scan_number(false)?;
        let res = self.s[..len].parse::<f32>().ok()?;
        self.advance_raw(len, len);
        Some(res)
    }

    /// Consumes a double-precision number.
    pub fn try_parse_double(&mut self) -> Option<f64> {
        self.skip_whitespace();
        if self.s.is_empty() {
            return None;
        }

        let len = self.scan_number(true)?;
        let res = self.s[..len].parse::<f64>().ok()?;
        self.advance_raw(len, len);
        Some(res)
    }

    /// Whether only white space is left (which is consumed).
    pub fn is_eof_with_whitespace(&mut self) -> bool {
        self.skip_whitespace();
        self.s.is_empty()
    }

    /// The text that has not been consumed yet.
    pub fn as_str(&self) -> &'a str {
        self.s
    }
}

impl std::fmt::Display for TokenParser<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumes_characters_and_strings_after_white_space() {
        let mut p = TokenParser::new("  ( >= x");
        assert!(!p.try_consume(')'));
        assert_eq!(p.position(), 2);
        assert!(p.try_consume('('));
        assert!(!p.try_consume_str(">>"));
        assert!(p.try_consume_str(">="));
        assert_eq!(p.try_consume_any("+-"), None);
        assert_eq!(p.try_consume_any("yx"), Some('x'));
        assert_eq!(p.length(), 0);
        assert_eq!(p.position(), 8);
        assert!(!p.try_consume('x'));
        assert_eq!(p.try_consume_any("x"), None);
        assert!(p.is_eof_with_whitespace());
    }

    #[test]
    fn white_space_follows_the_reference_definition() {
        let mut p = TokenParser::new("\u{1c}\u{a0}\u{2003}\t\r\nx");
        assert!(p.next_is_whitespace());
        p.skip_whitespace();
        assert_eq!(p.position(), 6);
        assert_eq!(p.to_string(), "x");
        assert!(!p.next_is_whitespace());
    }

    #[test]
    fn parses_keywords() {
        let mut p = TokenParser::new(" This.TARGET.Offset");
        assert!(!p.try_parse_keyword("this.target"));
        assert!(p.try_parse_keyword_lower_case("this.target"));
        assert_eq!(p.as_str(), ".Offset");
        assert_eq!(p.position(), 12);

        let mut p = TokenParser::new("pixel");
        assert!(!p.try_parse_keyword_lower_case("pi"));
        assert!(!p.try_parse_keyword("pi"));
        assert_eq!(p.position(), 0);

        let mut p = TokenParser::new("pi");
        assert!(p.try_parse_keyword("pi"));
        assert_eq!(p.length(), 0);

        let mut p = TokenParser::new("p");
        assert!(!p.try_parse_keyword("pi"));
        assert!(!p.try_parse_keyword_lower_case("pi"));

        let mut p = TokenParser::new("PI*2");
        assert!(p.try_parse_keyword_lower_case("pi"));
        assert_eq!(p.as_str(), "*2");
    }

    #[test]
    fn parses_identifiers() {
        let mut p = TokenParser::new(" abc1_d");
        assert_eq!(p.try_parse_identifier(), Some("abc1"));
        assert_eq!(p.try_parse_identifier(), None);
        assert_eq!(p.as_str(), "_d");

        let mut p = TokenParser::new("abc1_d.e f");
        assert_eq!(p.try_parse_identifier_with("_."), Some("abc1_d.e"));
        assert_eq!(p.position(), 8);

        assert_eq!(TokenParser::new("1abc").try_parse_identifier(), None);
        assert_eq!(TokenParser::new("1abc").try_parse_identifier_with("1"), None);
        assert_eq!(TokenParser::new("").try_parse_identifier(), None);
    }

    #[test]
    fn parses_calls() {
        let mut p = TokenParser::new(" Matrix3x2.CreateScale  (1)");
        assert_eq!(p.try_parse_call(), Some("Matrix3x2.CreateScale"));
        assert_eq!(p.as_str(), "1)");
        assert_eq!(p.position(), 25);

        // Nothing but leading white space is consumed when there is no parenthesis.
        let mut p = TokenParser::new(" abc + (1)");
        assert_eq!(p.try_parse_call(), None);
        assert_eq!(p.as_str(), "abc + (1)");
        assert_eq!(TokenParser::new("abc").try_parse_call(), None);
        assert_eq!(TokenParser::new("(1)").try_parse_call(), None);
    }

    #[test]
    fn parses_numbers() {
        let mut p = TokenParser::new(" 4.0-0.5)");
        assert_eq!(p.try_parse_float(), Some(4.0));
        assert_eq!(p.as_str(), "-0.5)");
        assert_eq!(p.try_parse_float(), Some(-0.5));
        assert_eq!(p.try_parse_float(), None);
        assert_eq!(p.as_str(), ")");

        assert_eq!(TokenParser::new("1.").try_parse_float(), Some(1.0));
        assert_eq!(TokenParser::new(".5").try_parse_float(), Some(0.5));
        assert_eq!(TokenParser::new("-.5").try_parse_float(), Some(-0.5));
        assert_eq!(TokenParser::new("0.1").try_parse_float(), Some(0.1));
        assert_eq!(TokenParser::new("-").try_parse_float(), None);
        assert_eq!(TokenParser::new(".").try_parse_float(), None);
        assert_eq!(TokenParser::new("-.").try_parse_float(), None);
        assert_eq!(TokenParser::new("").try_parse_float(), None);
        assert_eq!(TokenParser::new("x").try_parse_float(), None);
        assert_eq!(
            TokenParser::new("99999999999999999999999999999999999999999999").try_parse_float(),
            Some(f32::INFINITY)
        );

        let mut p = TokenParser::new("1.2.3");
        assert_eq!(p.try_parse_float(), Some(1.2));
        assert_eq!(p.as_str(), ".3");

        assert_eq!(TokenParser::new("2.5e").try_parse_double(), Some(2.5));
        assert_eq!(TokenParser::new("-2.5").try_parse_double(), Some(-2.5));
        // Unlike the single-precision variant, a minus sign after the number fails.
        assert_eq!(TokenParser::new("4.0-0.5").try_parse_double(), None);
    }

    #[test]
    fn advance_counts_utf16_code_units() {
        let mut p = TokenParser::new("a\u{e9}\u{1f600}b");
        assert_eq!(p.length(), 5);
        p.advance(2);
        assert_eq!(p.position(), 2);
        p.advance(2);
        assert_eq!(p.position(), 4);
        assert_eq!(p.as_str(), "b");
    }
}
