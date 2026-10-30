// Direct Rust contracts plus literal cases from the isolated system MSFTEDIT
// probe. External WinUI-engine parity and synthetic terminal EOP remain separate.
use super::*;
use crate::documents::ParagraphSnapshot;
use crate::test_support::test_scope;
use crate::RichTextBlockOverflow;
use ferroui_base::input::IScrollable;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FlowDirection, TextTrimming, TextWrapping};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

fn measure(view: &RichTextBlock, width: f64, height: f64) {
    view.measure(Size::new(width, height));
    view.arrange(Rect::new(0.0, 0.0, width, height));
}

fn displayed(text: &str) -> (Ref<RichTextBlock>, RichEditTextDocument) {
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text(text));
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    measure(&view, 300.0, 500.0);
    (view, document)
}

#[test]
fn rich_selection_vertical_same_view_callback_retains_desired_column() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (owner, view, document) = owner_with_view("aaaaaa\na\naaaaaa");
    owner.select(5, 5);
    let editor = document.editor();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    owner.selection_changing(move |_, _| {
        if !changed.replace(true) {
            editor.set_text_view(Some(view.clone())).unwrap();
        }
    });
    for position in [8, 14] {
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Line, 1, false)
                .unwrap(),
            1
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(position)
        );
    }
    assert!(observed.get());
}

#[test]
fn rich_selection_vertical_keeps_desired_column_across_short_lines_and_facades() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (_view, document) = displayed("aaaaaa\na\naaaaaa");
    document.selection().set_range(5, 5).unwrap();
    let snapshot = document.document().snapshot();
    let revision = document.document().revision();
    for (down, position) in [(true, 8), (true, 14), (false, 8), (false, 5)] {
        // Each call obtains a new facade; the desired column belongs to its editor.
        let selection = document.selection();
        let moved = if down {
            selection.move_down(TextRangeUnit::Line, 1, false)
        } else {
            selection.move_up(TextRangeUnit::Line, 1, false)
        };
        assert_eq!(moved.unwrap(), 1);
        assert_eq!(selection.selection(), EditorSelection::caret(position));
    }
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(document.document().revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_vertical_extension_clamps_each_intermediate_line_in_one_command() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    // Fresh-control native rows from MSFTEDIT 6.2.26100.33648: collapsed Down2
    // retains column 5 (15), extended Down2 clamps through "xy" to column 2 (12).
    let (_view, document) = displayed("abcdef\nxy\nabcdef\npqrs");
    for (down, count) in [(true, 2), (false, -2)] {
        for (extend, expected) in [(false, 15), (true, 12)] {
            document.selection().set_range(5, 5).unwrap();
            let moved = if down {
                document
                    .selection()
                    .move_down(TextRangeUnit::Line, count, extend)
            } else {
                document
                    .selection()
                    .move_up(TextRangeUnit::Line, count, extend)
            };
            assert_eq!(moved.unwrap(), count);
            assert_eq!(
                document.selection().selection(),
                EditorSelection::new(if extend { 5 } else { expected }, expected)
            );
        }
    }
}

#[test]
fn rich_selection_vertical_extension_retains_reached_column_across_facades() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    // The fresh native sequence is Down1, Down1, Up1, Up1. Each operation uses
    // one live editor; obtaining a new Rust facade does not reset its affinity.
    let (_view, document) = displayed("aaaaaa\na\naaaaaa");
    document.selection().set_range(5, 5).unwrap();
    let snapshot = document.document().snapshot();
    let revision = document.document().revision();
    for (down, expected) in [(true, 8), (true, 10), (false, 8), (false, 1)] {
        let selection = document.selection();
        let moved = if down {
            selection.move_down(TextRangeUnit::Line, 1, true)
        } else {
            selection.move_up(TextRangeUnit::Line, 1, true)
        };
        assert_eq!(moved.unwrap(), 1);
        assert_eq!(selection.selection(), EditorSelection::new(5, expected));
    }
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(document.document().revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_vertical_mixed_modes_preserve_collapsed_desired_column() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    // Exact four-step sequences from system MSFTEDIT run 39765696678, job
    // 119780463926. Each sequence starts on a fresh native control at caret 5.
    // The first preserves collapsed column 5 through extension and restores it
    // on the last Up. The second starts extension without prior column affinity.
    for steps in [
        [
            (true, false, 8, 8),
            (true, true, 8, 14),
            (false, true, 8, 8),
            (false, false, 5, 5),
        ],
        [
            (true, true, 5, 8),
            (true, false, 10, 10),
            (false, false, 8, 8),
            (false, true, 8, 1),
        ],
    ] {
        let (_view, document) = displayed("abcdef\nx\nabcdef");
        document.selection().set_range(5, 5).unwrap();
        let snapshot = document.document().snapshot();
        let revision = document.document().revision();
        for (down, extend, anchor, active) in steps {
            let selection = document.selection();
            let moved = if down {
                selection.move_down(TextRangeUnit::Line, 1, extend)
            } else {
                selection.move_up(TextRangeUnit::Line, 1, extend)
            };
            assert_eq!(moved.unwrap(), 1);
            assert_eq!(
                selection.selection(),
                EditorSelection::new(anchor, active),
                "down={down}, extend={extend}"
            );
        }
        assert_eq!(document.document().snapshot(), snapshot);
        assert_eq!(document.document().revision(), revision);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_selection_vertical_extension_clamps_upward_with_signed_counts() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (_view, document) = displayed("aaaaaa\na\naaaaaa");
    for (down, count) in [(false, 2), (true, -2)] {
        document.selection().set_range(14, 14).unwrap();
        let moved = if down {
            document
                .selection()
                .move_down(TextRangeUnit::Line, count, true)
        } else {
            document
                .selection()
                .move_up(TextRangeUnit::Line, count, true)
        };
        assert_eq!(moved.unwrap(), count);
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(14, 1)
        );
    }
}

#[test]
fn rich_selection_vertical_extension_empty_lines_clamp_without_request_sized_iteration() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (_view, document) = displayed("aaaaaa\n\naaaaaa");
    document.selection().set_range(5, 5).unwrap();
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 2, true)
            .unwrap(),
        2
    );
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 8));
    document.selection().set_range(13, 13).unwrap();
    assert_eq!(
        document
            .selection()
            .move_up(TextRangeUnit::Line, i32::MAX, true)
            .unwrap(),
        2
    );
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(13, 0)
    );
    document.selection().set_range(5, 5).unwrap();
    assert_eq!(
        document
            .selection()
            .move_up(TextRangeUnit::Line, i32::MIN, true)
            .unwrap(),
        -3
    );
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(5, 14)
    );
}

#[test]
fn rich_selection_vertical_extension_redirected_edge_uses_original_caret_projection() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (owner, _view, document) = owner_with_view("aaaaaa\n\naaaaaa");
    owner.select(5, 5);
    let weak = owner.downgrade();
    owner.selection_changing(move |_, args| {
        weak.upgrade().unwrap().select(5, 0);
        args.set_cancel(true);
    });
    // The requested next line clamps to x=0, but the callback's opposite edge
    // movement starts from the original nonzero caret and still costs one unit.
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, true)
            .unwrap(),
        -1
    );
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 0));
}

#[test]
fn rich_selection_vertical_counts_actual_adjacent_lines_with_mixed_heights() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    for size in [32.0, 8.0, 24.0] {
        let paragraph = Paragraph::with_text("aaaaaa");
        paragraph.set_font_size(size);
        view.blocks().add(paragraph);
    }
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    measure(&view, 300.0, 500.0);
    let lines = view.line_metrics();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].bounds.height > lines[1].bounds.height * 2.0);
    document.selection().set_range(2, 2).unwrap();
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    let position = document.selection().selection().active;
    assert!((7..=13).contains(&position));
    assert_eq!(
        view.get_rect_from_character_index(position).y,
        lines[1].bounds.y
    );
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert!((14..=20).contains(&document.selection().selection().active));
}

#[test]
fn rich_selection_vertical_collapse_and_extension_respect_directed_endpoints() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (_view, document) = displayed("aaaa\naaaa\naaaa\naaaa");
    for (anchor, active) in [(6, 11), (11, 6)] {
        for (down, count, expected, expected_moved) in [
            (true, 1, 16, 1),
            (false, 1, 1, 1),
            (true, 2, 16, 1),
            (false, 2, 1, 1),
        ] {
            document.selection().set_range(anchor, active).unwrap();
            let moved = if down {
                document
                    .selection()
                    .move_down(TextRangeUnit::Line, count, false)
            } else {
                document
                    .selection()
                    .move_up(TextRangeUnit::Line, count, false)
            };
            assert_eq!(moved.unwrap(), expected_moved);
            assert_eq!(
                document.selection().selection(),
                EditorSelection::caret(expected)
            );
        }
        document.selection().set_range(anchor, active).unwrap();
        assert_eq!(
            document
                .selection()
                .move_up(TextRangeUnit::Line, 1, true)
                .unwrap(),
            1
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(anchor, active - 5)
        );
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Line, 2, true)
                .unwrap(),
            2
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(anchor, active + 5)
        );
    }
}

#[test]
fn rich_selection_vertical_paragraphs_handle_utf16_empty_units_and_extreme_counts() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("A😀B\n\ncdef\nxy");
    document.selection().set_range(3, 3).unwrap();
    for position in [5, 6, 11, 13] {
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Paragraph, 1, false)
                .unwrap(),
            1
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(position)
        );
    }
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Paragraph, 1, false)
            .unwrap(),
        0
    );
    assert_eq!(
        document
            .selection()
            .move_up(TextRangeUnit::Paragraph, i32::MAX, false)
            .unwrap(),
        4
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(0));
    assert_eq!(
        document
            .selection()
            .move_up(TextRangeUnit::Paragraph, i32::MIN, true)
            .unwrap(),
        -4
    );
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(0, 13)
    );
    document.selection().set_range(11, 11).unwrap();
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Paragraph, -2, false)
            .unwrap(),
        -2
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(5));
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_vertical_paragraph_collapse_moves_from_the_directional_endpoint() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc\ndefg\nhij");
    for (down, count, position, expected_moved) in [
        (true, 1, 9, 1),
        (false, 1, 0, 1),
        (true, 2, 12, 2),
        (false, 2, 0, 1),
    ] {
        document.selection().set_range(6, 1).unwrap();
        let moved = if down {
            document
                .selection()
                .move_down(TextRangeUnit::Paragraph, count, false)
        } else {
            document
                .selection()
                .move_up(TextRangeUnit::Paragraph, count, false)
        };
        assert_eq!(moved.unwrap(), expected_moved);
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(position)
        );
    }
}

#[test]
fn rich_selection_vertical_matches_system_msftedit_collapse_and_count_observations() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    // Recorded on system MSFTEDIT 6.2.26100.33648. Hard CR paragraphs are
    // represented by the model's one-position structural LF paragraphs here.
    // These collapse rows never address the native synthetic terminal EOP.
    for (text, start, end, line_down, paragraph_down) in [
        ("abcdef\nxy\nabcdef\npqrs", 2, 12, 19, 17),
        ("A😀B\nxy\nA😀B", 1, 6, 9, 8),
    ] {
        let (_view, document) = displayed(text);
        for (anchor, active) in [(start, end), (end, start)] {
            for unit in [TextRangeUnit::Line, TextRangeUnit::Paragraph] {
                let up = if unit == TextRangeUnit::Line {
                    start
                } else {
                    0
                };
                let down = if unit == TextRangeUnit::Line {
                    line_down
                } else {
                    paragraph_down
                };
                let (down_two, down_count) = if unit == TextRangeUnit::Line {
                    (line_down, 1)
                } else {
                    (document.text_length(), 2)
                };
                for (downward, count, expected, expected_count) in [
                    (false, 0, active, 0),
                    (false, 1, up, 1),
                    (false, 2, up, 1),
                    (false, -1, down, -1),
                    (true, 0, active, 0),
                    (true, 1, down, 1),
                    (true, 2, down_two, down_count),
                    (true, -1, up, -1),
                ] {
                    document.selection().set_range(anchor, active).unwrap();
                    let moved = if downward {
                        document.selection().move_down(unit, count, false)
                    } else {
                        document.selection().move_up(unit, count, false)
                    };
                    assert_eq!(
                        moved.unwrap(),
                        expected_count,
                        "{text:?} {unit:?}, down={downward}, count={count}"
                    );
                    assert_eq!(
                        document.selection().selection(),
                        if count == 0 {
                            EditorSelection::new(anchor, active)
                        } else {
                            EditorSelection::caret(expected)
                        },
                        "{text:?} {unit:?}, down={downward}, count={count}"
                    );
                }
            }
        }
    }
}

#[test]
fn rich_selection_vertical_extension_reaches_story_edges_without_synthetic_eop() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (_view, document) = displayed("abcdef\nxy\nabcdef\npqrs");
    for count in [1, 2, i32::MAX] {
        document.selection().set_range(5, 5).unwrap();
        assert_eq!(
            document
                .selection()
                .move_up(TextRangeUnit::Line, count, false)
                .unwrap(),
            0
        );
        assert_eq!(document.selection().selection(), EditorSelection::caret(5));
        assert_eq!(
            document
                .selection()
                .move_up(TextRangeUnit::Line, count, true)
                .unwrap(),
            1
        );
        assert_eq!(document.selection().selection(), EditorSelection::new(5, 0));
    }
    document.selection().set_range(2, 12).unwrap();
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 2, true)
            .unwrap(),
        2
    );
    // Native extended Down2 reaches22, including its implicit final EOP. The
    // Rust document ends at21; it selects all actual content without inventing
    // an addressable terminal character or counting later no-op extensions.
    assert_eq!(document.text_length(), 21);
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(2, 21)
    );
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, true)
            .unwrap(),
        0
    );
    let (_empty_view, empty) = displayed("");
    for unit in [TextRangeUnit::Line, TextRangeUnit::Paragraph] {
        assert_eq!(empty.selection().move_down(unit, 1, true).unwrap(), 0);
        assert_eq!(empty.selection().selection(), EditorSelection::caret(0));
    }
}

#[test]
fn rich_selection_vertical_rejects_unsupported_units_and_zero_needs_no_layout() {
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
        TextRangeUnit::Story,
        TextRangeUnit::HardParagraph,
        TextRangeUnit::Cluster,
    ] {
        for count in [0, 1, -1] {
            assert!(document.selection().move_up(unit, count, false).is_err());
            assert!(document.selection().move_down(unit, count, true).is_err());
        }
    }
    assert_eq!(
        document
            .selection()
            .move_up(TextRangeUnit::Line, 0, true)
            .unwrap(),
        0
    );
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Paragraph, 0, false)
            .unwrap(),
        0
    );
    assert!(document
        .selection()
        .move_down(TextRangeUnit::Line, 1, false)
        .is_err());
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.selection().selection(), selection);
    assert!(document.editor().is_composing());
    assert!(document.can_redo());
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_vertical_reflows_wrapped_lines_and_saturates_without_iteration() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (view, document) = displayed("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    view.set_text_wrapping(TextWrapping::Wrap);
    let mut previous_lines = 0;
    for width in [40.0, 80.0] {
        measure(&view, width, 1000.0);
        let lines = view.line_metrics();
        assert!(lines.len() > 2);
        if previous_lines != 0 {
            assert!(lines.len() < previous_lines);
        }
        previous_lines = lines.len();
        document.selection().set_range(1, 1).unwrap();
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Line, 2, false)
                .unwrap(),
            2
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(lines[2].start + 1)
        );
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Line, i32::MAX, false)
                .unwrap(),
            (lines.len() - 3) as i32
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(lines.last().unwrap().start + 1)
        );
        assert_eq!(
            document
                .selection()
                .move_up(TextRangeUnit::Line, i32::MIN, false)
                .unwrap(),
            0
        );
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Line, i32::MIN, false)
                .unwrap(),
            -((lines.len() - 1) as i32)
        );
        assert_eq!(document.selection().selection(), EditorSelection::caret(1));
    }
}

#[test]
fn rich_selection_vertical_keeps_empty_soft_break_lines_and_structural_carets() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let soft = InlineSnapshot {
        format: CharacterFormat::default(),
        kind: InlineSnapshotKind::LineBreak,
    };
    let length = soft.text_length();
    let (view, document) = displayed("");
    document
        .document()
        .restore(&DocumentSnapshot {
            paragraphs: vec![
                ParagraphSnapshot {
                    inlines: vec![
                        InlineSnapshot::run("aa"),
                        soft.clone(),
                        soft,
                        InlineSnapshot::run("aa"),
                    ],
                    ..Default::default()
                },
                ParagraphSnapshot::default(),
            ],
            ..Default::default()
        })
        .unwrap();
    measure(&view, 300.0, 500.0);
    assert_eq!(view.line_metrics().len(), 4);
    document.selection().set_range(1, 1).unwrap();
    for position in [2 + length, 3 + 2 * length, 5 + 2 * length] {
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Line, 1, false)
                .unwrap(),
            1
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(position)
        );
    }
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        0
    );
}

#[test]
fn rich_selection_vertical_follows_complete_linked_overflow_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (view, document) = displayed("aaaa\na\naaaa");
    view.set_max_lines(1);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 300.0, 100.0);
    measure(&overflow, 300.0, 100.0);
    document.selection().set_range(3, 3).unwrap();
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(6));
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(10));
}

#[test]
fn rich_selection_vertical_uses_shaped_rtl_and_utf16_caret_geometry() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (text, active) in [("אבגד", 1), ("A😀BC", 3), ("abc אבג xyz", 5)] {
        let (view, document) = displayed(&format!("{text}\n{text}"));
        view.set_flow_direction(FlowDirection::RightToLeft);
        measure(&view, 300.0, 500.0);
        let offset = text.encode_utf16().count() + 1;
        document.selection().set_range(active, active).unwrap();
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Line, 1, true)
                .unwrap(),
            1
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(active, offset + active)
        );
        assert!(document
            .document()
            .validate_range(document.selection().selection().range())
            .is_ok());
        assert_eq!(
            document
                .selection()
                .move_up(TextRangeUnit::Line, 1, true)
                .unwrap(),
            1
        );
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(active)
        );
    }
}

#[test]
fn rich_selection_vertical_requires_complete_untrimmed_and_live_view() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (view, document) = displayed("one\ntwo");
    document.selection().set_range(1, 1).unwrap();
    view.set_max_lines(1);
    measure(&view, 100.0, 100.0);
    assert!(document
        .selection()
        .move_down(TextRangeUnit::Line, 1, false)
        .is_err());
    view.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    measure(&view, 100.0, 100.0);
    assert!(view.is_text_trimmed());
    assert!(document
        .selection()
        .move_up(TextRangeUnit::Line, 1, true)
        .is_err());
    assert_eq!(document.selection().selection(), EditorSelection::caret(1));
    assert!(document
        .editor()
        .set_text_view(Some(RichTextBlock::new()))
        .is_err());
    drop(view);
    assert!(document
        .selection()
        .move_down(TextRangeUnit::Line, 1, false)
        .is_err());
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Paragraph, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(4));
}

#[test]
fn rich_selection_vertical_readonly_navigation_preserves_preview_and_redo() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (view, document) = displayed("aaa\naaa");
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    document.selection().set_range(1, 1).unwrap();
    document.editor().set_preedit("a", Some(0)).unwrap();
    document.editor().set_is_read_only(true);
    measure(&view, 300.0, 500.0);
    let snapshot = document.document().snapshot();
    let revision = document.document().revision();
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, true)
            .unwrap(),
        1
    );
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(document.document().revision(), revision);
    assert!(document.editor().is_composing());
    assert!(document.can_redo());
    assert!(!document.can_undo());
    document.editor().cancel_composition().unwrap();
    assert_eq!(document.text(), "aaa\naaa");
    assert_eq!(document.selection().selection(), EditorSelection::caret(1));
}

#[test]
fn rich_selection_vertical_recaptures_selection_after_layout_callbacks() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text("aa\naaaa\naa"));
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    let editor = document.editor();
    let subscription = view.property_changed(move |change| {
        if change.property().name() == "BaselineOffset" && !changed.replace(true) {
            editor.set_selection(EditorSelection::new(1, 5)).unwrap();
        }
    });
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, true)
            .unwrap(),
        1
    );
    assert!(observed.get());
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(1, 10)
    );
    subscription.dispose();
}

#[test]
fn rich_selection_vertical_preserves_external_layout_edits_and_invalidates_groups() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for grouped in [false, true] {
        let view = RichTextBlock::new();
        view.set_document(RichTextDocument::from_text("aaa\naaa"));
        let document = RichEditTextDocument::new(view.document());
        document.editor().set_text_view(Some(view.clone())).unwrap();
        document.set_text("changed").unwrap();
        document.undo().unwrap();
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
                    document
                        .selection()
                        .move_down(TextRangeUnit::Line, 1, false),
                    Err(EditError::ReentrantEdit)
                ));
                Ok(0)
            })
        } else {
            document
                .selection()
                .move_down(TextRangeUnit::Line, 1, false)
        };
        assert!(observed.get());
        assert!(matches!(result, Err(EditError::ReentrantEdit)));
        assert_eq!(document.text(), "external");
        assert!(!document.can_undo());
        assert!(!document.can_redo());
        subscription.dispose();
    }
}

#[test]
fn rich_selection_vertical_cancel_retains_column_and_redirect_counts_accepted_lines() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (owner, _view, document) = owner_with_view("aaaaaa\na\naaaaaa");
    owner.select(5, 5);
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    let cancel = Rc::new(Cell::new(true));
    let callback_cancel = cancel.clone();
    owner.selection_changing(move |_, args| args.set_cancel(callback_cancel.get()));
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        0
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(8));
    cancel.set(false);
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(14));

    let (owner, _view, document) = owner_with_view("aaa\naaa\naaa");
    owner.select(1, 1);
    let weak = owner.downgrade();
    owner.selection_changing(move |_, args| {
        weak.upgrade().unwrap().select(9, 9);
        args.set_cancel(true);
    });
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        2
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(9));
}

#[test]
fn rich_selection_vertical_redirected_extensions_count_actual_story_edge_units() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (start, edge, downward) in [(5, 0, true), (10, 15, false)] {
        let (owner, _view, document) = owner_with_view("aaaaaa\na\naaaaaa");
        owner.select(start, start);
        let snapshot = document.document().snapshot();
        let revision = document.document().revision();
        let weak = owner.downgrade();
        let redirected = Rc::new(Cell::new(false));
        let observed = redirected.clone();
        owner.selection_changing(move |_, args| {
            if !redirected.replace(true) {
                weak.upgrade().unwrap().select(start, edge);
                args.set_cancel(true);
            }
        });
        let moved = if downward {
            document.selection().move_down(TextRangeUnit::Line, 1, true)
        } else {
            document.selection().move_up(TextRangeUnit::Line, 1, true)
        };
        // The accepted edge is in the opposite direction from the request and
        // on the original display line. It still crosses one extension unit.
        assert_eq!(moved.unwrap(), -1);
        assert!(observed.get());
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(start, edge)
        );
        assert_eq!(document.document().snapshot(), snapshot);
        assert_eq!(document.document().revision(), revision);
        assert!(!document.can_undo());
        assert!(!document.can_redo());
    }
}

#[test]
fn rich_selection_vertical_view_detach_during_selection_does_not_store_old_column() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (owner, view, document) = owner_with_view("aaaaaa\na\naaaaaa");
    owner.select(5, 5);
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
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(8));
    document.editor().set_text_view(Some(view)).unwrap();
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(10));
}

#[test]
fn rich_selection_vertical_nested_boundary_command_owns_its_column_at_same_offsets() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (owner, _view, document) = owner_with_view("aaaaaa\na");
    owner.select(5, 5);
    let nested = document.selection();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    owner.selection_changing(move |_, _| {
        if !changed.replace(true) {
            assert_eq!(nested.selection(), EditorSelection::caret(8));
            assert_eq!(nested.move_down(TextRangeUnit::Line, 1, false).unwrap(), 0);
        }
    });
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert!(observed.get());
    assert_eq!(
        document
            .selection()
            .move_up(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(1));
}

#[test]
fn rich_selection_vertical_explicit_selection_and_edits_invalidate_column() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let (_view, document) = displayed("aaaaaa\na\naaaaaa");
    document.selection().set_range(5, 5).unwrap();
    document
        .selection()
        .move_down(TextRangeUnit::Line, 1, false)
        .unwrap();
    document.selection().set_range(7, 7).unwrap();
    document.selection().set_range(8, 8).unwrap();
    document
        .selection()
        .move_down(TextRangeUnit::Line, 1, false)
        .unwrap();
    assert_eq!(document.selection().selection(), EditorSelection::caret(10));
    document.selection().set_range(5, 5).unwrap();
    document
        .selection()
        .move_down(TextRangeUnit::Line, 1, false)
        .unwrap();
    document.document().replace_text(9..15, "aaaaa").unwrap();
    document
        .selection()
        .move_down(TextRangeUnit::Line, 1, false)
        .unwrap();
    assert_eq!(document.selection().selection(), EditorSelection::caret(10));
}

#[test]
fn rich_selection_move_by_external_editor_publication_preserves_handler_state() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("one two three");
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    document.selection().set_range(1, 1).unwrap();
    let model = document.document();
    let weak = Rc::downgrade(&document.editor());
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    let subscription = document.editor().changed(move || {
        if !changed.replace(true) {
            model
                .replace_text(0..model.text_length(), "external")
                .unwrap();
            weak.upgrade()
                .unwrap()
                .set_selection(EditorSelection::caret(0))
                .unwrap();
        }
    });
    assert!(matches!(
        document.selection().move_by(TextRangeUnit::Word, 1, false),
        Err(EditError::ReentrantEdit)
    ));
    assert!(observed.get());
    assert_eq!(document.text(), "external");
    assert_eq!(document.selection().selection(), EditorSelection::caret(0));
    assert!(!document.can_undo());
    assert!(!document.can_redo());
    subscription.dispose();
}
