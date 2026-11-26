//! Keyword look-ahead on top of [`CharacterReader`].

use super::character_reader::CharacterReader;

impl CharacterReader<'_> {
    /// Checks whether the input continues with optional white space followed
    /// by `keyword`, without consuming anything.
    pub fn check_keyword(&self, keyword: &str) -> bool {
        self.check_keyword_internal(keyword).is_some()
    }

    /// Returns the number of characters (white space plus keyword) the keyword
    /// occupies, or `None` when the input does not continue with it.
    fn check_keyword_internal(&self, keyword: &str) -> Option<usize> {
        let ws = self.peek_whitespace();
        let count = ws.chars().count() + keyword.chars().count();

        let chars = self.try_peek(count);
        if chars.is_empty() {
            return None;
        }
        if &chars[ws.len()..] == keyword {
            return Some(count);
        }
        None
    }

    /// Consumes optional white space followed by `keyword` if the input
    /// continues with it.
    ///
    /// As upstream, the consumed characters are skipped without advancing
    /// [`CharacterReader::position`].
    pub fn take_if_keyword(&mut self, keyword: &str) -> bool {
        match self.check_keyword_internal(keyword) {
            Some(l) => {
                self.skip(l);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_keyword_ignores_leading_whitespace() {
        let r = CharacterReader::new("  :> Foo");
        assert!(r.check_keyword(":>"));
        assert!(!r.check_keyword(":="));
        assert_eq!(r.peek(), Some(' '));
    }

    #[test]
    fn check_keyword_fails_on_short_input() {
        assert!(!CharacterReader::new(" a").check_keyword("as "));
        assert!(!CharacterReader::new("").check_keyword(":="));
    }

    #[test]
    fn take_if_keyword_consumes_whitespace_and_keyword() {
        let mut r = CharacterReader::new(" as Foo");
        assert!(!r.take_if_keyword(":="));
        assert!(r.take_if_keyword("as "));
        assert_eq!(r.peek(), Some('F'));
        // Upstream quirk: the position is not advanced.
        assert_eq!(r.position(), 0);
        assert!(!r.take_if_keyword("as "));
    }
}
