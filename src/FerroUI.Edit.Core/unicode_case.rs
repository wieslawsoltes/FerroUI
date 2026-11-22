mod data;

use data::{LOWER, UPPER, UPPERCASE_LETTERS};

#[derive(Clone, Copy)]
pub(crate) enum CaseLocale {
    Invariant,
    Default,
    Turkic,
}

impl CaseLocale {
    pub(crate) fn from_language_tag(tag: &str) -> Self {
        let language = tag.trim().split(['-', '_']).next().unwrap_or("");
        if language.eq_ignore_ascii_case("tr") || language.eq_ignore_ascii_case("az") {
            Self::Turkic
        } else if language.is_empty() || language.eq_ignore_ascii_case("und") {
            Self::Invariant
        } else {
            Self::Default
        }
    }

    pub(crate) fn upper(self, character: char) -> char {
        match (self, character) {
            (Self::Invariant, 'ı') => character,
            (Self::Turkic, 'i') => 'İ',
            _ => char::from_u32(simple_upper(u32::from(character))).unwrap_or(character),
        }
    }

    pub(crate) fn lower(self, character: char) -> char {
        match (self, character) {
            (Self::Invariant, 'İ') => character,
            (Self::Turkic, 'I') => 'ı',
            _ if character.is_ascii() => character.to_ascii_lowercase(),
            _ => char::from_u32(map(u32::from(character), LOWER)).unwrap_or(character),
        }
    }

    /// Yields the characters that share this character's lowercase form.
    pub(crate) fn lowercase_equivalents(self, character: char) -> impl Iterator<Item = char> {
        let lower = self.lower(character);
        let sources = LOWER.iter().filter_map(move |&(start, end, stride, delta)| {
            let source = u32::from(lower).checked_add_signed(-delta)?;
            if !(start..=end).contains(&source) || (source - start) % stride != 0 {
                return None;
            }
            char::from_u32(source).filter(|&candidate| self.lower(candidate) == lower)
        });
        let turkic_upper = (matches!(self, Self::Turkic) && lower == 'ı').then_some('I');
        std::iter::once(lower).chain(sources).chain(turkic_upper)
    }
}

pub(crate) fn simple_upper(value: u32) -> u32 {
    if value <= 0x7f {
        u32::from((value as u8).to_ascii_uppercase())
    } else {
        map(value, UPPER)
    }
}

pub(crate) fn is_uppercase_letter(unit: u16) -> bool {
    if unit <= 0x7f {
        return (unit as u8).is_ascii_uppercase();
    }
    let index = UPPERCASE_LETTERS.partition_point(|&(start, _, _)| start <= unit);
    let Some(index) = index.checked_sub(1) else { return false };
    let (start, end, stride) = UPPERCASE_LETTERS[index];
    unit <= end && (unit - start) % stride == 0
}

fn map(value: u32, ranges: &[(u32, u32, u32, i32)]) -> u32 {
    let index = ranges.partition_point(|&(start, _, _, _)| start <= value);
    let Some(index) = index.checked_sub(1) else { return value };
    let (start, end, stride, delta) = ranges[index];
    if value <= end && (value - start) % stride == 0 {
        value.wrapping_add_signed(delta)
    } else {
        value
    }
}
