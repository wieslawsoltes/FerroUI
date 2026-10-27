//! Portable dictionary word segmentation.
//!
//! Compiled ICU data makes segmentation available on browser and native targets
//! without dynamic libraries or a process-global locale. An unset locale uses
//! invariant rules; callers can supply the content language explicitly.

use icu_locale_core::LanguageIdentifier;
use icu_segmenter::{options::WordBreakOptions, WordSegmenter};

pub(crate) fn try_enumerate_locale_aware_word_segments<'a>(
    text: &'a str,
    locale: Option<&str>,
) -> Option<Vec<&'a str>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    let mut segments = Vec::new();
    let mut previous = 0;
    if let Some(locale) = locale.filter(|locale| !locale.trim().is_empty()) {
        let language: LanguageIdentifier = locale.replace('_', "-").parse().ok()?;
        let mut options = WordBreakOptions::default();
        options.content_locale = Some(&language);
        let segmenter = WordSegmenter::try_new_dictionary(options).ok()?;
        for next in segmenter.as_borrowed().segment_str(text) {
            if next > previous {
                segments.push(&text[previous..next]);
            }
            previous = next;
        }
    } else {
        let segmenter = WordSegmenter::new_dictionary(Default::default());
        for next in segmenter.segment_str(text) {
            if next > previous {
                segments.push(&text[previous..next]);
            }
            previous = next;
        }
    }
    if previous < text.len() {
        segments.push(&text[previous..]);
    }
    Some(segments)
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
}
