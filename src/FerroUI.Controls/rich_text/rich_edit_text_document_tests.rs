// The UseLfAndUseCrLf case preserves the pinned native TOM fixture's assertions.
// Other cases are independent contract, transaction and Unicode regressions.
use super::*;
use crate::documents::{BlockCollection, Hyperlink, Paragraph, ParagraphSnapshot, Run};
use ferroui_base::media::{FontWeight, TextAlignment, TextDecorationCollection};
use ferroui_base::threading::Dispatcher;

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

#[path = "rich_edit_text_empty_fragment_tests.rs"]
mod empty_fragment_tests;

#[test]
fn use_lf_and_use_cr_lf() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("");
    document
        .set_text_with_options(TextSetOptions::NONE, "Text")
        .unwrap();
    assert_eq!(document.get_text(TextGetOptions::NONE).unwrap(), "Text\r");
    assert_eq!(
        document
            .get_text(TextGetOptions::USE_CRLF | TextGetOptions::ALLOW_FINAL_EOP)
            .unwrap(),
        "Text\r\n"
    );
    assert_eq!(
        document
            .get_text(TextGetOptions::USE_LF | TextGetOptions::ALLOW_FINAL_EOP)
            .unwrap(),
        "Text\n"
    );
    assert!(matches!(
        document.get_text(TextGetOptions::USE_LF | TextGetOptions::USE_CRLF),
        Err(TextOperationError::InvalidOptions(_))
    ));
    assert_eq!(document.text(), "Text");
    assert_eq!(document.text_length(), 4);
}

#[test]
fn retrieval_options_distinguish_story_terminal_mark_from_live_ranges() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("first\r\nsecond\n");
    assert_eq!(
        document.get_text(TextGetOptions::ADJUST_CRLF).unwrap(),
        "first\rsecond\r"
    );
    assert_eq!(
        document.get_text(TextGetOptions::USE_LF).unwrap(),
        "first\nsecond\n"
    );
    let range = document.get_range(6, 12).unwrap();
    let before = range.range();
    assert_eq!(
        range.get_text(TextGetOptions::ALLOW_FINAL_EOP).unwrap(),
        "second"
    );
    assert_eq!(range.range(), before);
    document.set_text("").unwrap();
    assert_eq!(document.get_text(TextGetOptions::ADJUST_CRLF).unwrap(), "");
    assert_eq!(document.get_text(TextGetOptions::NONE).unwrap(), "\r");
}

#[test]
fn retrieval_adjusts_variation_selector_start_without_moving_live_range() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("x\u{FE0F}z");
    let range = document.get_range(1, 2).unwrap();
    assert_eq!(range.get_text(TextGetOptions::NONE).unwrap(), "\u{FE0F}");
    assert_eq!(
        range.get_text(TextGetOptions::ADJUST_CRLF).unwrap(),
        "x\u{FE0F}"
    );
    assert_eq!(range.range(), 1..2);
}

#[test]
fn unknown_and_unimplemented_options_are_explicit_and_do_not_edit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("keep");
    let before = document.document().snapshot();
    for options in [
        TextGetOptions::USE_OBJECT_TEXT,
        TextGetOptions::INCLUDE_NUMBERING,
        TextGetOptions::NO_HIDDEN,
    ] {
        assert!(matches!(
            document.get_text(options),
            Err(TextOperationError::UnsupportedOptions { .. })
        ));
    }
    for options in [TextSetOptions::UNICODE_BIDI, TextSetOptions::UNHIDE] {
        assert!(matches!(
            document.set_text_with_options(options, "bad"),
            Err(TextOperationError::UnsupportedOptions { .. })
        ));
        assert!(matches!(
            document
                .document_range()
                .unwrap()
                .set_text_with_options(options, "bad"),
            Err(TextOperationError::UnsupportedOptions { .. })
        ));
    }
    assert!(matches!(
        document.get_text(TextGetOptions::from_bits_retain(0x80000000)),
        Err(TextOperationError::InvalidOptions(_))
    ));
    assert!(matches!(
        document.get_text(TextGetOptions::FORMAT_RTF | TextGetOptions::USE_LF),
        Err(TextOperationError::InvalidOptions(_))
    ));
    assert!(matches!(
        document.set_text_with_options(TextSetOptions::from_bits_retain(0x80000000), "bad"),
        Err(TextOperationError::InvalidOptions(_))
    ));
    assert!(matches!(
        document.set_text_with_options(TextSetOptions::APPLY_RTF_DOCUMENT_DEFAULTS, "bad"),
        Err(TextOperationError::InvalidOptions(_))
    ));
    assert!(matches!(
        document.set_text_with_options(
            TextSetOptions::FORMAT_RTF | TextSetOptions::APPLY_RTF_DOCUMENT_DEFAULTS,
            "{\\rtf1 x}"
        ),
        Err(TextOperationError::UnsupportedOptions { .. })
    ));
    assert_eq!(document.document().snapshot(), before);
    assert!(!document.can_undo());
}

#[test]
fn document_defaults_are_shared_independent_value_snapshots() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("");
    let another_facade = RichEditTextDocument::from_editor(document.editor());
    let decorations = TextDecorationCollection::parse("Underline").unwrap();
    document
        .set_default_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            text_decorations: Some(Some(decorations.clone())),
            ..Default::default()
        })
        .unwrap();
    decorations.list().clear();
    let copy = document.get_default_character_format();
    copy.text_decorations.unwrap().unwrap().list().clear();
    let retained = another_facade.get_default_character_format();
    assert_eq!(retained.font_weight, Some(FontWeight::Bold));
    assert_eq!(retained.text_decorations.unwrap().unwrap().len(), 1);
    assert_eq!(document.text(), "");
}

#[test]
fn defaults_apply_to_empty_typing_and_each_replacement_paragraph() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("");
    document
        .set_default_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    document
        .set_default_paragraph_format(&ParagraphFormat {
            text_alignment: Some(TextAlignment::Center),
            text_indent: Some(8.0),
            ..Default::default()
        })
        .unwrap();
    document.selection().type_text("typed").unwrap();
    assert_eq!(
        document
            .document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        document.document().paragraphs()[0].text_alignment(),
        TextAlignment::Center
    );
    document.set_text("first\nsecond").unwrap();
    for paragraph in document.document().paragraphs() {
        assert_eq!(paragraph.text_alignment(), TextAlignment::Center);
        assert_eq!(paragraph.text_indent(), 8.0);
        assert_eq!(paragraph.font_weight(), FontWeight::Bold);
    }
}

#[test]
fn changing_defaults_preserves_existing_format_until_document_replacement() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("existing");
    let original = document.document().snapshot();
    document
        .set_default_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(document.document().snapshot(), original);
    assert_ne!(
        document
            .document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    document
        .set_text_with_options(TextSetOptions::NONE, "replacement")
        .unwrap();
    assert_eq!(
        document
            .document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), original);
    assert_eq!(
        document.get_default_character_format().font_weight,
        Some(FontWeight::Bold)
    );
    assert!(document.undo().unwrap());
    assert_eq!(document.get_default_character_format().font_weight, None);
}

#[test]
fn default_format_updates_rollback_with_a_failed_group_and_reject_invalid_values() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("");
    let snapshot = document.document().snapshot();
    assert!(document
        .edit_group(|document| {
            document.set_default_character_format(&CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            })?;
            document.set_default_paragraph_format(&ParagraphFormat {
                text_alignment: Some(TextAlignment::Right),
                ..Default::default()
            })?;
            document.get_range(0, 999)?;
            Ok(())
        })
        .is_err());
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(
        document.get_default_character_format(),
        CharacterFormat::default()
    );
    assert_eq!(
        document.get_default_paragraph_format(),
        ParagraphFormat::default()
    );
    assert!(!document.can_undo());
    assert!(document
        .set_default_character_format(&CharacterFormat {
            font_size: Some(f64::NAN),
            ..Default::default()
        })
        .is_err());
    document.editor().set_is_read_only(true);
    assert!(matches!(
        document.set_default_paragraph_format(&ParagraphFormat::default()),
        Err(EditError::ReadOnly)
    ));
}

#[test]
fn explicit_text_limit_flag_truncates_on_scalar_boundaries_and_none_grows_limit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("old");
    document.editor().set_max_length(3);
    document
        .set_text_with_options(TextSetOptions::CHECK_TEXT_LIMIT, "A😀B")
        .unwrap();
    assert_eq!(document.text(), "A😀");
    assert_eq!(document.editor().max_length(), 3);
    document
        .set_text_with_options(TextSetOptions::NONE, "A😀BC")
        .unwrap();
    assert_eq!(document.text(), "A😀BC");
    assert_eq!(document.editor().max_length(), 5);
}

#[test]
fn failed_outer_group_restores_the_limit_grown_by_set_text() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("old");
    document.editor().set_max_length(3);
    assert!(document
        .edit_group(|document| {
            document
                .set_text_with_options(TextSetOptions::NONE, "expanded")
                .map_err(|error| match error {
                    TextOperationError::Edit(error) => error,
                    _ => panic!("The plain insertion options are valid"),
                })?;
            document.get_range(0, 999)?;
            Ok(())
        })
        .is_err());
    assert_eq!(document.text(), "old");
    assert_eq!(document.editor().max_length(), 3);
    assert!(!document.can_undo());
}

#[test]
fn ranged_text_options_preserve_independent_selection_and_honor_retained_length() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abXY");
    document.selection().set_range(4, 3).unwrap();
    document.editor().set_max_length(5);
    let range = document.get_range(2, 4).unwrap();
    range
        .set_text_with_options(TextSetOptions::CHECK_TEXT_LIMIT, "😀ZZ")
        .unwrap();
    assert_eq!(document.text(), "ab😀Z");
    assert_eq!(range.range(), 2..5);
    assert_eq!(document.selection().selection(), EditorSelection::caret(5));
    document.undo().unwrap();
    assert_eq!(document.text(), "abXY");
    assert_eq!(document.selection().selection(), EditorSelection::new(4, 3));
}

#[test]
fn rtf_options_replace_first_paragraph_metadata_and_round_trip_formatting() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("old");
    document
        .set_text_with_options(
            TextSetOptions::FORMAT_RTF,
            "{\\rtf1\\ansi\\qc\\b Bold\\par\\qr tail}",
        )
        .unwrap();
    assert_eq!(document.text(), "Bold\ntail");
    assert_eq!(
        document.document().paragraphs()[0].text_alignment(),
        TextAlignment::Center
    );
    assert_eq!(
        document.document().paragraphs()[1].text_alignment(),
        TextAlignment::Right
    );
    let serialized = document.get_text(TextGetOptions::FORMAT_RTF).unwrap();
    let restored = services("");
    restored
        .set_text_with_options(TextSetOptions::FORMAT_RTF, &serialized)
        .unwrap();
    assert_eq!(restored.text(), document.text());
    assert_eq!(
        restored
            .document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        restored.document().paragraphs()[0].text_alignment(),
        TextAlignment::Center
    );
    document.undo().unwrap();
    assert_eq!(document.text(), "old");
}

#[test]
fn widened_rtf_bytes_are_not_accidentally_reencoded_as_utf8() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("");
    document
        .set_text_with_options(TextSetOptions::FORMAT_RTF, "{\\rtf1\\ansi caf\u{e9}}")
        .unwrap();
    assert_eq!(document.text(), "café");
    let before = document.document().snapshot();
    assert!(document
        .set_text_with_options(TextSetOptions::FORMAT_RTF, "{\\rtf1 Γ}")
        .is_err());
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn malformed_rtf_and_invalid_flags_leave_composition_preview_untouched() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("committed");
    document.editor().set_preedit("preview", None).unwrap();
    let before = document.document().snapshot();
    let selection = document.selection().selection();
    assert!(matches!(
        document.set_text_with_options(TextSetOptions::FORMAT_RTF, "{\\rtf1 truncated"),
        Err(TextOperationError::Rtf(_))
    ));
    assert!(document
        .document_range()
        .unwrap()
        .set_text_with_options(TextSetOptions::from_bits_retain(2), "bad")
        .is_err());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), selection);
    assert!(document.editor().is_composing());
}

#[test]
fn invalid_format_and_fragment_values_preserve_the_composition_preview() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("committed");
    document.editor().set_preedit("preview", None).unwrap();
    let before = document.document().snapshot();
    let selection = document.selection().selection();
    let invalid = CharacterFormat {
        font_size: Some(f64::NAN),
        ..Default::default()
    };
    assert!(document.selection().set_character_format(&invalid).is_err());
    assert!(document
        .selection()
        .set_paragraph_format(&ParagraphFormat {
            text_indent: Some(f64::INFINITY),
            ..Default::default()
        })
        .is_err());
    let mut fragment = DocumentSnapshot::from_text("bad");
    fragment.paragraphs[0].format.character = invalid;
    assert!(document
        .document_range()
        .unwrap()
        .set_snapshot(&fragment)
        .is_err());
    assert!(document
        .editor()
        .insert_fragment(&fragment, EditKind::Paste)
        .is_err());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), selection);
    assert!(document.editor().is_composing());
}

fn text_under_links(inlines: &[InlineSnapshot], linked: bool, result: &mut Vec<(String, bool)>) {
    for inline in inlines {
        match &inline.kind {
            InlineSnapshotKind::Run(text) => result.push((text.clone(), linked)),
            InlineSnapshotKind::Hyperlink { inlines, .. } => {
                text_under_links(inlines, true, result)
            }
            _ => {
                if let Some(children) = inline.children() {
                    text_under_links(children, linked, result);
                }
            }
        }
    }
}

#[test]
fn unlink_insertion_keeps_neighboring_hyperlinks_but_excludes_inserted_text() {
    let _scope = Dispatcher::unit_test_scope();
    let paragraph = Paragraph::new();
    paragraph
        .inlines()
        .add(Hyperlink::with_text("ab", Some("https://example.com")));
    let blocks = BlockCollection::new();
    blocks.add(paragraph);
    let document = RichEditTextDocument::new(RichTextDocument::with_blocks(blocks));
    document
        .get_range(1, 1)
        .unwrap()
        .set_text_with_options(TextSetOptions::UNLINK, "!")
        .unwrap();
    assert_eq!(document.text(), "a!b");
    let snapshot = document.document().snapshot();
    let mut pieces = Vec::new();
    text_under_links(&snapshot.paragraphs[0].inlines, false, &mut pieces);
    assert!(pieces.iter().any(|(text, linked)| text == "!" && !linked));
    assert!(
        pieces
            .iter()
            .filter(|(_, linked)| *linked)
            .map(|(text, _)| text.as_str())
            .collect::<String>()
            == "ab"
    );
}

#[test]
fn independent_collapsed_range_keeps_its_own_typing_format() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.selection().set_range(3, 3).unwrap();
    let range = document.get_range(1, 1).unwrap();
    range
        .set_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    let copy = range.get_clone().unwrap();
    assert!(
        matches!(copy.character_format().unwrap(), TextFormatValue::Uniform(format) if format.font_weight == Some(FontWeight::Bold))
    );
    assert_ne!(
        document.selection().character_format().unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    range.set_text("X").unwrap();
    assert_eq!(document.text(), "aXbc");
    assert_eq!(
        document
            .document()
            .character_format_at(2)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(4));
    assert_eq!(range.range(), 1..2);
    document.undo().unwrap();
    assert_eq!(document.text(), "abc");
}

#[test]
fn character_format_scan_does_not_count_a_missing_embedded_control() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("");
    document
        .document()
        .restore(&DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                format: ParagraphFormat::default(),
                inlines: vec![
                    InlineSnapshot::run("A"),
                    InlineSnapshot {
                        format: CharacterFormat::default(),
                        kind: InlineSnapshotKind::Embedded(None),
                    },
                    InlineSnapshot {
                        format: CharacterFormat {
                            font_weight: Some(FontWeight::Bold),
                            ..Default::default()
                        },
                        kind: InlineSnapshotKind::Run("B".to_owned()),
                    },
                ],
            }],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(document.text(), "AB");
    assert!(matches!(
        document
            .document_range()
            .unwrap()
            .character_format()
            .unwrap(),
        TextFormatValue::Mixed
    ));
    document.selection().select_all().unwrap();
    assert!(matches!(
        document.selection().character_format_value().unwrap(),
        TextFormatValue::Mixed
    ));
}

#[test]
fn paragraph_query_compares_effective_values_instead_of_local_assignment_presence() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("one\ntwo");
    let paragraphs = document.document().paragraphs();
    paragraphs[1].set_text_alignment(paragraphs[0].text_alignment());
    assert!(matches!(
        document
            .document_range()
            .unwrap()
            .paragraph_format()
            .unwrap(),
        TextFormatValue::Uniform(_)
    ));
    paragraphs[1].set_text_indent(12.0);
    assert!(matches!(
        document
            .document_range()
            .unwrap()
            .paragraph_format()
            .unwrap(),
        TextFormatValue::Mixed
    ));
    assert!(matches!(
        document
            .get_range(0, 4)
            .unwrap()
            .paragraph_format()
            .unwrap(),
        TextFormatValue::Uniform(_)
    ));
}

#[test]
fn paragraph_query_does_not_mix_character_only_differences() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("one\ntwo");
    document.document().paragraphs()[1].set_font_weight(FontWeight::Bold);
    let range = document.document_range().unwrap();
    assert!(matches!(
        range.character_format().unwrap(),
        TextFormatValue::Mixed
    ));
    assert!(
        matches!(range.paragraph_format().unwrap(), TextFormatValue::Uniform(format)
        if format.character == CharacterFormat::default())
    );
}

#[test]
fn rtf_range_export_materializes_inherited_host_formatting() {
    let _scope = Dispatcher::unit_test_scope();
    let view = RichTextBlock::new();
    view.set_font_size(24.0);
    let paragraph = Paragraph::new();
    paragraph.inlines().add(Run::with_text(Some("abc")));
    view.blocks().add(paragraph);
    let document = RichEditTextDocument::new(view.document());
    let rtf = document
        .get_range(1, 2)
        .unwrap()
        .get_text(TextGetOptions::FORMAT_RTF)
        .unwrap();
    let decoded = RtfReader::read_str(&rtf).unwrap();
    let restored = RichTextDocument::new();
    restored.restore(&decoded).unwrap();
    assert_eq!(restored.text(), "b");
    assert_eq!(
        restored.character_format_at(0).unwrap().font_size,
        Some(24.0)
    );
}

#[test]
fn get_character_utf32_uses_end_relative_offsets_and_decodes_both_surrogates() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("A😀B");
    let range = document.get_range(0, 1).unwrap();
    assert_eq!(range.get_character_utf32(-1), 'A' as u32);
    assert_eq!(range.get_character_utf32(0), '😀' as u32);
    assert_eq!(range.get_character_utf32(1), '😀' as u32);
    assert_eq!(range.get_character_utf32(2), 'B' as u32);
    assert_eq!(range.get_character_utf32(3), 0);
    assert_eq!(range.get_character_utf32(i32::MIN), 0);
    assert_eq!(range.get_character_utf32(i32::MAX), 0);
    assert_eq!(range.range(), 0..1);
}

#[test]
fn range_endpoint_setters_collapse_crossings_and_reject_surrogate_interiors() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("A😀B");
    let range = document.get_range(0, 1).unwrap();
    range.set_start_position(3).unwrap();
    assert_eq!(range.range(), 3..3);
    range.set_end_position(1).unwrap();
    assert_eq!(range.range(), 1..1);
    assert!(range.set_start_position(2).is_err());
    assert_eq!(range.range(), 1..1);
    document.selection().set_range(4, 1).unwrap();
    range.match_selection().unwrap();
    assert_eq!(range.range(), 1..4);
    assert!(range.in_story(&document.get_range(0, 0).unwrap()));
    assert!(!range.in_story(&services("A😀B").document_range().unwrap()));
}

#[test]
fn bounded_search_skips_previous_match_and_retains_overlaps_in_both_directions() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("aaaa");
    let range = document.get_range(0, 0).unwrap();
    assert_eq!(
        range
            .find_text_with_length("aa", i32::MAX, FindOptions::CASE)
            .unwrap(),
        2
    );
    assert_eq!(range.range(), 0..2);
    assert_eq!(
        range
            .find_text_with_length("aa", i32::MAX, FindOptions::CASE)
            .unwrap(),
        2
    );
    assert_eq!(range.range(), 1..3);
    assert_eq!(
        range
            .find_text_with_length("aa", i32::MAX, FindOptions::CASE)
            .unwrap(),
        2
    );
    assert_eq!(range.range(), 2..4);
    assert_eq!(
        range
            .find_text_with_length("aa", i32::MIN, FindOptions::CASE)
            .unwrap(),
        2
    );
    assert_eq!(range.range(), 1..3);
    assert_eq!(
        range
            .find_text_with_length("aa", i32::MIN, FindOptions::CASE)
            .unwrap(),
        2
    );
    assert_eq!(range.range(), 0..2);
}

#[test]
fn bounded_search_respects_scan_edges_utf16_and_whole_word_options() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("A😀 alpha alphabet");
    let range = document.get_range(1, 1).unwrap();
    assert_eq!(
        range
            .find_text_with_length("😀", 1, FindOptions::NONE)
            .unwrap(),
        0
    );
    assert_eq!(range.range(), 1..1);
    assert_eq!(
        range
            .find_text_with_length("😀", 2, FindOptions::NONE)
            .unwrap(),
        2
    );
    assert_eq!(range.range(), 1..3);
    range.set_range(3, 3).unwrap();
    assert_eq!(
        range
            .find_text_with_length("ALPHA", 0, FindOptions::WORD)
            .unwrap(),
        5
    );
    assert_eq!(range.range(), 4..9);
    assert_eq!(
        range
            .find_text_with_length("alpha", i32::MAX, FindOptions::WORD)
            .unwrap(),
        0
    );
    assert_eq!(range.range(), 4..9);
    assert!(range
        .find_text_with_length("alpha", 0, FindOptions::from_bits_retain(8))
        .is_err());
}

#[test]
fn extreme_movement_counts_saturate_without_repeated_linear_scans() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services(&format!("{}😀", "x".repeat(4096)));
    let range = document.get_range(0, 0).unwrap();
    assert_eq!(
        range.move_by(TextRangeUnit::Character, i32::MAX).unwrap(),
        4097
    );
    assert_eq!(range.range(), 4098..4098);
    assert_eq!(
        range.move_by(TextRangeUnit::Character, i32::MIN).unwrap(),
        -4097
    );
    assert_eq!(range.range(), 0..0);
    assert_eq!(
        range
            .move_start(TextRangeUnit::Character, i32::MIN)
            .unwrap(),
        0
    );
}

#[test]
fn formatted_text_transfer_and_history_facade_keep_document_identity_separate() {
    let _scope = Dispatcher::unit_test_scope();
    let source = services("bold");
    source
        .document_range()
        .unwrap()
        .set_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    let target = services("replace");
    target.set_undo_limit(1);
    assert_eq!(target.undo_limit(), 1);
    target
        .document_range()
        .unwrap()
        .set_formatted_text(&source.document_range().unwrap())
        .unwrap();
    assert_eq!(target.text(), "bold");
    assert_eq!(
        target
            .document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    source.set_text("changed").unwrap();
    assert_eq!(target.text(), "bold");
    assert!(target.can_undo());
    target.clear_undo_redo_history();
    assert!(!target.can_undo());
    assert!(!target.can_redo());
}
