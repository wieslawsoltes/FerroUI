// Direct selection-facade regressions; these are not external engine fixtures.
use super::*;
use crate::documents::ParagraphSnapshot;
use crate::test_support::test_scope;
use crate::{RichEditBox, RichTextBlockOverflow};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FlowDirection, TextTrimming, TextWrapping};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;
#[allow(unused_imports)]
use crate::{RichEditBox, RichEditBoxTextPresenter};

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

fn measure(view: &RichTextBlock, width: f64, height: f64) {
    view.measure(Size::new(width, height));
    view.arrange(Rect::new(0.0, 0.0, width, height));
}

fn displayed(view: &Ref<RichTextBlock>) -> RichEditTextDocument {
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    document
}

fn navigate(
    document: &RichEditTextDocument,
    unit: TextRangeUnit,
    extend: bool,
    to_end: bool,
) -> Result<isize, EditError> {
    if to_end {
        document.selection().end_key(unit, extend)
    } else {
        document.selection().home_key(unit, extend)
    }
}

#[test]
fn rich_selection_home_end_matches_observed_msftedit_real_paragraph_endpoints() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    // Literal observations from MSFTEDIT 6.2.26100.33438 on Windows 10.0.26100.
    // FerroUI structural LF and the oracle's CR occupy the same UTF-16 position.
    // These rows contain real paragraph marks, not the native synthetic EOP.
    for associated in [false, true] {
        for (text, rows) in [
            (
                "abc\ndefg\nhij",
                vec![
                    (1, 6, false, false, 0, -6),
                    (1, 6, false, true, 8, 2),
                    (1, 6, true, false, 4, -2),
                    (1, 6, true, true, 9, 3),
                    (6, 1, false, false, 0, -1),
                    (6, 1, false, true, 8, 7),
                    (6, 1, true, false, 0, -1),
                    (6, 1, true, true, 4, 3),
                ],
            ),
            (
                "A😀B\nxy",
                vec![
                    (1, 6, false, false, 0, -6),
                    (1, 6, false, true, 7, 1),
                    (1, 6, true, false, 5, -1),
                    (6, 1, false, false, 0, -1),
                    (6, 1, false, true, 7, 6),
                    (6, 1, true, false, 0, -1),
                    (6, 1, true, true, 5, 4),
                ],
            ),
        ] {
            let view = RichTextBlock::new();
            view.set_document(RichTextDocument::from_text(text));
            let document = if associated {
                measure(&view, 300.0, 500.0);
                displayed(&view)
            } else {
                RichEditTextDocument::new(view.document())
            };
            for (anchor, active, extend, to_end, target, delta) in rows {
                document.selection().set_range(anchor, active).unwrap();
                assert_eq!(
                    navigate(&document, TextRangeUnit::Line, extend, to_end).unwrap(),
                    delta
                );
                assert_eq!(
                    document.selection().selection(),
                    EditorSelection::new(if extend { anchor } else { target }, target)
                );
                assert_eq!(
                    navigate(&document, TextRangeUnit::Line, extend, to_end).unwrap(),
                    0
                );
            }
        }
    }
}

#[test]
fn rich_selection_extended_end_includes_empty_paragraph_marks_without_synthetic_eop() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab\n\nxy\n");
    document.selection().set_range(3, 3).unwrap();
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::new(3, 4));
    assert_eq!(document.selection().text().unwrap(), "\n");
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        0
    );
    document.selection().set_range(7, 7).unwrap();
    for unit in [TextRangeUnit::Line, TextRangeUnit::Story] {
        assert_eq!(document.selection().end_key(unit, true).unwrap(), 0);
        assert_eq!(document.selection().selection(), EditorSelection::caret(7));
    }
    let empty = services("");
    assert_eq!(
        empty
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        0
    );
    assert_eq!(
        empty
            .selection()
            .end_key(TextRangeUnit::Story, true)
            .unwrap(),
        0
    );
    assert_eq!(empty.text_length(), 0);
}

#[test]
fn rich_selection_extended_end_affinity_invalidates_on_selection_model_and_view_changes() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for change in 0..3 {
        let view = RichTextBlock::new();
        view.set_document(RichTextDocument::from_text("abc\ndefg\nhij"));
        measure(&view, 300.0, 500.0);
        let document = displayed(&view);
        document.selection().set_range(1, 6).unwrap();
        assert_eq!(
            document
                .selection()
                .end_key(TextRangeUnit::Line, true)
                .unwrap(),
            3
        );
        let replacement = RichTextBlock::new();
        match change {
            0 => {
                document.selection().set_range(1, 10).unwrap();
                document.selection().set_range(1, 9).unwrap();
            }
            1 => {
                document.document().replace_text(0..1, "q").unwrap();
            }
            _ => {
                // A document has one owning root view. Explicitly release that
                // ownership before attaching it to a different shaped view.
                assert!(view.try_set_document(RichTextDocument::new()));
                assert!(replacement.try_set_document(document.document()));
                measure(&replacement, 300.0, 500.0);
                document
                    .editor()
                    .set_text_view(Some(replacement.clone()))
                    .unwrap();
            }
        }
        assert_eq!(
            document
                .selection()
                .end_key(TextRangeUnit::Line, true)
                .unwrap(),
            3
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(1, 12)
        );
    }
}

#[test]
fn rich_selection_home_end_story_uses_active_utf16_end_and_preserves_extension_anchor() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("A😀B\ncd");
    let snapshot = document.document().snapshot();
    let revision = document.document().revision();
    assert_eq!(document.text_length(), 7);
    for (anchor, active) in [(1, 5), (5, 1)] {
        for extend in [false, true] {
            for to_end in [false, true] {
                document.selection().set_range(anchor, active).unwrap();
                let expected = if to_end { 7 } else { 0 };
                assert_eq!(
                    navigate(&document, TextRangeUnit::Story, extend, to_end).unwrap(),
                    expected as isize - active as isize
                );
                assert_eq!(
                    document.selection().selection(),
                    EditorSelection::new(if extend { anchor } else { expected }, expected)
                );
            }
        }
    }
    // Collapsing a selection whose active end is already at the target moves
    // zero UTF-16 positions, even though its anchor changes.
    document.selection().set_range(7, 0).unwrap();
    assert_eq!(
        document
            .selection()
            .home_key(TextRangeUnit::Story, false)
            .unwrap(),
        0
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(0));
    document.selection().set_range(0, 7).unwrap();
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Story, false)
            .unwrap(),
        0
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(7));
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(document.document().revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_home_end_headless_lines_keep_paragraph_ends_and_empty_lines() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc\n\n😀z\n");
    for (active, start, end) in [
        (0, 0, 3),
        (2, 0, 3),
        (3, 0, 3),
        (4, 4, 4),
        (5, 5, 8),
        (7, 5, 8),
        (8, 5, 8),
        (9, 9, 9),
    ] {
        for extend in [false, true] {
            for to_end in [false, true] {
                document.selection().set_range(9, active).unwrap();
                let target = if to_end {
                    if extend {
                        (end + 1).min(9)
                    } else {
                        9
                    }
                } else {
                    start
                };
                assert_eq!(
                    navigate(&document, TextRangeUnit::Line, extend, to_end).unwrap(),
                    target as isize - active as isize
                );
                assert_eq!(
                    document.selection().selection(),
                    EditorSelection::new(if extend { 9 } else { target }, target)
                );
            }
        }
    }
    for text in ["", "\n", "\n\n"] {
        let document = services(text);
        for position in 0..=document.text_length() {
            document.selection().set_range(position, position).unwrap();
            let end = (position + 1).min(document.text_length());
            assert_eq!(
                document
                    .selection()
                    .home_key(TextRangeUnit::Line, false)
                    .unwrap(),
                0
            );
            assert_eq!(
                document
                    .selection()
                    .end_key(TextRangeUnit::Line, true)
                    .unwrap(),
                end as isize - position as isize
            );
            assert_eq!(
                document.selection().selection(),
                EditorSelection::new(position, end)
            );
        }
    }
}

#[test]
fn rich_selection_home_end_excludes_platform_soft_break_and_structural_separator() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for associated in [false, true] {
        let soft = InlineSnapshot {
            format: CharacterFormat::default(),
            kind: InlineSnapshotKind::LineBreak,
        };
        let following = 2 + soft.text_length();
        let view = RichTextBlock::new();
        view.document()
            .restore(&DocumentSnapshot {
                paragraphs: vec![
                    ParagraphSnapshot {
                        inlines: vec![InlineSnapshot::run("ab"), soft, InlineSnapshot::run("cd")],
                        ..Default::default()
                    },
                    ParagraphSnapshot::default(),
                ],
                ..Default::default()
            })
            .unwrap();
        let document = if associated {
            measure(&view, 300.0, 200.0);
            displayed(&view)
        } else {
            RichEditTextDocument::new(view.document())
        };
        for (active, start, end) in [
            (1, 0, 2),
            (2, 0, 2),
            (following, following, following + 2),
            (following + 2, following, following + 2),
            (following + 3, following + 3, following + 3),
        ] {
            document.selection().set_range(active, active).unwrap();
            assert_eq!(
                document
                    .selection()
                    .home_key(TextRangeUnit::Line, false)
                    .unwrap(),
                start as isize - active as isize
            );
            assert_eq!(
                document.selection().selection(),
                EditorSelection::caret(start)
            );
            document.selection().set_range(active, active).unwrap();
            assert_eq!(
                document
                    .selection()
                    .end_key(TextRangeUnit::Line, false)
                    .unwrap(),
                end as isize - active as isize
            );
            assert_eq!(
                document.selection().selection(),
                EditorSelection::caret(end)
            );
        }
    }
}

#[test]
fn rich_selection_home_end_headless_crlf_and_paragraph_controls_exclude_the_whole_break() {
    let _scope = Dispatcher::unit_test_scope();
    let model = RichTextDocument::new();
    model
        .restore(&DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                inlines: vec![InlineSnapshot::run("a\r\nb\u{b}c\u{c}d\u{2029}")],
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    let document = RichEditTextDocument::new(model);
    for (active, start, end) in [
        (1, 0, 1),
        (2, 0, 1),
        (3, 3, 4),
        (5, 5, 6),
        (7, 7, 8),
        (9, 9, 9),
    ] {
        document.selection().set_range(active, active).unwrap();
        document
            .selection()
            .home_key(TextRangeUnit::Line, false)
            .unwrap();
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(start)
        );
        document.selection().set_range(active, active).unwrap();
        document
            .selection()
            .end_key(TextRangeUnit::Line, false)
            .unwrap();
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(end)
        );
    }
}

#[test]
fn rich_selection_home_end_uses_wrapped_lines_and_forward_boundary_affinity_after_reflow() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text(
        "one two three four five six seven eight",
    ));
    view.set_text_wrapping(TextWrapping::Wrap);
    let document = displayed(&view);
    for width in [120.0, 40.0] {
        measure(&view, width, 1000.0);
        let lines = view.line_metrics();
        assert!(lines.len() >= 2);
        let index = (lines.len() - 1) / 2;
        let start = lines[index].start;
        let end = lines[index + 1].start;
        assert!(end > start + 1);
        let active = start + 1;
        let anchor = document.text_length();
        document.selection().set_range(anchor, active).unwrap();
        assert_eq!(
            document
                .selection()
                .home_key(TextRangeUnit::Line, true)
                .unwrap(),
            -1
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(anchor, start)
        );
        document.selection().set_range(anchor, active).unwrap();
        assert_eq!(
            document
                .selection()
                .end_key(TextRangeUnit::Line, true)
                .unwrap(),
            end as isize - active as isize
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(anchor, end)
        );
        // A shared offset at a soft wrap belongs to the following display line.
        assert_eq!(
            document
                .selection()
                .home_key(TextRangeUnit::Line, false)
                .unwrap(),
            0
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(end)
        );
    }
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_home_end_follows_logical_source_order_in_rtl_lines() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
        let view = RichTextBlock::new();
        view.set_flow_direction(direction);
        view.blocks().add(Paragraph::with_text("abc אבג xyz"));
        measure(&view, 300.0, 100.0);
        let document = displayed(&view);
        document.selection().set_range(5, 5).unwrap();
        assert_eq!(
            document
                .selection()
                .home_key(TextRangeUnit::Line, true)
                .unwrap(),
            -5
        );
        assert_eq!(document.selection().selection(), EditorSelection::new(5, 0));
        document.selection().set_range(5, 5).unwrap();
        assert_eq!(
            document
                .selection()
                .end_key(TextRangeUnit::Line, true)
                .unwrap(),
            6
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(5, 11)
        );
    }
}

#[test]
fn rich_selection_home_end_uses_complete_linked_overflow_lines() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text("one\ntwo\nthree"));
    view.set_max_lines(1);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 300.0, 100.0);
    measure(&overflow, 300.0, 100.0);
    let document = displayed(&view);
    document.selection().set_range(13, 5).unwrap();
    assert_eq!(
        document
            .selection()
            .home_key(TextRangeUnit::Line, true)
            .unwrap(),
        -1
    );
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(13, 4)
    );
    document.selection().set_range(13, 5).unwrap();
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        3
    );
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(13, 8)
    );
}

#[test]
fn rich_selection_home_end_rejects_unsupported_units_without_touching_preview_or_redo() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    document.selection().set_range(0, 0).unwrap();
    document.editor().set_preedit("preview", Some(0)).unwrap();
    let snapshot = document.document().snapshot();
    let revision = document.document().revision();
    let selection = document.selection().selection();
    for unit in [
        TextRangeUnit::Character,
        TextRangeUnit::Word,
        TextRangeUnit::Sentence,
        TextRangeUnit::Paragraph,
        TextRangeUnit::HardParagraph,
        TextRangeUnit::Cluster,
    ] {
        assert!(document.selection().home_key(unit, false).is_err());
        assert!(document.selection().end_key(unit, true).is_err());
    }
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.selection().selection(), selection);
    assert!(document.editor().is_composing());
    assert!(document.can_redo());
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_home_end_readonly_navigation_preserves_preview_and_history() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    document.selection().set_range(0, 0).unwrap();
    document.editor().set_preedit("preview", Some(0)).unwrap();
    document.editor().set_is_read_only(true);
    let snapshot = document.document().snapshot();
    let revision = document.document().revision();
    assert_eq!(
        document
            .selection()
            .home_key(TextRangeUnit::Story, false)
            .unwrap(),
        0
    );
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Story, true)
            .unwrap(),
        10
    );
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(0, 10)
    );
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(document.document().revision(), revision);
    assert!(document.editor().is_composing());
    assert!(document.can_redo());
    assert!(!document.can_undo());
    document.editor().cancel_composition().unwrap();
    assert_eq!(document.text(), "abc");
    assert_eq!(document.selection().selection(), EditorSelection::caret(0));
    assert!(document.can_redo());
}

#[test]
fn rich_selection_home_end_rejects_incomplete_or_trimmed_lines_but_story_needs_no_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text("one\ntwo"));
    view.set_max_lines(1);
    let document = displayed(&view);
    document.selection().set_range(5, 2).unwrap();
    measure(&view, 100.0, 100.0);
    assert!(document
        .selection()
        .home_key(TextRangeUnit::Line, false)
        .is_err());
    assert!(document
        .selection()
        .end_key(TextRangeUnit::Line, true)
        .is_err());
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 2));
    view.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    measure(&view, 100.0, 100.0);
    assert!(view.is_text_trimmed());
    assert!(document
        .selection()
        .home_key(TextRangeUnit::Line, false)
        .is_err());
    assert!(document
        .selection()
        .end_key(TextRangeUnit::Line, true)
        .is_err());
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 2));
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Story, true)
            .unwrap(),
        5
    );
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 7));
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_home_end_recaptures_active_end_after_layout_selection_callbacks() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for to_end in [false, true] {
        for extend in [false, true] {
            let view = RichTextBlock::new();
            view.set_document(RichTextDocument::from_text("ab\ncdef"));
            let document = displayed(&view);
            let editor = document.editor();
            let changed = Rc::new(Cell::new(false));
            let observed = changed.clone();
            let subscription = view.property_changed(move |change| {
                if change.property().name() == "BaselineOffset" && !changed.replace(true) {
                    editor.set_selection(EditorSelection::new(1, 5)).unwrap();
                }
            });
            let target = if to_end {
                7
            } else if extend {
                3
            } else {
                0
            };
            assert_eq!(
                navigate(&document, TextRangeUnit::Line, extend, to_end).unwrap(),
                target as isize - 5
            );
            assert!(observed.get());
            assert_eq!(
                document.selection().selection(),
                EditorSelection::new(if extend { 1 } else { target }, target)
            );
            assert!(!document.can_undo());
            subscription.dispose();
        }
    }
}

#[test]
fn rich_selection_home_end_layout_model_mutation_survives_nested_group_rejection() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for grouped in [false, true] {
        let view = RichTextBlock::new();
        view.set_document(RichTextDocument::from_text("abc"));
        let document = displayed(&view);
        document.set_text("changed").unwrap();
        document.undo().unwrap();
        document.selection().set_range(2, 2).unwrap();
        assert!(document.can_redo());
        let model = document.document();
        let changed = Rc::new(Cell::new(false));
        let observed = changed.clone();
        let subscription = view.property_changed(move |change| {
            if change.property().name() == "BaselineOffset" && !changed.replace(true) {
                model
                    .replace_text(0..model.text_length(), "external")
                    .unwrap();
            }
        });
        let result = if grouped {
            document.edit_group(|document| {
                assert!(matches!(
                    document.selection().home_key(TextRangeUnit::Line, false),
                    Err(EditError::ReentrantEdit)
                ));
                Ok(0)
            })
        } else {
            document.selection().home_key(TextRangeUnit::Line, false)
        };
        assert!(observed.get());
        assert!(matches!(result, Err(EditError::ReentrantEdit)));
        assert_eq!(document.text(), "external");
        assert!(!document.can_undo());
        assert!(!document.can_redo());
        assert!(document
            .document()
            .validate_range(document.selection().selection().range())
            .is_ok());
        subscription.dispose();
    }
}

#[test]
fn rich_selection_home_end_presentation_or_view_reentrancy_preserves_selection_and_redo() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for replace_view in [false, true] {
        let view = RichTextBlock::new();
        view.set_document(RichTextDocument::from_text("abc"));
        let document = displayed(&view);
        document.set_text("changed").unwrap();
        document.undo().unwrap();
        document.selection().set_range(2, 1).unwrap();
        let before = document.document().snapshot();
        let editor = document.editor();
        let weak = view.downgrade();
        let changed = Rc::new(Cell::new(false));
        let observed = changed.clone();
        let subscription = view.property_changed(move |change| {
            if change.property().name() == "BaselineOffset" && !changed.replace(true) {
                if replace_view {
                    editor.set_text_view(None).unwrap();
                } else {
                    weak.upgrade().unwrap().set_font_size(24.0);
                }
            }
        });
        assert!(matches!(
            document.selection().end_key(TextRangeUnit::Line, true),
            Err(EditError::ReentrantEdit)
        ));
        assert!(observed.get());
        assert_eq!(document.document().snapshot(), before);
        assert_eq!(document.selection().selection(), EditorSelection::new(2, 1));
        assert!(document.can_redo());
        assert!(!document.can_undo());
        subscription.dispose();
    }
}

#[test]
fn rich_selection_home_end_editor_publication_model_write_preserves_external_state() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcdef");
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    document.selection().set_range(2, 2).unwrap();
    let model = document.document();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    let weak = Rc::downgrade(&document.editor());
    let _subscription = document.editor().changed(move || {
        if !changed.replace(true) {
            model.replace_text(0..model.text_length(), "q").unwrap();
            weak.upgrade()
                .unwrap()
                .set_selection(EditorSelection::caret(0))
                .unwrap();
        }
    });
    assert!(matches!(
        document.selection().end_key(TextRangeUnit::Story, true),
        Err(EditError::ReentrantEdit)
    ));
    assert!(observed.get());
    assert_eq!(document.text(), "q");
    assert!(!document.can_undo());
    assert!(!document.can_redo());
    assert_eq!(document.selection().selection(), EditorSelection::caret(0));
}

#[test]
fn rich_selection_extended_end_cancellation_retains_affinity() {
    let _scope = Dispatcher::unit_test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abc\ndefg\nhij").unwrap();
    owner.select(1, 6);
    let document = owner.text_document();
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        3
    );
    owner.selection_changing(|_, args| args.set_cancel(true));
    assert_eq!(
        document
            .selection()
            .home_key(TextRangeUnit::Story, false)
            .unwrap(),
        0
    );
    assert_eq!(document.selection().selection(), EditorSelection::new(1, 9));
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        0
    );
}

#[test]
fn rich_selection_extended_end_nested_same_offset_navigation_owns_affinity() {
    let _scope = Dispatcher::unit_test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abc\ndefg\nhij").unwrap();
    owner.select(1, 6);
    let document = owner.text_document();
    let nested = document.selection();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    owner.selection_changing(move |_, _| {
        if !changed.replace(true) {
            assert_eq!(nested.selection(), EditorSelection::new(1, 9));
            // Home accepts the same endpoint as a following-line start. The
            // enclosing End must not overwrite this newer affinity decision.
            assert_eq!(nested.home_key(TextRangeUnit::Line, true).unwrap(), 0);
        }
    });
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        3
    );
    assert!(observed.get());
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        3
    );
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(1, 12)
    );
}

#[test]
fn rich_selection_extended_end_view_detach_during_publication_invalidates_old_affinity() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let owner = RichEditBox::new();
    owner.set_text("abc\ndefg\nhij").unwrap();
    owner.select(1, 6);
    let document = owner.text_document();
    let view = owner.text_presenter().upcast::<RichTextBlock>();
    measure(&view, 300.0, 500.0);
    document.editor().set_text_view(Some(view.clone())).unwrap();
    let editor = document.editor();
    let changed = Rc::new(Cell::new(false));
    owner.selection_changing(move |_, _| {
        if !changed.replace(true) {
            editor.set_text_view(None).unwrap();
        }
    });
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        3
    );
    assert_eq!(
        document
            .selection()
            .end_key(TextRangeUnit::Line, true)
            .unwrap(),
        3
    );
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(1, 12)
    );
}

#[test]
fn rich_selection_home_end_reports_canceled_or_reentrant_accepted_selection() {
    let _scope = Dispatcher::unit_test_scope();
    for override_selection in [false, true] {
        for to_end in [false, true] {
            let owner = RichEditBox::new();
            owner.set_text("abcdef").unwrap();
            owner.select(1, 4);
            let document = owner.text_document();
            document.clear_undo_redo_history();
            let snapshot = document.document().snapshot();
            let weak = owner.downgrade();
            owner.selection_changing(move |_, args| {
                if override_selection {
                    weak.upgrade().unwrap().select(5, 2);
                }
                args.set_cancel(true);
            });
            assert_eq!(
                navigate(&document, TextRangeUnit::Story, false, to_end).unwrap(),
                if override_selection { -2 } else { 0 }
            );
            let expected = if override_selection {
                EditorSelection::new(5, 2)
            } else {
                EditorSelection::new(1, 4)
            };
            assert_eq!(document.selection().selection(), expected);
            assert_eq!(
                (owner.selection_start(), owner.selection_end()),
                (expected.anchor, expected.active)
            );
            assert_eq!(document.document().snapshot(), snapshot);
            assert!(!document.can_undo());
        }
    }
}

#[test]
fn rich_selection_home_end_selection_callback_model_write_rejects_stale_result_in_outer_batch() {
    let _scope = Dispatcher::unit_test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abcdef").unwrap();
    owner.select(4, 4);
    let document = owner.text_document();
    let model = document.document();
    let revision = model.revision();
    let batch = model.begin_edit();
    let callback_model = model.clone();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    owner.selection_changing(move |_, _| {
        if !changed.replace(true) {
            callback_model
                .replace_text(0..callback_model.text_length(), "external")
                .unwrap();
        }
    });
    assert!(matches!(
        document.selection().home_key(TextRangeUnit::Story, false),
        Err(EditError::ReentrantEdit)
    ));
    assert!(observed.get());
    assert_eq!(document.text(), "external");
    assert_eq!(model.revision(), revision);
    assert!(!document.can_undo());
    assert!(!document.can_redo());
    drop(batch);
    assert_eq!(document.text(), "external");
}
