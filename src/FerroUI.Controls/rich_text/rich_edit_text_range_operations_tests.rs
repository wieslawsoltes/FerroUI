// Independent regressions of the pinned native contract and Rust scalar safety.
use super::*;
use crate::documents::{ParagraphSnapshot, UnderlineStyle};
use crate::test_support::test_scope;
use crate::Border;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Brushes, FontStyle, FontWeight, TextAlignment};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Size};

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

#[test]
fn rich_range_delete_zero_removes_selection_without_resolving_units() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀bc");
    document.selection().set_range(5, 4).unwrap();
    let range = document.get_range(1, 3).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Cluster, 0).unwrap(), 1);
    assert_eq!(document.text(), "abc");
    assert_eq!(range.range(), 1..1);
    assert_eq!(document.selection().selection(), EditorSelection::new(3, 2));
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "a😀bc");
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 4));
    assert!(!document.can_undo());
    assert!(document.redo().unwrap());
    assert_eq!(document.text(), "abc");
}

#[test]
fn rich_range_delete_nonempty_span_counts_once_then_uses_direction() {
    let _scope = Dispatcher::unit_test_scope();
    for (count, expected, position) in [(3, "abg", 2), (-3, "efg", 0)] {
        let document = services("abcdefg");
        let range = document.get_range(2, 4).unwrap();
        assert_eq!(range.delete(TextRangeUnit::Character, count).unwrap(), 3);
        assert_eq!(document.text(), expected);
        assert_eq!(range.range(), position..position);
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "abcdefg");
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_range_delete_scalar_counts_preserve_utf16_and_extreme_requests() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀b");
    let range = document.get_range(0, 0).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Character, 2).unwrap(), 2);
    assert_eq!(document.text(), "b");
    assert!(document.undo().unwrap());
    range.set_range(4, 4).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Character, i32::MIN).unwrap(), 3);
    assert_eq!(document.text(), "");
    assert_eq!(range.range(), 0..0);
    assert!(document.undo().unwrap());
    range.set_range(0, 0).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Character, i32::MAX).unwrap(), 3);
    assert_eq!(document.text(), "");
}

#[test]
fn rich_range_delete_word_units_include_following_whitespace() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("one  two three");
    let range = document.get_range(2, 2).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Word, 1).unwrap(), 1);
    assert_eq!(document.text(), "ontwo three");
    assert_eq!(range.range(), 2..2);
    assert_eq!(range.delete(TextRangeUnit::Word, 1).unwrap(), 1);
    assert_eq!(document.text(), "onthree");
    range.set_range(7, 7).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Word, -1).unwrap(), 1);
    assert_eq!(document.text(), "");
}

#[test]
fn rich_range_delete_noop_preserves_redo_revision_and_composition() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    let range = document.get_range(3, 3).unwrap();
    let revision = document.document().revision();
    assert_eq!(range.delete(TextRangeUnit::Character, 1).unwrap(), 0);
    assert_eq!(range.delete(TextRangeUnit::Cluster, 0).unwrap(), 0);
    assert_eq!(document.document().revision(), revision);
    assert!(document.can_redo());
    document.selection().set_range(1, 1).unwrap();
    document.editor().set_preedit("preview", Some(0)).unwrap();
    let snapshot = document.document().snapshot();
    let range = document.get_range(0, 0).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Character, -1).unwrap(), 0);
    assert_eq!(range.delete(TextRangeUnit::Cluster, 0).unwrap(), 0);
    assert!(document.editor().is_composing());
    assert_eq!(document.document().snapshot(), snapshot);
}

#[test]
fn rich_range_delete_unavailable_layout_and_readonly_preserve_preview() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.selection().set_range(1, 1).unwrap();
    document.editor().set_preedit("😀", Some(2)).unwrap();
    let range = document.get_range(0, 1).unwrap();
    let snapshot = document.document().snapshot();
    assert!(range.delete(TextRangeUnit::Cluster, 2).is_err());
    assert_eq!(document.document().snapshot(), snapshot);
    assert!(document.editor().is_composing());
    document.editor().set_is_read_only(true);
    assert!(matches!(
        range.delete(TextRangeUnit::Character, 1),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(document.document().snapshot(), snapshot);
    assert!(document.editor().is_composing());
}

fn formatted_paragraphs() -> RichEditTextDocument {
    let document = services("abcd\nefgh");
    document
        .get_range(0, 4)
        .unwrap()
        .set_paragraph_format(&ParagraphFormat {
            text_alignment: Some(TextAlignment::Left),
            left_indent: Some(12.0),
            ..Default::default()
        })
        .unwrap();
    document
        .get_range(5, 9)
        .unwrap()
        .set_paragraph_format(&ParagraphFormat {
            text_alignment: Some(TextAlignment::Right),
            left_indent: Some(28.0),
            ..Default::default()
        })
        .unwrap();
    document.clear_undo_redo_history();
    document
}

#[test]
fn rich_range_delete_paragraph_mark_and_partial_paragraph_keep_native_boundaries() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted_paragraphs();
    let snapshot = document.document().snapshot();
    document
        .get_range(4, 5)
        .unwrap()
        .delete(TextRangeUnit::Character, 0)
        .unwrap();
    assert_eq!(document.text(), "abcdefgh");
    assert_eq!(document.document().paragraphs()[0].left_indent(), 12.0);
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), snapshot);
    let range = document.get_range(2, 5).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Character, 0).unwrap(), 1);
    assert_eq!(document.text(), "ab\nefgh");
    assert_eq!(range.range(), 2..2);
    assert_eq!(document.document().paragraphs().len(), 2);
    assert_eq!(document.document().paragraphs()[1].left_indent(), 28.0);
}

#[test]
fn rich_range_delete_join_uses_surviving_following_paragraph_format() {
    let _scope = Dispatcher::unit_test_scope();
    for (range, expected) in [(2..7, "abgh"), (0..5, "efgh")] {
        let document = formatted_paragraphs();
        let snapshot = document.document().snapshot();
        document
            .get_range(range.start, range.end)
            .unwrap()
            .delete(TextRangeUnit::Character, 0)
            .unwrap();
        assert_eq!(document.text(), expected);
        assert_eq!(document.document().paragraphs()[0].left_indent(), 28.0);
        assert_eq!(
            document.document().paragraphs()[0].text_alignment(),
            TextAlignment::Right
        );
        assert!(document.undo().unwrap());
        assert_eq!(document.document().snapshot(), snapshot);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_range_delete_whole_paragraph_drops_its_assigned_character_defaults() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    view.set_font_size(18.0);
    let first = Paragraph::with_text("a");
    first.set_font_size(32.0);
    view.blocks().add(first);
    view.blocks().add(Paragraph::with_text("bc"));
    let document = RichEditTextDocument::new(view.document());
    let before = document.document().snapshot();
    document
        .get_range(0, 2)
        .unwrap()
        .delete(TextRangeUnit::Character, 0)
        .unwrap();
    assert_eq!(document.text(), "bc");
    assert_eq!(
        document.document().snapshot().paragraphs[0]
            .format
            .character
            .font_size,
        None
    );
    assert_eq!(
        document
            .document()
            .character_format_at(0)
            .unwrap()
            .font_size,
        Some(18.0)
    );
    view.set_font_size(24.0);
    assert_eq!(
        document
            .document()
            .character_format_at(0)
            .unwrap()
            .font_size,
        Some(24.0)
    );
    document.undo().unwrap();
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn rich_range_delete_character_count_includes_an_explicit_paragraph_mark() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted_paragraphs();
    let range = document.get_range(2, 2).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Character, 3).unwrap(), 3);
    assert_eq!(document.text(), "abefgh");
    assert_eq!(document.document().paragraphs()[0].left_indent(), 12.0);
    assert_eq!(range.range(), 2..2);
}

#[test]
fn rich_range_delete_shaped_cluster_removes_ligatures_and_reflows_after_first_span() {
    let _scope = test_scope();
    let text = TextTestScope::new();
    text.shaper().add_ligature("fi");
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text("fXice"));
    view.measure(Size::new(300.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 100.0));
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    let range = document.get_range(1, 2).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Cluster, 2).unwrap(), 2);
    assert_eq!(document.text(), "fce");
    document.undo().unwrap();
    assert_eq!(document.text(), "fXice");
    document
        .get_range(1, 2)
        .unwrap()
        .delete(TextRangeUnit::Character, 0)
        .unwrap();
    assert_eq!(document.text(), "fice");
    assert_eq!(
        document
            .get_range(0, 0)
            .unwrap()
            .delete(TextRangeUnit::Cluster, 1)
            .unwrap(),
        1
    );
    assert_eq!(document.text(), "ce");
}

#[test]
fn rich_range_delete_line_after_first_span_uses_new_wrapped_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let source = "one two three four five six";
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text(source));
    view.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    view.measure(Size::new(60.0, 1000.0));
    view.arrange(Rect::new(0.0, 0.0, 60.0, 1000.0));
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    let expected_view = RichTextBlock::new();
    expected_view
        .blocks()
        .add(Paragraph::with_text(&source[4..]));
    expected_view.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    expected_view.measure(Size::new(60.0, 1000.0));
    expected_view.arrange(Rect::new(0.0, 0.0, 60.0, 1000.0));
    let expected_end = expected_view.line_metrics()[1].start;
    let range = document.get_range(0, 4).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Line, 2).unwrap(), 2);
    assert_eq!(document.text(), &source[4 + expected_end..]);
    assert_eq!(range.range(), 0..0);
    document.undo().unwrap();
    assert_eq!(document.text(), source);
    assert!(!document.can_undo());
}

#[test]
fn rich_range_delete_keeps_nested_formatting_and_embedded_control_identity() {
    let _scope = Dispatcher::unit_test_scope();
    let child = Border::new();
    let snapshot = DocumentSnapshot {
        paragraphs: vec![ParagraphSnapshot {
            inlines: vec![
                InlineSnapshot {
                    format: CharacterFormat {
                        font_style: Some(FontStyle::Italic),
                        ..Default::default()
                    },
                    kind: InlineSnapshotKind::Hyperlink {
                        field_id: None,
                        behavior: Default::default(),
                        inlines: vec![InlineSnapshot::run("ab😀cd")],
                        navigate_uri: Some("https://example.test/kept".to_owned()),
                        target_name: None,
                        is_enabled: true,
                        is_tab_stop: true,
                        tab_index: 4,
                        is_visited: false,
                        underline_style: UnderlineStyle::Single,
                    },
                },
                InlineSnapshot {
                    format: CharacterFormat::default(),
                    kind: InlineSnapshotKind::Embedded(Some(child.clone().upcast())),
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    let model = RichTextDocument::new();
    model.restore(&snapshot).unwrap();
    // Installing a semantic Hyperlink supplies its default underline. Undo must
    // restore the actual pre-edit model, including that canonical formatting.
    let before = model.snapshot();
    let document = RichEditTextDocument::new(model);
    document
        .get_range(2, 4)
        .unwrap()
        .delete(TextRangeUnit::Character, 0)
        .unwrap();
    assert_eq!(document.text(), "abcd\u{fffc}");
    let after = document.document().snapshot();
    assert_eq!(
        after.paragraphs[0].inlines[0].format.font_style,
        Some(FontStyle::Italic)
    );
    assert!(matches!(&after.paragraphs[0].inlines[0].kind,
        InlineSnapshotKind::Hyperlink { navigate_uri: Some(uri), .. } if uri == "https://example.test/kept"));
    assert!(matches!(&after.paragraphs[0].inlines[1].kind,
        InlineSnapshotKind::Embedded(Some(kept)) if *kept == child.clone().upcast::<crate::Control>()));
    document.undo().unwrap();
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn rich_range_delete_selection_facade_collapses_directed_selection_and_undoes_once() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcdef");
    document.selection().set_range(4, 2).unwrap();
    assert_eq!(
        document
            .selection()
            .delete_by(TextRangeUnit::Character, -2)
            .unwrap(),
        2
    );
    assert_eq!(document.text(), "aef");
    assert_eq!(document.selection().selection(), EditorSelection::caret(1));
    document.undo().unwrap();
    assert_eq!(document.text(), "abcdef");
    assert_eq!(document.selection().selection(), EditorSelection::new(4, 2));
    assert!(!document.can_undo());
}

#[test]
fn rich_range_delete_selection_collapses_when_its_paragraph_mark_is_retained() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted_paragraphs();
    document.selection().set_range(5, 2).unwrap();
    assert_eq!(
        document
            .selection()
            .delete_by(TextRangeUnit::Character, 0)
            .unwrap(),
        1
    );
    assert_eq!(document.text(), "ab\nefgh");
    assert_eq!(document.selection().selection(), EditorSelection::caret(2));
    document.undo().unwrap();
    assert_eq!(document.text(), "abcd\nefgh");
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 2));
    assert!(!document.can_undo());
}

#[test]
fn rich_range_delete_selection_recaptures_directed_endpoints_after_layout_callbacks() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (anchor, active, expected, removed) in
        [(2, 2, "abd", 1), (4, 2, "ab", 1), (4, 4, "abcd", 0)]
    {
        let view = RichTextBlock::new();
        view.set_document(RichTextDocument::from_text("abcd"));
        let document = RichEditTextDocument::new(view.document());
        document.editor().set_text_view(Some(view.clone())).unwrap();
        let editor = document.editor();
        let subscription = view.property_changed(move |change| {
            if change.property().name() == "BaselineOffset" {
                editor
                    .set_selection(EditorSelection::new(anchor, active))
                    .unwrap();
            }
        });
        assert_eq!(
            document
                .selection()
                .delete_by(TextRangeUnit::Cluster, 1)
                .unwrap(),
            removed
        );
        assert_eq!(document.text(), expected);
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(anchor.min(active))
        );
        subscription.dispose();
        if removed != 0 {
            document.undo().unwrap();
            assert_eq!(document.text(), "abcd");
            assert_eq!(
                document.selection().selection(),
                EditorSelection::new(anchor, active)
            );
            assert!(!document.can_undo());
        } else {
            assert!(!document.can_undo());
        }
    }
}

#[test]
fn rich_range_delete_failed_outer_transaction_restores_structure_and_history() {
    let _scope = Dispatcher::unit_test_scope();
    let document = formatted_paragraphs();
    let before = document.document().snapshot();
    document.selection().set_range(8, 7).unwrap();
    let result: Result<(), EditError> = document.edit_group(|document| {
        document
            .get_range(2, 7)?
            .delete(TextRangeUnit::Character, 2)?;
        Err(EditError::ReadOnly)
    });
    assert!(result.is_err());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), EditorSelection::new(8, 7));
    assert!(!document.can_undo());
}

#[test]
fn rich_range_delete_layout_callback_preserves_external_model_edits_in_nested_groups() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for nested in [false, true] {
        let view = RichTextBlock::new();
        let first = Paragraph::with_text("a");
        first.set_font_size(32.0);
        view.blocks().add(first);
        view.blocks().add(Paragraph::with_text("bc"));
        view.measure(Size::new(300.0, 100.0));
        view.arrange(Rect::new(0.0, 0.0, 300.0, 100.0));
        let document = RichEditTextDocument::new(view.document());
        document.editor().set_text_view(Some(view.clone())).unwrap();
        document.selection().set_range(4, 4).unwrap();
        document
            .selection()
            .set_character_format(&CharacterFormat {
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            })
            .unwrap();
        assert!(document.can_undo());
        let range = document.get_range(0, 2).unwrap();
        let changed = Rc::new(Cell::new(false));
        let observed = changed.clone();
        let composition_changes = Rc::new(Cell::new(0));
        let observed_composition_changes = composition_changes.clone();
        let _composition_subscription = document.editor().composition_changed(move |_| {
            composition_changes.set(composition_changes.get() + 1);
        });
        let model = document.document();
        let subscription = view.property_changed(move |change| {
            if change.property().name() == "BaselineOffset" && !changed.replace(true) {
                model
                    .replace_text(0..model.text_length(), "external")
                    .unwrap();
            }
        });
        let result = if nested {
            document.edit_group(|_| {
                document.editor().set_preedit("preview", Some(2))?;
                // Even catching the immediate operation's error must not allow
                // the outer transaction to publish stale undo snapshots.
                assert!(matches!(
                    range.delete(TextRangeUnit::Cluster, 2),
                    Err(EditError::ReentrantEdit)
                ));
                Ok(0)
            })
        } else {
            range.delete(TextRangeUnit::Cluster, 2)
        };
        assert!(
            observed.get(),
            "the deletion must rebuild the first baseline"
        );
        assert!(matches!(result, Err(EditError::ReentrantEdit)));
        assert_eq!(document.text(), "external");
        assert!(!document.can_undo());
        assert!(!document.can_redo());
        assert!(!document.editor().is_composing());
        assert_eq!(observed_composition_changes.get(), 0);
        assert_eq!(
            document.editor().typing_format(),
            CharacterFormat::default()
        );
        assert!(document.document().validate_range(range.range()).is_ok());
        subscription.dispose();
    }
}

#[test]
fn rich_range_delete_deliberate_reentrant_error_still_rolls_back() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("original");
    document.selection().set_range(8, 5).unwrap();
    let before = document.document().snapshot();
    let result: Result<(), EditError> = document.edit_group(|document| {
        document
            .get_range(1, 4)?
            .delete(TextRangeUnit::Character, 0)?;
        Err(EditError::ReentrantEdit)
    });
    assert!(matches!(result, Err(EditError::ReentrantEdit)));
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), EditorSelection::new(8, 5));
    assert!(!document.can_undo());
}

#[test]
fn rich_range_publication_callback_preserves_external_edits_and_live_range_rebasing() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    for replace_character in [false, true] {
        let document = services("abcdef");
        document.selection().set_range(6, 6).unwrap();
        document
            .selection()
            .set_character_format(&CharacterFormat {
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            })
            .unwrap();
        let range = document.get_range(3, 3).unwrap();
        let changed = Rc::new(Cell::new(false));
        let observed = changed.clone();
        let model = document.document();
        let callback_model = model.clone();
        let _subscription = model.changed(move |_| {
            if !changed.replace(true) {
                callback_model
                    .replace_text(0..callback_model.text_length(), "q")
                    .unwrap();
            }
        });
        let result = if replace_character {
            range.set_character_scalar('X').map(|_| ())
        } else {
            range.delete(TextRangeUnit::Character, 1).map(|_| ())
        };
        assert!(observed.get());
        assert!(matches!(result, Err(EditError::ReentrantEdit)));
        assert_eq!(document.text(), "q");
        assert_eq!(range.range(), 1..1);
        assert_eq!(document.selection().selection(), EditorSelection::caret(1));
        assert_eq!(
            document.editor().typing_format(),
            CharacterFormat::default()
        );
        assert!(!document.can_undo());
        assert!(!document.can_redo());
    }
}

#[test]
fn rich_range_editor_publication_in_an_outer_batch_discards_history_and_composition_causes() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcdef");
    document.selection().set_range(6, 6).unwrap();
    let range = document.get_range(3, 3).unwrap();
    let model = document.document();
    let revision = model.revision();
    let batch = model.begin_edit();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    let callback_model = model.clone();
    let _subscription = document.editor().changed(move || {
        if !changed.replace(true) {
            callback_model
                .replace_text(0..callback_model.text_length(), "q")
                .unwrap();
        }
    });
    let composition_changes = Rc::new(Cell::new(0));
    let observed_composition_changes = composition_changes.clone();
    let _composition_subscription = document.editor().composition_changed(move |_| {
        composition_changes.set(composition_changes.get() + 1);
    });
    let result = document.edit_group(|_| {
        document.editor().set_preedit("preview", Some(2))?;
        range.set_character_scalar('X')?;
        Ok(())
    });
    assert!(observed.get());
    assert!(matches!(result, Err(EditError::ReentrantEdit)));
    assert_eq!(
        model.revision(),
        revision,
        "the application batch still defers revision delivery"
    );
    assert_eq!(document.text(), "q");
    assert_eq!(range.range(), 1..1);
    assert_eq!(document.selection().selection(), EditorSelection::caret(1));
    assert!(!document.can_undo());
    assert!(!document.can_redo());
    assert!(!document.editor().is_composing());
    assert_eq!(observed_composition_changes.get(), 0);
    drop(batch);
    assert_eq!(document.text(), "q");
    assert!(!document.can_undo());
}

#[test]
fn rich_range_delete_layout_only_reentrancy_rolls_back_the_group() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let first = Paragraph::with_text("a");
    first.set_font_size(32.0);
    view.blocks().add(first);
    view.blocks().add(Paragraph::with_text("bc"));
    view.measure(Size::new(300.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 100.0));
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    let before = document.document().snapshot();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    let weak = view.downgrade();
    let subscription = view.property_changed(move |change| {
        if change.property().name() == "BaselineOffset" && !changed.replace(true) {
            weak.upgrade().unwrap().set_font_size(24.0);
        }
    });
    let range = document.get_range(0, 2).unwrap();
    assert!(matches!(
        range.delete(TextRangeUnit::Cluster, 2),
        Err(EditError::ReentrantEdit)
    ));
    assert!(observed.get());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(range.range(), 0..2);
    assert!(!document.can_undo());
    subscription.dispose();
}

#[test]
fn rich_range_character_reads_exact_utf16_and_complete_scalars() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀\0z");
    let range = document.get_range(1, 1).unwrap();
    assert_eq!(range.character(), 0xd83d);
    assert_eq!(range.character_scalar(), Some('😀'));
    range.set_range(3, 3).unwrap();
    assert_eq!(range.character(), 0);
    assert_eq!(range.character_scalar(), Some('\0'));
    range.set_range(5, 5).unwrap();
    assert_eq!(range.character(), 0);
    assert_eq!(range.character_scalar(), None);
}

#[test]
fn rich_range_character_replaces_first_scalar_and_retains_range_start_and_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcdef");
    document.selection().set_range(6, 3).unwrap();
    let range = document.get_range(1, 4).unwrap();
    assert!(range.set_character_scalar('😀').unwrap());
    assert_eq!(document.text(), "a😀cdef");
    assert_eq!(range.range(), 1..5);
    assert_eq!(document.selection().selection(), EditorSelection::new(7, 4));
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "abcdef");
    assert_eq!(document.selection().selection(), EditorSelection::new(6, 3));
    assert!(!document.can_undo());
    range.set_range(1, 1).unwrap();
    assert!(range.set_character(u16::from(b'B')).unwrap());
    assert_eq!(document.text(), "aBcdef");
    assert_eq!(range.range(), 1..1);
}

#[test]
fn rich_range_character_retains_overwritten_run_format_at_boundaries() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab");
    document
        .get_range(1, 2)
        .unwrap()
        .set_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            font_style: Some(FontStyle::Italic),
            ..Default::default()
        })
        .unwrap();
    document.clear_undo_redo_history();
    let before = document.document().snapshot();
    document
        .get_range(1, 1)
        .unwrap()
        .set_character_scalar('😀')
        .unwrap();
    assert_eq!(document.text(), "a😀");
    let TextFormatValue::Uniform(format) = document
        .get_range(1, 3)
        .unwrap()
        .character_format()
        .unwrap()
    else {
        panic!("replacement format must be uniform");
    };
    assert_eq!(format.font_weight, Some(FontWeight::Bold));
    assert_eq!(format.font_style, Some(FontStyle::Italic));
    document.undo().unwrap();
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn rich_range_character_run_splice_does_not_freeze_inherited_formatting() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    let paragraph = document.document().paragraphs()[0].clone();
    paragraph.set_font_size(18.0);
    document
        .get_range(1, 1)
        .unwrap()
        .set_character_scalar('😀')
        .unwrap();
    assert_eq!(document.text(), "a😀c");
    paragraph.set_font_size(24.0);
    let TextFormatValue::Uniform(format) = document
        .get_range(1, 3)
        .unwrap()
        .character_format()
        .unwrap()
    else {
        panic!("replacement format must be uniform");
    };
    assert_eq!(format.font_size, Some(24.0));
    let after = document.document().snapshot();
    assert_eq!(after.paragraphs[0].inlines[0].format.font_size, None);
}

#[test]
fn rich_range_character_run_and_paragraph_replacements_follow_owner_presentation() {
    let _scope = test_scope();
    for (text, offset, value, expected) in [
        ("ab", 0, 'X', "Xb"),
        ("ab", 0, '\n', "\nb"),
        ("a\nb", 1, 'X', "aXb"),
    ] {
        let view = RichTextBlock::new();
        view.set_font_size(18.0);
        view.set_foreground(Some(Brushes::red()));
        view.set_document(RichTextDocument::from_text(text));
        let document = RichEditTextDocument::new(view.document());
        document
            .get_range(offset, offset)
            .unwrap()
            .set_character_scalar(value)
            .unwrap();
        assert_eq!(document.text(), expected);
        view.set_font_size(24.0);
        view.set_foreground(Some(Brushes::blue()));
        let format = document.document().character_format_at(offset).unwrap();
        assert_eq!(format.font_size, Some(24.0));
        assert_eq!(
            format.foreground,
            Some(Some(
                Brushes::blue() as std::rc::Rc<dyn ferroui_base::media::IBrush>
            ))
        );
        let snapshot = document.document().snapshot();
        for paragraph in &snapshot.paragraphs {
            assert_eq!(paragraph.format.character.font_size, None);
            assert_eq!(paragraph.format.character.foreground, None);
            for inline in &paragraph.inlines {
                assert_eq!(inline.format.font_size, None);
                assert_eq!(inline.format.foreground, None);
            }
        }
    }
}

#[test]
fn rich_range_character_replaces_inline_object_with_its_assigned_container_context() {
    let _scope = test_scope();
    let child = Border::new();
    let snapshot = DocumentSnapshot {
        paragraphs: vec![ParagraphSnapshot {
            inlines: vec![
                InlineSnapshot::run("a"),
                InlineSnapshot {
                    format: CharacterFormat::default(),
                    kind: InlineSnapshotKind::Bold(vec![InlineSnapshot {
                        format: CharacterFormat {
                            font_style: Some(FontStyle::Italic),
                            ..Default::default()
                        },
                        kind: InlineSnapshotKind::Embedded(Some(child.clone().upcast())),
                    }]),
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    let view = RichTextBlock::new();
    view.set_font_size(18.0);
    view.set_foreground(Some(Brushes::red()));
    view.document().restore(&snapshot).unwrap();
    let document = RichEditTextDocument::new(view.document());
    let before = document.document().snapshot();
    document
        .get_range(1, 1)
        .unwrap()
        .set_character_scalar('😀')
        .unwrap();
    assert_eq!(document.text(), "a😀");
    view.set_font_size(24.0);
    view.set_foreground(Some(Brushes::blue()));
    let format = CharacterFormat::effective(
        &document
            .document()
            .element_at(1, TextGravity::Forward)
            .unwrap(),
    );
    assert_eq!(format.font_size, Some(24.0));
    assert_eq!(
        format.foreground,
        Some(Some(
            Brushes::blue() as std::rc::Rc<dyn ferroui_base::media::IBrush>
        ))
    );
    assert_eq!(format.font_weight, Some(FontWeight::Bold));
    assert_eq!(format.font_style, Some(FontStyle::Italic));
    let after = document.document().snapshot();
    assert!(matches!(&after.paragraphs[0].inlines[1].kind,
        InlineSnapshotKind::Bold(children) if matches!(&children[0].kind,
            InlineSnapshotKind::Run(text) if text == "😀")));
    document.undo().unwrap();
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn rich_range_character_paragraph_split_retains_local_inheritance() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.document().paragraphs()[0].set_font_size(18.0);
    document
        .get_range(1, 1)
        .unwrap()
        .set_character_scalar('\n')
        .unwrap();
    assert_eq!(document.text(), "a\nc");
    let paragraphs = document.document().paragraphs();
    assert_eq!(paragraphs.len(), 2);
    for paragraph in &paragraphs {
        assert_eq!(paragraph.font_size(), 18.0);
        paragraph.set_font_size(24.0);
    }
    assert_eq!(
        document
            .document()
            .character_format_at(0)
            .unwrap()
            .font_size,
        Some(24.0)
    );
    assert_eq!(
        document
            .document()
            .character_format_at(2)
            .unwrap()
            .font_size,
        Some(24.0)
    );
    let after = document.document().snapshot();
    for paragraph in &after.paragraphs {
        for inline in &paragraph.inlines {
            assert_eq!(inline.format.font_size, None);
        }
    }
}

#[test]
fn rich_range_character_invalid_utf16_end_and_length_leave_preview_untouched() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀b");
    document.selection().set_range(4, 4).unwrap();
    document.editor().set_preedit("x", Some(1)).unwrap();
    let before = document.document().snapshot();
    assert!(document
        .get_range(0, 0)
        .unwrap()
        .set_character(0xd800)
        .is_err());
    assert!(document
        .get_range(1, 1)
        .unwrap()
        .set_character(u16::from(b'x'))
        .is_err());
    assert!(document
        .get_range(5, 5)
        .unwrap()
        .set_character_scalar('x')
        .is_err());
    document.editor().set_max_length(5);
    assert!(document
        .get_range(0, 0)
        .unwrap()
        .set_character_scalar('😀')
        .is_err());
    assert_eq!(document.document().snapshot(), before);
    assert!(document.editor().is_composing());
    assert!(!document.can_undo());
}

#[test]
fn rich_range_character_noop_preserves_composition_and_redo() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    let revision = document.document().revision();
    assert!(!document
        .get_range(1, 2)
        .unwrap()
        .set_character(u16::from(b'b'))
        .unwrap());
    assert!(document.can_redo());
    assert_eq!(document.document().revision(), revision);
    document.selection().set_range(3, 3).unwrap();
    document.editor().set_preedit("x", Some(1)).unwrap();
    assert!(!document
        .get_range(1, 1)
        .unwrap()
        .set_character_scalar('b')
        .unwrap());
    assert!(document.editor().is_composing());
}

#[test]
fn rich_range_character_lowered_limit_never_truncates_nonexpanding_replacement() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀bc");
    document.editor().set_max_length(1);
    assert!(document
        .get_range(0, 0)
        .unwrap()
        .set_character_scalar('A')
        .unwrap());
    assert_eq!(document.text(), "A😀bc");
    assert_eq!(document.editor().max_length(), 1);
    assert!(document
        .get_range(1, 1)
        .unwrap()
        .set_character_scalar('x')
        .unwrap());
    assert_eq!(document.text(), "Axbc");
    assert_eq!(document.editor().max_length(), 1);
}

#[test]
fn rich_range_changed_operations_recapture_committed_content_after_preview_cancellation() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    document.selection().set_range(1, 3).unwrap();
    document.editor().set_preedit("long", Some(4)).unwrap();
    let range = document.get_range(1, 5).unwrap();
    assert!(range.set_character_scalar('X').unwrap());
    assert_eq!(document.text(), "aXcd");
    assert_eq!(range.range(), 1..3);
    assert!(!document.editor().is_composing());
    document.undo().unwrap();
    assert_eq!(document.text(), "abcd");
    assert!(!document.can_undo());
    document.selection().set_range(1, 3).unwrap();
    document.editor().set_preedit("long", Some(4)).unwrap();
    let range = document.get_range(1, 5).unwrap();
    assert_eq!(range.delete(TextRangeUnit::Character, 0).unwrap(), 1);
    assert_eq!(document.text(), "ad");
    assert_eq!(range.range(), 1..1);
    assert!(!document.editor().is_composing());
    document.undo().unwrap();
    assert_eq!(document.text(), "abcd");
}

#[test]
fn rich_range_character_external_format_starts_one_current_history_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("seed");
    document.set_text("abc").unwrap();
    assert!(document.can_undo());
    document.selection().set_range(2, 0).unwrap();
    document.document().paragraphs()[0].set_font_size(18.0);
    let before = document.document().snapshot();
    assert!(!document.can_undo());

    assert!(document
        .get_range(1, 1)
        .unwrap()
        .set_character_scalar('😀')
        .unwrap());
    assert_eq!(document.text(), "a😀c");
    assert_eq!(document.selection().selection(), EditorSelection::new(3, 0));
    let after = document.document().snapshot();
    assert_eq!(after.paragraphs[0].format.character.font_size, Some(18.0));
    assert_eq!(after.paragraphs[0].inlines[0].format.font_size, None);
    assert!(document.can_undo());
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), EditorSelection::new(2, 0));
    assert!(!document.can_undo());
    assert!(document.redo().unwrap());
    assert_eq!(document.document().snapshot(), after);
}
