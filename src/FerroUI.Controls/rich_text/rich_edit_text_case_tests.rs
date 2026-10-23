use super::*;
use crate::documents::{ParagraphSnapshot, UnderlineStyle};
use crate::Border;
use ferroui_base::media::{FontStyle, FontWeight, TextAlignment, TextDecorationLocation};
use ferroui_base::threading::Dispatcher;

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

#[test]
fn rich_text_case_native_values_and_unicode_expansions_preserve_directed_selection() {
    let _scope = Dispatcher::unit_test_scope();
    assert_eq!(LetterCase::default(), LetterCase::Lower);
    assert_eq!(LetterCase::Lower as i32, 0);
    assert_eq!(LetterCase::Upper as i32, 1);
    let document = services("Straße Ω");
    document.selection().set_range(6, 4).unwrap();
    let range = document.document_range().unwrap();
    assert!(range.change_case(LetterCase::Upper).unwrap());
    assert_eq!(document.text(), "STRASSE Ω");
    assert_eq!(range.range(), 0..9);
    assert_eq!(document.selection().selection(), EditorSelection::new(7, 4));
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "Straße Ω");
    assert_eq!(document.selection().selection(), EditorSelection::new(6, 4));
    assert!(!document.can_undo());
    assert!(document.redo().unwrap());
    assert_eq!(document.text(), "STRASSE Ω");
    assert_eq!(document.selection().selection(), EditorSelection::new(7, 4));
}

#[test]
fn rich_text_case_contextual_sigma_crosses_formatted_run_boundaries() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = DocumentSnapshot {
        paragraphs: vec![ParagraphSnapshot {
            format: ParagraphFormat::default(),
            inlines: vec![
                InlineSnapshot::run("Ο"),
                InlineSnapshot {
                    format: CharacterFormat { font_weight: Some(FontWeight::Bold), ..Default::default() },
                    kind: InlineSnapshotKind::Span(vec![InlineSnapshot::run("Σ")]),
                },
                InlineSnapshot::run(" ΟΣΑ İ"),
            ],
        }],
        ..Default::default()
    };
    let model = RichTextDocument::new();
    model.restore(&snapshot).unwrap();
    let document = RichEditTextDocument::new(model);
    document.document_range().unwrap().change_case(LetterCase::Lower).unwrap();
    assert_eq!(document.text(), "ος οσα i\u{307}");
    let after = document.document().snapshot();
    let styled = &after.paragraphs[0].inlines[1];
    assert_eq!(styled.format.font_weight, Some(FontWeight::Bold));
    assert_eq!(styled.children().unwrap()[0].kind, InlineSnapshotKind::Run("ς".to_owned()));
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), snapshot);
}

#[test]
fn rich_text_case_preserves_nested_links_soft_breaks_paragraph_formats_and_controls() {
    let _scope = Dispatcher::unit_test_scope();
    let child = Border::new();
    let fragment = DocumentSnapshot {
        paragraphs: vec![
            ParagraphSnapshot {
                format: ParagraphFormat {
                    text_alignment: Some(TextAlignment::Right),
                    ..Default::default()
                },
                inlines: vec![
                    InlineSnapshot {
                        format: CharacterFormat::default(),
                        kind: InlineSnapshotKind::Hyperlink {
                            field_id: None,
                            behavior: Default::default(),
                            inlines: vec![InlineSnapshot {
                                format: CharacterFormat::default(),
                                kind: InlineSnapshotKind::Italic(vec![InlineSnapshot::run("aß")]),
                            }],
                            navigate_uri: Some("https://example.test/keep".to_owned()),
                            target_name: Some("target".to_owned()),
                            is_enabled: true,
                            is_tab_stop: true,
                            tab_index: 7,
                            is_visited: true,
                            underline_style: UnderlineStyle::Single,
                        },
                    },
                    InlineSnapshot {
                        format: CharacterFormat::default(),
                        kind: InlineSnapshotKind::LineBreak,
                    },
                    InlineSnapshot {
                        format: CharacterFormat::default(),
                        kind: InlineSnapshotKind::Embedded(Some(child.clone().upcast())),
                    },
                    InlineSnapshot::run("z"),
                ],
            },
            ParagraphSnapshot {
                format: ParagraphFormat::default(),
                inlines: vec![InlineSnapshot::run("é")],
            },
        ],
        ..Default::default()
    };
    let model = RichTextDocument::new();
    model.restore(&fragment).unwrap();
    // Semantic defaults are effective presentation, not local assignments.
    // Restore must preserve the exact fragment while resolving underline and
    // italic through the live element tree.
    let before = model.snapshot();
    assert_eq!(before, fragment);
    let effective = model.character_format_at(1).unwrap();
    let link = &before.paragraphs[0].inlines[0];
    assert!(matches!(
        &link.kind,
        InlineSnapshotKind::Hyperlink {
            navigate_uri: Some(uri),
            target_name: Some(target),
            is_enabled: true,
            is_tab_stop: true,
            tab_index: 7,
            is_visited: true,
            underline_style: UnderlineStyle::Single,
            ..
        } if uri == "https://example.test/keep" && target == "target"
    ));
    assert!(effective
        .text_decorations
        .as_ref()
        .and_then(Option::as_ref)
        .is_some_and(|decorations| decorations
            .list()
            .snapshot()
            .iter()
            .any(|decoration| decoration.location() == TextDecorationLocation::Underline)));
    let italic = &link.children().unwrap()[0];
    assert!(matches!(italic.kind, InlineSnapshotKind::Italic(_)));
    assert_eq!(italic.format.font_style, None);
    assert_eq!(effective.font_style, Some(FontStyle::Italic));
    let document = RichEditTextDocument::new(model);
    document
        .document_range()
        .unwrap()
        .change_case(LetterCase::Upper)
        .unwrap();
    let after = document.document().snapshot();
    let first = &after.paragraphs[0];
    assert_eq!(first.format, fragment.paragraphs[0].format);
    let mut expected = before.clone();
    let InlineSnapshotKind::Run(text) =
        &mut expected.paragraphs[0].inlines[0].children_mut().unwrap()[0]
            .children_mut()
            .unwrap()[0]
            .kind
    else {
        panic!("the hyperlink contains an italic run");
    };
    assert_eq!(text, "aß");
    *text = "ASS".to_owned();
    expected.paragraphs[0].inlines[3].kind = InlineSnapshotKind::Run("Z".to_owned());
    expected.paragraphs[1].inlines[0].kind = InlineSnapshotKind::Run("É".to_owned());
    assert_eq!(after, expected);
    let effective = document.document().character_format_at(1).unwrap();
    assert_eq!(effective.font_style, Some(FontStyle::Italic));
    assert!(effective
        .text_decorations
        .as_ref()
        .and_then(Option::as_ref)
        .is_some_and(|decorations| decorations
            .list()
            .snapshot()
            .iter()
            .any(|decoration| decoration.location() == TextDecorationLocation::Underline)));
    assert!(matches!(
        first.inlines[1].kind,
        InlineSnapshotKind::LineBreak
    ));
    assert!(
        matches!(&first.inlines[2].kind, InlineSnapshotKind::Embedded(Some(control)) if control == &child.clone().upcast::<crate::Control>())
    );
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn rich_text_case_partial_range_maps_astral_and_interior_caret_and_keeps_typing_format() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("[a😀ßx]");
    document.selection().set_range(5, 5).unwrap();
    document.selection().set_character_format(&CharacterFormat { font_weight: Some(FontWeight::Bold), ..Default::default() }).unwrap();
    document.clear_undo_redo_history();
    let range = document.get_range(1, 6).unwrap();
    assert!(range.change_case(LetterCase::Upper).unwrap());
    assert_eq!(document.text(), "[A😀SSX]");
    assert_eq!(range.range(), 1..7);
    assert_eq!(document.selection().selection(), EditorSelection::caret(6));
    assert_eq!(document.selection().character_format().unwrap().font_weight, Some(FontWeight::Bold));
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "[a😀ßx]");
    assert_eq!(document.selection().selection(), EditorSelection::caret(5));
}

#[test]
fn rich_text_case_noop_preserves_revision_redo_and_live_endpoints() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("already lower");
    document.get_range(0, 0).unwrap().set_text("x").unwrap();
    document.undo().unwrap();
    let range = document.document_range().unwrap();
    let revision = document.document().revision();
    let pointer = document.document().create_pointer(4, TextGravity::Forward).unwrap();
    assert!(!range.change_case(LetterCase::Lower).unwrap());
    assert!(!document.get_range(4, 4).unwrap().change_case(LetterCase::Upper).unwrap());
    assert_eq!(document.document().revision(), revision);
    assert_eq!(pointer.offset(), 4);
    assert_eq!(range.range(), 0..13);
    assert!(document.can_redo());
    assert!(!document.can_undo());
}

#[test]
fn rich_text_case_readonly_and_length_failure_preserve_document_and_composition() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ß");
    document.editor().set_max_length(1);
    document.editor().set_preedit("ß", None).unwrap();
    let before = document.document().snapshot();
    let revision = document.document().revision();
    let range = document.document_range().unwrap();
    assert!(matches!(range.change_case(LetterCase::Upper), Err(EditError::Document(DocumentError::InvalidFormat(_)))));
    assert!(document.editor().is_composing());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.editor().max_length(), 1);
    document.editor().set_is_read_only(true);
    assert!(matches!(range.change_case(LetterCase::Lower), Err(EditError::ReadOnly)));
    assert!(document.editor().is_composing());
}

#[test]
fn rich_text_case_lowered_limit_does_not_truncate_nonexpanding_change() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ABCDEF");
    document.editor().set_max_length(2);
    assert!(document.document_range().unwrap().change_case(LetterCase::Lower).unwrap());
    assert_eq!(document.text(), "abcdef");
    assert_eq!(document.editor().max_length(), 2);
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "ABCDEF");
    assert_eq!(document.editor().max_length(), 2);
}

#[test]
fn rich_text_case_settles_preedit_before_capturing_committed_snapshot() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("word");
    document.selection().set_range(0, 4).unwrap();
    document.editor().set_preedit("preview", None).unwrap();
    let range = document.document_range().unwrap();
    assert!(range.change_case(LetterCase::Upper).unwrap());
    assert_eq!(document.text(), "WORD");
    assert!(!document.editor().is_composing());
    assert_eq!(range.range(), 0..4);
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "word");
    assert!(!document.can_undo());
}

#[test]
fn rich_text_case_rollback_restores_text_selection_and_history() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("aßc");
    document.selection().set_range(2, 1).unwrap();
    let before = document.document().snapshot();
    let result: Result<(), EditError> = document.edit_group(|document| {
        document.document_range()?.change_case(LetterCase::Upper)?;
        Err(DocumentError::InvalidFormat("rollback").into())
    });
    assert!(result.is_err());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), EditorSelection::new(2, 1));
    assert!(!document.can_undo());
    assert!(!document.can_redo());
}
