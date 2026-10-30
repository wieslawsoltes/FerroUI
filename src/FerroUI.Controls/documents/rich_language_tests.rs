//! Independent language, shaping, history and RTF contract regressions.
use super::{CharacterFormat, Paragraph, RichTextDocument, Run, Span, TextElement};
use crate::test_support::test_scope;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Rect, Size};
#[allow(unused_imports)]
use crate::{RichTextBlock, RichTextBlockOverflow};

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

#[test]
fn rich_language_inherits_into_shaping_and_local_values_override() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    view.set_language(CultureInfo::get_culture_info("ja-JP"));
    let paragraph = Paragraph::new();
    let span = Span::new();
    let run = Run::with_text(Some("漢字"));
    span.add_child(run.clone());
    paragraph.inlines().add(span.clone());
    view.blocks().add(paragraph);
    let culture = || {
        run.create_text_run_properties()
            .culture_info()
            .cloned()
            .unwrap()
    };
    assert_eq!(culture().name(), "ja-JP");
    span.set_language(CultureInfo::get_culture_info("zh-CN"));
    assert_eq!(culture().name(), "zh-CN");
    run.set_language(CultureInfo::get_culture_info("zh-TW"));
    assert_eq!(culture().name(), "zh-TW");
    view.measure(Size::new(200.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));
    assert!(view.is_measure_valid());
    run.clear_value(TextElement::language_property());
    assert!(!view.is_measure_valid());
    assert_eq!(culture().name(), "zh-CN");
    run.set_language(CultureInfo::invariant_culture());
    assert!(
        !culture().is_invariant(),
        "an empty assigned language resolves for shaping"
    );
}
