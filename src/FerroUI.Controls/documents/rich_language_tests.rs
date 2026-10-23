//! Independent language, shaping, history and RTF contract regressions.
use super::{CharacterFormat, Paragraph, RichTextDocument, Run, Span, TextElement};
use crate::rich_text::{RichEditTextDocument, RtfError, RtfReader, RtfWriter, TextFormatValue};
use crate::test_support::test_scope;
use crate::{RichEditBox, RichTextBlock};
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

#[test]
fn rich_language_snapshots_and_editor_history_preserve_tags() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("abc");
    let services = RichEditTextDocument::new(document.clone());
    let before = document.snapshot();
    let mut format = CharacterFormat::default();
    format.set_language_tag("tr-TR").unwrap();
    services
        .get_range(0, 3)
        .unwrap()
        .set_character_format(&format)
        .unwrap();
    let changed = document.snapshot();
    assert_eq!(
        document.character_format_at(1).unwrap().language_tag(),
        Some("tr-TR")
    );
    services.undo().unwrap();
    assert_eq!(document.snapshot(), before);
    services.redo().unwrap();
    assert_eq!(document.snapshot(), changed);
    document.replace_text(1..1, "x\ny").unwrap();
    assert_eq!(
        document.character_format_at(2).unwrap().language_tag(),
        Some("tr-TR")
    );
    document.restore(&changed).unwrap();
    assert_eq!(document.snapshot(), changed);
}

#[test]
fn rich_language_rtf_default_scope_plain_and_round_trip() {
    let _scope = test_scope();
    let snapshot =
        RtfReader::read_str(r"{\rtf1\ansi\deflang1033 a{\lang1041 b}c\plain d}").unwrap();
    let document = RichTextDocument::new();
    document.restore(&snapshot).unwrap();
    assert_eq!(document.text(), "abcd");
    let services = RichEditTextDocument::new(document.clone());
    // A nonempty range queries the character to the right of the boundary.
    // character_format_at is an insertion-format API with backward gravity.
    let language_at = |offset| match services
        .get_range(offset, offset + 1)
        .unwrap()
        .character_format()
        .unwrap()
    {
        TextFormatValue::Uniform(format) => format.language_tag().unwrap().to_owned(),
        TextFormatValue::Mixed => panic!("one scalar cannot have mixed formatting"),
    };
    for (offset, language) in [(0, "en-US"), (1, "ja-JP"), (2, "en-US"), (3, "en-US")] {
        assert_eq!(language_at(offset), language);
    }
    let encoded = RtfWriter::write(&snapshot).unwrap();
    assert!(encoded.contains("\\lang1041"));
    document
        .restore(&RtfReader::read_str(&encoded).unwrap())
        .unwrap();
    assert_eq!(language_at(1), "ja-JP");
    assert_eq!(language_at(3), "en-US");
}

#[test]
fn rich_language_rtf_reports_unrepresentable_tags_and_unknown_ids() {
    let _scope = test_scope();
    for input in [
        r"{\rtf1\lang-1 text}",
        r"{\rtf1\lang65536 text}",
        r"{\rtf1\lang text}",
    ] {
        assert!(matches!(
            RtfReader::read_str(input),
            Err(RtfError::InvalidParameter("language identifier"))
        ));
    }
    assert!(matches!(
        RtfReader::read_str(r"{\rtf1\lang65535 text}"),
        Err(RtfError::UnsupportedFeature(_))
    ));
    let document = RichTextDocument::from_text("x");
    let mut format = CharacterFormat::default();
    format.set_language_tag("qaa-Qaaa").unwrap();
    document.apply_format(0..1, &format).unwrap();
    assert!(matches!(
        RtfWriter::write(&document.snapshot()),
        Err(RtfError::UnsupportedFeature(_))
    ));
    let no_language = RtfReader::read_str(r"{\rtf1\lang1024 text}").unwrap();
    assert_eq!(
        no_language.paragraphs[0].inlines[0].format.language_tag(),
        Some("")
    );
    assert!(RtfWriter::write(&no_language)
        .unwrap()
        .contains("\\lang1024"));
}

#[test]
fn rich_language_owner_uses_the_same_inherited_presenter_value() {
    let _scope = test_scope();
    let editor = RichEditBox::new();
    editor.set_language(CultureInfo::get_culture_info("ko-KR"));
    assert_eq!(editor.text_presenter().language().name(), "ko-KR");
    editor.set_document(RichTextDocument::from_text("한글"));
    let block = editor.blocks().get(0);
    assert_eq!(block.language().name(), "ko-KR");
    editor.set_language(CultureInfo::get_culture_info("ja-JP"));
    assert_eq!(editor.text_presenter().language().name(), "ja-JP");
    assert_eq!(block.language().name(), "ja-JP");
}
