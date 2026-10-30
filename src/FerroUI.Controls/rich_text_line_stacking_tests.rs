//! Numeric cases from the pinned ParagraphNode::ApplyLineStackingStrategy,
//! GetTextBounds and continuation paths. The 20-em / 6- and 26-pixel cases
//! adapt the Full-bounds entries in TextBlockLineStackingStrategies.xaml.
//! That upstream fixture is a layout smoke test, not a pixel oracle.

use super::RichTextLayout;
use crate::documents::{Block, LineSpacingRule, LineStackingStrategy, Paragraph, Run};
use crate::test_support::test_scope;
use crate::{RichTextBlock, RichTextBlockOverflow};
use ferroui_base::media::text_formatting::testing::{
    DrawCall, RecordingDrawingSink, TextTestScope,
};
use ferroui_base::media::{Brushes, TextTrimming, TextWrapping};
use ferroui_base::{Point, Rect, Ref, Size};

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}

fn rich(text: &str, strategy: LineStackingStrategy, height: f64) -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.set_font_size(20.0);
    view.set_foreground(Some(Brushes::black()));
    view.set_text_wrapping(TextWrapping::NoWrap);
    view.set_line_stacking_strategy(strategy);
    view.set_line_height(height);
    view.blocks().add(Paragraph::with_text(text));
    view
}

fn measure(view: &RichTextBlock, height: f64) {
    view.measure(Size::new(300.0, height));
    view.arrange(Rect::new(0.0, 0.0, 300.0, height));
}

fn assert_lines(layout: &RichTextLayout, advances: &[f64], offsets: &[f64]) {
    assert_eq!(layout.lines.len(), advances.len());
    assert_eq!(advances.len(), offsets.len());
    let mut allocation = 0.0;
    for ((entry, advance), offset) in layout.lines.iter().zip(advances).zip(offsets) {
        close(entry.advance, *advance);
        close(entry.allocation_y, allocation);
        close(entry.origin.y, allocation + offset);
        allocation += advance;
    }
    close(layout.size.height, allocation);
}

#[test]
fn rich_line_stacking_automatic_height_uses_natural_metrics_for_all_strategies() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for strategy in [
        LineStackingStrategy::MaxHeight,
        LineStackingStrategy::BlockLineHeight,
        LineStackingStrategy::BaselineToBaseline,
    ] {
        let view = rich("a\nb\nc", strategy, f64::NAN);
        measure(&view, 200.0);
        let layout = view.page_layout();
        assert_lines(&layout, &[22.0; 3], &[0.0; 3]);
        for line in &layout.lines {
            close(line.line.height(), 22.0);
            close(line.line.baseline(), 16.0);
        }
    }
}

#[test]
fn rich_line_stacking_full_bounds_fixture_matrix_retains_natural_glyph_boxes() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    // The synthetic font has ascent 800, descent 200 and gap 100. The native
    // regular-family baseline ratio is 8/11, without half of the line gap.
    let cases = [
        (LineStackingStrategy::MaxHeight, 6.0, [22.0; 3], [0.0; 3]),
        (LineStackingStrategy::MaxHeight, 26.0, [26.0; 3], [0.0; 3]),
        (
            LineStackingStrategy::BlockLineHeight,
            6.0,
            [6.0; 3],
            [-128.0 / 11.0; 3],
        ),
        (
            LineStackingStrategy::BlockLineHeight,
            26.0,
            [26.0; 3],
            [32.0 / 11.0; 3],
        ),
        (
            LineStackingStrategy::BaselineToBaseline,
            6.0,
            [194.0 / 11.0, 6.0, 114.0 / 11.0],
            [0.0, -128.0 / 11.0, -128.0 / 11.0],
        ),
        (
            LineStackingStrategy::BaselineToBaseline,
            26.0,
            [254.0 / 11.0, 26.0, 274.0 / 11.0],
            [0.0, 32.0 / 11.0, 32.0 / 11.0],
        ),
    ];
    for (strategy, height, advances, offsets) in cases {
        let view = rich("a\nb\nc", strategy, height);
        measure(&view, 200.0);
        let layout = view.page_layout();
        assert_lines(&layout, &advances, &offsets);
        let metrics = view.line_metrics();
        for (entry, metric) in layout.lines.iter().zip(metrics) {
            close(entry.line.height(), 22.0);
            close(entry.line.baseline(), 16.0);
            close(metric.bounds.y, entry.allocation_y);
            close(metric.bounds.height, entry.advance);
            close(metric.baseline, entry.origin.y + 16.0);
        }
    }
}

#[test]
fn rich_line_stacking_single_and_empty_lines_take_first_line_precedence() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for text in ["a", ""] {
        let view = rich(text, LineStackingStrategy::BaselineToBaseline, 33.0);
        if text.is_empty() {
            view.blocks().clear();
        }
        measure(&view, 100.0);
        let layout = view.page_layout();
        // A one-line fragment is both first and last. The native `if / else if`
        // applies the leading adjustment only: 33 - (33 - 22) * 8/11 = 25.
        assert_lines(&layout, &[25.0], &[0.0]);
        close(view.baseline_offset(), 16.0);
        let caret = view.get_rect_from_character_index(0);
        close(caret.y, 0.0);
        close(caret.height, 25.0);
        assert!(!view.has_overflow_content());
    }
}

#[test]
fn rich_line_stacking_mixed_run_sizes_use_paragraph_font_ratio() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("", LineStackingStrategy::BlockLineHeight, 55.0);
    let paragraph = Paragraph::new();
    for (text, size) in [("a\n", 30.0), ("b\n", 20.0), ("c", 40.0)] {
        let run = Run::with_text(Some(text));
        run.set_font_size(size);
        paragraph.add_child(run);
    }
    view.blocks().clear();
    view.blocks().add(paragraph);
    measure(&view, 300.0);
    let layout = view.page_layout();
    assert_lines(&layout, &[55.0; 3], &[16.0, 24.0, 8.0]);
    for (entry, natural_height) in layout.lines.iter().zip([33.0, 22.0, 44.0]) {
        close(entry.line.height(), natural_height);
    }
    drop(layout);

    view.set_line_stacking_strategy(LineStackingStrategy::BaselineToBaseline);
    measure(&view, 300.0);
    assert_lines(&view.page_layout(), &[39.0, 55.0, 52.0], &[0.0, 24.0, 8.0]);
}

#[test]
fn rich_line_stacking_draws_at_glyph_origin_and_exposes_allocated_caret_and_range_bounds() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("a\nb\nc", LineStackingStrategy::BlockLineHeight, 33.0);
    measure(&view, 200.0);
    let layout = view.page_layout();
    assert_lines(&layout, &[33.0; 3], &[8.0; 3]);

    let mut sink = RecordingDrawingSink::new();
    layout.draw(&mut sink, Point::new(4.0, 7.0));
    let calls = sink.calls();
    for (index, letter) in ['a', 'b', 'c'].into_iter().enumerate() {
        let entry = &layout.lines[index];
        let drawn = calls
            .iter()
            .find_map(|call| match call {
                DrawCall::GlyphRun { text, origin } if text.contains(letter) => Some(*origin),
                _ => None,
            })
            .expect("the real line must draw its glyph run");
        close(drawn.y, 14.0 + index as f64 * 33.0);
        close(drawn.x, 4.0);

        let caret = view.get_rect_from_character_index(entry.start());
        close(caret.y, index as f64 * 33.0);
        close(caret.height, 33.0);
        let rectangles = view.get_rects_for_range(entry.start()..entry.start() + 1);
        assert_eq!(rectangles.len(), 1);
        close(rectangles[0].y, caret.y);
        close(rectangles[0].height, caret.height);
        close(rectangles[0].width, 10.0);
        assert_eq!(
            view.hit_test_text_position(Point::new(caret.x, caret.y + 16.5)),
            entry.start()
        );
    }
    assert_eq!(sink.transform_depth(), 0);
    // Native hit testing includes the glyph offset in its line boundary.
    assert_eq!(
        view.hit_test_text_position(Point::new(0.0, 40.0)),
        layout.lines[0].start()
    );
    assert_eq!(
        view.hit_test_text_position(Point::new(0.0, 41.0)),
        layout.lines[1].start()
    );
}

#[test]
fn rich_line_stacking_continuations_reapply_first_line_rule_without_losing_text() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("a\nb\nc", LineStackingStrategy::BaselineToBaseline, 33.0);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));

    for _ in 0..2 {
        measure(&view, 55.0);
        measure(&overflow, 55.0);
        assert_lines(&view.page_layout(), &[25.0], &[0.0]);
        assert_lines(&overflow.page_layout(), &[25.0, 30.0], &[0.0, 8.0]);
        assert!(view.has_overflow_content());
        assert!(!overflow.has_overflow_content());
        assert_eq!(view.content_end(), overflow.content_start());
        assert_eq!(overflow.content_end(), view.document().text_length());

        measure(&view, 58.0);
        measure(&overflow, 55.0);
        assert_lines(&view.page_layout(), &[25.0, 33.0], &[0.0, 8.0]);
        assert_lines(&overflow.page_layout(), &[25.0], &[0.0]);
        assert_eq!(view.content_end(), overflow.content_start());
        assert_eq!(overflow.content_end(), view.document().text_length());

        measure(&view, 88.0);
        measure(&overflow, 55.0);
        assert_lines(&view.page_layout(), &[25.0, 33.0, 30.0], &[0.0, 8.0, 8.0]);
        assert!(!view.has_overflow_content());
        assert!(overflow.page_layout().lines.is_empty());
    }
}

#[test]
fn rich_line_stacking_local_values_and_inherited_changes_reflow_each_paragraph() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("a", LineStackingStrategy::BlockLineHeight, 33.0);
    let second = Paragraph::with_text("b");
    second.set_line_height(40.0);
    second.set_line_stacking_strategy(LineStackingStrategy::MaxHeight);
    view.blocks().add(second.clone());
    measure(&view, 200.0);
    assert_lines(&view.page_layout(), &[33.0, 40.0], &[8.0, 0.0]);

    second.set_line_stacking_strategy(LineStackingStrategy::BaselineToBaseline);
    second.set_line_height(11.0);
    view.set_line_height(44.0);
    measure(&view, 200.0);
    assert_lines(&view.page_layout(), &[44.0, 19.0], &[16.0, 0.0]);

    second.clear_value(Block::line_height_property());
    second.clear_value(Block::line_stacking_strategy_property());
    measure(&view, 200.0);
    assert_lines(&view.page_layout(), &[44.0; 2], &[16.0; 2]);
}

#[test]
fn rich_line_stacking_explicit_spacing_rules_keep_precedence_and_range_geometry() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (rule, spacing, advance, natural, baseline) in [
        (LineSpacingRule::Multiple, 2.0, 44.0, 22.0, 16.0),
        (LineSpacingRule::Exactly, 12.0, 12.0, 12.0, 16.0),
    ] {
        for strategy in [
            LineStackingStrategy::MaxHeight,
            LineStackingStrategy::BlockLineHeight,
            LineStackingStrategy::BaselineToBaseline,
        ] {
            let view = rich("", strategy, 99.0);
            let paragraph = Paragraph::with_text("a\nb");
            paragraph.set_line_spacing_rule(rule);
            paragraph.set_line_spacing(spacing);
            view.blocks().clear();
            view.blocks().add(paragraph);
            measure(&view, 200.0);
            let layout = view.page_layout();
            assert_lines(&layout, &[advance; 2], &[0.0; 2]);
            for entry in &layout.lines {
                close(entry.line.height(), natural);
                close(entry.line.baseline(), baseline);
                let caret = view.get_rect_from_character_index(entry.start());
                close(caret.y, entry.allocation_y);
                close(caret.height, advance);
                let rectangles = view.get_rects_for_range(entry.start()..entry.start() + 1);
                assert_eq!(rectangles.len(), 1);
                close(rectangles[0].y, entry.allocation_y);
                close(rectangles[0].height, natural);
            }
        }
    }
}

#[test]
fn rich_line_stacking_max_lines_trimming_and_oversized_first_line_make_progress() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich(
        "alpha\nbeta\ngamma",
        LineStackingStrategy::BaselineToBaseline,
        33.0,
    );
    view.set_max_lines(2);
    view.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    measure(&view, 200.0);
    assert_lines(&view.page_layout(), &[25.0, 33.0], &[0.0, 8.0]);
    assert!(view.is_text_trimmed());
    assert!(view.has_overflow_content());

    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 1.0);
    measure(&overflow, 200.0);
    assert_lines(&view.page_layout(), &[25.0], &[0.0]);
    assert!(!view.is_text_trimmed());
    assert_lines(&overflow.page_layout(), &[25.0, 30.0], &[0.0, 8.0]);
    assert_eq!(view.content_end(), overflow.content_start());
    assert_eq!(overflow.content_end(), view.document().text_length());
}
