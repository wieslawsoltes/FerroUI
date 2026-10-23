//! Independent regressions for rich exchange boundaries: empty paragraph marks,
//! physical alignment, and format data which the supported RTF subset cannot carry.

use super::{RtfError, RtfReader, RtfWriter};
use crate::documents::{
    CharacterFormat, DocumentSnapshot, LineSpacingRule, LineStackingStrategy, ParagraphFormat,
    ParagraphSnapshot, RichTextDocument,
};
use crate::rich_text::{EditKind, EditorSelection, RichTextEditor};
use ferroui_base::media::{
    BaselineAlignment, Brushes, FlowDirection, FontFamily, FontStretch, FontStyle, FontWeight,
    MediaCollection, PenLineCap, TextAlignment, TextDecoration, TextDecorationCollection,
    TextDecorationLocation, TextDecorationUnit, TextDecorations,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::Thickness;

fn empty_paragraph_format() -> ParagraphFormat {
    ParagraphFormat {
        character: CharacterFormat {
            language: Some(CultureInfo::get_culture_info("pl-PL")),
            font_family: Some(FontFamily::new("Empty paragraph font")),
            font_size: Some(24.0),
            font_style: Some(FontStyle::Italic),
            font_weight: Some(FontWeight::Bold),
            font_stretch: Some(FontStretch::Normal),
            foreground: Some(Some(Brushes::red())),
            background: Some(None),
            text_decorations: Some(Some(TextDecorationCollection::from_items([
                TextDecorations::underline().get(0),
                TextDecorations::strikethrough().get(0),
            ]))),
            baseline_alignment: Some(BaselineAlignment::Baseline),
            letter_spacing: Some(0.0),
            ..Default::default()
        },
        text_alignment: Some(TextAlignment::Center),
        flow_direction: Some(FlowDirection::RightToLeft),
        line_spacing_rule: Some(LineSpacingRule::Exactly),
        line_spacing: Some(30.0),
        line_height: Some(30.0),
        line_stacking_strategy: Some(LineStackingStrategy::BlockLineHeight),
        text_indent: Some(3.0),
        left_indent: Some(-4.0),
        right_indent: Some(6.0),
        margin: Some(Thickness::new(0.0, 8.0, 0.0, 12.0)),
        default_tab_stop: Some(64.0),
        ..Default::default()
    }
}

fn round_trip(snapshot: &DocumentSnapshot) -> DocumentSnapshot {
    RtfReader::read_str(&RtfWriter::write(snapshot).unwrap()).unwrap()
}

#[test]
fn rich_rtf_standalone_empty_paragraph_keeps_character_and_paragraph_defaults() {
    let _scope = Dispatcher::unit_test_scope();
    let source = DocumentSnapshot {
        paragraphs: vec![ParagraphSnapshot {
            format: empty_paragraph_format(),
            inlines: Vec::new(),
        }],
        ..Default::default()
    };
    let decoded = round_trip(&source);
    assert_eq!(decoded.text(), "");
    assert_eq!(decoded.paragraphs.len(), 1);
    assert_eq!(decoded.paragraphs[0].format, source.paragraphs[0].format);
    assert_eq!(round_trip(&decoded), decoded);

    let document = RichTextDocument::new();
    document.restore(&decoded).unwrap();
    let editor = RichTextEditor::new(document.clone());
    assert_eq!(
        editor.current_character_format().unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    editor.insert_text("😀", EditKind::Typing).unwrap();
    let inserted = document.character_format_at(2).unwrap();
    assert_eq!(inserted.font_weight, Some(FontWeight::Bold));
    assert_eq!(inserted.font_style, Some(FontStyle::Italic));
    assert_eq!(inserted.font_size, Some(24.0));
    assert_eq!(
        document.snapshot().paragraphs[0].format,
        decoded.paragraphs[0].format
    );
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), decoded);
}

#[test]
fn rich_rtf_intermediate_and_trailing_empty_paragraphs_keep_distinct_defaults() {
    let _scope = Dispatcher::unit_test_scope();
    let mut source = DocumentSnapshot::from_text("before\n\n");
    source.paragraphs[0].format.default_tab_stop = Some(64.0);
    source.paragraphs[1].format = empty_paragraph_format();
    let mut last = empty_paragraph_format();
    last.character.font_weight = Some(FontWeight::Normal);
    last.character.font_style = Some(FontStyle::Normal);
    last.character.font_size = Some(18.0);
    last.text_alignment = Some(TextAlignment::Right);
    last.line_spacing = Some(24.0);
    last.line_height = Some(24.0);
    last.left_indent = Some(2.0);
    last.margin = Some(Thickness::new(0.0, 4.0, 0.0, 6.0));
    source.paragraphs[2].format = last;

    let decoded = round_trip(&source);
    assert_eq!(decoded.text(), "before\n\n");
    assert_eq!(decoded.paragraphs.len(), 3);
    assert_eq!(decoded.paragraphs[1].format, source.paragraphs[1].format);
    assert_eq!(decoded.paragraphs[2].format, source.paragraphs[2].format);

    let document = RichTextDocument::new();
    document.restore(&decoded).unwrap();
    let editor = RichTextEditor::new(document.clone());
    editor
        .set_selection(EditorSelection::caret(document.text_length()))
        .unwrap();
    editor.insert_text("X", EditKind::Typing).unwrap();
    let inserted = document
        .character_format_at(document.text_length())
        .unwrap();
    assert_eq!(inserted.font_weight, Some(FontWeight::Normal));
    assert_eq!(inserted.font_style, Some(FontStyle::Normal));
    assert_eq!(inserted.font_size, Some(18.0));
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), decoded);
}

#[test]
fn rich_rtf_empty_paragraph_character_defaults_follow_mark_scope() {
    let _scope = Dispatcher::unit_test_scope();
    let decoded = RtfReader::read_str(r"{\rtf1\plain\b\fs36\qc{\i\par}\pard\plain\qr}").unwrap();
    assert_eq!(decoded.text(), "\n");
    let first = &decoded.paragraphs[0].format;
    assert_eq!(first.character.font_weight, Some(FontWeight::Bold));
    assert_eq!(first.character.font_style, Some(FontStyle::Italic));
    assert_eq!(first.character.font_size, Some(24.0));
    assert_eq!(first.text_alignment, Some(TextAlignment::Center));
    let last = &decoded.paragraphs[1].format;
    assert_eq!(last.character.font_weight, Some(FontWeight::Normal));
    assert_eq!(last.character.font_style, Some(FontStyle::Normal));
    assert_eq!(last.character.font_size, Some(16.0));
    assert_eq!(last.text_alignment, Some(TextAlignment::Right));

    let unused_group = RtfReader::read_str(r"{\rtf1\plain{\b}}").unwrap();
    assert_eq!(
        unused_group.paragraphs[0].format.character.font_weight,
        Some(FontWeight::Normal)
    );
}

#[test]
fn rich_rtf_logical_alignment_resolves_against_paragraph_direction() {
    let _scope = Dispatcher::unit_test_scope();
    for (direction, alignment, physical) in [
        (
            FlowDirection::LeftToRight,
            TextAlignment::Start,
            TextAlignment::Left,
        ),
        (
            FlowDirection::LeftToRight,
            TextAlignment::End,
            TextAlignment::Right,
        ),
        (
            FlowDirection::RightToLeft,
            TextAlignment::Start,
            TextAlignment::Right,
        ),
        (
            FlowDirection::RightToLeft,
            TextAlignment::End,
            TextAlignment::Left,
        ),
        (
            FlowDirection::RightToLeft,
            TextAlignment::Left,
            TextAlignment::Left,
        ),
        (
            FlowDirection::RightToLeft,
            TextAlignment::Right,
            TextAlignment::Right,
        ),
    ] {
        let mut source = DocumentSnapshot::from_text("text العربية");
        source.paragraphs[0].format.flow_direction = Some(direction);
        source.paragraphs[0].format.text_alignment = Some(alignment);
        let decoded = round_trip(&source);
        assert_eq!(decoded.paragraphs[0].format.text_alignment, Some(physical));
        assert_eq!(decoded.paragraphs[0].format.flow_direction, Some(direction));
        assert_eq!(decoded.text(), source.text());
    }
}

#[test]
fn rich_rtf_content_detected_alignment_requires_resolved_layout_context() {
    let _scope = Dispatcher::unit_test_scope();
    for text in ["Latin", "العربية", ""] {
        let mut source = DocumentSnapshot::from_text(text);
        source.paragraphs[0].format.text_alignment = Some(TextAlignment::DetectFromContent);
        assert!(matches!(
            RtfWriter::write(&source),
            Err(RtfError::UnsupportedFeature(
                "content-detected paragraph alignment requires a layout context"
            ))
        ));
    }
}

#[test]
fn rich_rtf_standard_underline_and_strike_remain_supported() {
    let _scope = Dispatcher::unit_test_scope();
    for decorations in [
        TextDecorations::underline(),
        TextDecorations::strikethrough(),
        TextDecorationCollection::from_items([
            TextDecorations::underline().get(0),
            TextDecorations::strikethrough().get(0),
        ]),
    ] {
        let expected = CharacterFormat {
            text_decorations: Some(Some(decorations)),
            ..Default::default()
        };
        let mut source = DocumentSnapshot::from_text("decorated");
        source.paragraphs[0].inlines[0].format = expected.clone();
        let decoded = round_trip(&source);
        let actual = CharacterFormat {
            text_decorations: decoded.paragraphs[0].inlines[0]
                .format
                .text_decorations
                .clone(),
            ..Default::default()
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn rich_rtf_custom_decoration_payloads_fail_before_data_can_be_lost() {
    let _scope = Dispatcher::unit_test_scope();
    let changes: [(&str, fn(&TextDecoration)); 8] = [
        ("stroke", |value| value.set_stroke(Some(Brushes::red()))),
        ("thickness unit", |value| {
            value.set_stroke_thickness_unit(TextDecorationUnit::Pixel)
        }),
        ("dash array", |value| {
            value.set_stroke_dash_array(Some(MediaCollection::from_items([2.0, 1.0])))
        }),
        ("dash offset", |value| value.set_stroke_dash_offset(1.0)),
        ("thickness", |value| value.set_stroke_thickness(2.0)),
        ("line cap", |value| {
            value.set_stroke_line_cap(PenLineCap::Round)
        }),
        ("offset", |value| value.set_stroke_offset(1.0)),
        ("offset unit", |value| {
            value.set_stroke_offset_unit(TextDecorationUnit::Pixel)
        }),
    ];
    for location in [
        TextDecorationLocation::Underline,
        TextDecorationLocation::Strikethrough,
    ] {
        for (name, change) in changes {
            let decoration = TextDecoration::new();
            decoration.set_location(location);
            change(&decoration);
            let mut source = DocumentSnapshot::from_text("decorated");
            source.paragraphs[0].inlines[0].format.text_decorations =
                Some(Some(TextDecorationCollection::from_items([decoration])));
            let before = source.clone();
            assert!(
                matches!(
                    RtfWriter::write(&source),
                    Err(RtfError::UnsupportedFeature("custom text decorations"))
                ),
                "{location:?}: {name}"
            );
            assert_eq!(source, before);
        }
    }
}
