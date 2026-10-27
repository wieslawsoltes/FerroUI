use crate::test_support::{engine, FONT};
use crate::*;

fn collect(prepared: &PreparedRichInline, width: f64) -> Vec<RichInlineLine> {
    let mut lines = Vec::new();
    PretextLayout::walk_rich_inline_line_ranges(prepared, width, |range| {
        lines.push(PretextLayout::materialize_rich_inline_line_range(
            prepared, &range,
        ));
    });
    lines
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}

// Port of RichInline_NonMaterializingRangeWalkerMatchesMaterialization.
#[test]
fn non_materializing_range_walker_matches_materialization() {
    let layout = engine();
    let prepared = layout
        .prepare_rich_inline(&[
            RichInlineItem::new("Ship ", FONT),
            RichInlineItem::new("@maya", "700 12px Test Sans")
                .with_break_mode(RichInlineBreakMode::Never)
                .with_extra_width(18.0),
            RichInlineItem::new("'s rich note wraps cleanly", FONT),
        ])
        .unwrap();
    let mut ranged_lines = Vec::new();
    let range_count = PretextLayout::walk_rich_inline_line_ranges(&prepared, 120.0, |line| {
        ranged_lines.push(line)
    });
    let lines = collect(&prepared, 120.0);
    assert_eq!(range_count, lines.len());
    assert_eq!(ranged_lines.len(), lines.len());
    let max_width = ranged_lines
        .iter()
        .map(|line| line.width)
        .fold(0.0, f64::max);
    assert_eq!(
        RichInlineStats {
            line_count: range_count,
            max_line_width: max_width
        },
        PretextLayout::measure_rich_inline_stats(&prepared, 120.0),
    );
    for (range, line) in ranged_lines.iter().zip(&lines) {
        assert_eq!(range.width, line.width);
        assert_eq!(range.end, line.end);
        let fragments: Vec<_> = line
            .fragments
            .iter()
            .map(|fragment| RichInlineFragmentRange {
                item_index: fragment.item_index,
                gap_before: fragment.gap_before,
                occupied_width: fragment.occupied_width,
                start: fragment.start,
                end: fragment.end,
            })
            .collect();
        assert_eq!(range.fragments, fragments);
    }
}

#[test]
fn boundary_whitespace_collapses_across_empty_source_items() {
    let layout = engine();
    let prepared = layout
        .prepare_rich_inline(&[
            RichInlineItem::new("  alpha\t", FONT),
            RichInlineItem::new("", FONT),
            RichInlineItem::new("\r\n\t", "32px Test Sans"),
            RichInlineItem::new("  beta  ", FONT),
        ])
        .unwrap();
    assert_eq!(prepared.item_count(), 2);
    assert_eq!(prepared.source_item_count(), 4);
    assert!(prepared.prepared_item(1).is_none());
    assert!(prepared.prepared_item(2).is_none());
    let lines = collect(&prepared, 1000.0);
    assert_eq!(lines.len(), 1);
    let fragments = &lines[0].fragments;
    assert_eq!(
        fragments
            .iter()
            .map(|fragment| fragment.item_index)
            .collect::<Vec<_>>(),
        [0, 3]
    );
    assert_eq!(fragments[0].text, "alpha");
    assert_eq!(fragments[1].text, "beta");
    assert_eq!(fragments[0].gap_before, 0.0);
    // The first pending gap keeps the font of the preceding item, even when
    // another whitespace-only item has a different font size.
    close(fragments[1].gap_before, 16.0 * 0.33);
}

#[test]
fn atomic_items_overflow_intact_and_drop_their_gap_at_line_start() {
    let layout = engine();
    let prepared = layout
        .prepare_rich_inline(&[
            RichInlineItem::new("hi ", FONT),
            RichInlineItem::new("long badge", FONT)
                .with_break_mode(RichInlineBreakMode::Never)
                .with_extra_width(18.0),
            RichInlineItem::new(" tail", FONT),
        ])
        .unwrap();
    let lines = collect(&prepared, 40.0);
    assert_eq!(lines[0].fragments[0].text, "hi");
    assert_eq!(lines[1].fragments.len(), 1);
    assert_eq!(lines[1].fragments[0].text, "long badge");
    assert_eq!(lines[1].fragments[0].gap_before, 0.0);
    assert!(lines[1].width > 40.0);
    assert_eq!(lines[2].fragments[0].text, "tail");
    assert_eq!(lines[2].fragments[0].gap_before, 0.0);
}

#[test]
fn a_word_uses_a_fresh_line_before_splitting_in_the_remaining_gap() {
    let layout = engine();
    let prepared = layout
        .prepare_rich_inline(&[
            RichInlineItem::new("abc ", FONT),
            RichInlineItem::new("elephant", FONT),
        ])
        .unwrap();
    let lines = collect(&prepared, 80.0);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].fragments[0].text, "abc");
    assert_eq!(lines[1].fragments[0].text, "elephant");
}

#[test]
fn mixed_fonts_and_reserved_widths_are_applied_to_every_fragment() {
    let layout = engine();
    let prepared = layout
        .prepare_rich_inline(&[
            RichInlineItem::new("ab", "10px Test Sans"),
            RichInlineItem::new("abcdef", "20px Test Sans").with_extra_width(5.0),
        ])
        .unwrap();
    let lines = collect(&prepared, 43.0);
    assert!(lines.len() >= 2);
    close(lines[0].fragments[0].occupied_width, 12.0);
    for line in &lines {
        close(
            line.width,
            line.fragments
                .iter()
                .map(|fragment| fragment.gap_before + fragment.occupied_width)
                .sum(),
        );
        for fragment in &line.fragments {
            if fragment.item_index == 1 {
                close(
                    fragment.occupied_width,
                    fragment.text.chars().count() as f64 * 12.0 + 5.0,
                );
            }
        }
    }
    let text: String = lines
        .iter()
        .flat_map(|line| &line.fragments)
        .map(|fragment| fragment.text.as_str())
        .collect();
    assert_eq!(text, "ababcdef");
}

#[test]
fn variable_width_streaming_preserves_source_item_and_grapheme_progress() {
    let layout = engine();
    let prepared = layout
        .prepare_rich_inline(&[
            RichInlineItem::new("alpha ", FONT),
            RichInlineItem::new("👨‍👩‍👧‍👦", FONT).with_break_mode(RichInlineBreakMode::Never),
            RichInlineItem::new("beta gamma", FONT),
        ])
        .unwrap();
    let mut cursor = RichInlineCursor::default();
    let mut fragments = Vec::new();
    let widths = [25.0, 190.0, 40.0, 400.0];
    for width in widths {
        let Some(range) =
            PretextLayout::layout_next_rich_inline_line_range(&prepared, width, Some(cursor))
        else {
            break;
        };
        assert!(range.end > cursor);
        cursor = range.end;
        fragments
            .extend(PretextLayout::materialize_rich_inline_line_range(&prepared, &range).fragments);
    }
    assert!(
        PretextLayout::layout_next_rich_inline_line_range(&prepared, 400.0, Some(cursor)).is_none()
    );
    assert_eq!(
        fragments
            .iter()
            .filter(|fragment| fragment.item_index == 1)
            .count(),
        1
    );
    let restored: String = fragments
        .iter()
        .map(|fragment| fragment.text.as_str())
        .collect();
    assert_eq!(restored.replace(' ', ""), "alpha👨‍👩‍👧‍👦betagamma");
}

#[test]
fn nonbreaking_boundary_characters_are_content() {
    let layout = engine();
    let prepared = layout
        .prepare_rich_inline(&[RichInlineItem::new("\u{a0}alpha\u{202f}", FONT)])
        .unwrap();
    let lines = collect(&prepared, 1000.0);
    assert_eq!(lines[0].fragments[0].text, "\u{a0}alpha\u{202f}");
}

#[test]
fn empty_and_whitespace_only_flows_have_no_lines() {
    let layout = engine();
    for items in [Vec::new(), vec![RichInlineItem::new(" \t\n\r\u{c}", FONT)]] {
        let prepared = layout.prepare_rich_inline(&items).unwrap();
        assert!(prepared.is_empty());
        assert!(collect(&prepared, 40.0).is_empty());
        assert_eq!(
            PretextLayout::measure_rich_inline_stats(&prepared, 40.0),
            RichInlineStats::default()
        );
    }
}

#[test]
fn zero_width_text_is_materialized_and_advances_the_rich_cursor() {
    let layout = PretextLayout::new(std::rc::Rc::new(DelegateTextMeasurerFactory::new(
        |_: &str, _: &str| 0.0,
    )));
    let prepared = layout
        .prepare_rich_inline(&[
            RichInlineItem::new("alpha", FONT),
            RichInlineItem::new("badge", FONT).with_break_mode(RichInlineBreakMode::Never),
            RichInlineItem::new("omega", FONT),
        ])
        .unwrap();
    let range = PretextLayout::layout_next_rich_inline_line_range(&prepared, 0.0, None).unwrap();
    assert_eq!(range.width, 0.0);
    assert_eq!(range.end.item_index, 3);
    assert_eq!(range.fragments.len(), 3);
    let line = PretextLayout::materialize_rich_inline_line_range(&prepared, &range);
    assert_eq!(
        line.fragments
            .iter()
            .map(|fragment| fragment.text.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "badge", "omega"],
    );
    assert!(
        PretextLayout::layout_next_rich_inline_line_range(&prepared, 0.0, Some(range.end))
            .is_none()
    );
    assert_eq!(
        PretextLayout::measure_rich_inline_stats(&prepared, 0.0),
        RichInlineStats {
            line_count: 1,
            max_line_width: 0.0
        },
    );
}
