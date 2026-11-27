use crate::{RibbonGroupState, RibbonItemSize};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RibbonSizeDefinition {
    pub large: RibbonItemSize,
    pub medium: RibbonItemSize,
    pub small: RibbonItemSize,
}

impl RibbonSizeDefinition {
    pub const fn new(large: RibbonItemSize, medium: RibbonItemSize, small: RibbonItemSize) -> Self {
        Self {
            large,
            medium,
            small,
        }
    }
    pub const fn large_medium_small() -> Self {
        Self::new(
            RibbonItemSize::Large,
            RibbonItemSize::Medium,
            RibbonItemSize::Small,
        )
    }
    pub const fn always_large() -> Self {
        Self::new(
            RibbonItemSize::Large,
            RibbonItemSize::Large,
            RibbonItemSize::Large,
        )
    }
    pub const fn medium_small() -> Self {
        Self::new(
            RibbonItemSize::Medium,
            RibbonItemSize::Small,
            RibbonItemSize::Small,
        )
    }
    pub const fn always_medium() -> Self {
        Self::new(
            RibbonItemSize::Medium,
            RibbonItemSize::Medium,
            RibbonItemSize::Medium,
        )
    }
    pub const fn always_small() -> Self {
        Self::new(
            RibbonItemSize::Small,
            RibbonItemSize::Small,
            RibbonItemSize::Small,
        )
    }
    pub fn for_preferred_size(size: RibbonItemSize) -> Self {
        match size {
            RibbonItemSize::Large => Self::large_medium_small(),
            RibbonItemSize::Medium => Self::medium_small(),
            _ => Self::always_small(),
        }
    }
    pub fn get_size(self, state: RibbonGroupState) -> RibbonItemSize {
        match state {
            RibbonGroupState::Medium => self.medium,
            RibbonGroupState::Small => self.small,
            _ => self.large,
        }
    }
    pub fn parse(text: &str) -> Result<Self, SizeDefinitionParseError> {
        Self::try_parse(Some(text)).ok_or_else(|| SizeDefinitionParseError(text.to_owned()))
    }
    pub fn try_parse(text: Option<&str>) -> Option<Self> {
        let parts: Vec<_> = text?
            .split([',', ' ', ';'])
            .filter(|p| !p.is_empty())
            .collect();
        if !(1..=3).contains(&parts.len()) {
            return None;
        }
        let mut sizes = [RibbonItemSize::Large; 3];
        for i in 0..3 {
            let part = parts[i.min(parts.len() - 1)];
            // Source alias comparison precedes Enum.TryParse's whitespace trimming.
            sizes[i] = if part.eq_ignore_ascii_case("Middle") {
                RibbonItemSize::Medium
            } else {
                parse_size(part)?
            };
        }
        Some(Self::new(sizes[0], sizes[1], sizes[2]))
    }

    /// UTF-16 entry point for source strings, including invalid surrogate sequences.
    /// Every accepted enum token consists of ASCII names/numbers and whitespace;
    /// an unpaired surrogate therefore always fails the original parser.
    pub fn try_parse_utf16(text: Option<&[u16]>) -> Option<Self> {
        let text = String::from_utf16(text?).ok()?;
        Self::try_parse(Some(&text))
    }
}

fn dotnet_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{0085}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}

fn parse_size(part: &str) -> Option<RibbonItemSize> {
    let part = part.trim_matches(dotnet_whitespace);
    for (name, value) in [
        ("Large", RibbonItemSize::Large),
        ("Medium", RibbonItemSize::Medium),
        ("Small", RibbonItemSize::Small),
    ] {
        if part.eq_ignore_ascii_case(name) {
            return Some(value);
        }
    }
    // Integer parsing accepts trailing NULs; named enum parsing does not.
    part.trim_end_matches('\0')
        .trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000b}' | '\u{000c}'))
        .parse::<i32>()
        .ok()
        .map(RibbonItemSize::from_raw)
}

impl fmt::Display for RibbonSizeDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}, {}", self.large, self.medium, self.small)
    }
}
impl FromStr for RibbonSizeDefinition {
    type Err = SizeDefinitionParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SizeDefinitionParseError(String);
impl fmt::Display for SizeDefinitionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "'{}' is not a valid size definition. Use e.g. \"Large, Medium, Small\".",
            self.0
        )
    }
}
impl std::error::Error for SizeDefinitionParseError {}
