//! Characterization tests of the wrap algorithm. They assert invariants
//! (lengths, boundaries, content), not font specific values, so they run
//! unchanged on the test harness (em size 12: every glyph is 6 wide).

use std::collections::HashSet;
use std::rc::Rc;

use crate::media::text_formatting::testing::{
    format_line, format_lines, line_text, paragraph_properties, run_properties, SingleBufferTextSource, TextTestScope,
};
use crate::media::text_formatting::unicode::GraphemeEnumerator;
use crate::media::text_formatting::{ShapedTextRun, TextLine};
use crate::media::TextWrapping;

fn wrap_single_line(text: &str, paragraph_width: f64, wrapping: TextWrapping) -> Rc<dyn TextLine> {
    let default_properties = run_properties(12.0);
    let paragraph_properties = paragraph_properties(&default_properties, wrapping);
    let text_source = SingleBufferTextSource::new(text, default_properties);

    format_line(&text_source, 0, paragraph_width, &paragraph_properties, None).expect("a line")
}

fn wrap_all_lines(text: &str, paragraph_width: f64, wrapping: TextWrapping) -> Vec<Rc<dyn TextLine>> {
    let default_properties = run_properties(12.0);
    let paragraph_properties = paragraph_properties(&default_properties, wrapping);
    let text_source = SingleBufferTextSource::new(text, default_properties);

    let lines = format_lines(&text_source, text.encode_utf16().count() as i32, paragraph_width, &paragraph_properties);

    assert!(lines.len() <= 200, "Wrap appears to be looping.");

    lines
}

#[test]
fn wrap_with_infinite_width_yields_single_line_with_all_runs() {
    let _scope = TextTestScope::new();

    let line = wrap_single_line("Hello world", f64::INFINITY, TextWrapping::Wrap);

    assert_eq!(line.length(), "Hello world".len() as i32);
    assert!(line.width_including_trailing_whitespace() > 0.0);
}

#[test]
fn wrap_with_zero_width_forces_minimum_cluster() {
    // Width too small to fit any cluster — the implementation falls back to one grapheme.
    let _scope = TextTestScope::new();

    let line = wrap_single_line("Hello", 0.001, TextWrapping::Wrap);

    // Stricter than upstream (>= 1): exactly one cluster.
    assert_eq!(line.length(), 1);
}

#[test]
fn wrap_sum_of_line_lengths_equals_input_length() {
    let _scope = TextTestScope::new();

    for paragraph_width in [40.0, 80.0, 120.0] {
        let text = "AAAA BBBB CCCC DDDD";
        let lines = wrap_all_lines(text, paragraph_width, TextWrapping::Wrap);

        let total_length: i32 = lines.iter().map(|line| line.length()).sum();

        assert_eq!(total_length, text.len() as i32);
    }
}

#[test]
fn wrap_each_line_width_within_paragraph_width() {
    let _scope = TextTestScope::new();

    for paragraph_width in [40.0, 80.0] {
        let lines = wrap_all_lines("AAAA BBBB CCCC DDDD", paragraph_width, TextWrapping::Wrap);

        for line in &lines {
            assert!(line.width() <= paragraph_width, "line width {} exceeds {paragraph_width}", line.width());
        }
    }
}

/// Addition: the exact wrap positions for the fixed advance of the harness
/// (6 per glyph: 40 fits six glyphs, 80 fits thirteen).
#[test]
fn wrap_positions_follow_the_fixed_advance() {
    let _scope = TextTestScope::new();

    let lines = wrap_all_lines("AAAA BBBB CCCC DDDD", 40.0, TextWrapping::Wrap);

    assert_eq!(lines.iter().map(|line| line_text(&**line)).collect::<Vec<_>>(), ["AAAA ", "BBBB ", "CCCC ", "DDDD"]);
    assert_eq!(lines[0].width(), 24.0);
    assert_eq!(lines[0].width_including_trailing_whitespace(), 30.0);
    assert_eq!(lines[0].trailing_whitespace_length(), 1);

    let lines = wrap_all_lines("AAAA BBBB CCCC DDDD", 80.0, TextWrapping::Wrap);

    assert_eq!(lines.iter().map(|line| line_text(&**line)).collect::<Vec<_>>(), ["AAAA BBBB ", "CCCC DDDD"]);
}

#[test]
fn wrap_does_not_produce_empty_lines_for_non_empty_input() {
    let _scope = TextTestScope::new();

    let lines = wrap_all_lines("the quick brown fox jumps over the lazy dog", 50.0, TextWrapping::Wrap);

    for line in &lines {
        assert!(line.length() > 0, "Wrap should never emit a zero-length line for non-empty input.");
    }
}

#[test]
fn wrap_points_are_grapheme_boundaries() {
    // Multi-unit graphemes (emoji) must never be split by the wrap algorithm.
    // The emoji come from the fallback font of the harness (12 wide each).
    let _scope = TextTestScope::new();

    let text = "abc \u{1F600}\u{1F600}\u{1F600}\u{1F600} xyz";
    let units: Vec<u16> = text.encode_utf16().collect();

    let lines = wrap_all_lines(text, 30.0, TextWrapping::Wrap);

    let mut boundaries = HashSet::new();
    boundaries.insert(0);

    let mut pos = 0;

    for grapheme in GraphemeEnumerator::new(&units) {
        pos += grapheme.length() as i32;
        boundaries.insert(pos);
    }

    let mut cumulative = 0;

    for line in &lines {
        cumulative += line.length();
        assert!(boundaries.contains(&cumulative), "{cumulative} is not a grapheme boundary");
    }

    assert_eq!(cumulative, units.len() as i32);
}

#[test]
fn wrap_honours_required_break_even_with_available_width() {
    let _scope = TextTestScope::new();

    let line = wrap_single_line("ab\ncd", f64::INFINITY, TextWrapping::Wrap);

    // Hard break sits at index 2 (the '\n'); the wrap position is 3 (consumes the '\n').
    assert_eq!(line.length(), 3);
    assert_eq!(line.new_line_length(), 1);
}

#[test]
fn wrap_hard_break_with_crlf_counts_both_characters() {
    let _scope = TextTestScope::new();

    let line = wrap_single_line("ab\r\ncd", f64::INFINITY, TextWrapping::Wrap);

    // CRLF is a single break with a wrap position of 4 (consumes both chars).
    assert_eq!(line.length(), 4);
    assert_eq!(line.new_line_length(), 2);
}

#[test]
fn wrap_with_overflow_long_word_followed_by_space_wraps_after_space() {
    // The word has no break inside it. At a small paragraph width with
    // WrapWithOverflow, the wrap algorithm lets the word overflow as a whole,
    // then wraps on the next break (after the trailing space).
    let _scope = TextTestScope::new();

    let line = wrap_single_line("supercalifragilistic next", 30.0, TextWrapping::WrapWithOverflow);

    assert_eq!(line.length(), "supercalifragilistic ".len() as i32);
    assert!(line.has_overflowed());
}

#[test]
fn wrap_strict_long_word_splits_inside_when_no_wrap_position_available() {
    // Pure Wrap on an unbreakable word: the implementation falls back to
    // splitting inside the word at the best available cluster boundary.
    let _scope = TextTestScope::new();

    let line = wrap_single_line("supercalifragilistic", 30.0, TextWrapping::Wrap);

    // 30 / 6 = five glyphs.
    assert_eq!(line.length(), 5);
}

#[test]
fn wrap_continues_from_previous_line_break() {
    let _scope = TextTestScope::new();

    let text = "AAAA BBBB CCCC DDDD";
    let lines = wrap_all_lines(text, 40.0, TextWrapping::Wrap);

    assert!(lines.len() >= 2, "Test setup must wrap onto at least two lines.");

    // Lines after the first reuse runs via the wrapping line break.
    // The contract: concatenated, they reproduce the original.
    let rebuilt: String = lines.iter().map(|line| line_text(&**line)).collect();

    assert_eq!(rebuilt, text);
}

#[test]
fn wrap_with_ltr_text_does_not_touch_trailing_whitespace_bidi() {
    // Resetting the trailing whitespace bidi levels is a no-op when the run's
    // bidi level already matches the paragraph.
    let _scope = TextTestScope::new();

    let text = "Hello world from the formatter";
    let wrapped_lines = wrap_all_lines(text, 80.0, TextWrapping::Wrap);

    let rebuilt: String = wrapped_lines.iter().map(|line| line_text(&**line)).collect();

    assert_eq!(rebuilt, text);

    for line in &wrapped_lines {
        for run in line.text_runs().iter() {
            assert_eq!(run.downcast_ref::<ShapedTextRun>().unwrap().bidi_level(), 0);
        }
    }
}

#[test]
fn wrap_empty_text_yields_null() {
    let _scope = TextTestScope::new();

    let default_properties = run_properties(12.0);
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);
    let text_source = SingleBufferTextSource::new("", default_properties);

    assert!(format_line(&text_source, 0, 100.0, &paragraph_properties, None).is_none());
}
