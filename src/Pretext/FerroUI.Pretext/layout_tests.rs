use std::rc::Rc;

use crate::pretext_layout_line_break::{count_prepared_lines, walk_prepared_lines};
use crate::pretext_layout_line_text::segment_grapheme_offsets;
use crate::test_support::{engine, measure_width, FONT, LINE_HEIGHT, PRE_WRAP};
use crate::{
    LayoutCursor, LayoutLine, LayoutLineRange, LayoutLinesResult, LayoutResult, LineStats,
    PrepareOptions, PreparedTextWithSegments, PretextLayout, WordBreakMode,
};

fn cursor(segment_index: usize, grapheme_index: usize) -> LayoutCursor {
    LayoutCursor {
        segment_index,
        grapheme_index,
    }
}

fn prepare(text: &str) -> Rc<PreparedTextWithSegments> {
    prepare_options(text, PrepareOptions::default())
}

fn prepare_options(text: &str, options: PrepareOptions) -> Rc<PreparedTextWithSegments> {
    engine().prepare_with_segments(text, FONT, options).unwrap()
}

fn lines(prepared: &PreparedTextWithSegments, width: f64) -> LayoutLinesResult {
    PretextLayout::layout_with_lines(prepared, width, LINE_HEIGHT)
}

fn texts(result: &LayoutLinesResult) -> Vec<&str> {
    result.lines.iter().map(|line| line.text.as_str()).collect()
}

fn stream(
    prepared: &PreparedTextWithSegments,
    width: f64,
    start: Option<LayoutCursor>,
) -> Vec<LayoutLine> {
    let mut result = Vec::new();
    let mut start = start.unwrap_or(cursor(0, 0));
    while let Some(line) = PretextLayout::layout_next_line(prepared, start, width) {
        assert!(line.end > start, "line cursor must advance");
        start = line.end;
        result.push(line);
    }
    result
}

fn stream_widths(prepared: &PreparedTextWithSegments, widths: &[f64]) -> Vec<LayoutLine> {
    let mut result = Vec::new();
    let mut start = cursor(0, 0);
    for &width in widths {
        let Some(line) = PretextLayout::layout_next_line(prepared, start, width) else {
            return result;
        };
        assert!(line.end > start, "line cursor must advance");
        start = line.end;
        result.push(line);
    }
    assert!(
        PretextLayout::layout_next_line(prepared, start, *widths.last().unwrap()).is_none(),
        "enough widths must finish the paragraph"
    );
    result
}

fn slice(prepared: &PreparedTextWithSegments, start: LayoutCursor, end: LayoutCursor) -> String {
    let mut result = String::new();
    for index in start.segment_index..=end.segment_index.min(prepared.segments.len()) {
        let Some(segment) = prepared.segments.get(index) else {
            break;
        };
        let offsets = segment_grapheme_offsets(prepared, index);
        let first = if index == start.segment_index {
            start.grapheme_index
        } else {
            0
        };
        let last = if index == end.segment_index {
            end.grapheme_index
        } else {
            offsets.len() - 1
        };
        result.push_str(&segment[offsets[first]..offsets[last]]);
    }
    result
}

fn reconstruct(prepared: &PreparedTextWithSegments, lines: &[LayoutLine]) -> String {
    lines
        .iter()
        .map(|line| slice(prepared, line.start, line.end))
        .collect()
}

fn terminal(prepared: &PreparedTextWithSegments) -> LayoutCursor {
    cursor(prepared.segments.len(), 0)
}

fn soft_hyphen_case() -> (Rc<PreparedTextWithSegments>, f64) {
    let prepared = prepare("foo trans\u{ad}atlantic said \"hello\" to 世界 and waved.");
    let width = prepared.widths[0]
        + prepared.widths[1]
        + prepared.widths[2]
        + prepared.breakable_widths[4].as_ref().unwrap()[0]
        + prepared.discretionary_hyphen_width
        + 0.1;
    (prepared, width)
}

#[test]
fn line_count_grows_monotonically_as_width_shrinks() {
    let prepared = engine()
        .prepare(
            "The quick brown fox jumps over the lazy dog",
            FONT,
            PrepareOptions::default(),
        )
        .unwrap();
    let mut previous = 0;
    for width in [320.0, 200.0, 140.0, 90.0] {
        let count = PretextLayout::layout(&prepared, width, LINE_HEIGHT).line_count;
        assert!(count >= previous);
        previous = count;
    }
}

#[test]
fn trailing_whitespace_hangs_past_line_edge() {
    let prepared = prepare("Hello ");
    let width = prepared.widths[0];
    assert_eq!(
        1,
        PretextLayout::layout(&prepared, width, LINE_HEIGHT).line_count
    );
    let result = lines(&prepared, width);
    assert_eq!(1, result.line_count);
    assert_eq!(
        result.lines,
        [LayoutLine {
            text: "Hello".into(),
            width,
            start: cursor(0, 0),
            end: cursor(1, 0)
        }]
    );
}

#[test]
fn breaks_long_words_at_grapheme_boundaries_and_keeps_apis_aligned() {
    let prepared = prepare("Superlongword");
    let widths = prepared.breakable_widths[0].as_ref().unwrap();
    let width = widths[..3].iter().sum::<f64>() + 0.1;
    let plain = PretextLayout::layout(&prepared, width, LINE_HEIGHT);
    let rich = lines(&prepared, width);
    assert!(plain.line_count > 1);
    assert_eq!(plain.line_count, rich.line_count);
    assert_eq!(plain.height, rich.height);
    assert_eq!(texts(&rich).concat(), "Superlongword");
    assert_eq!(rich.lines[0].start, cursor(0, 0));
    assert_eq!(rich.lines.last().unwrap().end, cursor(1, 0));
}

#[test]
fn mixed_direction_text_is_stable_smoke_test() {
    let text = "According to محمد الأحمد, the results improved.";
    let result = lines(&prepare(text), 120.0);
    assert!(result.line_count >= 1);
    assert_eq!(result.height, result.line_count as f64 * LINE_HEIGHT);
    assert_eq!(texts(&result).concat(), text);
}

#[test]
fn layout_next_line_reproduces_layout_with_lines_exactly() {
    let (prepared, width) = soft_hyphen_case();
    assert_eq!(
        lines(&prepared, width).lines,
        stream(&prepared, width, None)
    );
}

#[test]
fn pre_wrap_keeps_hanging_spaces_visible_at_line_end() {
    let prepared = prepare_options("foo   bar", PRE_WRAP);
    let width = measure_width("foo", FONT) + 0.1;
    let result = lines(&prepared, width);
    assert_eq!(2, result.line_count);
    assert_eq!(texts(&result), ["foo   ", "bar"]);
    assert_eq!(
        2,
        PretextLayout::layout(&prepared, width, LINE_HEIGHT).line_count
    );
}

#[test]
fn pre_wrap_treats_hard_breaks_as_forced_line_boundaries() {
    let prepared = prepare_options("a\nb", PRE_WRAP);
    assert_eq!(texts(&lines(&prepared, 200.0)), ["a", "b"]);
    assert_eq!(
        2,
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT).line_count
    );
}

fn next_tab_advance(width: f64) -> f64 {
    let stop = measure_width(" ", FONT) * 8.0;
    let remainder = width % stop;
    if remainder.abs() <= 1e-12 {
        stop
    } else {
        stop - remainder
    }
}

#[test]
fn pre_wrap_treats_tabs_as_hanging_whitespace_aligned_to_tab_stops() {
    let prepared = prepare_options("a\tb", PRE_WRAP);
    let prefix_width = measure_width("a", FONT);
    let width = prefix_width + next_tab_advance(prefix_width) + measure_width("b", FONT) - 0.1;
    assert_eq!(texts(&lines(&prepared, width)), ["a\t", "b"]);
    assert_eq!(
        2,
        PretextLayout::layout(&prepared, width, LINE_HEIGHT).line_count
    );
}

#[test]
fn pre_wrap_treats_consecutive_tabs_as_distinct_tab_stops() {
    let prepared = prepare_options("a\t\tb", PRE_WRAP);
    let prefix_width = measure_width("a", FONT);
    let first_tab = next_tab_advance(prefix_width);
    let second_tab = next_tab_advance(prefix_width + first_tab);
    let width = prefix_width + first_tab + second_tab - 0.1;
    assert_eq!(texts(&lines(&prepared, width)), ["a\t\t", "b"]);
    assert_eq!(
        2,
        PretextLayout::layout(&prepared, width, LINE_HEIGHT).line_count
    );
}

#[test]
fn pre_wrap_keeps_whitespace_only_middle_lines_visible() {
    let prepared = prepare_options("foo\n  \nbar", PRE_WRAP);
    assert_eq!(texts(&lines(&prepared, 200.0)), ["foo", "  ", "bar"]);
    assert_eq!(
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 3,
            height: LINE_HEIGHT * 3.0
        }
    );
}

#[test]
fn pre_wrap_keeps_trailing_spaces_before_hard_break_on_current_line() {
    let prepared = prepare_options("foo  \nbar", PRE_WRAP);
    assert_eq!(texts(&lines(&prepared, 200.0)), ["foo  ", "bar"]);
    assert_eq!(
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 2,
            height: LINE_HEIGHT * 2.0
        }
    );
}

#[test]
fn pre_wrap_keeps_trailing_tabs_before_hard_break_on_current_line() {
    let prepared = prepare_options("foo\t\nbar", PRE_WRAP);
    assert_eq!(texts(&lines(&prepared, 200.0)), ["foo\t", "bar"]);
    assert_eq!(
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 2,
            height: LINE_HEIGHT * 2.0
        }
    );
}

#[test]
fn pre_wrap_restarts_tab_stops_after_hard_break() {
    let prepared = prepare_options("foo\n\tbar", PRE_WRAP);
    let result = lines(&prepared, 200.0);
    assert_eq!(texts(&result), ["foo", "\tbar"]);
    assert!(
        (result.lines[1].width - (next_tab_advance(0.0) + measure_width("bar", FONT))).abs()
            <= 1e-5
    );
}

#[test]
fn layout_next_line_stays_aligned_with_layout_with_lines_in_pre_wrap_mode() {
    let prepared = prepare_options("foo\n  bar baz\nquux", PRE_WRAP);
    let width = measure_width("  bar", FONT) + 0.1;
    assert_eq!(
        lines(&prepared, width).lines,
        stream(&prepared, width, None)
    );
}

#[test]
fn pre_wrap_keeps_empty_lines_from_consecutive_hard_breaks() {
    let prepared = prepare_options("\n\n", PRE_WRAP);
    assert_eq!(texts(&lines(&prepared, 200.0)), ["", ""]);
    assert_eq!(
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 2,
            height: LINE_HEIGHT * 2.0
        }
    );
}

#[test]
fn pre_wrap_does_not_invent_extra_trailing_empty_line() {
    let prepared = prepare_options("a\n", PRE_WRAP);
    assert_eq!(texts(&lines(&prepared, 200.0)), ["a"]);
    assert_eq!(
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 1,
            height: LINE_HEIGHT
        }
    );
}

#[test]
fn overlong_breakable_segments_wrap_onto_fresh_line_when_current_line_has_content() {
    let prepared = prepare("foo abcdefghijk");
    let word_breaks = prepared.breakable_widths[2].as_ref().unwrap();
    let width = prepared.widths[0] + prepared.widths[1] + word_breaks[0] + word_breaks[1] + 0.1;
    let batched = lines(&prepared, width);
    assert_eq!(batched.lines[0].text, "foo ");
    assert!(batched.lines[1].text.starts_with("ab"));
    assert_eq!(
        PretextLayout::layout_next_line(&prepared, cursor(0, 0), width)
            .unwrap()
            .text,
        "foo "
    );
    assert_eq!(
        batched.line_count,
        PretextLayout::layout(&prepared, width, LINE_HEIGHT).line_count
    );
}

#[test]
fn walk_line_ranges_reproduces_geometry_without_materializing_text() {
    let (prepared, width) = soft_hyphen_case();
    let expected = lines(&prepared, width);
    let mut actual = Vec::new();
    let count = PretextLayout::walk_line_ranges(&prepared, width, |line| actual.push(line));
    assert_eq!(count, expected.line_count);
    assert_eq!(
        actual,
        expected
            .lines
            .iter()
            .map(|line| LayoutLineRange {
                width: line.width,
                start: line.start,
                end: line.end
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn materialize_line_range_reproduces_layout_with_lines() {
    let (prepared, width) = soft_hyphen_case();
    let mut actual = Vec::new();
    PretextLayout::walk_line_ranges(&prepared, width, |line| {
        actual.push(PretextLayout::materialize_line_range(&prepared, &line))
    });
    assert_eq!(lines(&prepared, width).lines, actual);
}

#[test]
fn measure_line_stats_matches_walked_line_count_and_widest_line() {
    let (prepared, width) = soft_hyphen_case();
    let mut max_line_width = 0.0_f64;
    let line_count = PretextLayout::walk_line_ranges(&prepared, width, |line| {
        max_line_width = max_line_width.max(line.width)
    });
    assert_eq!(
        PretextLayout::measure_line_stats(&prepared, width),
        LineStats {
            line_count,
            max_line_width
        }
    );
}

#[test]
fn measure_natural_width_matches_widest_forced_line() {
    let prepared = prepare_options("alpha beta\n世界 gamma", PRE_WRAP);
    let mut widest = 0.0_f64;
    PretextLayout::walk_line_ranges(&prepared, f64::INFINITY, |line| {
        widest = widest.max(line.width)
    });
    assert!((PretextLayout::measure_natural_width(&prepared) - widest).abs() < 1e-6);
}

#[test]
fn layout_next_line_range_stays_aligned_with_range_geometry_walking() {
    let (prepared, _) = soft_hyphen_case();
    for width in [48.0, 72.0, 120.0] {
        let mut start = cursor(0, 0);
        let mut streamed = Vec::new();
        while let Some(line) = PretextLayout::layout_next_line_range(&prepared, start, width) {
            start = line.end;
            streamed.push(line);
        }
        let mut walked = Vec::new();
        PretextLayout::walk_line_ranges(&prepared, width, |line| walked.push(line));
        assert_eq!(walked, streamed);
    }
}

#[test]
fn mixed_script_canary_keeps_layout_apis_aligned() {
    let prepared = prepare("Hello 世界 مرحبا 🌍 test");
    let expected = lines(&prepared, 80.0);
    assert_eq!(texts(&expected), ["Hello 世", "界 مرحبا ", "🌍 test"]);
    assert_eq!(expected.lines, stream(&prepared, 80.0, None));
}

#[test]
fn zwsp_triggered_narrow_breaking_keeps_apis_aligned() {
    let engine = engine();
    for text in ["alpha\u{200b}beta", "alpha\u{200b}beta\u{200c}gamma"] {
        let plain = engine
            .prepare(text, FONT, PrepareOptions::default())
            .unwrap();
        let rich = engine
            .prepare_with_segments(text, FONT, PrepareOptions::default())
            .unwrap();
        assert_eq!(
            PretextLayout::layout(&plain, 10.0, LINE_HEIGHT).line_count,
            lines(&rich, 10.0).line_count
        );
    }
}

#[test]
fn layout_with_lines_strips_leading_collapsible_space_after_zwsp_break() {
    let prepared = prepare("生活就像海洋\u{200b} 只有意志坚定的人才能到达彼岸");
    let width = prepared.widths[0] - 1.0;
    assert_eq!(
        lines(&prepared, width).lines,
        stream(&prepared, width, None)
    );
}

#[test]
fn line_count_stays_aligned_after_zwsp_break_followed_by_collapsible_space() {
    let prepared = prepare("生活就像海洋\u{200b} 只有意志坚定的人才能到达彼岸");
    let width = prepared.widths[0] - 1.0;
    let count = lines(&prepared, width).line_count;
    assert_eq!(
        count,
        PretextLayout::layout(&prepared, width, LINE_HEIGHT).line_count
    );
    assert_eq!(count, count_prepared_lines(&prepared, width));
    assert_eq!(count, walk_prepared_lines(&prepared, width, |_| {}));
}

#[test]
fn adjacent_cjk_text_units_stay_breakable_after_visible_text() {
    let prepared = prepare("foo 世界 bar");
    assert_eq!(prepared.segments, ["foo", " ", "世", "界", " ", "bar"]);
    let width = prepared.widths[0] + prepared.widths[1] + prepared.widths[2] + 0.1;
    let batched = lines(&prepared, width);
    assert_eq!(texts(&batched), ["foo 世", "界 bar"]);
    assert_eq!(batched.lines, stream(&prepared, width, None));
    assert_eq!(
        PretextLayout::layout(&prepared, width, LINE_HEIGHT),
        LayoutResult {
            line_count: 2,
            height: LINE_HEIGHT * 2.0
        }
    );
}

#[test]
fn layout_next_line_can_resume_from_any_fixed_width_line_start() {
    let prepared =
        prepare("foo trans\u{ad}atlantic said \"hello\" to 世界 and waved. alpha\u{200b}beta 🚀");
    let expected = lines(&prepared, 90.0);
    assert!(expected.lines.len() > 2);
    for (index, line) in expected.lines.iter().enumerate() {
        assert_eq!(
            expected.lines[index..],
            stream(&prepared, 90.0, Some(line.start))
        );
    }
    assert!(PretextLayout::layout_next_line(&prepared, terminal(&prepared), 90.0).is_none());
}

#[test]
fn line_boundary_cursors_reconstruct_normalized_source_text() {
    for text in [
        "a b c",
        "  Hello\t \n  World  ",
        "foo trans\u{ad}atlantic said \"hello\" to 世界 and waved.",
        "According to محمد الأحمد, the results improved.",
        "see https://example.com/reports/q3?lang=ar&mode=full now",
        "alpha\u{200b}beta gamma",
    ] {
        let prepared = prepare(text);
        let expected = prepared.segments.concat();
        for width in [40.0, 80.0, 120.0, 200.0] {
            assert_eq!(
                expected,
                reconstruct(&prepared, &lines(&prepared, width).lines)
            );
            assert_eq!(
                expected,
                reconstruct(&prepared, &stream(&prepared, width, None))
            );
            let mut walked = String::new();
            PretextLayout::walk_line_ranges(&prepared, width, |line| {
                walked.push_str(&slice(&prepared, line.start, line.end))
            });
            assert_eq!(expected, walked);
        }
    }
}

#[test]
fn soft_hyphen_round_trip_uses_source_slices() {
    let prepared = prepare("foo trans\u{ad}atlantic");
    let width = prepared.widths[0]
        + prepared.widths[1]
        + prepared.widths[2]
        + prepared.breakable_widths[4].as_ref().unwrap()[0]
        + prepared.discretionary_hyphen_width
        + 0.1;
    let result = lines(&prepared, width);
    assert_eq!(texts(&result).concat(), "foo trans-atlantic");
    assert_eq!(
        reconstruct(&prepared, &result.lines),
        "foo trans\u{ad}atlantic"
    );
}

#[test]
fn soft_hyphen_fallback_does_not_crash_when_overflow_happens_later() {
    let prepared = prepare("foo trans\u{ad}atlantic labels");
    let width = measure_width("foo transatlantic", FONT) + 0.1;
    let result = lines(&prepared, width);
    assert_eq!(texts(&result), ["foo transatlantic ", "labels"]);
    assert_eq!(
        result.line_count,
        PretextLayout::layout(&prepared, width, LINE_HEIGHT).line_count
    );
}

fn assert_contiguous(prepared: &PreparedTextWithSegments, lines: &[LayoutLine]) {
    assert_eq!(lines[0].start, cursor(0, 0));
    for (index, line) in lines.iter().enumerate() {
        assert!(line.end > line.start);
        if index > 0 {
            assert_eq!(lines[index - 1].end, line.start);
        }
    }
    assert_eq!(lines.last().unwrap().end, terminal(prepared));
    assert_eq!(reconstruct(prepared, lines), prepared.segments.concat());
}

#[test]
fn variable_width_streaming_stays_contiguous_and_reconstructs_text() {
    let prepared = prepare("foo trans\u{ad}atlantic said \"hello\" to 世界 and waved. According to محمد الأحمد, alpha\u{200b}beta 🚀");
    let widths = [
        140.0, 72.0, 108.0, 64.0, 160.0, 84.0, 116.0, 70.0, 180.0, 92.0, 128.0, 76.0,
    ];
    let result = stream_widths(&prepared, &widths);
    assert!(result.len() > 2);
    assert_contiguous(&prepared, &result);
    assert!(PretextLayout::layout_next_line(
        &prepared,
        terminal(&prepared),
        *widths.last().unwrap()
    )
    .is_none());
}

#[test]
fn variable_width_streaming_stays_contiguous_in_pre_wrap_mode() {
    let prepared = prepare_options("foo\n  bar baz\n\tquux quuz", PRE_WRAP);
    let widths = [200.0, 62.0, 80.0, 200.0, 72.0, 200.0];
    let result = stream_widths(&prepared, &widths);
    assert!(result.len() >= 4);
    assert_contiguous(&prepared, &result);
    assert!(PretextLayout::layout_next_line(
        &prepared,
        terminal(&prepared),
        *widths.last().unwrap()
    )
    .is_none());
}

#[test]
fn mixed_cjk_plus_numeric_runs_use_cumulative_widths() {
    let prepared = prepare("中文11111111111111111");
    assert_eq!(prepared.segments, ["中", "文", "11111111111111111"]);
    let width = measure_width("11111", FONT) + 0.1;
    let batched = lines(&prepared, width);
    assert_eq!(texts(&batched), ["中文", "11111", "11111", "11111", "11"]);
    assert_eq!(batched.lines, stream(&prepared, width, None));
    assert_eq!(
        PretextLayout::layout(&prepared, width, LINE_HEIGHT),
        LayoutResult {
            line_count: 5,
            height: LINE_HEIGHT * 5.0
        }
    );
}

#[test]
fn keep_all_suppresses_ordinary_cjk_intra_word_breaks_after_existing_line_content() {
    let normal = prepare("A 中文测试");
    let keep_all = prepare_options(
        "A 中文测试",
        PrepareOptions {
            word_break: WordBreakMode::KeepAll,
            ..PrepareOptions::default()
        },
    );
    let width = measure_width("A 中", FONT) + 0.1;
    assert_eq!(lines(&normal, width).lines[0].text, "A 中");
    assert_eq!(lines(&keep_all, width).lines[0].text, "A ");
    assert!(
        PretextLayout::layout(&keep_all, width, LINE_HEIGHT).line_count
            > PretextLayout::layout(&normal, width, LINE_HEIGHT).line_count
    );
}

#[test]
fn keep_all_lets_mixed_no_space_cjk_runs_break_through_script_boundary() {
    let normal = prepare("日本語foo-bar");
    let keep_all = prepare_options(
        "日本語foo-bar",
        PrepareOptions {
            word_break: WordBreakMode::KeepAll,
            ..PrepareOptions::default()
        },
    );
    let width = measure_width("日本語f", FONT) + 0.1;
    assert_eq!(lines(&normal, width).lines[0].text, "日本語");
    assert_eq!(lines(&keep_all, width).lines[0].text, "日本語f");
}

#[test]
fn count_prepared_lines_stays_aligned_with_walked_line_counter() {
    for text in [
        "The quick brown fox jumps over the lazy dog.",
        "said \"hello\" to 世界 and waved.",
        "مرحبا، عالم؟",
        "author 7:00-9:00 only",
        "alpha\u{200b}beta gamma",
    ] {
        let prepared = prepare(text);
        for width in [40.0, 80.0, 120.0, 200.0] {
            assert_eq!(
                count_prepared_lines(&prepared, width),
                walk_prepared_lines(&prepared, width, |_| {})
            );
        }
    }
}

#[test]
fn invalid_cursors_do_not_panic_or_reenter_the_same_position() {
    let prepared = prepare("abc 世界");
    for start in [
        cursor(usize::MAX, 0),
        cursor(0, usize::MAX),
        cursor(2, 2),
        cursor(1, 1),
    ] {
        assert!(PretextLayout::layout_next_line(&prepared, start, 10.0).is_none());
    }
}

#[test]
fn zero_and_negative_widths_still_advance_complete_graphemes() {
    let prepared = prepare("e\u{301} 👩‍🚀 中");
    for width in [0.0, -1.0, f64::NEG_INFINITY] {
        let result = stream(&prepared, width, None);
        assert!(result.len() < 20);
        assert_eq!(lines(&prepared, width).lines, result);
        assert!(result.iter().any(|line| line.text.contains("e\u{301}")));
        assert!(result.iter().any(|line| line.text.contains("👩‍🚀")));
    }
}
