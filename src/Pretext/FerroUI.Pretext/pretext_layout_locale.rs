//! Portable dictionary word segmentation.
//!
//! Compiled ICU data makes segmentation available on browser and native targets
//! without dynamic libraries or a process-global locale. An unset locale uses
//! invariant rules; callers can supply the content language explicitly.

use icu_locale_core::LanguageIdentifier;
use icu_properties::{
    props::{LineBreak, Script, WordBreak},
    CodePointMapData,
};
use icu_segmenter::{options::WordBreakOptions, WordSegmenter};

pub(crate) fn try_enumerate_locale_aware_word_segments<'a>(
    text: &'a str,
    locale: Option<&str>,
) -> Option<Vec<&'a str>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    let boundaries: Vec<usize> =
        if let Some(locale) = locale.filter(|locale| !locale.trim().is_empty()) {
            let language: LanguageIdentifier = locale.replace('_', "-").parse().ok()?;
            let mut options = WordBreakOptions::default();
            options.content_locale = Some(&language);
            let segmenter = WordSegmenter::try_new_dictionary(options).ok()?;
            segmenter.as_borrowed().segment_str(text).collect()
        } else {
            let segmenter = WordSegmenter::new_dictionary(Default::default());
            segmenter.segment_str(text).collect()
        };
    Some(native_compatible_segments(text, &boundaries))
}

/// ICU4C's word rules exclude the three colon forms from MidLetter and treat
/// dictionary-script letters as ALetter around a dictionary run. ICU4X emits
/// additional boundaries where those runs meet another script. Preserve the
/// source's word-like metadata by applying these rule differences before its
/// punctuation merge passes, while keeping dictionary boundaries inside runs.
fn native_compatible_segments<'a>(text: &'a str, boundaries: &[usize]) -> Vec<&'a str> {
    let mut tailored = Vec::with_capacity(boundaries.len());
    tailored.push(0);
    let mut previous = 0;
    for &end in boundaries {
        let mut after_colon = false;
        for (offset, ch) in text[previous..end].char_indices() {
            let offset = previous + offset;
            if is_native_colon(ch) {
                if tailored.last() != Some(&offset) {
                    tailored.push(offset);
                }
                after_colon = true;
            } else if after_colon && !is_ignored_word_character(ch) {
                if tailored.last() != Some(&offset) {
                    tailored.push(offset);
                }
                after_colon = false;
            }
        }
        if tailored.last() != Some(&end) {
            tailored.push(end);
        }
        previous = end;
    }

    let mut segments = Vec::with_capacity(tailored.len().saturating_sub(1));
    let mut start = 0;
    for end in tailored.into_iter().skip(1) {
        if end < text.len() && is_artificial_complex_boundary(text, end) {
            continue;
        }
        segments.push(&text[start..end]);
        start = end;
    }
    segments
}

fn is_native_colon(ch: char) -> bool {
    matches!(ch, ':' | '\u{fe55}' | '\u{ff1a}')
}

fn is_ignored_word_character(ch: char) -> bool {
    matches!(
        CodePointMapData::<WordBreak>::new().get(ch),
        WordBreak::Extend | WordBreak::Format | WordBreak::ZWJ
    )
}

fn dictionary_script(ch: char) -> Option<Script> {
    if CodePointMapData::<LineBreak>::new().get(ch) != LineBreak::ComplexContext {
        return None;
    }
    let script = CodePointMapData::<Script>::new().get(ch);
    matches!(
        script,
        Script::Thai | Script::Lao | Script::Myanmar | Script::Khmer
    )
    .then_some(script)
}

fn native_word_property(ch: char) -> WordBreak {
    if is_native_colon(ch) || CodePointMapData::<Script>::new().get(ch) == Script::Hangul {
        WordBreak::Other
    } else if dictionary_script(ch).is_some() {
        WordBreak::ALetter
    } else {
        CodePointMapData::<WordBreak>::new().get(ch)
    }
}

fn is_letter(property: WordBreak) -> bool {
    matches!(property, WordBreak::ALetter | WordBreak::HebrewLetter)
}

fn is_letter_joiner(property: WordBreak) -> bool {
    matches!(
        property,
        WordBreak::MidLetter | WordBreak::MidNumLet | WordBreak::SingleQuote
    )
}

fn is_artificial_complex_boundary(text: &str, boundary: usize) -> bool {
    let Some(raw_left) = text[..boundary].chars().next_back() else {
        return false;
    };
    let Some(raw_right) = text[boundary..].chars().next() else {
        return false;
    };
    let mut left = text[..boundary]
        .chars()
        .rev()
        .filter(|ch| !is_ignored_word_character(*ch));
    let mut right = text[boundary..]
        .chars()
        .filter(|ch| !is_ignored_word_character(*ch));
    let Some(before) = left.next() else {
        return false;
    };
    if is_ignored_word_character(raw_right) && dictionary_script(before).is_some() {
        return true;
    }
    let Some(after) = right.next() else {
        return false;
    };
    // Two words within a dictionary run must keep their dictionary boundary.
    let left_script = dictionary_script(raw_left);
    if left_script.is_some() && left_script == dictionary_script(raw_right) {
        return false;
    }
    let before_property = native_word_property(before);
    let after_property = native_word_property(after);
    let second_left = left.next();
    let second_right = right.next();
    if dictionary_script(before).is_none()
        && dictionary_script(after).is_none()
        && !(is_letter_joiner(before_property)
            && second_left.is_some_and(|ch| dictionary_script(ch).is_some()))
        && !(is_letter_joiner(after_property)
            && second_right.is_some_and(|ch| dictionary_script(ch).is_some()))
    {
        return false;
    }
    let before = before_property;
    let after = after_property;
    if (is_letter(before) && (is_letter(after) || after == WordBreak::Numeric))
        || (before == WordBreak::Numeric && is_letter(after))
        || (before == WordBreak::ExtendNumLet && is_letter(after))
        || (is_letter(before) && after == WordBreak::ExtendNumLet)
    {
        return true;
    }
    if is_letter(before) && is_letter_joiner(after) {
        return second_right.is_some_and(|ch| is_letter(native_word_property(ch)));
    }
    if is_letter_joiner(before) && is_letter(after) {
        return second_left.is_some_and(|ch| is_letter(native_word_property(ch)));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_segments_thai_without_native_libraries() {
        let text = "ทุกสองสัปดาห์";
        let parts = try_enumerate_locale_aware_word_segments(text, Some("th")).unwrap();
        assert_eq!(parts.concat(), text);
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn invalid_language_uses_the_callers_portable_fallback() {
        assert!(try_enumerate_locale_aware_word_segments("test", Some("!invalid")).is_none());
    }

    #[test]
    fn native_colon_tailoring_preserves_punctuation_tokens_and_marks() {
        for (text, expected) in [
            ("مرحبا:بكم", vec!["مرحبا", ":", "بكم"]),
            ("A：B", vec!["A", "：", "B"]),
            ("A﹕B", vec!["A", "﹕", "B"]),
            ("A:\u{301}B", vec!["A", ":\u{301}", "B"]),
            ("7:00", vec!["7", ":", "00"]),
            ("A꞉B", vec!["A꞉B"]),
        ] {
            assert_eq!(
                try_enumerate_locale_aware_word_segments(text, Some("en")).unwrap(),
                expected,
                "{text}"
            );
        }
    }

    #[test]
    fn dictionary_run_transitions_keep_native_word_context() {
        for (text, expected) in [
            ("worldမြန်မာ၊စာ", vec!["worldမြန်မာ", "၊", "စာ"]),
            (
                "ทุกสองสัปดาห์12,345.67a\u{200d}",
                vec!["ทุก", "สอง", "สัปดาห์12,345.67a\u{200d}"],
            ),
            ("wordภาษาไทย", vec!["wordภาษา", "ไทย"]),
            ("ภาษาไทยword", vec!["ภาษา", "ไทยword"]),
            ("မြန်မာภาษาไทย", vec!["မြန်မာภาษา", "ไทย"]),
            ("ไทย·abc", vec!["ไทย·abc"]),
            ("abc'ไทย", vec!["abc'ไทย"]),
            ("ไทย.a", vec!["ไทย.a"]),
            ("ไทย\u{301}ไทย", vec!["ไทย\u{301}ไทย"]),
            ("ไทย한글", vec!["ไทย", "한글"]),
            ("中ภาษาไทย", vec!["中", "ภาษา", "ไทย"]),
        ] {
            assert_eq!(
                try_enumerate_locale_aware_word_segments(text, Some("en")).unwrap(),
                expected,
                "{text}"
            );
        }
    }
}
