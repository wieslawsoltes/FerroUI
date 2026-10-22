//! Cross-paragraph RTF fields retain one common-model identity and result span.
//! Paragraph-owned inline hyperlinks remain navigation projections of that field.

use super::{RtfError, RtfLimits, RtfReader, RtfWriter};
use crate::documents::{
    CharacterFormat, DocumentLinkField, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind,
    RichTextDocument,
};
use crate::rich_text::{TextSetOptions, TextStreamOptions};
use crate::RichEditBox;
use ferroui_base::media::{FontStyle, FontWeight, TextAlignment};
use ferroui_base::threading::Dispatcher;

fn link(inline: &InlineSnapshot) -> (&[InlineSnapshot], &str, Option<&str>) {
    match &inline.kind {
        InlineSnapshotKind::Hyperlink {
            inlines,
            navigate_uri,
            target_name,
            ..
        } => (
            inlines,
            navigate_uri.as_deref().unwrap(),
            target_name.as_deref(),
        ),
        _ => panic!("expected an inline hyperlink"),
    }
}

fn links(document: &DocumentSnapshot) -> usize {
    document
        .paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.inlines)
        .filter(|inline| matches!(inline.kind, InlineSnapshotKind::Hyperlink { .. }))
        .count()
}

fn projected_ids(document: &DocumentSnapshot) -> Vec<u64> {
    fn collect(inlines: &[InlineSnapshot], result: &mut Vec<u64>) {
        for inline in inlines {
            if let InlineSnapshotKind::Hyperlink {
                field_id: Some(id), ..
            } = &inline.kind
            {
                result.push(*id);
            }
            if let Some(children) = inline.children() {
                collect(children, result);
            }
        }
    }
    let mut result = Vec::new();
    for paragraph in &document.paragraphs {
        collect(&paragraph.inlines, &mut result);
    }
    result
}

#[test]
fn rich_rtf_field_across_paragraphs_retains_targets_metrics_and_surrounding_text() {
    let _scope = Dispatcher::unit_test_scope();
    let source = concat!(
        r#"{\rtf1 prefix {\field{\*\fldinst HYPERLINK "https://example.test" \\t "_blank"}"#,
        r"{\fldrslt {\b first}\par\qc second\par\par tail}} suffix}"
    );
    let document = RtfReader::read_str(source).unwrap();
    assert_eq!(document.text(), "prefix first\nsecond\n\ntail suffix");
    assert_eq!(document.paragraphs.len(), 4);
    assert_eq!(links(&document), 3);
    assert_eq!(document.link_fields.len(), 1);
    let field = &document.link_fields[0];
    assert_eq!(field.id, 1);
    assert_eq!(field.range, 7..25);
    assert_eq!(field.target, "https://example.test");
    assert_eq!(field.target_name.as_deref(), Some("_blank"));
    assert!(!field.has_sentinel);
    assert_eq!(document.next_link_field_id, 2);
    assert!(projected_ids(&document).iter().all(|id| *id == 1));
    assert_eq!(document.paragraphs[0].inlines[0].text(), "prefix ");
    assert_eq!(document.paragraphs[3].inlines[1].text(), " suffix");
    let first = link(&document.paragraphs[0].inlines[1]);
    assert_eq!(first.0[0].format.font_weight, Some(FontWeight::Bold));
    for (paragraph, index) in [(0, 1), (1, 0), (3, 0)] {
        let (_, uri, target) = link(&document.paragraphs[paragraph].inlines[index]);
        assert_eq!(uri, "https://example.test");
        assert_eq!(target, Some("_blank"));
    }
    assert_eq!(
        document.paragraphs[1].format.text_alignment,
        Some(TextAlignment::Center)
    );
    assert_eq!(
        document.paragraphs[3].format.text_alignment,
        Some(TextAlignment::Center)
    );
    assert!(document.paragraphs[2].inlines.is_empty());
    assert!(document.validate().is_ok());
}

#[test]
fn rich_rtf_field_empty_edges_do_not_create_invisible_hyperlinks() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RtfReader::read_str(concat!(
        r#"{\rtf1{\field{\*\fldinst HYPERLINK "https://example.test"}"#,
        r"{\fldrslt\par middle\par}}after}"
    ))
    .unwrap();
    assert_eq!(document.text(), "\nmiddle\nafter");
    assert_eq!(links(&document), 1);
    assert_eq!(document.link_fields.len(), 1);
    assert_eq!(document.link_fields[0].range, 0..8);
    assert_eq!(projected_ids(&document), vec![1]);
    assert!(document.paragraphs[0].inlines.is_empty());
    assert_eq!(
        link(&document.paragraphs[1].inlines[0]).1,
        "https://example.test"
    );
    assert!(matches!(
        document.paragraphs[2].inlines[0].kind,
        InlineSnapshotKind::Run(_)
    ));
}

#[test]
fn rich_rtf_field_paragraph_separators_count_toward_the_text_limit() {
    let _scope = Dispatcher::unit_test_scope();
    let source = concat!(
        r#"{\rtf1{\field{\*\fldinst HYPERLINK "x"}"#,
        r"{\fldrslt abcdefghijklmnopqrstuvwxyz12345\par x}}}"
    );
    assert_eq!(
        RtfReader::read_with_limits(
            source.as_bytes(),
            RtfLimits {
                max_text_length: 32,
                ..Default::default()
            }
        ),
        Err(RtfError::LimitExceeded("text length"))
    );
    let document = RtfReader::read_with_limits(
        source.as_bytes(),
        RtfLimits {
            max_text_length: 33,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(document.text_length(), 33);
    assert_eq!(links(&document), 2);
}

#[test]
fn rich_rtf_field_paragraph_segments_round_trip_through_canonical_rtf() {
    let _scope = Dispatcher::unit_test_scope();
    let original = RtfReader::read_str(concat!(
        r#"{\rtf1{\field{\*\fldinst HYPERLINK "https://example.test"}"#,
        r"{\fldrslt\b first\par\qr\i second}}}"
    ))
    .unwrap();
    let encoded = RtfWriter::write(&original).unwrap();
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(encoded.matches("\\field").count(), 1);
    assert_eq!(restored.link_fields, original.link_fields);
    assert_eq!(restored.text(), original.text());
    assert_eq!(links(&restored), 2);
    assert_eq!(
        restored.paragraphs[1].format.text_alignment,
        Some(TextAlignment::Right)
    );
    for paragraph in &restored.paragraphs {
        assert_eq!(link(&paragraph.inlines[0]).1, "https://example.test");
    }
    assert_eq!(
        RtfReader::read_str(&RtfWriter::write(&restored).unwrap()).unwrap(),
        restored
    );
}

#[test]
fn rich_rtf_field_metadata_expansion_is_bounded_across_the_whole_document() {
    let _scope = Dispatcher::unit_test_scope();
    let field = format!(
        r#"{{\field{{\*\fldinst HYPERLINK "{}"}}{{\fldrslt a\par b\par c}}}}"#,
        "x".repeat(128)
    );
    let source = format!(r"{{\rtf1{field}{field}}}");
    assert!(source.len() < 512);
    assert_eq!(
        RtfReader::read_with_limits(
            source.as_bytes(),
            RtfLimits {
                max_bytes: 512,
                ..Default::default()
            }
        ),
        Err(RtfError::LimitExceeded("field metadata expansion"))
    );
    let accepted = RtfReader::read_with_limits(
        source.as_bytes(),
        RtfLimits {
            max_bytes: 1024,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(links(&accepted), 6);
    assert_eq!(accepted.link_fields.len(), 2);
    assert_eq!(accepted.text(), "a\nb\nca\nb\nc");
    // Each of the two fields retains its target once in the common record
    // and once per three paragraph-local navigation projections: 8 * 128.
    assert_eq!(
        RtfReader::read_with_limits(
            source.as_bytes(),
            RtfLimits {
                max_bytes: 1023,
                ..Default::default()
            }
        ),
        Err(RtfError::LimitExceeded("field metadata expansion"))
    );

    // The stream adapter passes its byte policy into the codec and parses the
    // whole snapshot before cancelling an existing preview or editing history.
    let control = RichEditBox::new();
    control.set_text("before").unwrap();
    control.editor().set_preedit("preview", None).unwrap();
    let snapshot = control.document().snapshot();
    let selection = control.editor().selection();
    let revision = control.document().revision();
    assert!(control
        .text_document()
        .load_from_stream(
            TextSetOptions::FORMAT_RTF,
            &mut source.as_bytes(),
            TextStreamOptions {
                max_bytes: 512,
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(control.document().snapshot(), snapshot);
    assert_eq!(control.document().revision(), revision);
    assert_eq!(control.editor().selection(), selection);
    assert!(control.editor().is_composing());
}

#[test]
fn rich_rtf_fields_with_equal_targets_remain_distinct_when_adjacent() {
    let _scope = Dispatcher::unit_test_scope();
    let source = concat!(
        r#"{\rtf1{\field{\*\fldinst HYPERLINK "x"}{\fldrslt a}}"#,
        r#"{\field{\*\fldinst HYPERLINK "x"}{\fldrslt b}}}"#,
    );
    let original = RtfReader::read_str(source).unwrap();
    assert_eq!(original.text(), "ab");
    assert_eq!(original.link_fields.len(), 2);
    assert_eq!(original.link_fields[0].range, 0..1);
    assert_eq!(original.link_fields[1].range, 1..2);
    assert_ne!(original.link_fields[0].id, original.link_fields[1].id);
    assert_eq!(projected_ids(&original), vec![1, 2]);
    let encoded = RtfWriter::write(&original).unwrap();
    assert_eq!(encoded.matches("\\field").count(), 2);
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(restored.link_fields, original.link_fields);
    assert_eq!(projected_ids(&restored), vec![1, 2]);
}

#[test]
fn rich_rtf_separator_only_field_retains_target_and_frame_without_invisible_inline() {
    let _scope = Dispatcher::unit_test_scope();
    let original = RtfReader::read_str(concat!(
        r#"{\rtf1 before{\field{\*\fldinst HYPERLINK "x" \\t "frame"}"#,
        r"{\fldrslt\par\par}}after}",
    ))
    .unwrap();
    assert_eq!(original.text(), "before\n\nafter");
    assert_eq!(links(&original), 0);
    assert!(projected_ids(&original).is_empty());
    assert_eq!(original.link_fields.len(), 1);
    assert_eq!(original.link_fields[0].range, 6..8);
    assert_eq!(
        original.link_fields[0].target_name.as_deref(),
        Some("frame")
    );
    let encoded = RtfWriter::write(&original).unwrap();
    assert_eq!(encoded.matches("\\field").count(), 1);
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(restored.text(), original.text());
    assert_eq!(restored.link_fields, original.link_fields);
    assert_eq!(links(&restored), 0);
}

#[test]
fn rich_rtf_cross_paragraph_field_closure_preserves_suffix_metrics_and_nested_formats() {
    let _scope = Dispatcher::unit_test_scope();
    let mut original = RtfReader::read_str(concat!(
        r#"{\rtf1\qr\fs18 P{\field{\*\fldinst HYPERLINK "x"}"#,
        r"{\fldrslt {\b a}\par\qc\fs30 {\i b}}}{\b Z}}",
    ))
    .unwrap();
    // Ancestor containers can start before a field or surround only one of
    // its paragraph projections. They cannot dictate RTF field group scope.
    let inlines = std::mem::take(&mut original.paragraphs[0].inlines);
    original.paragraphs[0].inlines = vec![InlineSnapshot {
        format: CharacterFormat {
            font_style: Some(FontStyle::Italic),
            ..Default::default()
        },
        kind: InlineSnapshotKind::Span(inlines),
    }];
    let before = RichTextDocument::new();
    before.restore(&original).unwrap();
    let encoded = RtfWriter::write(&original).unwrap();
    assert_eq!(encoded.matches("\\field").count(), 1);
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(restored.text(), "Pa\nbZ");
    assert_eq!(restored.link_fields, original.link_fields);
    assert_eq!(
        restored.paragraphs[0].format.text_alignment,
        Some(TextAlignment::Right)
    );
    assert_eq!(
        restored.paragraphs[1].format.text_alignment,
        Some(TextAlignment::Center)
    );
    let after = RichTextDocument::new();
    after.restore(&restored).unwrap();
    for offset in [0, 1, 3, 4] {
        let expected = before.character_format_at(offset).unwrap();
        let actual = after.character_format_at(offset).unwrap();
        assert_eq!(
            actual.font_weight, expected.font_weight,
            "weight at {offset}"
        );
        assert_eq!(actual.font_style, expected.font_style, "style at {offset}");
        assert_eq!(actual.font_size, expected.font_size, "size at {offset}");
    }
    assert_eq!(
        RtfReader::read_str(&RtfWriter::write(&restored).unwrap()).unwrap(),
        restored
    );
}

#[test]
fn rich_rtf_field_slices_preserve_one_identity_across_empty_paragraphs() {
    let _scope = Dispatcher::unit_test_scope();
    let source = RtfReader::read_str(concat!(
        r#"{\rtf1 P{\field{\*\fldinst HYPERLINK "x" \\t "frame"}"#,
        r"{\fldrslt a\par\par b}}Z}",
    ))
    .unwrap();
    let model = RichTextDocument::new();
    model.restore(&source).unwrap();
    let field = source.link_fields[0].clone();
    for (selection, expected_text, expected_range) in [
        (1..5, "a\n\nb", 0..4),
        (2..4, "\n\n", 0..2),
        (3..5, "\nb", 0..2),
        (0..3, "Pa\n", 1..3),
    ] {
        let fragment = model.slice(selection).unwrap();
        assert_eq!(fragment.text(), expected_text);
        assert_eq!(fragment.link_fields.len(), 1);
        assert_eq!(fragment.link_fields[0].id, field.id);
        assert_eq!(fragment.link_fields[0].range, expected_range);
        assert_eq!(fragment.link_fields[0].target_name, field.target_name);
        assert_eq!(fragment.next_link_field_id, source.next_link_field_id);
        let encoded = RtfWriter::write(&fragment).unwrap();
        assert_eq!(encoded.matches("\\field").count(), 1);
        let restored = RtfReader::read_str(&encoded).unwrap();
        assert_eq!(restored.text(), expected_text);
        assert_eq!(restored.link_fields[0].range, expected_range);
        assert_eq!(restored.link_fields[0].target_name, field.target_name);
    }
    let empty = model.slice(2..2).unwrap();
    assert!(empty.link_fields.is_empty());
}

#[test]
fn rich_rtf_rejects_unrepresentable_field_state_without_dropping_metadata() {
    let _scope = Dispatcher::unit_test_scope();
    let mut document =
        RtfReader::read_str(r#"{\rtf1{\field{\*\fldinst HYPERLINK "x"}{\fldrslt text}}}"#).unwrap();
    document.link_fields[0].has_sentinel = true;
    let expected = document.clone();
    assert_eq!(
        RtfWriter::write(&document),
        Err(RtfError::UnsupportedFeature(
            "hyperlink sentinel RTF encoding"
        )),
    );
    assert_eq!(document, expected);

    assert_eq!(
        RtfReader::read_str(r#"{\rtf1{\field{\*\fldinst HYPERLINK "x"}{\fldrslt }}}"#),
        Err(RtfError::UnsupportedFeature("empty hyperlink field result")),
    );
    assert_eq!(
        RtfReader::read_str(concat!(
            r#"{\rtf1{\field{\*\fldinst HYPERLINK "x"}{\fldrslt a"#,
            r#"{\field{\*\fldinst HYPERLINK "y"}{\fldrslt b}}}}}"#,
        )),
        Err(RtfError::UnsupportedFeature("nested hyperlink fields")),
    );
}

#[test]
fn rich_rtf_legacy_inline_links_reject_empty_and_nested_field_export() {
    let _scope = Dispatcher::unit_test_scope();
    let mut legacy =
        RtfReader::read_str(r#"{\rtf1{\field{\*\fldinst HYPERLINK "x"}{\fldrslt text}}}"#).unwrap();
    legacy.link_fields.clear();
    legacy.next_link_field_id = 1;
    if let InlineSnapshotKind::Hyperlink { field_id, .. } =
        &mut legacy.paragraphs[0].inlines[0].kind
    {
        *field_id = None;
    }
    let mut empty = legacy.clone();
    if let InlineSnapshotKind::Hyperlink { inlines, .. } = &mut empty.paragraphs[0].inlines[0].kind
    {
        inlines.clear();
    }
    assert_eq!(
        RtfWriter::write(&empty),
        Err(RtfError::UnsupportedFeature("empty hyperlink field result"))
    );
    let inner = legacy.paragraphs[0].inlines[0].clone();
    if let InlineSnapshotKind::Hyperlink { inlines, .. } = &mut legacy.paragraphs[0].inlines[0].kind
    {
        *inlines = vec![inner];
    }
    assert_eq!(
        RtfWriter::write(&legacy),
        Err(RtfError::UnsupportedFeature("nested hyperlink fields"))
    );
}

#[test]
fn rich_rtf_field_spans_and_limits_count_the_platform_structured_soft_break_length() {
    let _scope = Dispatcher::unit_test_scope();
    let soft_break = InlineSnapshot {
        format: Default::default(),
        kind: InlineSnapshotKind::LineBreak,
    };
    let width = soft_break.text_length();
    // Keep the visible text longer than the independently bounded instruction
    // destination, so the exact/one-less policy isolates result text accounting.
    let padding = "abcdefghijklmnopqrst";
    let expected = format!("{padding}p{}qa{}bZ", soft_break.text(), soft_break.text());
    let source = r#"{\rtf1 abcdefghijklmnopqrstp\line q{\field{\*\fldinst HYPERLINK "x"}{\fldrslt a\line b}}Z}"#;
    let original = RtfReader::read_str(source).unwrap();
    assert_eq!(original.text(), expected);
    assert_eq!(
        original.link_fields[0].range,
        padding.len() + 2 + width..padding.len() + 4 + 2 * width
    );
    assert_eq!(original.text_length(), padding.len() + 5 + 2 * width);
    let encoded = RtfWriter::write(&original).unwrap();
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(restored.text(), expected);
    assert_eq!(restored.link_fields, original.link_fields);
    let exact = RtfLimits {
        max_text_length: original.text_length(),
        ..Default::default()
    };
    assert!(RtfReader::read_with_limits(source.as_bytes(), exact).is_ok());
    assert_eq!(
        RtfReader::read_with_limits(
            source.as_bytes(),
            RtfLimits {
                max_text_length: exact.max_text_length - 1,
                ..exact
            }
        ),
        Err(RtfError::LimitExceeded("text length")),
    );
}

#[test]
fn rich_rtf_literal_run_line_separators_use_unicode_without_platform_expansion() {
    let _scope = Dispatcher::unit_test_scope();
    let text = "A\nB\rC\u{2028}D😀e\u{301}";
    let mut source = DocumentSnapshot::from_text("");
    source.paragraphs[0].inlines = vec![InlineSnapshot::run(text)];
    let encoded = RtfWriter::write(&source).unwrap();
    assert!(encoded.contains("\\u10?"));
    assert!(encoded.contains("\\u13?"));
    assert!(encoded.contains("\\u8232?"));
    assert!(!encoded.contains("\\line"));
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(restored.text(), text);
    assert_eq!(restored.paragraphs.len(), 1);
    assert_eq!(restored.text_length(), text.encode_utf16().count());
    assert!(
        matches!(&restored.paragraphs[0].inlines[0].kind, InlineSnapshotKind::Run(value) if value == text)
    );
    let exact = RtfLimits {
        max_text_length: source.text_length(),
        ..Default::default()
    };
    assert!(RtfReader::read_with_limits(encoded.as_bytes(), exact).is_ok());
    assert_eq!(
        RtfReader::read_with_limits(
            encoded.as_bytes(),
            RtfLimits {
                max_text_length: exact.max_text_length - 1,
                ..exact
            }
        ),
        Err(RtfError::LimitExceeded("text length")),
    );
}

#[test]
fn rich_rtf_unicode_field_targets_frames_and_results_preserve_literal_scalars() {
    let _scope = Dispatcher::unit_test_scope();
    let text = "😀e\u{301}\t\n\u{2028}end";
    let target = "https://example.test/😀e\u{301}/\t\n\r\u{2028}";
    let frame = "frame😀\t\nrow\rnext\u{2028}e\u{301}";
    let mut source = DocumentSnapshot::from_text("");
    // Literal tabs need an explicit spacing policy when no layout host supplies
    // the automatic tab interval. This fixture exercises scalar preservation.
    source.paragraphs[0].format.default_tab_stop = Some(48.0);
    source.paragraphs[0].inlines = vec![InlineSnapshot::run(text)];
    source
        .add_link_field(DocumentLinkField {
            id: 1,
            range: 0..text.encode_utf16().count(),
            target: target.into(),
            target_name: Some(frame.into()),
            has_sentinel: false,
        })
        .unwrap();
    let encoded = RtfWriter::write(&source).unwrap();
    assert_eq!(encoded.matches("\\field").count(), 1);
    assert!(!encoded.contains("\\line"));
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(restored.text(), text);
    assert_eq!(restored.link_fields, source.link_fields);
    let (_, uri, name) = link(&restored.paragraphs[0].inlines[0]);
    assert_eq!(uri, target);
    assert_eq!(name, Some(frame));
    assert_eq!(
        RtfReader::read_str(&RtfWriter::write(&restored).unwrap()).unwrap(),
        restored
    );
}

#[test]
fn rich_rtf_quoted_unc_and_switch_like_targets_remain_literal_data() {
    let _scope = Dispatcher::unit_test_scope();
    for (expected, source) in [
        (
            r"\\server\share\file",
            r#"{\rtf1{\field{\*\fldinst HYPERLINK "\\\\server\\share\\file"}{\fldrslt x}}}"#,
        ),
        (
            r"\t",
            r#"{\rtf1{\field{\*\fldinst HYPERLINK "\\t"}{\fldrslt x}}}"#,
        ),
        (
            r"\l",
            r#"{\rtf1{\field{\*\fldinst HYPERLINK "\\l"}{\fldrslt x}}}"#,
        ),
        (
            r"\unknown",
            r#"{\rtf1{\field{\*\fldinst HYPERLINK "\\unknown"}{\fldrslt x}}}"#,
        ),
    ] {
        let document = RtfReader::read_str(source).unwrap();
        assert_eq!(document.text(), "x");
        assert_eq!(document.link_fields[0].target, expected);
        let encoded = RtfWriter::write(&document).unwrap();
        let restored = RtfReader::read_str(&encoded).unwrap();
        assert_eq!(restored.link_fields, document.link_fields);
        assert_eq!(link(&restored.paragraphs[0].inlines[0]).1, expected);
    }
    // Actual unquoted switches retain their instruction meaning.
    let bookmark = RtfReader::read_str(
        r#"{\rtf1{\field{\*\fldinst HYPERLINK \\l "mark" \\t "\\t"}{\fldrslt x}}}"#,
    )
    .unwrap();
    assert_eq!(bookmark.link_fields[0].target, "#mark");
    assert_eq!(bookmark.link_fields[0].target_name.as_deref(), Some(r"\t"));
    assert_eq!(
        RtfReader::read_str(r#"{\rtf1{\field{\*\fldinst HYPERLINK \\unknown "x"}{\fldrslt x}}}"#),
        Err(RtfError::UnsupportedFeature("hyperlink field switch"))
    );
}
