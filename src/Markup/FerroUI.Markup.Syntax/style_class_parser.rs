//! Style class name parsing on top of [`CharacterReader`].

use super::character_reader::{is_identifier_part_category, is_letter, CharacterReader};

impl<'a> CharacterReader<'a> {
    /// Parses a style class name: an identifier that may also contain `-`.
    /// Returns an empty string (consuming nothing) when the input does not
    /// start with a class name.
    pub fn parse_style_class(&mut self) -> &'a str {
        if self.peek().is_some_and(is_valid_identifier_start) {
            self.take_while(is_valid_identifier_char)
        } else {
            ""
        }
    }
}

fn is_valid_identifier_start(c: char) -> bool {
    is_letter(c) || c == '_'
}

fn is_valid_identifier_char(c: char) -> bool {
    is_valid_identifier_start(c) || c == '-' || is_identifier_part_category(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_style_class_with_dashes() {
        let mut r = CharacterReader::new("nth-last-child(2)");
        assert_eq!(r.parse_style_class(), "nth-last-child");
        assert_eq!(r.peek(), Some('('));
        assert_eq!(r.position(), 14);
    }

    #[test]
    fn style_class_cannot_start_with_dash_digit_or_symbol() {
        assert_eq!(CharacterReader::new("-foo").parse_style_class(), "");
        assert_eq!(CharacterReader::new("1foo").parse_style_class(), "");
        assert_eq!(CharacterReader::new("%foo").parse_style_class(), "");
        assert_eq!(CharacterReader::new("").parse_style_class(), "");
    }

    #[test]
    fn parses_underscored_class_with_digits() {
        assert_eq!(CharacterReader::new("_foo1 bar").parse_style_class(), "_foo1");
    }
}
