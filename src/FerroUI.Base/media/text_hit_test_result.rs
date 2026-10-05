use crate::media::CharacterHit;

/// Holds a hit test result from a text layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextHitTestResult {
    character_hit: CharacterHit,
    text_position: i32,
    is_inside: bool,
    is_trailing: bool,
}

impl TextHitTestResult {
    pub const fn new(character_hit: CharacterHit, text_position: i32, is_inside: bool, is_trailing: bool) -> Self {
        Self { character_hit, text_position, is_inside, is_trailing }
    }

    /// Gets the character hit of the hit test result.
    pub const fn character_hit(&self) -> CharacterHit {
        self.character_hit
    }

    /// Gets a value indicating whether the point is inside the bounds of the text layout.
    pub const fn is_inside(&self) -> bool {
        self.is_inside
    }

    /// Gets the index of the hit character in the text.
    pub const fn text_position(&self) -> i32 {
        self.text_position
    }

    /// Gets a value indicating whether the hit is on the trailing edge of the character.
    pub const fn is_trailing(&self) -> bool {
        self.is_trailing
    }
}
