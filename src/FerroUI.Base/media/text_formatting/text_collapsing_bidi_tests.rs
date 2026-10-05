//! Port of upstream's collapsing tests for left to right, right to left and
//! mixed lines. The assertions are about logical content (what survives the
//! collapse, in which order), so they run unchanged on the test harness; exact
//! strings are added where the fixed advance (6 per glyph) determines them.

use std::rc::Rc;

use crate::media::text_formatting::logical_text_run_enumerator::LogicalTextRunEnumerator;
use crate::media::text_formatting::testing::{
    format_line, paragraph_properties_with, run_properties, SingleBufferTextSource, TextTestScope,
};
use crate::media::text_formatting::{
    DrawableTextRun, ITextSource, ShapedTextRun, TextCharacters, TextCollapsingProperties, TextLeadingPrefixCharacterEllipsis,
    TextLine, TextLineImpl, TextRun, TextRunProperties, TextTrailingCharacterEllipsis, TextTrailingWordEllipsis,
};
use crate::media::{FlowDirection, TextAlignment, TextPathSegmentEllipsis, TextWrapping};

const ELLIPSIS: &str = "\u{2026}";
const ARABIC: &str = "\u{0627}\u{0644}\u{0633}\u{0644}\u{0627}\u{0645} \u{0639}\u{0644}\u{064A}\u{0643}\u{0645} \u{0648}\u{0631}\u{062D}\u{0645}\u{0629} \u{0627}\u{0644}\u{0644}\u{0647} \u{0648}\u{0628}\u{0631}\u{0643}\u{0627}\u{062A}\u{0647}";
const MIXED: &str = "Hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627} world";

fn properties() -> Rc<dyn TextRunProperties> {
    run_properties(12.0)
}

fn build_line(text: &str, flow: FlowDirection) -> Rc<dyn TextLine> {
    let props = properties();
    let paragraph_props = paragraph_properties_with(&props, TextWrapping::NoWrap, TextAlignment::Left, flow);
    let source = SingleBufferTextSource::new(text, props);

    format_line(&source, 0, f64::INFINITY, &paragraph_props, None).unwrap()
}

fn trailing_char(width: f64, flow: FlowDirection) -> Rc<dyn TextCollapsingProperties> {
    Rc::new(TextTrailingCharacterEllipsis::new(ELLIPSIS, width, properties(), flow))
}

fn trailing_word(width: f64, flow: FlowDirection) -> Rc<dyn TextCollapsingProperties> {
    Rc::new(TextTrailingWordEllipsis::new(ELLIPSIS, width, properties(), flow))
}

fn leading_prefix(prefix_length: i32, width: f64, flow: FlowDirection) -> Rc<dyn TextCollapsingProperties> {
    Rc::new(TextLeadingPrefixCharacterEllipsis::new(ELLIPSIS, prefix_length, width, properties(), flow))
}

fn path_segment(width: f64, flow: FlowDirection) -> Rc<dyn TextCollapsingProperties> {
    Rc::new(TextPathSegmentEllipsis::new(ELLIPSIS, width, properties(), flow))
}

fn collapse(line: &Rc<dyn TextLine>, collapsing: Rc<dyn TextCollapsingProperties>) -> Rc<dyn TextLine> {
    line.clone().collapse(&[Some(collapsing)])
}

fn assert_collapsed(collapsed: &Rc<dyn TextLine>, original: &Rc<dyn TextLine>) {
    assert!(!Rc::ptr_eq(collapsed, original));
    assert!(collapsed.has_collapsed(), "Collapsed line must report has_collapsed = true.");
}

fn logical_text(line: &dyn TextLine) -> String {
    let mut enumerator = LogicalTextRunEnumerator::new(line);
    let mut units: Vec<u16> = Vec::new();

    while let Some(run) = enumerator.move_next() {
        units.extend_from_slice(run.text_span());
    }

    String::from_utf16_lossy(&units)
}

fn take_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

/// A text source that returns the run that starts at the requested index.
struct FixedRunsTextSource {
    text_runs: Vec<Rc<dyn TextRun>>,
}

impl ITextSource for FixedRunsTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let mut pos = 0;

        for run in &self.text_runs {
            if pos == text_source_index {
                return Some(run.clone());
            }

            pos += run.length();
        }

        None
    }
}

fn build_line_of_runs(texts: &[&str]) -> Rc<dyn TextLine> {
    let props = properties();

    let source = FixedRunsTextSource {
        text_runs: texts
            .iter()
            .map(|text| Rc::new(TextCharacters::from_str(text, props.clone())) as Rc<dyn TextRun>)
            .collect(),
    };

    let paragraph_props =
        paragraph_properties_with(&props, TextWrapping::NoWrap, TextAlignment::Left, FlowDirection::LeftToRight);

    format_line(&source, 0, f64::INFINITY, &paragraph_props, None).unwrap()
}

#[test]
fn ltr_trailing_character_trims_from_end() {
    let _scope = TextTestScope::new();

    let line = build_line("Hello world", FlowDirection::LeftToRight);
    let collapsed = collapse(&line, trailing_char(line.width() / 2.0, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    let text = logical_text(&*collapsed);

    assert!(text.contains(ELLIPSIS));
    assert!(text.starts_with('H'));

    // 33 leaves 27 next to the ellipsis: four glyphs.
    assert_eq!(text, "Hell\u{2026}");
}

#[test]
fn ltr_trailing_word_trims_on_word_boundary() {
    let _scope = TextTestScope::new();

    let line = build_line("Hello world foo", FlowDirection::LeftToRight);
    let collapsed = collapse(&line, trailing_word(line.width() / 2.0, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    // 45 leaves 39 next to the ellipsis: six glyphs, of which the first word is kept.
    assert_eq!(logical_text(&*collapsed), "Hello\u{2026}");
}

#[test]
fn ltr_prefix_character_ellipsis_preserves_prefix_and_suffix() {
    let _scope = TextTestScope::new();

    let line = build_line("01234 01234 01234", FlowDirection::LeftToRight);

    // Upstream collapses at 120 with its font; the line of the harness is 102 wide, so 96 is used.
    let collapsed = collapse(&line, leading_prefix(8, 96.0, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    let text = logical_text(&*collapsed);

    assert!(text.starts_with("01234 01"));
    assert!(text.contains(ELLIPSIS));
    // Suffix must reappear after the symbol.
    assert!(text.ends_with("4 01234"));
    assert_eq!(collapsed.width_including_trailing_whitespace(), 96.0);
}

#[test]
fn ltr_path_segment_ellipsis_collapses_middle() {
    let _scope = TextTestScope::new();

    let line = build_line("verylongdirectory\\file.txt", FlowDirection::LeftToRight);
    let collapsed = collapse(&line, path_segment(line.width() / 2.0, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    let text = logical_text(&*collapsed);

    assert!(text.contains(ELLIPSIS));
    // Last segment ("file.txt") should be preserved on at least some prefix.
    assert!(text.contains(".txt"));

    // Half of the 26 glyphs: the ellipsis and the last twelve characters.
    assert_eq!(text, "\u{2026}ory\\file.txt");
}

#[test]
fn width_greater_than_line_returns_same_line() {
    let _scope = TextTestScope::new();

    let line = build_line("abc", FlowDirection::LeftToRight);
    let collapsed = collapse(&line, trailing_char(line.width() + 100.0, FlowDirection::LeftToRight));

    // No collapsed runs → the line returns itself.
    assert!(Rc::ptr_eq(&line, &collapsed));
    assert!(!collapsed.has_collapsed());
}

#[test]
fn width_less_than_symbol_returns_empty_collapsed_line() {
    let _scope = TextTestScope::new();

    let line = build_line("abcdef", FlowDirection::LeftToRight);

    // Width below symbol width → no runs → the line is collapsed but empty.
    let collapsed = collapse(&line, trailing_char(0.001, FlowDirection::LeftToRight));

    assert!(collapsed.has_collapsed());
    assert!(collapsed.text_runs().is_empty());
}

#[test]
fn rtl_trailing_character_preserves_logical_prefix() {
    let _scope = TextTestScope::new();

    let line = build_line(ARABIC, FlowDirection::RightToLeft);
    let collapsed = collapse(&line, trailing_char(line.width() / 2.0, FlowDirection::RightToLeft));

    assert_collapsed(&collapsed, &line);

    let logical = logical_text(&*collapsed);

    assert!(logical.contains(ELLIPSIS));
    assert!(logical.starts_with(&take_chars(ARABIC, 1)));

    // 31 glyphs: half is 93, which leaves 87 next to the ellipsis: fourteen glyphs.
    assert_eq!(logical, format!("{}{ELLIPSIS}", take_chars(ARABIC, 14)));

    // The ellipsis is the leftmost run of the right to left line.
    assert_eq!(collapsed.text_runs()[0].text().to_string_lossy(), ELLIPSIS);
}

#[test]
fn rtl_trailing_word_preserves_logical_prefix() {
    let _scope = TextTestScope::new();

    let line = build_line(ARABIC, FlowDirection::RightToLeft);
    let collapsed = collapse(&line, trailing_word(line.width() / 2.0, FlowDirection::RightToLeft));

    assert_collapsed(&collapsed, &line);

    // Fourteen glyphs fit; the last word boundary before them is after the second word.
    assert_eq!(logical_text(&*collapsed), format!("{}{ELLIPSIS}", take_chars(ARABIC, 12)));
}

#[test]
fn rtl_prefix_character_ellipsis_preserves_logical_prefix() {
    let _scope = TextTestScope::new();

    let line = build_line(ARABIC, FlowDirection::RightToLeft);
    let collapsed = collapse(&line, leading_prefix(4, line.width() / 2.0, FlowDirection::RightToLeft));

    assert_collapsed(&collapsed, &line);

    let logical = logical_text(&*collapsed);

    assert!(logical.starts_with(&take_chars(ARABIC, 4)));
    assert!(logical.contains(ELLIPSIS));

    // Addition: the logical tail of the line follows the ellipsis.
    assert!(logical.ends_with(&ARABIC.chars().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect::<String>()));
}

#[test]
fn mixed_trailing_character_preserves_logical_prefix() {
    let _scope = TextTestScope::new();

    let line = build_line(MIXED, FlowDirection::LeftToRight);
    let collapsed = collapse(&line, trailing_char(line.width() * 0.6, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);
    assert!(logical_text(&*collapsed).starts_with("Hello"));
}

#[test]
fn mixed_prefix_character_ellipsis_preserves_logical_prefix_and_suffix() {
    let _scope = TextTestScope::new();

    let line = build_line(MIXED, FlowDirection::LeftToRight);
    let collapsed = collapse(&line, leading_prefix(5, line.width() * 0.6, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    let logical = logical_text(&*collapsed);

    assert!(logical.starts_with("Hello"));
    assert!(logical.contains(ELLIPSIS));
}

#[test]
fn mixed_trailing_word_preserves_logical_prefix() {
    let _scope = TextTestScope::new();

    let line = build_line(MIXED, FlowDirection::LeftToRight);
    let collapsed = collapse(&line, trailing_word(line.width() * 0.6, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    let logical = logical_text(&*collapsed);

    assert!(logical.contains(ELLIPSIS));
    assert!(logical.starts_with("Hello"));
}

#[test]
fn mixed_path_segment_ellipsis_preserves_last_segment() {
    let _scope = TextTestScope::new();

    // Mixed-bidi path: ASCII-only separators with an RTL directory
    // name embedded. Segmentation is separator-driven, so the
    // logical-tail segment ("file.txt") must survive.
    let line = build_line("C:\\folder\\\u{0645}\u{062C}\u{0644}\u{062F}\\file.txt", FlowDirection::LeftToRight);
    let collapsed = collapse(&line, path_segment(line.width() / 2.0, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    let logical = logical_text(&*collapsed);

    assert!(logical.contains(ELLIPSIS));
    assert!(logical.contains("file.txt"));
}

#[test]
fn rtl_path_segment_ellipsis_preserves_last_segment() {
    let _scope = TextTestScope::new();

    // Pure-RTL path. Segmentation is character-driven (separators are ASCII '/' and '\\') so the
    // logical-tail segment must still be detected and preserved.
    let last_segment = "\u{0645}\u{0644}\u{0641}.txt";
    let text = format!("\u{0645}\u{062C}\u{0644}\u{062F}/\u{0645}\u{062C}\u{0644}\u{062F}2/{last_segment}");

    let line = build_line(&text, FlowDirection::RightToLeft);
    let collapsed = collapse(&line, path_segment(line.width() / 2.0, FlowDirection::RightToLeft));

    assert_collapsed(&collapsed, &line);

    let logical = logical_text(&*collapsed);

    assert!(logical.contains(ELLIPSIS));
    assert!(logical.contains(last_segment));
}

#[test]
fn ltr_path_segment_ellipsis_middle_collapse_preserves_first_and_last_segments() {
    let _scope = TextTestScope::new();

    // 3-segment path; middle is intentionally long so collapsing it alone produces a fitting result.
    let line = build_line(
        "a/middlemiddlemiddlemiddlemiddlemiddlemiddlemiddlemiddlemiddle/c.txt",
        FlowDirection::LeftToRight,
    );

    let collapsed = collapse(&line, path_segment(line.width() * 0.3, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    assert_eq!(logical_text(&*collapsed), "a/\u{2026}/c.txt");
}

#[test]
fn rtl_path_segment_ellipsis_middle_collapse_preserves_first_and_last_segments() {
    let _scope = TextTestScope::new();

    let first = "\u{0627}\u{0648}\u{0644}";
    let middle = "\u{0645}\u{0646}\u{062A}\u{0635}\u{0641}".repeat(8);
    let last = "\u{0627}\u{062E}\u{0631}.txt";

    let line = build_line(&format!("{first}/{middle}/{last}"), FlowDirection::RightToLeft);
    let collapsed = collapse(&line, path_segment(line.width() * 0.3, FlowDirection::RightToLeft));

    assert_collapsed(&collapsed, &line);

    assert_eq!(logical_text(&*collapsed), format!("{first}/{ELLIPSIS}/{last}"));
}

#[test]
fn try_measure_characters_returned_length_fits_logical_leading_in_budget() {
    let _scope = TextTestScope::new();

    for (text, flow) in
        [("Hello world abcdef", FlowDirection::LeftToRight), (&ARABIC[..], FlowDirection::RightToLeft)]
    {
        let line = build_line(text, flow);
        let runs = line.text_runs();

        let shaped_run = runs.iter().find_map(|run| run.downcast_ref::<ShapedTextRun>()).unwrap();

        let buffer = shaped_run.shaped_buffer();
        let total_width = shaped_run.size().width;

        // Probe at several budget points; the contract must hold at all of them.
        for i in 1..10 {
            let budget = total_width * i as f64 / 10.0;

            let Some(measured) = shaped_run.try_measure_characters(budget) else {
                continue;
            };

            // The width of the LOGICAL leading `measured` characters must fit in `budget`.
            let actual_leading_width = buffer.get_char_range_width(0, measured);

            assert!(actual_leading_width <= budget + 1e-9, "{flow:?}: budget={budget}, measured={measured}");

            // Addition: with the fixed advance the measured length is exact.
            assert_eq!(measured, (budget / 6.0 + 1e-9).floor() as i32);
        }
    }
}

#[test]
fn try_measure_characters_backwards_returned_length_fits_logical_trailing_in_budget() {
    let _scope = TextTestScope::new();

    for (text, flow) in
        [("Hello world abcdef", FlowDirection::LeftToRight), (&ARABIC[..], FlowDirection::RightToLeft)]
    {
        let line = build_line(text, flow);
        let runs = line.text_runs();

        let shaped_run = runs.iter().find_map(|run| run.downcast_ref::<ShapedTextRun>()).unwrap();

        let buffer = shaped_run.shaped_buffer();
        let total_width = shaped_run.size().width;
        let text_length = shaped_run.length();

        for i in 1..10 {
            let budget = total_width * i as f64 / 10.0;

            let Some((measured, width)) = shaped_run.try_measure_characters_backwards(budget) else {
                continue;
            };

            let actual_trailing_width = buffer.get_char_range_width(text_length - measured, text_length);

            assert!(actual_trailing_width <= budget + 1e-9, "{flow:?}: budget={budget}, measured={measured}");
            assert_eq!(actual_trailing_width, width);
        }
    }
}

#[test]
fn logical_text_run_enumerator_without_indexed_runs_returns_distinct_runs() {
    let _scope = TextTestScope::new();

    let props = properties();

    let runs: Vec<Rc<dyn TextRun>> = ["AAA", "BBB", "CCC"]
        .iter()
        .map(|text| Rc::new(TextCharacters::from_str(text, props.clone())) as Rc<dyn TextRun>)
        .collect();

    // Construct the line directly and SKIP finalizing it so that the indexed runs stay unset.
    let paragraph_props =
        paragraph_properties_with(&props, TextWrapping::NoWrap, TextAlignment::Left, FlowDirection::LeftToRight);

    let line =
        TextLineImpl::new(runs.clone(), 0, 9, f64::INFINITY, paragraph_props, FlowDirection::LeftToRight, None, false);

    let mut enumerator = LogicalTextRunEnumerator::new(&*line);

    assert_eq!(enumerator.count(), 3);

    let mut seen: Vec<Rc<dyn TextRun>> = Vec::new();

    while let Some(run) = enumerator.move_next() {
        seen.push(run);
    }

    assert_eq!(seen.len(), 3);

    for (run, seen) in runs.iter().zip(&seen) {
        assert!(std::ptr::addr_eq(Rc::as_ptr(run), Rc::as_ptr(seen)));
    }

    // Addition: backwards.
    let mut enumerator = LogicalTextRunEnumerator::with_direction(&*line, true);

    let mut texts = Vec::new();

    while let Some(run) = enumerator.move_next() {
        texts.push(run.text().to_string_lossy());
    }

    assert_eq!(texts, ["CCC", "BBB", "AAA"]);
}

/// Addition: for a finalized line the enumerator gives the runs in logical
/// order, whatever their visual order is.
#[test]
fn logical_text_run_enumerator_returns_logical_order_for_bidi_lines() {
    let _scope = TextTestScope::new();

    let line = build_line(MIXED, FlowDirection::RightToLeft);

    let visual: String = line.text_runs().iter().map(|run| run.text().to_string_lossy()).collect();

    assert_ne!(visual, MIXED);
    assert_eq!(logical_text(&*line), MIXED);
}

#[test]
#[should_panic(expected = "prefixLength")]
fn leading_prefix_negative_prefix_length_throws() {
    let _scope = TextTestScope::new();

    TextLeadingPrefixCharacterEllipsis::new(ELLIPSIS, -1, 100.0, properties(), FlowDirection::LeftToRight);
}

#[test]
fn leading_prefix_honours_flow_direction_for_symbol() {
    let _scope = TextTestScope::new();

    let text = take_chars(ARABIC, 18);

    let line = build_line(&text, FlowDirection::RightToLeft);
    let collapsed = collapse(&line, leading_prefix(4, line.width() / 2.0, FlowDirection::RightToLeft));

    assert_collapsed(&collapsed, &line);

    // The ellipsis symbol's run picks up the RTL bidi level from the flow direction.
    let runs = collapsed.text_runs();

    let ellipsis_run = runs
        .iter()
        .filter_map(|run| run.downcast_ref::<ShapedTextRun>())
        .find(|run| run.text().to_string_lossy().contains(ELLIPSIS))
        .unwrap();

    assert!(!ellipsis_run.shaped_buffer().is_left_to_right());
}

#[test]
fn collapse_with_multiple_shaped_runs_preserves_ellipsis() {
    let _scope = TextTestScope::new();

    // Three independent runs. Trim point lands somewhere in the middle — collapse must not
    // silently drop a run or duplicate one.
    let line = build_line_of_runs(&["AAAA", "BBBB", "CCCC"]);

    assert_eq!(line.text_runs().len(), 3);

    let collapsed = collapse(&line, trailing_char(line.width() / 2.0, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    let text = logical_text(&*collapsed);

    assert!(text.contains(ELLIPSIS));

    // Every preserved character must come from the original source text in original order.
    let preserved = text.replace(ELLIPSIS, "");

    assert!("AAAABBBBCCCC".starts_with(&preserved));

    // 36 leaves 30 next to the ellipsis: five glyphs.
    assert_eq!(text, "AAAAB\u{2026}");
}

#[test]
fn leading_prefix_with_fully_fitting_tail_run_does_not_throw() {
    let _scope = TextTestScope::new();

    // On a multi-run line a logical-tail run can fit entirely within the remaining suffix
    // budget. The long leading run forces the collapse; the short trailing run wholly
    // fits the suffix budget and exercises that boundary.
    let line = build_line_of_runs(&["AAAAAAAAAAAA", "B"]);

    let collapsed = collapse(&line, leading_prefix(2, line.width() * 0.7, FlowDirection::LeftToRight));

    assert_collapsed(&collapsed, &line);

    let text = logical_text(&*collapsed);

    assert!(text.contains(ELLIPSIS));

    // The fully-fitting trailing run must survive in the logical-tail suffix.
    assert!(text.contains('B'));

    // 54.6 leaves 48.6 next to the ellipsis: the prefix of two, the tail run and five more glyphs.
    assert_eq!(text, "AA\u{2026}AAAAAB");
}
