// Independent contract regressions derived from the pinned TOM API descriptions.
// They are not claimed to be upstream fixtures from the external text engine.
use super::*;
use crate::documents::{Paragraph, ParagraphSnapshot};
use crate::test_support::test_scope;
use crate::RichTextBlockOverflow;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{TextTrimming, TextWrapping};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Size};

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

fn strings(text: &str, ranges: Vec<Range<usize>>) -> Vec<String> {
    let utf16: Vec<_> = text.encode_utf16().collect();
    ranges
        .into_iter()
        .map(|range| String::from_utf16(&utf16[range]).unwrap())
        .collect()
}

fn measure(view: &RichTextBlock, width: f64, height: f64) {
    view.measure(Size::new(width, height));
    view.arrange(Rect::new(0.0, 0.0, width, height));
}

#[test]
fn text_range_units_preserve_native_numeric_values_and_story_alias() {
    assert_eq!(TextRangeUnit::Character as i32, 0);
    assert_eq!(TextRangeUnit::Word as i32, 1);
    assert_eq!(TextRangeUnit::Sentence as i32, 2);
    assert_eq!(TextRangeUnit::Paragraph as i32, 3);
    assert_eq!(TextRangeUnit::Line as i32, 4);
    assert_eq!(TextRangeUnit::Story as i32, 5);
    assert_eq!(TextRangeUnit::HardParagraph as i32, 12);
    assert_eq!(TextRangeUnit::Cluster as i32, 13);
    assert_eq!(TextRangeUnit::Document, TextRangeUnit::Story);
}

#[test]
fn tom_character_distinguishes_combining_scalars_without_splitting_surrogates() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a\u{301}😀👨\u{200d}👩");
    let range = document.get_range(0, 0).unwrap();
    for position in [1, 2, 4, 6, 7, 9] {
        assert_eq!(range.move_by(TextRangeUnit::Character, 1).unwrap(), 1);
        assert_eq!(range.range(), position..position);
    }
    assert_eq!(range.get_index(TextRangeUnit::Character).unwrap(), 6);
    assert_eq!(
        range.move_by(TextRangeUnit::Character, i32::MIN).unwrap(),
        -6
    );
    range.set_range(1, 1).unwrap();
    assert_eq!(range.expand(TextRangeUnit::Character).unwrap(), 1);
    assert_eq!(range.text().unwrap(), "\u{301}");
    assert_eq!(
        unit_ranges(&document.text(), UnicodeTextUnit::Character)[0],
        0..2,
        "automation's grapheme boundaries remain independent of TOM Character"
    );
}

#[test]
fn native_word_units_include_following_blanks_without_changing_search_boundaries() {
    let _scope = Dispatcher::unit_test_scope();
    let text = " one  two!\t \n next";
    assert_eq!(
        strings(text, word_ranges(text)),
        [" ", "one  ", "two", "!\t ", "\n ", "next"]
    );
    let document = services("alpha  alphabet alpha\t");
    let range = document.get_range(2, 2).unwrap();
    range.expand(TextRangeUnit::Word).unwrap();
    assert_eq!(range.text().unwrap(), "alpha  ");
    let found = document
        .document_range()
        .unwrap()
        .find("alpha", TextFindOptions::WHOLE_WORD)
        .unwrap()
        .unwrap();
    assert_eq!(found.range(), 0..5);
    assert_eq!(found.text().unwrap(), "alpha");
    let backwards = document
        .document_range()
        .unwrap()
        .find(
            "alpha",
            TextFindOptions::WHOLE_WORD | TextFindOptions::BACKWARDS,
        )
        .unwrap()
        .unwrap();
    assert_eq!(backwards.text().unwrap(), "alpha");
    assert_eq!(backwards.range(), 16..21);
}

#[test]
fn tom_sentence_units_follow_native_punctuation_and_ascii_whitespace_rules() {
    assert_eq!(
        strings("Dr. Watson!  Last", sentence_ranges("Dr. Watson!  Last")),
        ["Dr. ", "Watson!  ", "Last"]
    );
    let repeated = "Hi... \t? Still! \u{2029}End";
    assert_eq!(
        strings(repeated, sentence_ranges(repeated)),
        ["Hi... \t? ", "Still! \u{2029}", "End"]
    );
    let non_ascii = "One.\u{a0}Two. Next";
    assert_eq!(
        strings(non_ascii, sentence_ranges(non_ascii)),
        ["One.\u{a0}Two. ", "Next"]
    );
    assert_eq!(
        strings("unterminated", sentence_ranges("unterminated")),
        ["unterminated"]
    );
    assert_eq!(strings("  ", sentence_ranges("  ")), ["  "]);
    assert!(sentence_ranges("").is_empty());
}

#[test]
fn paragraph_units_recognize_native_separators_and_keep_crlf_together() {
    let text = "a\rb\r\nc\nd\u{b}e\u{c}f\u{2029}";
    assert_eq!(
        strings(text, paragraph_ranges(text)),
        ["a\r", "b\r\n", "c\n", "d\u{b}", "e\u{c}", "f\u{2029}", ""]
    );
}

#[test]
fn hard_paragraph_units_distinguish_inline_soft_breaks_and_paragraph_objects() {
    let _scope = Dispatcher::unit_test_scope();
    let soft = InlineSnapshot {
        format: CharacterFormat::default(),
        kind: InlineSnapshotKind::LineBreak,
    };
    let soft_length = soft.text_length();
    let document = RichTextDocument::new();
    document
        .restore(&DocumentSnapshot {
            paragraphs: vec![
                ParagraphSnapshot {
                    inlines: vec![
                        InlineSnapshot::run("a"),
                        soft,
                        InlineSnapshot::run("b\u{b}c"),
                    ],
                    ..Default::default()
                },
                ParagraphSnapshot {
                    inlines: vec![InlineSnapshot::run("d")],
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .unwrap();
    let services = RichEditTextDocument::new(document);
    let range = services.get_range(0, 0).unwrap();
    range.expand(TextRangeUnit::Paragraph).unwrap();
    assert_eq!(range.range(), 0..1 + soft_length);
    range
        .set_index(TextRangeUnit::HardParagraph, 1, true)
        .unwrap();
    assert_eq!(range.range(), 0..5 + soft_length);
    assert_eq!(range.get_index(TextRangeUnit::HardParagraph).unwrap(), 1);
    range
        .set_index(TextRangeUnit::HardParagraph, -1, true)
        .unwrap();
    assert_eq!(range.text().unwrap(), "d");
    assert_eq!(range.get_index(TextRangeUnit::Paragraph).unwrap(), 4);
}

#[test]
fn hard_crlf_boundary_can_cross_nested_character_format_runs() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::new();
    document
        .restore(&DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                inlines: vec![
                    InlineSnapshot::run("a\r"),
                    InlineSnapshot {
                        format: CharacterFormat::default(),
                        kind: InlineSnapshotKind::Bold(vec![
                            InlineSnapshot::run(""),
                            InlineSnapshot::run("\nb"),
                        ]),
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    let services = RichEditTextDocument::new(document);
    let range = services.get_range(0, 0).unwrap();
    range
        .set_index(TextRangeUnit::HardParagraph, 1, true)
        .unwrap();
    assert_eq!(range.range(), 0..3);
    assert_eq!(range.text().unwrap(), "a\r\n");
    range
        .set_index(TextRangeUnit::HardParagraph, 2, true)
        .unwrap();
    assert_eq!(range.text().unwrap(), "b");
}

#[test]
fn move_counts_nonempty_collapse_and_alignment_as_the_first_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("alpha  beta gamma");
    let range = document.get_range(0, 5).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Word, 1).unwrap(), 1);
    assert_eq!(range.range(), 7..7);
    range.set_range(0, 7).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Word, 1).unwrap(), 1);
    assert_eq!(range.range(), 7..7);
    assert_eq!(range.move_by(TextRangeUnit::Word, 1).unwrap(), 1);
    assert_eq!(range.range(), 12..12);
    range.set_range(8, 10).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Word, -1).unwrap(), -1);
    assert_eq!(range.range(), 7..7);
    range.set_range(8, 10).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Word, -2).unwrap(), -2);
    assert_eq!(range.range(), 0..0);
    range.set_range(0, document.text_length()).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Word, i32::MAX).unwrap(), 1);
    assert_eq!(range.range(), 17..17);
    assert_eq!(range.move_by(TextRangeUnit::Word, 1).unwrap(), 0);
}

#[test]
fn selection_move_keeps_extension_direction_and_uses_native_collapse_count() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("one  two three");
    document.selection().set_range(8, 1).unwrap();
    assert_eq!(
        document
            .selection()
            .move_by(TextRangeUnit::Word, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(9));
    document.selection().set_range(9, 5).unwrap();
    assert_eq!(
        document
            .selection()
            .move_by(TextRangeUnit::Word, -1, true)
            .unwrap(),
        -1
    );
    assert_eq!(document.selection().selection(), EditorSelection::new(9, 0));
    assert_eq!(
        document
            .selection()
            .move_by(TextRangeUnit::Word, 1, true)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::new(9, 5));
}

#[test]
fn start_of_and_end_of_report_native_displacements_and_collapse_counts() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("one  two! tail");
    let range = document.get_range(1, 7).unwrap();
    assert_eq!(range.start_of(TextRangeUnit::Word, true).unwrap(), -1);
    assert_eq!(range.range(), 0..7);
    assert_eq!(range.end_of(TextRangeUnit::Word, true).unwrap(), 1);
    assert_eq!(range.range(), 0..8);
    assert_eq!(range.end_of(TextRangeUnit::Word, false).unwrap(), 1);
    assert_eq!(range.range(), 8..8);
    assert_eq!(range.end_of(TextRangeUnit::Word, false).unwrap(), 0);
    assert_eq!(range.start_of(TextRangeUnit::Word, false).unwrap(), 0);
    range.set_range(3, 4).unwrap();
    assert_eq!(range.end_of(TextRangeUnit::Story, false).unwrap(), 11);
    assert_eq!(range.range(), 14..14);
    assert_eq!(range.start_of(TextRangeUnit::Story, false).unwrap(), -14);
    assert_eq!(range.range(), 0..0);
}

#[test]
fn character_end_of_preserves_unit_boundaries_except_at_story_start() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("😀x");
    let range = document.get_range(0, 0).unwrap();
    assert_eq!(range.end_of(TextRangeUnit::Character, false).unwrap(), 2);
    assert_eq!(range.range(), 2..2);
    assert_eq!(range.end_of(TextRangeUnit::Character, false).unwrap(), 0);
    range.set_range(0, 2).unwrap();
    assert_eq!(range.end_of(TextRangeUnit::Character, false).unwrap(), 1);
    assert_eq!(range.range(), 2..2);
    range.set_range(0, 2).unwrap();
    assert_eq!(range.start_of(TextRangeUnit::Character, false).unwrap(), 0);
    assert_eq!(range.range(), 0..0);
}

#[test]
fn get_index_and_set_index_use_one_based_positive_and_end_relative_negative_units() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("one  two three");
    let range = document.get_range(6, 6).unwrap();
    assert_eq!(range.get_index(TextRangeUnit::Word).unwrap(), 2);
    range.set_index(TextRangeUnit::Word, -1, true).unwrap();
    assert_eq!(range.range(), 9..14);
    assert_eq!(range.text().unwrap(), "three");
    range.set_index(TextRangeUnit::Word, 2, true).unwrap();
    assert_eq!(
        range.range(),
        5..9,
        "extend relocates both endpoints to the entire indexed unit"
    );
    range.set_index(TextRangeUnit::Word, -3, false).unwrap();
    assert_eq!(range.range(), 0..0);
    range
        .set_range(document.text_length(), document.text_length())
        .unwrap();
    assert_eq!(range.get_index(TextRangeUnit::Word).unwrap(), 3);
    assert_eq!(range.get_index(TextRangeUnit::Story).unwrap(), 1);
    range.expand(TextRangeUnit::Story).unwrap();
    assert_eq!(range.range(), 0..14);
}

#[test]
fn invalid_unit_indices_are_nonmutating_including_extreme_signed_values() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("one two");
    let range = document.get_range(2, 5).unwrap();
    let revision = document.document().revision();
    for index in [0, 3, -3, i32::MAX, i32::MIN] {
        assert!(range.set_index(TextRangeUnit::Word, index, true).is_err());
        assert_eq!(range.range(), 2..5);
        assert_eq!(document.document().revision(), revision);
        assert!(!document.can_undo());
    }
}

#[test]
fn empty_and_terminal_paragraph_units_keep_the_model_coordinate_contract() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("");
    let range = document.document_range().unwrap();
    for unit in [
        TextRangeUnit::Character,
        TextRangeUnit::Word,
        TextRangeUnit::Sentence,
    ] {
        assert_eq!(range.get_index(unit).unwrap(), 0);
        assert!(range.set_index(unit, 1, true).is_err());
        assert_eq!(range.end_of(unit, false).unwrap(), 0);
    }
    for unit in [
        TextRangeUnit::Paragraph,
        TextRangeUnit::Line,
        TextRangeUnit::HardParagraph,
        TextRangeUnit::Story,
    ] {
        assert_eq!(range.get_index(unit).unwrap(), 1);
        range.set_index(unit, -1, true).unwrap();
        assert_eq!(range.range(), 0..0);
    }
    assert_eq!(range.story_length(), 0);
    document.set_text("a😀\n").unwrap();
    assert_eq!(range.story_length(), 4);
    assert_eq!(
        document
            .get_text(TextGetOptions::NONE)
            .unwrap()
            .encode_utf16()
            .count(),
        5
    );
    range.set_index(TextRangeUnit::Paragraph, -1, true).unwrap();
    assert_eq!(range.range(), 4..4);
    assert_eq!(range.get_index(TextRangeUnit::Paragraph).unwrap(), 2);
    assert_eq!(range.start_of(TextRangeUnit::Paragraph, false).unwrap(), 0);
}

#[test]
fn endpoint_movement_collapses_crossings_and_saturates_at_story_limits() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀b");
    let range = document.get_range(0, 1).unwrap();
    assert_eq!(
        range
            .move_start(TextRangeUnit::Character, i32::MAX)
            .unwrap(),
        3
    );
    assert_eq!(range.range(), 4..4);
    assert_eq!(
        range.move_end(TextRangeUnit::Character, i32::MIN).unwrap(),
        -3
    );
    assert_eq!(range.range(), 0..0);
    assert_eq!(
        range.move_end(TextRangeUnit::Character, i32::MIN).unwrap(),
        0
    );
    range.set_range(1, 3).unwrap();
    assert_eq!(range.move_start(TextRangeUnit::Story, 0).unwrap(), 0);
    assert_eq!(range.range(), 1..3);
}

#[test]
fn attached_line_units_follow_wrapping_and_reflow_while_headless_lines_are_paragraphs() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text(
        "one two three four five six seven eight",
    ));
    view.set_text_wrapping(TextWrapping::Wrap);
    let document = RichEditTextDocument::new(view.document());
    let range = document
        .get_range(document.text_length(), document.text_length())
        .unwrap();
    assert_eq!(range.get_index(TextRangeUnit::Line).unwrap(), 1);
    document.editor().set_text_view(Some(view.clone())).unwrap();
    measure(&view, 120.0, 1000.0);
    let wide_lines = view.line_metrics().len();
    assert!(wide_lines > 1);
    assert_eq!(range.get_index(TextRangeUnit::Line).unwrap(), wide_lines);
    measure(&view, 40.0, 1000.0);
    let narrow = view.line_metrics();
    assert!(narrow.len() > wide_lines);
    assert_eq!(range.get_index(TextRangeUnit::Line).unwrap(), narrow.len());
    range.set_index(TextRangeUnit::Line, 2, true).unwrap();
    assert_eq!(range.range(), narrow[1].start..narrow[2].start);
    assert_eq!(range.get_index(TextRangeUnit::Paragraph).unwrap(), 1);
    assert!(!document.can_undo());
    document.editor().set_text_view(None).unwrap();
    range.set_index(TextRangeUnit::Line, 1, true).unwrap();
    assert_eq!(range.range(), 0..document.text_length());
}

#[test]
fn line_units_cover_structural_marks_across_linked_overflow_views() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text("one"));
    view.blocks().add(Paragraph::with_text("two"));
    view.blocks().add(Paragraph::with_text("three"));
    view.set_max_lines(1);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 300.0, 100.0);
    measure(&overflow, 300.0, 100.0);
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    let range = document.get_range(0, 0).unwrap();
    range.set_index(TextRangeUnit::Line, 1, true).unwrap();
    assert_eq!(range.text().unwrap(), "one\n");
    range.set_index(TextRangeUnit::Line, 2, true).unwrap();
    assert_eq!(range.text().unwrap(), "two\n");
    range.set_index(TextRangeUnit::Line, -1, true).unwrap();
    assert_eq!(range.text().unwrap(), "three");
    assert_eq!(range.get_index(TextRangeUnit::Line).unwrap(), 3);
}

#[test]
fn incomplete_or_trimmed_line_layout_returns_an_error_without_moving_range() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text("one"));
    view.blocks().add(Paragraph::with_text("two"));
    view.set_max_lines(1);
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    let range = document.get_range(1, 2).unwrap();
    measure(&view, 100.0, 100.0);
    assert!(range.set_index(TextRangeUnit::Line, 1, true).is_err());
    assert_eq!(range.range(), 1..2);
    view.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    measure(&view, 100.0, 100.0);
    assert!(view.is_text_trimmed());
    assert!(range.expand(TextRangeUnit::Line).is_err());
    assert!(range.move_by(TextRangeUnit::Line, 1).is_err());
    assert_eq!(range.range(), 1..2);
    assert_eq!(range.move_by(TextRangeUnit::Line, 0).unwrap(), 0);
}

#[test]
fn text_view_association_is_weak_and_rejects_another_story() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.blocks()
        .add(Paragraph::with_text("one two three four five six"));
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    let other = RichTextBlock::new();
    assert!(document.editor().set_text_view(Some(other)).is_err());
    measure(&view, 40.0, 1000.0);
    let range = document
        .get_range(document.text_length(), document.text_length())
        .unwrap();
    assert!(range.get_index(TextRangeUnit::Line).unwrap() > 1);
    let weak = view.downgrade();
    drop(view);
    assert!(weak.upgrade().is_none());
    assert_eq!(range.get_index(TextRangeUnit::Line).unwrap(), 1);
}

fn displayed_services(view: &Ref<RichTextBlock>) -> RichEditTextDocument {
    let document = RichEditTextDocument::new(view.document());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    document
}

fn cluster_strings(document: &RichEditTextDocument) -> Vec<String> {
    let range = document
        .get_range(document.text_length(), document.text_length())
        .unwrap();
    let count = range.get_index(TextRangeUnit::Cluster).unwrap();
    (1..=count)
        .map(|index| {
            range
                .set_index(TextRangeUnit::Cluster, index as i32, true)
                .unwrap();
            range.text().unwrap()
        })
        .collect()
}

#[test]
fn cluster_units_use_shaping_ligatures_instead_of_unicode_graphemes() {
    let _scope = test_scope();
    let text = TextTestScope::new();
    text.shaper().add_ligature("ffi");
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text("office a\u{301}😀"));
    measure(&view, 500.0, 100.0);
    let document = displayed_services(&view);
    assert_eq!(
        cluster_strings(&document),
        ["o", "ffi", "c", "e", " ", "a\u{301}", "😀"]
    );
    assert_eq!(
        unit_ranges(&document.text(), UnicodeTextUnit::Character).len(),
        9,
        "Unicode graphemes do not merge the font's ffi ligature"
    );
    let range = document.get_range(2, 2).unwrap();
    assert_eq!(range.expand(TextRangeUnit::Cluster).unwrap(), 3);
    assert_eq!(range.range(), 1..4);
    assert_eq!(range.get_index(TextRangeUnit::Cluster).unwrap(), 2);
    assert!(!document.can_undo());
}

#[test]
fn cluster_units_keep_indic_shaping_groups_distinct_from_scalar_navigation() {
    use ferroui_base::media::text_formatting::testing::{TestFont, DEFAULT_FAMILY};
    let _scope = test_scope();
    let text = TextTestScope::with_fonts(vec![
        TestFont::new(DEFAULT_FAMILY).with_ranges(&[(0x20, 0x7e), (0x900, 0x97f)])
    ]);
    text.shaper().add_ligature("\u{915}\u{94d}\u{937}\u{93f}");
    let view = RichTextBlock::new();
    view.blocks()
        .add(Paragraph::with_text("\u{915}\u{94d}\u{937}\u{93f} x"));
    measure(&view, 500.0, 100.0);
    let document = displayed_services(&view);
    assert_eq!(
        cluster_strings(&document),
        ["\u{915}\u{94d}\u{937}\u{93f}", " ", "x"]
    );
    let range = document.get_range(0, 0).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Cluster, 1).unwrap(), 1);
    assert_eq!(range.range(), 4..4);
    range.set_range(0, 0).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Character, 1).unwrap(), 1);
    assert_eq!(range.range(), 1..1);
}

#[test]
fn cluster_units_follow_logical_bidi_order_and_rebase_wrapped_buffer_origins() {
    use ferroui_base::media::FlowDirection;
    let _scope = test_scope();
    let text = TextTestScope::new();
    text.shaper().add_ligature("fi");
    text.shaper().add_ligature("אב");
    for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
        let view = RichTextBlock::new();
        let paragraph = Paragraph::with_text("xx אבג fi zz");
        paragraph.set_flow_direction(direction);
        view.blocks().add(paragraph);
        view.set_text_wrapping(TextWrapping::Wrap);
        let document = displayed_services(&view);
        for width in [500.0, 40.0, 15.0, 500.0] {
            measure(&view, width, 1000.0);
            assert_eq!(
                cluster_strings(&document),
                ["x", "x", " ", "אב", "ג", " ", "fi", " ", "z", "z"]
            );
        }
    }
}

#[test]
fn cluster_units_cover_embedded_objects_soft_breaks_and_structural_paragraph_marks() {
    use crate::documents::{InlineUIContainer, LineBreak, Run};
    use crate::Border;
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("a");
    paragraph.add_child(InlineUIContainer::with_child(Border::new()));
    paragraph.add_child(LineBreak::new());
    paragraph.add_child(Run::with_text(Some("b")));
    view.blocks().add(paragraph);
    view.blocks().add(Paragraph::with_text("c"));
    measure(&view, 500.0, 1000.0);
    let document = displayed_services(&view);
    let soft = InlineSnapshot {
        format: CharacterFormat::default(),
        kind: InlineSnapshotKind::LineBreak,
    };
    let soft_text = if soft.text_length() == 2 {
        "\r\n"
    } else {
        "\n"
    };
    assert_eq!(
        cluster_strings(&document),
        ["a", "\u{fffc}", soft_text, "b", "\n", "c"]
    );
}

#[test]
fn cluster_units_keep_omitted_line_break_glyphs_addressable() {
    let _scope = test_scope();
    let text = TextTestScope::new();
    text.shaper().set_drop_line_breaks(true);
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text("a\r\nb\u{2028}c"));
    measure(&view, 500.0, 1000.0);
    let document = displayed_services(&view);
    assert_eq!(
        cluster_strings(&document),
        ["a", "\r\n", "b", "\u{2028}", "c"]
    );
}

#[test]
fn cluster_units_traverse_linked_pages_and_refresh_after_format_changes() {
    let _scope = test_scope();
    let text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("fi");
    view.blocks().add(paragraph.clone());
    view.blocks().add(Paragraph::with_text("fi"));
    view.set_max_lines(1);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 300.0, 100.0);
    measure(&overflow, 300.0, 100.0);
    let document = displayed_services(&view);
    assert_eq!(cluster_strings(&document), ["f", "i", "\n", "f", "i"]);
    text.shaper().add_ligature("fi");
    paragraph.set_font_size(18.0);
    measure(&view, 300.0, 100.0);
    measure(&overflow, 300.0, 100.0);
    assert_eq!(cluster_strings(&document), ["fi", "\n", "fi"]);
}

#[test]
fn cluster_navigation_requires_a_complete_untrimmed_live_view_without_fallback() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text("one"));
    view.blocks().add(Paragraph::with_text("two"));
    let document = RichEditTextDocument::new(view.document());
    let range = document.get_range(1, 2).unwrap();
    assert!(range.expand(TextRangeUnit::Cluster).is_err());
    document.editor().set_text_view(Some(view.clone())).unwrap();
    view.set_max_lines(1);
    measure(&view, 100.0, 100.0);
    assert!(range.set_index(TextRangeUnit::Cluster, 1, true).is_err());
    view.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    measure(&view, 100.0, 100.0);
    assert!(range.move_by(TextRangeUnit::Cluster, 1).is_err());
    assert_eq!(range.range(), 1..2);
    assert_eq!(range.move_by(TextRangeUnit::Cluster, 0).unwrap(), 0);
    assert!(!document.can_undo());
    drop(view);
    assert!(range.get_index(TextRangeUnit::Cluster).is_err());
}

#[test]
fn cluster_unit_endpoints_and_directed_selection_use_shaped_boundaries() {
    let _scope = test_scope();
    let text = TextTestScope::new();
    text.shaper().add_ligature("ffi");
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text("office"));
    measure(&view, 300.0, 100.0);
    let document = displayed_services(&view);
    let range = document.get_range(2, 3).unwrap();
    assert_eq!(range.start_of(TextRangeUnit::Cluster, true).unwrap(), -1);
    assert_eq!(range.end_of(TextRangeUnit::Cluster, true).unwrap(), 1);
    assert_eq!(range.range(), 1..4);
    assert_eq!(range.move_start(TextRangeUnit::Cluster, 2).unwrap(), 2);
    assert_eq!(range.range(), 5..5);
    assert_eq!(range.move_end(TextRangeUnit::Cluster, -2).unwrap(), -2);
    assert_eq!(range.range(), 1..1);
    document.selection().set_range(5, 2).unwrap();
    assert_eq!(
        document
            .selection()
            .move_by(TextRangeUnit::Cluster, -1, true)
            .unwrap(),
        -1
    );
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 1));
    assert_eq!(
        document
            .selection()
            .move_by(TextRangeUnit::Cluster, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(5));
    range.set_range(0, 0).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Cluster, i32::MAX).unwrap(), 4);
    assert_eq!(range.move_by(TextRangeUnit::Cluster, i32::MIN).unwrap(), -4);
}

#[test]
fn empty_shaped_story_has_no_cluster_but_keeps_its_caret() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    measure(&view, 100.0, 100.0);
    let document = displayed_services(&view);
    let range = document.document_range().unwrap();
    assert_eq!(range.get_index(TextRangeUnit::Cluster).unwrap(), 0);
    assert_eq!(range.expand(TextRangeUnit::Cluster).unwrap(), 0);
    assert_eq!(range.move_by(TextRangeUnit::Cluster, 1).unwrap(), 0);
    assert!(range.set_index(TextRangeUnit::Cluster, 1, true).is_err());
    assert_eq!(range.range(), 0..0);
}

#[test]
fn tom_layout_navigation_rejects_reentrant_formatting_inside_a_document_batch() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for unit in [TextRangeUnit::Cluster, TextRangeUnit::Line] {
        let view = RichTextBlock::new();
        let paragraph = Paragraph::with_text("unchanged text");
        view.blocks().add(paragraph.clone());
        let document = displayed_services(&view);
        let range = document.get_range(1, 2).unwrap();
        let revision = document.document().revision();
        let batch = document.document().begin_edit();
        let changed = Rc::new(Cell::new(false));
        let observed = changed.clone();
        let subscription = view.property_changed(move |change| {
            if change.property().name() == "BaselineOffset" && !changed.replace(true) {
                paragraph.set_font_size(24.0);
            }
        });
        assert!(matches!(range.expand(unit), Err(EditError::ReentrantEdit)));
        assert!(observed.get());
        assert_eq!(
            document.document().revision(),
            revision,
            "outer batch defers revisions"
        );
        assert_eq!(range.range(), 1..2);
        subscription.dispose();
        drop(batch);
    }
}

#[test]
fn malformed_shaped_clusters_are_rejected_before_returning_navigation_units() {
    use ferroui_base::media::text_formatting::testing::run_properties;
    use ferroui_base::media::text_formatting::{GlyphInfo, ShapedBuffer};
    use ferroui_base::utilities::ReadOnlyMemory;
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let properties = run_properties(12.0);
    for (text, clusters) in [
        ("ab", vec![-1, 0]),
        ("ab", vec![0, 2]),
        ("😀", vec![0, 1]),
        ("unshaped", vec![]),
    ] {
        let buffer = ShapedBuffer::new(
            ReadOnlyMemory::from_str(text),
            clusters.len(),
            properties.cached_glyph_typeface(),
            12.0,
            0,
        );
        for (index, cluster) in clusters.into_iter().enumerate() {
            buffer.set(index, GlyphInfo::new(1, cluster, 6.0));
        }
        let run = ShapedTextRun::new(buffer, properties.clone());
        let utf16: Vec<_> = text.encode_utf16().collect();
        let mut ranges = Vec::new();
        assert!(append_shaped_clusters(&utf16, 0..utf16.len(), &run, &mut ranges).is_err());
        assert!(
            ranges.is_empty(),
            "invalid shaping cannot yield guessed units"
        );
    }
}

#[test]
fn tom_selection_navigation_reads_directed_endpoints_after_layout_callbacks() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (unit, text, selected, expected) in [
        (TextRangeUnit::Cluster, "abcd", 2, 3),
        (TextRangeUnit::Line, "ab\ncd\nef", 3, 6),
    ] {
        let view = RichTextBlock::new();
        view.set_document(RichTextDocument::from_text(text));
        let document = displayed_services(&view);
        let editor = document.editor();
        let subscription = view.property_changed(move |change| {
            if change.property().name() == "BaselineOffset" {
                editor
                    .set_selection(EditorSelection::caret(selected))
                    .unwrap();
            }
        });
        assert_eq!(document.selection().move_by(unit, 1, false).unwrap(), 1);
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(expected)
        );
        subscription.dispose();
    }
}

#[test]
fn tom_layout_navigation_rejects_reentrant_presentation_without_a_document_edit() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for unit in [TextRangeUnit::Cluster, TextRangeUnit::Line] {
        let view = RichTextBlock::new();
        view.blocks().add(Paragraph::with_text("unchanged text"));
        let document = displayed_services(&view);
        let range = document.get_range(1, 2).unwrap();
        let version = document.document().mutation_version();
        let changed = Rc::new(Cell::new(false));
        let observed = changed.clone();
        let weak = view.downgrade();
        let subscription = view.property_changed(move |change| {
            if change.property().name() == "BaselineOffset" && !changed.replace(true) {
                weak.upgrade().unwrap().set_font_size(24.0);
            }
        });
        assert!(matches!(range.expand(unit), Err(EditError::ReentrantEdit)));
        assert!(observed.get());
        assert_eq!(document.document().mutation_version(), version);
        assert_eq!(range.range(), 1..2);
        subscription.dispose();
    }
}

#[test]
fn cluster_units_preserve_zero_advance_soft_hyphens_and_joining_controls() {
    let _scope = test_scope();
    let text = TextTestScope::new();
    text.shaper().add_ligature("fi");
    let view = RichTextBlock::new();
    view.blocks()
        .add(Paragraph::with_text("fi\u{ad}fi\u{200b}x"));
    view.set_text_wrapping(TextWrapping::Wrap);
    let document = displayed_services(&view);
    for width in [500.0, 12.0, 500.0] {
        measure(&view, width, 1000.0);
        assert_eq!(
            cluster_strings(&document),
            ["fi", "\u{ad}", "fi", "\u{200b}", "x"]
        );
    }
}
