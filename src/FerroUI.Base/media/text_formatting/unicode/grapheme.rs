use super::codepoint::Codepoint;

/// Represents the smallest unit of a writing system of any given language.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Grapheme {
    first_codepoint: Codepoint,
    offset: usize,
    length: usize,
}

impl Grapheme {
    #[inline]
    pub const fn new(first_codepoint: Codepoint, offset: usize, length: usize) -> Self {
        Self { first_codepoint, offset, length }
    }

    /// The first [`Codepoint`] of the grapheme cluster.
    #[inline]
    pub const fn first_codepoint(&self) -> Codepoint {
        self.first_codepoint
    }

    /// Gets the starting code unit offset of this grapheme inside its containing text.
    #[inline]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Gets the length of this grapheme, in code units.
    #[inline]
    pub const fn length(&self) -> usize {
        self.length
    }
}
