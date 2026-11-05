//! WinUI's `InitialsGenerator` (`controls/dev/PersonPicture/InitialsGenerator.cpp` and `.h` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md): the initials a [`PersonPicture`](crate::PersonPicture)
//! shows for a name.
//!
//! WinUI reads UTF-16 code units; this port reads characters, so the CJK ideographs outside the
//! basic multilingual plane (Extensions B to D, the Compatibility Ideographs Supplement) are
//! symbolic, as the ranges of WinUI intend, where a code unit of WinUI never reaches them.

use crate::Contact;

/// The kind of the first characters of a name (WinUI's `CharacterType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CharacterType {
    /// Not a character of the allowed sets.
    Other,
    /// Latin and Cyrillic: initials are made.
    Standard,
    /// CJK, Greek, Hebrew and Armenian: no initials.
    Symbolic,
    /// The scripts whose letters join (Arabic, the Indic scripts, Thai, Lao): no initials.
    Glyph,
}

/// The initials of a contact (WinUI's `InitialsFromContactObject`): the first letters of its
/// first and last names when it has both, else those of its display name. Empty when the name is
/// not in the standard character set; the picture then shows the glyph of a contact.
pub(crate) fn initials_from_contact_object(contact: Option<&Contact>) -> String {
    let Some(contact) = contact else { return String::new() };
    let first_name = contact.first_name();
    let last_name = contact.last_name();
    // "Optimal case is we have a clearly defined First and Last name."
    if !first_name.is_empty() && !last_name.is_empty() {
        if get_character_type(&first_name) == CharacterType::Standard {
            let mut result = get_first_full_character(&first_name);
            result.push_str(&get_first_full_character(&last_name));
            return result.to_uppercase();
        }
        return String::new();
    }
    let display_name = contact.display_name();
    if !display_name.is_empty() {
        return initials_from_display_name(&display_name);
    }
    String::new()
}

/// The initials of a display name (WinUI's `InitialsFromDisplayName`): without a final bracketed
/// part, the first letter of a single word, or the first letters of the first and the last word.
pub(crate) fn initials_from_display_name(display_name: &str) -> String {
    if get_character_type(display_name) != CharacterType::Standard {
        return String::new();
    }
    let display_name = strip_trailing_brackets(display_name);
    let words = split(display_name, ' ', 25);
    match words.as_slice() {
        [] => String::new(),
        [word] => get_first_full_character(word).to_uppercase(),
        [first, .., last] => {
            // "Based on current implementation, we could be showing punctuation. For example,
            // "John -Smith" would be "J-"."
            let mut result = get_first_full_character(first);
            result.push_str(&get_first_full_character(last));
            result.to_uppercase()
        }
    }
}

/// The first character of `text` that is not ASCII punctuation, with the combining diacritical
/// marks that follow it (WinUI's `GetFirstFullCharacter`); the first character when every
/// character is punctuation.
pub(crate) fn get_first_full_character(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let is_punctuation = |c: char| matches!(c as u32, 0x21..=0x2F | 0x3A..=0x40 | 0x7B..=0x7E);
    let mut start = characters.iter().position(|&c| !is_punctuation(c)).unwrap_or(characters.len());
    // "If no desireable characters exist, we'll start at index 1, as combining characters begin
    // only after the first character."
    if start >= characters.len() {
        start = 0;
    }
    if characters.is_empty() {
        return String::new();
    }
    let mut end = start + 1;
    while end < characters.len() && matches!(characters[end] as u32, 0x0300..=0x036F) {
        end += 1;
    }
    characters[start..end].iter().collect()
}

/// The words of `source` between the delimiters, without empty ones, from at most
/// `max_iterations` pieces (WinUI's `Split`).
fn split(source: &str, delimiter: char, max_iterations: usize) -> Vec<&str> {
    source.split(delimiter).take(max_iterations).filter(|token| !token.is_empty()).collect()
}

/// `source` without a final bracketed part (WinUI's `StripTrailingBrackets`): "John Smith (OSG)"
/// is "John Smith ".
fn strip_trailing_brackets(source: &str) -> &str {
    let Some(last) = source.chars().last() else { return source };
    for (open, close) in [('{', '}'), ('(', ')'), ('[', ']')] {
        if last != close {
            continue;
        }
        let Some(start) = source.rfind(open) else { continue };
        return &source[..start];
    }
    source
}

/// The kind of the first three characters of `text`, in the precedence glyph, symbolic, standard
/// (WinUI's `GetCharacterType` of a string); a byte order mark ends the text.
pub(crate) fn get_character_type(text: &str) -> CharacterType {
    let mut result = CharacterType::Other;
    for character in text.chars().take(3) {
        if character == '\u{FEFF}' {
            break;
        }
        match character_type(character) {
            CharacterType::Glyph => result = CharacterType::Glyph,
            CharacterType::Symbolic => {
                if result != CharacterType::Glyph {
                    result = CharacterType::Symbolic;
                }
            }
            CharacterType::Standard => {
                if result != CharacterType::Glyph && result != CharacterType::Symbolic {
                    result = CharacterType::Standard;
                }
            }
            CharacterType::Other => {}
        }
    }
    result
}

/// The kind of a character, by the Unicode blocks WinUI allows (WinUI's `GetCharacterType` of a
/// character).
pub(crate) fn character_type(character: char) -> CharacterType {
    match character as u32 {
        // GLYPH: IPA Extensions, Arabic, Arabic Supplement, Arabic Extended-A, Arabic
        // Presentation Forms-A and -B, Devanagari, Devanagari Extended, Bangla, Gurmukhi,
        // Gujarati, Odia, Tamil, Telugu, Kannada, Malayalam, Sinhala, Thai, Lao.
        0x0250..=0x02AF
        | 0x0600..=0x06FF
        | 0x0750..=0x077F
        | 0x08A0..=0x08FF
        | 0xFB50..=0xFDFF
        | 0xFE70..=0xFEFF
        | 0x0900..=0x097F
        | 0xA8E0..=0xA8FF
        | 0x0980..=0x09FF
        | 0x0A00..=0x0A7F
        | 0x0A80..=0x0AFF
        | 0x0B00..=0x0B7F
        | 0x0B80..=0x0BFF
        | 0x0C00..=0x0C7F
        | 0x0C80..=0x0CFF
        | 0x0D00..=0x0D7F
        | 0x0D80..=0x0DFF
        | 0x0E00..=0x0E7F
        | 0x0E80..=0x0EFF => CharacterType::Glyph,
        // SYMBOLIC: the CJK blocks, Greek and Coptic, Hebrew, Armenian.
        0x4E00..=0x9FFF
        | 0x3400..=0x4DBF
        | 0x20000..=0x2A6DF
        | 0x2A700..=0x2B73F
        | 0x2B740..=0x2B81F
        | 0x2E80..=0x2EFF
        | 0x3000..=0x303F
        | 0x31C0..=0x31EF
        | 0x3200..=0x32FF
        | 0x3300..=0x33FF
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0x2F800..=0x2FA1F
        | 0x0370..=0x03FF
        | 0x0590..=0x05FF
        | 0x0530..=0x058F => CharacterType::Symbolic,
        // LATIN: Basic Latin, Latin-1 Supplement, Latin Extended-A to -E, Latin Extended
        // Additional, Cyrillic, Cyrillic Supplement, Combining Diacritical Marks.
        0x0001..=0x007F
        | 0x0080..=0x00FF
        | 0x0100..=0x017F
        | 0x0180..=0x024F
        | 0x2C60..=0x2C7F
        | 0xA720..=0xA7FF
        | 0xAB30..=0xAB6F
        | 0x1E00..=0x1EFF
        | 0x0400..=0x04FF
        | 0x0500..=0x052F
        | 0x0300..=0x036F => CharacterType::Standard,
        _ => CharacterType::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_display_name_gives_the_first_letters_of_its_first_and_last_words() {
        assert_eq!("JS", initials_from_display_name("John Smith"));
        assert_eq!("DH", initials_from_display_name("Dr. Jordan von Hammerspike"));
        assert_eq!("J", initials_from_display_name("jane"));
        // "John -Smith" would be "J-": the punctuation of a word is skipped only at its start.
        assert_eq!("JS", initials_from_display_name("John -Smith"));
        assert_eq!("", initials_from_display_name("   "));
        assert_eq!("", initials_from_display_name(""));
    }

    #[test]
    fn a_final_bracketed_part_is_left_out() {
        assert_eq!("JS", initials_from_display_name("John Smith (OSG)"));
        assert_eq!("JS", initials_from_display_name("John Smith [x]"));
        assert_eq!("JO", initials_from_display_name("John (OSG"));
    }

    #[test]
    fn diacritics_stay_with_their_letter() {
        assert_eq!("E\u{301}Z", initials_from_display_name("e\u{301}mile zola"));
        assert_eq!("ÉZ", initials_from_display_name("émile zola"));
        assert_eq!("ИП", initials_from_display_name("иван петров"));
    }

    #[test]
    fn names_outside_the_standard_set_have_no_initials() {
        assert_eq!("", initials_from_display_name("王小明"));
        assert_eq!("", initials_from_display_name("محمد علي"));
        assert_eq!("", initials_from_display_name("Ελένη Παππά"));
        // A glyph among the first three characters wins over the Latin ones.
        assert_eq!(CharacterType::Glyph, get_character_type("abت"));
        assert_eq!(CharacterType::Symbolic, get_character_type("ab王"));
        assert_eq!(CharacterType::Standard, get_character_type("abc王"));
        assert_eq!(CharacterType::Symbolic, character_type('\u{20000}'));
    }

    #[test]
    fn a_name_of_punctuation_keeps_its_first_character() {
        assert_eq!("!", get_first_full_character("!!!"));
        assert_eq!("a", get_first_full_character("--a"));
        assert_eq!("", get_first_full_character(""));
    }

    #[test]
    fn a_contact_uses_its_first_and_last_names_else_its_display_name() {
        crate::register_types();
        let contact = Contact::new();
        assert_eq!("", initials_from_contact_object(Some(&contact)));
        contact.set_first_name("john");
        assert_eq!("J", initials_from_contact_object(Some(&contact)));
        contact.set_last_name("doe");
        assert_eq!("JD", initials_from_contact_object(Some(&contact)));
        contact.set_first_name("王");
        assert_eq!("", initials_from_contact_object(Some(&contact)));
        let contact = Contact::new();
        contact.set_display_name_override("Ann Lee (HR)");
        assert_eq!("AL", initials_from_contact_object(Some(&contact)));
        assert_eq!("", initials_from_contact_object(None));
    }
}
