/// Represents information about a character hit within a glyph run.
///
/// The `CharacterHit` structure provides information about the index of the
/// first character that got hit as well as information about leading or
/// trailing edge.
///
/// Indices and lengths are UTF-16 code units (see the index-unit note in
/// `media/text_formatting/mod.rs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CharacterHit {
    first_character_index: i32,
    trailing_length: i32,
}

impl CharacterHit {
    /// Creates a hit on the leading edge of the character at `first_character_index`.
    #[inline]
    pub const fn new(first_character_index: i32) -> Self {
        Self { first_character_index, trailing_length: 0 }
    }

    /// Creates a hit with a trailing length.
    #[inline]
    pub const fn with_trailing_length(first_character_index: i32, trailing_length: i32) -> Self {
        Self { first_character_index, trailing_length }
    }

    /// Gets the index of the first character that got hit.
    #[inline]
    pub const fn first_character_index(&self) -> i32 {
        self.first_character_index
    }

    /// Gets the trailing length value for the character that got hit.
    #[inline]
    pub const fn trailing_length(&self) -> i32 {
        self.trailing_length
    }
}
