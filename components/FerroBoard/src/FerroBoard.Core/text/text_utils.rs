// Derived from Text/TextUtils.cs; see ../NOTICE.md.
// Normalization uses the explicit UTF-16 canonical bridge.
use super::{TextCulture, Utf16Text, grapheme, text_properties::properties};
pub struct TextUtils;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextCaseError {
    CurrentCulture(super::locale_text::CollationError),
    UnsupportedCulture(super::UnsupportedCulture),
}
impl std::fmt::Display for TextCaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrentCulture(error) => {
                write!(f, "current text culture is unavailable: {error}")
            }
            Self::UnsupportedCulture(error) => std::fmt::Display::fmt(error, f),
        }
    }
}
impl std::error::Error for TextCaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CurrentCulture(error) => Some(error),
            Self::UnsupportedCulture(error) => Some(error),
        }
    }
}
fn casing_culture(culture: Option<TextCulture>) -> Result<TextCulture, TextCaseError> {
    match culture {
        Some(culture) => Ok(culture),
        // Source ToUpper(null) resolves the ambient object only at actual casing.
        None => TextCulture::current(),
    }
}
impl TextUtils {
    /// Source enumeration retains unpaired UTF-16 surrogates as integers.
    pub fn get_code_points(text: &Utf16Text) -> Vec<i32> {
        char::decode_utf16(text.units().iter().copied())
            .map(|r| match r {
                Ok(c) => c as i32,
                Err(e) => e.unpaired_surrogate() as i32,
            })
            .collect()
    }
    pub fn first_grapheme_length(text: &Utf16Text) -> i32 {
        grapheme::first_length(text.units()) as i32
    }
    pub fn last_grapheme_length(text: &Utf16Text) -> i32 {
        let mut offset = 0;
        let mut last = 0;
        while offset < text.len() {
            last = grapheme::first_length(&text.units()[offset..]);
            offset += last;
        }
        last as i32
    }
    pub fn is_word_char(c: u16) -> bool {
        properties(c as u32).0 & 1 != 0
    }
    pub fn is_word_core_char(c: u16) -> bool {
        properties(c as u32).0 & 2 != 0
    }
    /// Explicit .NET char.IsWhiteSpace bridge for engine/editor callers.
    pub fn is_white_space(c: u16) -> bool {
        properties(c as u32).0 & 4 != 0
    }
    pub fn get_current_word(text: &Utf16Text) -> Utf16Text {
        let end = text.len();
        let mut start = end;
        while start > 0 && Self::is_word_char(text.units()[start - 1]) {
            start -= 1;
        }
        while start < end && !Self::is_word_core_char(text.units()[start]) {
            start += 1;
        }
        Utf16Text::from_units(text.units()[start..end].to_vec())
    }
    pub fn find_previous_word_boundary(text: &Utf16Text, index: i32) -> i32 {
        let mut i = index.min(text.len() as i32);
        while i > 0 && Self::is_white_space(text.units()[i as usize - 1]) {
            i -= 1;
        }
        if i > 0 && !Self::is_word_char(text.units()[i as usize - 1]) {
            let c = text.units()[i as usize - 1];
            while i > 0
                && !Self::is_word_char(text.units()[i as usize - 1])
                && !Self::is_white_space(text.units()[i as usize - 1])
            {
                i -= 1;
                if text.units()[i as usize] != c {
                    break;
                }
            }
            return i;
        }
        while i > 0 && Self::is_word_char(text.units()[i as usize - 1]) {
            i -= 1;
        }
        i
    }
    pub fn find_next_word_boundary(text: &Utf16Text, index: i32) -> i32 {
        let mut i = index.max(0);
        let length = text.len() as i32;
        while i < length && Self::is_white_space(text.units()[i as usize]) {
            i += 1;
        }
        if i < length && !Self::is_word_char(text.units()[i as usize]) {
            return i + 1;
        }
        while i < length && Self::is_word_char(text.units()[i as usize]) {
            i += 1;
        }
        i
    }
    pub fn match_case(word: &Utf16Text, pattern: &Utf16Text, culture: TextCulture) -> Utf16Text {
        Self::match_case_nullable(word, pattern, Some(culture)).expect("nonnull culture")
    }
    pub fn match_case_nullable(
        word: &Utf16Text,
        pattern: &Utf16Text,
        culture: Option<TextCulture>,
    ) -> Result<Utf16Text, TextCaseError> {
        if pattern.is_empty() || word.is_empty() {
            return Ok(word.clone());
        }
        let mut letters = 0;
        let mut all_upper = true;
        for c in pattern.units() {
            let f = properties(*c as u32).0;
            if f & 8 != 0 {
                letters += 1;
                all_upper &= f & 16 != 0;
            }
        }
        if letters > 1 && all_upper {
            return Ok(casing_culture(culture)?.to_upper(word));
        }
        if properties(pattern.units()[0] as u32).0 & 16 != 0 {
            return Self::capitalize_nullable(word, culture);
        }
        Ok(word.clone())
    }
    pub fn capitalize(word: &Utf16Text, culture: TextCulture) -> Utf16Text {
        Self::capitalize_nullable(word, Some(culture)).expect("nonnull culture")
    }
    pub fn capitalize_nullable(
        word: &Utf16Text,
        culture: Option<TextCulture>,
    ) -> Result<Utf16Text, TextCaseError> {
        if word.is_empty() {
            return Ok(word.clone());
        }
        let first = Self::first_grapheme_length(word) as usize;
        let upper = casing_culture(culture)?
            .to_upper(&Utf16Text::from_units(word.units()[..first].to_vec()));
        let mut result = Vec::with_capacity(word.len());
        result.extend_from_slice(upper.units());
        result.extend_from_slice(&word.units()[first..]);
        Ok(Utf16Text::from_units(result))
    }
}

impl TextUtils {
    pub fn remove_diacritics(text: &Utf16Text) -> Result<Utf16Text, super::NormalizationError> {
        use super::normalization::{CanonicalForm, normalize};
        let normalized = normalize(text, CanonicalForm::D)?;
        // Source iterates UTF-16 chars: supplementary Mn scalars are intentionally retained.
        let filtered = Utf16Text::from_units(
            normalized
                .units()
                .iter()
                .copied()
                .filter(|c| properties(*c as u32).0 & 32 == 0)
                .collect::<Vec<_>>(),
        );
        normalize(&filtered, CanonicalForm::C)
    }
    pub fn base_char(c: u16) -> Result<u16, super::NormalizationError> {
        use super::normalization::{CanonicalForm, normalize};
        let normalized = normalize(&Utf16Text::from_units(vec![c]), CanonicalForm::D)?;
        Ok(
            TextCulture::to_lower_invariant(&Utf16Text::from_units(vec![normalized.units()[0]]))
                .units()[0],
        )
    }
}
