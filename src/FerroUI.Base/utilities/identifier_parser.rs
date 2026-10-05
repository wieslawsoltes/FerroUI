//! Identifier and number parsing on top of [`CharacterReader`].

use super::character_reader::{is_digit, is_identifier_part_category, is_letter, CharacterReader};

impl<'a> CharacterReader<'a> {
    /// Parses an identifier: a letter or `_` followed by letters, `_`, marks,
    /// connector punctuation, format characters and decimal digits. Returns an
    /// empty string (consuming nothing) when the input does not start with an
    /// identifier.
    pub fn parse_identifier(&mut self) -> &'a str {
        if self.peek().is_some_and(is_valid_identifier_start) {
            self.take_while(is_valid_identifier_char)
        } else {
            ""
        }
    }

    /// Parses an identifier that may name a nested type, e.g. "Outer+Inner".
    pub fn parse_type_identifier(&mut self) -> &'a str {
        if self.peek().is_some_and(is_valid_identifier_start) {
            self.take_while(|c| is_valid_identifier_char(c) || c == '+')
        } else {
            ""
        }
    }

    /// Parses a run of decimal digits (possibly empty).
    pub fn parse_number(&mut self) -> &'a str {
        self.take_while(is_valid_number_char)
    }
}

fn is_valid_identifier_start(c: char) -> bool {
    is_letter(c) || c == '_'
}

fn is_valid_identifier_char(c: char) -> bool {
    is_valid_identifier_start(c) || is_identifier_part_category(c)
}

fn is_valid_number_char(c: char) -> bool {
    is_digit(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_identifier() {
        let mut r = CharacterReader::new("_Foo1_b\u{0301}.Bar");
        assert_eq!(r.parse_identifier(), "_Foo1_b\u{0301}");
        assert_eq!(r.position(), 8);
        assert_eq!(r.peek(), Some('.'));
        assert_eq!(r.parse_identifier(), "");
        assert_eq!(r.position(), 8);
    }

    #[test]
    fn identifier_cannot_start_with_digit_or_symbol() {
        assert_eq!(CharacterReader::new("1Foo").parse_identifier(), "");
        assert_eq!(CharacterReader::new("%Foo").parse_identifier(), "");
        assert_eq!(CharacterReader::new("-Foo").parse_identifier(), "");
        assert_eq!(CharacterReader::new("").parse_identifier(), "");
    }

    #[test]
    fn identifier_stops_at_plus_and_dash() {
        assert_eq!(CharacterReader::new("Outer+Inner").parse_identifier(), "Outer");
        assert_eq!(CharacterReader::new("min-width").parse_identifier(), "min");
    }

    #[test]
    fn parses_unicode_identifier() {
        assert_eq!(CharacterReader::new("Żółw9 x").parse_identifier(), "Żółw9");
    }

    #[test]
    fn parses_type_identifier() {
        let mut r = CharacterReader::new("Outer+Inner)");
        assert_eq!(r.parse_type_identifier(), "Outer+Inner");
        assert_eq!(r.peek(), Some(')'));
        assert_eq!(CharacterReader::new("+Inner").parse_type_identifier(), "");
        assert_eq!(CharacterReader::new("").parse_type_identifier(), "");
    }

    #[test]
    fn parses_number() {
        let mut r = CharacterReader::new("123px");
        assert_eq!(r.parse_number(), "123");
        assert_eq!(r.parse_number(), "");
        assert_eq!(r.position(), 3);
        assert_eq!(CharacterReader::new("-1").parse_number(), "");
    }
}
