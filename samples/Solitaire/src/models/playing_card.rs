//! CardSuit, CardColour and CardType from the source model, represented as values.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardSuit {
    Hearts,
    Clubs,
    Diamonds,
    Spades,
}
impl CardSuit {
    pub const ALL: [Self; 4] = [Self::Hearts, Self::Clubs, Self::Diamonds, Self::Spades];
    pub fn is_red(self) -> bool {
        matches!(self, Self::Hearts | Self::Diamonds)
    }
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Hearts => "♥",
            Self::Clubs => "♣",
            Self::Diamonds => "♦",
            Self::Spades => "♠",
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Hearts => "H",
            Self::Clubs => "C",
            Self::Diamonds => "D",
            Self::Spades => "S",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayingCard {
    pub id: u8,
    pub suit: CardSuit,
    /// Ace = 1, King = 13.
    pub rank: u8,
    pub is_face_down: bool,
}
impl PlayingCard {
    pub fn resource_key(self) -> String {
        format!(
            "{}{}",
            self.suit.code(),
            match self.rank {
                1 => "A".into(),
                11 => "J".into(),
                12 => "Q".into(),
                13 => "K".into(),
                n => n.to_string(),
            }
        )
    }
    pub fn label(self) -> String {
        format!(
            "{}{}",
            match self.rank {
                1 => "A".into(),
                11 => "J".into(),
                12 => "Q".into(),
                13 => "K".into(),
                n => n.to_string(),
            },
            self.suit.symbol()
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawMode {
    #[default]
    DrawOne,
    DrawThree,
}
impl DrawMode {
    pub fn count(self) -> usize {
        if self == Self::DrawOne { 1 } else { 3 }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    #[default]
    Easy,
    Medium,
    Hard,
}
impl Difficulty {
    pub fn suits(self) -> &'static [CardSuit] {
        match self {
            Self::Easy => &[CardSuit::Hearts],
            Self::Medium => &[CardSuit::Hearts, CardSuit::Spades],
            Self::Hard => &CardSuit::ALL,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy · 1 suit",
            Self::Medium => "Medium · 2 suits",
            Self::Hard => "Hard · 4 suits",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameKind {
    Klondike,
    FreeCell,
    Spider,
}
impl GameKind {
    pub const ALL: [Self; 3] = [Self::Klondike, Self::FreeCell, Self::Spider];
    pub fn name(self) -> &'static str {
        match self {
            Self::Klondike => "Klondike",
            Self::FreeCell => "FreeCell",
            Self::Spider => "Spider",
        }
    }
    pub fn index(self) -> usize {
        match self {
            Self::Klondike => 0,
            Self::FreeCell => 1,
            Self::Spider => 2,
        }
    }
    pub fn columns(self) -> usize {
        match self {
            Self::Klondike => 7,
            Self::FreeCell => 8,
            Self::Spider => 10,
        }
    }
}
