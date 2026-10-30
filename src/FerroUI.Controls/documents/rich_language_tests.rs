//! Independent language, shaping, history and RTF contract regressions.
use super::{CharacterFormat, Paragraph, RichTextDocument, Run, Span, TextElement};
use crate::test_support::test_scope;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Rect, Size};

#[test]
fn rich_language_source_validation_and_patch_assignment_are_atomic() {
    let mut format = CharacterFormat::default();
    for tag in [
        "",
        "en",
        "en-US",
        "zh-Hans-CN",
        "es-419",
        "x-private",
        "abcdefgh-12345678",
    ] {
        format.set_language_tag(tag).unwrap();
        assert_eq!(format.language_tag(), Some(tag));
    }
    let before = format.clone();
    for tag in [
        "en_US",
        "1en-US",
        "en-",
        "-en",
        "en--US",
        "abcdefghi",
        "en-123456789",
        "é",
        "en US",
        "en\0US",
    ] {
        assert!(format.set_language_tag(tag).is_err(), "{tag:?}");
        assert_eq!(format, before);
        let malformed = CharacterFormat {
            language: Some(CultureInfo::get_culture_info(tag)),
            ..Default::default()
        };
        assert!(malformed.validate().is_err());
    }
}

/// Deviation (DEVIATIONS.md, Text block): the runs of a text element without
/// `Language` keep upstream's `None` culture; one with a language set on it or on
/// an ancestor gets the resolved language.
#[test]
fn runs_get_a_culture_only_from_a_set_language() {
    let _scope = test_scope();
    let span = Span::new();
    let run = Run::with_text(Some("text"));
    span.inlines().add(run.clone());
    assert!(!run.has_language());
    assert!(run.run_culture().is_none());

    span.set_language(CultureInfo::get_culture_info("ja-JP"));
    assert!(run.has_language());
    assert_eq!(run.run_culture().map(|culture| culture.name().to_owned()).as_deref(), Some("ja-JP"));
}
