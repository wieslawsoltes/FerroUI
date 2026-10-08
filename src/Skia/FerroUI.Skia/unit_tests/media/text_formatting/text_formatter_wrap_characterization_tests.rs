//! Port of upstream's `Media/TextFormatting/TextFormatterWrapCharacterizationTests.cs`
//! of the Skia unit tests: characterization tests for
//! `TextFormatterImpl::perform_text_wrapping` and its helpers
//! (`measure_length`, `split_text_runs`, `reset_trailing_whitespace_bidi_levels`).
//!
//! The text of `wrap_with_ltr_text_does_not_touch_trailing_whitespace_bidi`
//! ends with the port's name instead of upstream's (the name of the upstream
//! project must not appear in the sources); the test only compares the text
//! with itself.

use super::text_formatter_tests::start;
use crate::unit_tests::media::text_formatting::SingleBufferTextSource;
use ferroui_base::media::text_formatting::unicode::GraphemeEnumerator;
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, TextFormatter, TextFormatterImpl, TextLine,
    TextLineBreak, TextParagraphProperties,
};
use ferroui_base::media::{BaselineAlignment, Brushes, TextAlignment, TextWrapping, Typeface};
use std::collections::HashSet;
use std::rc::Rc;

#[test]
fn wrap_with_infinite_width_yields_single_line_with_all_runs() {
    let _scope = start();

    let line = wrap_single_line("Hello world", f64::INFINITY, TextWrapping::Wrap);

    assert_eq!(utf16_len("Hello world"), line.length());
    assert!(line.width_including_trailing_whitespace() > 0.0);
}

#[test]
fn wrap_with_zero_width_forces_minimum_cluster() {
    // Width too small to fit any cluster — the implementation falls
    // back to one grapheme. This is the documented WrapWithOverflow
    // contract that lines 882-902 of TextFormatterImpl encode.
    let _scope = start();

    let line = wrap_single_line("Hello", 0.001, TextWrapping::Wrap);

    assert!(line.length() >= 1, "Wrap should always advance at least one grapheme even at zero width.");
}

#[test]
fn wrap_sum_of_line_lengths_equals_input_length() {
    for (text, paragraph_width) in
        [("AAAA BBBB CCCC DDDD", 40.0), ("AAAA BBBB CCCC DDDD", 80.0), ("AAAA BBBB CCCC DDDD", 120.0)]
    {
        let _scope = start();

        let lines = wrap_all_lines(text, paragraph_width, TextWrapping::Wrap);
        let total_length: i32 = lines.iter().map(|l| l.length()).sum();
        assert_eq!(utf16_len(text), total_length, "{text:?}, {paragraph_width}");
    }
}

#[test]
fn wrap_each_line_width_within_paragraph_width() {
    for (text, paragraph_width) in [("AAAA BBBB CCCC DDDD", 40.0), ("AAAA BBBB CCCC DDDD", 80.0)] {
        let _scope = start();

        let lines = wrap_all_lines(text, paragraph_width, TextWrapping::Wrap);
        for line in &lines {
            // Width (excluding trailing whitespace) should fit the
            // paragraph. The +1.0 tolerance handles the documented
            // "single cluster wider than paragraph" overflow case.
            assert!(
                line.width() <= paragraph_width + 1.0,
                "Line width {} exceeds paragraph width {} by more than 1px.",
                line.width(),
                paragraph_width
            );
        }
    }
}

#[test]
fn wrap_does_not_produce_empty_lines_for_non_empty_input() {
    let _scope = start();

    let lines = wrap_all_lines("the quick brown fox jumps over the lazy dog", 50.0, TextWrapping::Wrap);
    for line in &lines {
        assert!(line.length() > 0, "Wrap should never emit a zero-length line for non-empty input.");
    }
}

#[test]
fn wrap_points_are_grapheme_boundaries() {
    // Multi-codepoint graphemes (emoji ZWJ sequences) must never be
    // split by the wrap algorithm — the wrap point has to coincide
    // with a grapheme boundary.
    let _scope = start();

    let text = "abc 😀😀😀😀 xyz";
    let lines = wrap_all_lines(text, 30.0, TextWrapping::Wrap);

    let units: Vec<u16> = text.encode_utf16().collect();

    let mut boundaries = HashSet::new();
    let mut grapheme_enumerator = GraphemeEnumerator::new(&units);
    boundaries.insert(0);
    let mut pos = 0;
    while let Some(grapheme) = grapheme_enumerator.move_next() {
        pos += grapheme.length() as i32;
        boundaries.insert(pos);
    }

    let mut cumulative = 0;
    for line in &lines {
        cumulative += line.length();
        assert!(boundaries.contains(&cumulative), "{cumulative} is not a grapheme boundary");
    }
}

#[test]
fn wrap_honours_required_break_even_with_available_width() {
    let _scope = start();

    let line = wrap_single_line("ab\ncd", f64::INFINITY, TextWrapping::Wrap);

    // Hard break sits at index 2 (the '\n'); PositionWrap is 3
    // (consumes the '\n').
    assert_eq!(3, line.length());
}

#[test]
fn wrap_hard_break_with_crlf_counts_both_characters() {
    let _scope = start();

    let line = wrap_single_line("ab\r\ncd", f64::INFINITY, TextWrapping::Wrap);

    // CRLF is a single break with PositionWrap = 4 (consumes both chars).
    assert_eq!(4, line.length());
}

#[test]
fn wrap_with_overflow_long_word_followed_by_space_wraps_after_space() {
    // The word "supercalifragilistic" has no break inside it. At a
    // small paragraph width with WrapWithOverflow, the wrap algorithm
    // should let the word overflow as a whole, then wrap on the next
    // break (the trailing space).
    let _scope = start();

    let line = wrap_single_line("supercalifragilistic next", 30.0, TextWrapping::WrapWithOverflow);

    assert!(
        line.length() >= utf16_len("supercalifragilistic"),
        "Expected first line to contain at least the whole long word; got length {}.",
        line.length()
    );
    assert!(
        line.length() <= utf16_len("supercalifragilistic "),
        "First line should not extend past the trailing space after the long word."
    );
}

#[test]
fn wrap_strict_long_word_splits_inside_when_no_wrap_position_available() {
    // Pure Wrap (not WrapWithOverflow) on an unbreakable word: the
    // implementation falls back to splitting inside the word at the
    // best available cluster boundary.
    let _scope = start();

    let line = wrap_single_line("supercalifragilistic", 30.0, TextWrapping::Wrap);

    assert!(line.length() > 0);
    assert!(line.length() < utf16_len("supercalifragilistic"), "Strict wrap should split inside the long word.");
}

#[test]
fn wrap_continues_from_previous_line_break() {
    let _scope = start();

    let text = "AAAA BBBB CCCC DDDD";
    let lines = wrap_all_lines(text, 40.0, TextWrapping::Wrap);

    assert!(lines.len() >= 2, "Test setup must wrap onto at least two lines.");

    // Lines after the first reuse runs via WrappingTextLineBreak.
    // The contract: concatenated, they reproduce the original.
    let rebuilt: String = lines.iter().map(|l| get_line_text(&**l)).collect();
    assert_eq!(text, rebuilt);
}

#[test]
fn wrap_with_ltr_text_does_not_touch_trailing_whitespace_bidi() {
    // ResetTrailingWhitespaceBidiLevels is a no-op when the run's
    // BidiLevel already matches the paragraph. The wrap result should
    // be identical to a non-wrapped layout of the same paragraph.
    let _scope = start();

    // Deviation: upstream's text ends with the name of the upstream project.
    let text = "Hello world from FerroUI";
    let wrapped_lines = wrap_all_lines(text, 80.0, TextWrapping::Wrap);
    let rebuilt: String = wrapped_lines.iter().map(|l| get_line_text(&**l)).collect();
    assert_eq!(text, rebuilt);
}

#[test]
fn wrap_empty_text_yields_null() {
    let _scope = start();

    let default_properties = default_properties();
    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties.clone(),
        TextAlignment::Left,
        TextWrapping::Wrap,
        0.0,
        0.0,
    ));
    let text_source = SingleBufferTextSource::new("", default_properties, false);
    let formatter = TextFormatterImpl::new();

    let line = formatter.format_line(&text_source, 0, 100.0, &paragraph_properties, None);
    assert!(line.is_none());
}

/// Upstream's `new GenericTextRunProperties(Typeface.Default, 12, foregroundBrush: Brushes.Black)`.
fn default_properties() -> Rc<GenericTextRunProperties> {
    Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        12.0,
        None,
        Some(Brushes::black()),
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ))
}

fn wrap_single_line(text: &str, paragraph_width: f64, wrapping: TextWrapping) -> Rc<dyn TextLine> {
    let default_properties = default_properties();
    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties.clone(),
        TextAlignment::Left,
        wrapping,
        0.0,
        0.0,
    ));
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let line = formatter.format_line(&text_source, 0, paragraph_width, &paragraph_properties, None);
    assert!(line.is_some());
    line.unwrap()
}

fn wrap_all_lines(text: &str, paragraph_width: f64, wrapping: TextWrapping) -> Vec<Rc<dyn TextLine>> {
    let default_properties = default_properties();
    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties.clone(),
        TextAlignment::Left,
        wrapping,
        0.0,
        0.0,
    ));
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let text_length = utf16_len(text);

    let mut lines: Vec<Rc<dyn TextLine>> = Vec::new();
    let mut pos = 0;
    let mut previous_line_break: Option<Rc<TextLineBreak>> = None;
    while pos < text_length {
        let Some(line) =
            formatter.format_line(&text_source, pos, paragraph_width, &paragraph_properties, previous_line_break.as_ref())
        else {
            break;
        };
        previous_line_break = line.text_line_break();
        pos += line.length();
        lines.push(line);

        if pos > 0 && lines.len() > 200 {
            panic!("Wrap appears to be looping; bailing out.");
        }
    }
    lines
}

fn get_line_text(line: &dyn TextLine) -> String {
    let mut sb: Vec<u16> = Vec::new();
    for run in line.text_runs().iter() {
        sb.extend_from_slice(run.text_span());
    }
    String::from_utf16_lossy(&sb)
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}
