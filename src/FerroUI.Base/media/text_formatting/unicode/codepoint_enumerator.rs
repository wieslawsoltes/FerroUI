use super::codepoint::Codepoint;

/// Enumerates the codepoints of UTF-16 text.
#[derive(Clone, Debug)]
pub struct CodepointEnumerator<'a> {
    text: &'a [u16],
    offset: usize,
}

impl<'a> CodepointEnumerator<'a> {
    #[inline]
    pub const fn new(text: &'a [u16]) -> Self {
        Self { text, offset: 0 }
    }

    /// Moves to the next [`Codepoint`].
    ///
    /// Returns `None` at the end of the text (where upstream returns `false`
    /// and sets the out value to the replacement codepoint).
    #[inline]
    pub fn move_next(&mut self) -> Option<Codepoint> {
        if self.offset >= self.text.len() {
            return None;
        }

        let (codepoint, count) = Codepoint::read_at(self.text, self.offset);

        self.offset += count;

        Some(codepoint)
    }
}

impl Iterator for CodepointEnumerator<'_> {
    type Item = Codepoint;

    #[inline]
    fn next(&mut self) -> Option<Codepoint> {
        self.move_next()
    }
}
