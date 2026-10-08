//! Port of upstream's `Media/TextFormatting/TextCollapsingBidiTests.cs` of the
//! Skia unit tests: characterization tests for `TextCollapsingProperties`
//! implementations, with emphasis on BiDi correctness.
//!
//! Upstream's `Assert.Throws<ArgumentOutOfRangeException>` is a caught panic
//! (the port panics where upstream throws for a programmer error).

use super::text_formatter_tests::start;
use crate::unit_tests::media::text_formatting::SingleBufferTextSource;
use ferroui_base::media::text_formatting::{
    DrawableTextRun, GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, LogicalTextRunEnumerator,
    ShapedTextRun, TextCharacters, TextCollapsingProperties, TextFormatter, TextFormatterImpl,
    TextLeadingPrefixCharacterEllipsis, TextLine, TextLineImpl, TextParagraphProperties, TextRun, TextRunProperties,
    TextTrailingCharacterEllipsis, TextTrailingWordEllipsis,
};
use ferroui_base::media::{
    BaselineAlignment, Brushes, FlowDirection, TextAlignment, TextPathSegmentEllipsis, TextWrapping, Typeface,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[test]
fn ltr_trailing_character_trims_from_end() {
    let _scope = start();

    let line = build_line("Hello world", FlowDirection::LeftToRight);
    let collapsing = trailing_char(line.width() / 2.0, FlowDirection::LeftToRight);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let text = logical_text(&*collapsed);
    assert!(text.contains('…'), "{text:?}");
    assert!(text.starts_with('H'), "{text:?}");
}

#[test]
fn ltr_trailing_word_trims_on_word_boundary() {
    let _scope = start();

    let line = build_line("Hello world foo", FlowDirection::LeftToRight);
    let collapsing = trailing_word(line.width() / 2.0, FlowDirection::LeftToRight);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    assert!(logical_text(&*collapsed).contains('…'));
}

#[test]
fn ltr_prefix_character_ellipsis_preserves_prefix_and_suffix() {
    let _scope = start();

    let line = build_line("01234 01234 01234", FlowDirection::LeftToRight);
    let collapsing = leading_prefix(8, 120.0, FlowDirection::LeftToRight);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let text = logical_text(&*collapsed);
    assert!(text.starts_with("01234 01"), "{text:?}");
    assert!(text.contains('…'), "{text:?}");
    // Suffix must reappear after the symbol.
    assert!(text.ends_with("4 01234"), "{text:?}");
}

#[test]
fn ltr_path_segment_ellipsis_collapses_middle() {
    let _scope = start();

    let line = build_line("verylongdirectory\\file.txt", FlowDirection::LeftToRight);
    let collapsing: Rc<dyn TextCollapsingProperties> = Rc::new(TextPathSegmentEllipsis::new(
        "…",
        line.width() / 2.0,
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        FlowDirection::LeftToRight,
    ));
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let text = logical_text(&*collapsed);
    assert!(text.contains('…'), "{text:?}");
    // Last segment ("file.txt") should be preserved on at least
    // some prefix; we don't assert exact width because Width math
    // depends on the font.
    assert!(text.contains(".txt"), "{text:?}");
}

#[test]
fn width_greater_than_line_returns_same_line() {
    let _scope = start();

    let line = build_line("abc", FlowDirection::LeftToRight);
    let collapsing = trailing_char(line.width() + 100.0, FlowDirection::LeftToRight);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    // Collapse returns null → TextLineImpl.Collapse returns `this`.
    assert!(Rc::ptr_eq(&line, &collapsed));
    assert!(!collapsed.has_collapsed());
}

#[test]
fn width_less_than_symbol_returns_empty_collapsed_line() {
    let _scope = start();

    let line = build_line("abcdef", FlowDirection::LeftToRight);

    // Width below symbol width → implementation returns [] → line
    // gets HasCollapsed = true but no runs.
    let collapsing = trailing_char(0.001, FlowDirection::LeftToRight);
    let collapsed = line.collapse(&[Some(collapsing)]);

    assert!(collapsed.has_collapsed());
    assert!(collapsed.text_runs().is_empty());
}

#[test]
fn rtl_trailing_character_preserves_logical_prefix() {
    let _scope = start();

    let text = "السلام عليكم ورحمة الله وبركاته";
    let line = build_line(text, FlowDirection::RightToLeft);
    let collapsing = trailing_char(line.width() / 2.0, FlowDirection::RightToLeft);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let logical = logical_text(&*collapsed);
    assert!(logical.contains('…'), "{logical:?}");
    assert!(logical.starts_with(&substring(text, 0, 1)), "{logical:?}");
}

#[test]
fn rtl_trailing_word_preserves_logical_prefix() {
    let _scope = start();

    let text = "السلام عليكم ورحمة الله وبركاته";
    let line = build_line(text, FlowDirection::RightToLeft);
    let collapsing = trailing_word(line.width() / 2.0, FlowDirection::RightToLeft);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    assert!(logical_text(&*collapsed).contains('…'));
}

#[test]
fn rtl_prefix_character_ellipsis_preserves_logical_prefix() {
    let _scope = start();

    let text = "السلام عليكم ورحمة الله وبركاته";
    let line = build_line(text, FlowDirection::RightToLeft);
    let collapsing = leading_prefix(4, line.width() / 2.0, FlowDirection::RightToLeft);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let logical = logical_text(&*collapsed);
    assert!(logical.starts_with(&substring(text, 0, 4)), "{logical:?}");
    assert!(logical.contains('…'), "{logical:?}");
}

#[test]
fn mixed_trailing_character_preserves_logical_prefix() {
    let _scope = start();

    let text = "Hello مرحبا world";
    let line = build_line(text, FlowDirection::LeftToRight);
    let collapsing = trailing_char(line.width() * 0.6, FlowDirection::LeftToRight);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    assert!(logical_text(&*collapsed).starts_with("Hello"));
}

#[test]
fn mixed_prefix_character_ellipsis_preserves_logical_prefix_and_suffix() {
    let _scope = start();

    let text = "Hello مرحبا world";
    let line = build_line(text, FlowDirection::LeftToRight);
    let collapsing = leading_prefix(5, line.width() * 0.6, FlowDirection::LeftToRight);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let logical = logical_text(&*collapsed);
    assert!(logical.starts_with("Hello"), "{logical:?}");
    assert!(logical.contains('…'), "{logical:?}");
}

#[test]
fn mixed_trailing_word_preserves_logical_prefix() {
    let _scope = start();

    let text = "Hello مرحبا world";
    let line = build_line(text, FlowDirection::LeftToRight);
    let collapsing = trailing_word(line.width() * 0.6, FlowDirection::LeftToRight);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let logical = logical_text(&*collapsed);
    assert!(logical.contains('…'), "{logical:?}");
    assert!(logical.starts_with("Hello"), "{logical:?}");
}

#[test]
fn mixed_path_segment_ellipsis_preserves_last_segment() {
    let _scope = start();

    // Mixed-bidi path: ASCII-only separators with an RTL directory
    // name embedded. Segmentation is separator-driven, so the
    // logical-tail segment ("file.txt") must survive.
    let text = "C:\\folder\\مجلد\\file.txt";
    let line = build_line(text, FlowDirection::LeftToRight);
    let collapsing: Rc<dyn TextCollapsingProperties> = Rc::new(TextPathSegmentEllipsis::new(
        "…",
        line.width() / 2.0,
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        FlowDirection::LeftToRight,
    ));
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let logical = logical_text(&*collapsed);
    assert!(logical.contains('…'), "{logical:?}");
    assert!(logical.contains("file.txt"), "{logical:?}");
}

#[test]
fn rtl_path_segment_ellipsis_preserves_last_segment() {
    let _scope = start();

    // Pure-RTL path. The font fallback may render Arabic as
    // .notdef glyphs in the test environment, but segmentation is
    // character-driven (separators are ASCII '/' and '\\') so the
    // logical-tail segment "ملف.txt" must still be detected and
    // preserved.
    let text = "مجلد/مجلد2/ملف.txt";
    let line = build_line(text, FlowDirection::RightToLeft);
    let collapsing: Rc<dyn TextCollapsingProperties> = Rc::new(TextPathSegmentEllipsis::new(
        "…",
        line.width() / 2.0,
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        FlowDirection::RightToLeft,
    ));
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let logical = logical_text(&*collapsed);
    assert!(logical.contains('…'), "{logical:?}");
    assert!(logical.contains("ملف.txt"), "{logical:?}");
}

#[test]
fn ltr_path_segment_ellipsis_middle_collapse_preserves_first_and_last_segments() {
    let _scope = start();

    // 3-segment path; middle is intentionally long so collapsing
    // it alone produces a fitting result.
    let text = "a/middlemiddlemiddlemiddlemiddlemiddlemiddlemiddlemiddlemiddle/c.txt";
    let line = build_line(text, FlowDirection::LeftToRight);
    let budget = line.width() * 0.3;
    let collapsing: Rc<dyn TextCollapsingProperties> = Rc::new(TextPathSegmentEllipsis::new(
        "…",
        budget,
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        FlowDirection::LeftToRight,
    ));
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let logical = logical_text(&*collapsed);
    assert!(logical.contains('…'), "{logical:?}");
    assert!(logical.contains('a'), "{logical:?}");
    assert!(logical.contains("c.txt"), "{logical:?}");
}

#[test]
fn rtl_path_segment_ellipsis_middle_collapse_preserves_first_and_last_segments() {
    let _scope = start();

    let text = "اول/منتصفمنتصفمنتصفمنتصفمنتصفمنتصفمنتصفمنتصف/اخر.txt";
    let line = build_line(text, FlowDirection::RightToLeft);
    let budget = line.width() * 0.3;
    let collapsing: Rc<dyn TextCollapsingProperties> = Rc::new(TextPathSegmentEllipsis::new(
        "…",
        budget,
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        FlowDirection::RightToLeft,
    ));
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let logical = logical_text(&*collapsed);
    assert!(logical.contains('…'), "{logical:?}");
    assert!(logical.contains("اول"), "{logical:?}");
    assert!(logical.contains("اخر.txt"), "{logical:?}");
}

#[test]
fn try_measure_characters_returned_length_fits_logical_leading_in_budget() {
    for (text, ltr) in [("Hello world abcdef", true), ("السلام عليكم ورحمة", false)] {
        let _scope = start();

        let dir = if ltr { FlowDirection::LeftToRight } else { FlowDirection::RightToLeft };
        let line = build_line(text, dir);
        let shaped_run = first_shaped_run(&*line);
        assert!(shaped_run.is_some());
        let shaped_run = shaped_run.unwrap();

        let buffer = shaped_run.shaped_buffer();
        let total_width = shaped_run.size().width;

        // Probe at several budget points; the contract must hold at all of them.
        for i in 1..10 {
            let budget = total_width * i as f64 / 10.0;
            let Some(measured) = shaped_run.try_measure_characters(budget) else {
                continue;
            };
            if measured <= 0 {
                continue;
            }

            // The width of the LOGICAL leading `measured` characters must fit
            // in `budget`. GetCharRangeWidth uses the cluster cache, which is
            // built in logical order for both directions.
            let actual_leading_width = buffer.get_char_range_width(0, measured);

            assert!(
                actual_leading_width <= budget + 0.5,
                "{dir:?}: budget={budget:.2}, measured={measured}, actual logical-leading width={actual_leading_width:.2}"
            );
        }
    }
}

#[test]
fn try_measure_characters_backwards_returned_length_fits_logical_trailing_in_budget() {
    for (text, ltr) in [("Hello world abcdef", true), ("السلام عليكم ورحمة", false)] {
        let _scope = start();

        let dir = if ltr { FlowDirection::LeftToRight } else { FlowDirection::RightToLeft };
        let line = build_line(text, dir);
        let shaped_run = first_shaped_run(&*line);
        assert!(shaped_run.is_some());
        let shaped_run = shaped_run.unwrap();

        let buffer = shaped_run.shaped_buffer();
        let total_width = shaped_run.size().width;
        let text_length = shaped_run.length();

        for i in 1..10 {
            let budget = total_width * i as f64 / 10.0;
            let Some((measured, _)) = shaped_run.try_measure_characters_backwards(budget) else {
                continue;
            };
            if measured <= 0 {
                continue;
            }

            let actual_trailing_width = buffer.get_char_range_width(text_length - measured, text_length);

            assert!(
                actual_trailing_width <= budget + 0.5,
                "{dir:?}: budget={budget:.2}, measured={measured}, actual logical-trailing width={actual_trailing_width:.2}"
            );
        }
    }
}

#[test]
fn logical_text_run_enumerator_without_indexed_runs_returns_distinct_runs() {
    let _scope = start();

    let props: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let runs: Vec<Rc<dyn TextRun>> = vec![
        Rc::new(TextCharacters::from_str("AAA", props.clone())),
        Rc::new(TextCharacters::from_str("BBB", props.clone())),
        Rc::new(TextCharacters::from_str("CCC", props.clone())),
    ];

    // Construct TextLineImpl directly and SKIP FinalizeLine so that
    // _indexedTextRuns stays null. This is exactly the branch
    // LogicalTextRunEnumerator handles incorrectly today.
    let paragraph_props: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::new(props));
    let line = TextLineImpl::new(
        runs.clone(),
        0,
        9,
        f64::INFINITY,
        paragraph_props,
        FlowDirection::LeftToRight,
        None,
        false,
    );

    let mut enumerator = LogicalTextRunEnumerator::new(&*line);
    let mut seen: Vec<Rc<dyn TextRun>> = Vec::new();
    while let Some(run) = enumerator.move_next() {
        seen.push(run);
    }

    assert_eq!(3, seen.len());
    assert!(Rc::ptr_eq(&runs[0], &seen[0]));
    assert!(Rc::ptr_eq(&runs[1], &seen[1]));
    assert!(Rc::ptr_eq(&runs[2], &seen[2]));
}

#[test]
fn leading_prefix_negative_prefix_length_throws() {
    let _scope = start();

    let props: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let result = catch_unwind(AssertUnwindSafe(|| {
        TextLeadingPrefixCharacterEllipsis::new("…", -1, 100.0, props.clone(), FlowDirection::LeftToRight)
    }));
    assert!(result.is_err());
}

#[test]
fn leading_prefix_honours_flow_direction_for_symbol() {
    let _scope = start();

    let text = "السلام عليكم ورحمة";
    let line = build_line(text, FlowDirection::RightToLeft);
    let collapsing = leading_prefix(4, line.width() / 2.0, FlowDirection::RightToLeft);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);

    // The ellipsis symbol's run should pick up the RTL bidi level
    // from the FlowDirection passed to the constructor. Today the
    // ctor in Collapse() hardcodes LeftToRight, so the symbol run
    // has IsLeftToRight == true.
    let ellipsis_run = collapsed
        .text_runs()
        .iter()
        .filter_map(|run| run.clone().downcast_rc::<ShapedTextRun>())
        .find(|r| r.text().to_string_lossy().contains('…'));
    assert!(ellipsis_run.is_some());
    assert!(!ellipsis_run.unwrap().shaped_buffer().is_left_to_right());
}

#[test]
fn collapse_with_multiple_shaped_runs_preserves_ellipsis() {
    // Three independent runs via FixedRunsTextSource. Trim point lands
    // somewhere in the middle — collapse must not silently drop a run
    // or duplicate one (covers the SplitTextRuns interaction).
    let _scope = start();

    let props: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let source_runs: Vec<Rc<dyn TextRun>> = vec![
        Rc::new(TextCharacters::from_str("AAAA", props.clone())),
        Rc::new(TextCharacters::from_str("BBBB", props.clone())),
        Rc::new(TextCharacters::from_str("CCCC", props.clone())),
    ];
    let src = FixedRunsTextSource::new(source_runs);
    let formatter = TextFormatterImpl::new();
    let line = formatter.format_line(
        &src,
        0,
        f64::INFINITY,
        &(Rc::new(GenericTextParagraphProperties::new(props)) as Rc<dyn TextParagraphProperties>),
        None,
    );
    assert!(line.is_some());
    let line = line.unwrap();

    let collapsing = trailing_char(line.width() / 2.0, FlowDirection::LeftToRight);
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let text = logical_text(&*collapsed);
    assert!(text.contains('…'), "{text:?}");
    // Every preserved character must come from the original source
    // text in original order — no garbage.
    let preserved = text.replace('…', "");
    assert!("AAAABBBBCCCC".starts_with(&preserved), "{preserved:?}");
}

#[test]
fn leading_prefix_with_fully_fitting_tail_run_does_not_throw() {
    // Regression: on a multi-run line a logical-tail run can fit entirely
    // within the remaining suffix budget. TryMeasureCharactersBackwards then
    // returns suffixCount == run.Length, and the old code called
    // ShapedTextRun.Split(0), which throws ArgumentOutOfRangeException. The
    // long leading run forces the collapse; the short trailing run wholly
    // fits the suffix budget and exercises that boundary.
    let _scope = start();

    let props: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let source_runs: Vec<Rc<dyn TextRun>> = vec![
        Rc::new(TextCharacters::from_str("AAAAAAAAAAAA", props.clone())), // long: forces the collapse
        Rc::new(TextCharacters::from_str("B", props.clone())),            // short: wholly fits the suffix budget
    ];
    let src = FixedRunsTextSource::new(source_runs);
    let formatter = TextFormatterImpl::new();
    let line = formatter.format_line(
        &src,
        0,
        f64::INFINITY,
        &(Rc::new(GenericTextParagraphProperties::new(props)) as Rc<dyn TextParagraphProperties>),
        None,
    );
    assert!(line.is_some());
    let line = line.unwrap();

    let collapsing = leading_prefix(2, line.width() * 0.7, FlowDirection::LeftToRight);

    // Previously threw ArgumentOutOfRangeException from Split(0).
    let collapsed = line.clone().collapse(&[Some(collapsing)]);

    assert_collapsed(&collapsed, &line);
    let text = logical_text(&*collapsed);
    assert!(text.contains('…'), "{text:?}");
    // The fully-fitting trailing run must survive in the logical-tail suffix.
    assert!(text.contains('B'), "{text:?}");
}

fn build_line(text: &str, flow: FlowDirection) -> Rc<dyn TextLine> {
    let props = Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        12.0,
        None,
        Some(Brushes::black()),
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ));
    let paragraph_props: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_all(
        flow,
        TextAlignment::Left,
        true,
        true,
        props.clone(),
        TextWrapping::NoWrap,
        0.0,
        0.0,
        0.0,
    ));
    let source = SingleBufferTextSource::new(text, props, false);
    let formatter = TextFormatterImpl::new();
    let line = formatter.format_line(&source, 0, f64::INFINITY, &paragraph_props, None);
    assert!(line.is_some());
    line.unwrap()
}

fn trailing_char(width: f64, flow: FlowDirection) -> Rc<dyn TextCollapsingProperties> {
    Rc::new(TextTrailingCharacterEllipsis::new(
        "…",
        width,
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        flow,
    ))
}

fn trailing_word(width: f64, flow: FlowDirection) -> Rc<dyn TextCollapsingProperties> {
    Rc::new(TextTrailingWordEllipsis::new(
        "…",
        width,
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        flow,
    ))
}

fn leading_prefix(prefix_length: i32, width: f64, flow: FlowDirection) -> Rc<dyn TextCollapsingProperties> {
    Rc::new(TextLeadingPrefixCharacterEllipsis::new(
        "…",
        prefix_length,
        width,
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        flow,
    ))
}

fn assert_collapsed(collapsed: &Rc<dyn TextLine>, original: &Rc<dyn TextLine>) {
    assert!(!Rc::ptr_eq(original, collapsed));
    assert!(collapsed.has_collapsed(), "Collapsed line must report HasCollapsed = true.");
}

/// Concatenates run text in logical order via [`LogicalTextRunEnumerator`].
/// For LTR-only lines this equals walking `text_runs` directly; for
/// RTL/mixed lines it returns the original-text order (what the collapse
/// contract requires) instead of the visual post-bidi order.
fn logical_text(line: &dyn TextLine) -> String {
    let mut enumerator = LogicalTextRunEnumerator::new(line);
    let mut sb: Vec<u16> = Vec::new();
    while let Some(run) = enumerator.move_next() {
        sb.extend_from_slice(run.text_span());
    }
    String::from_utf16_lossy(&sb)
}

/// C# `text.Substring(start, length)` (UTF-16 code units).
fn substring(text: &str, start: usize, length: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16_lossy(&units[start..start + length])
}

/// C# `line.TextRuns.OfType<ShapedTextRun>().FirstOrDefault()`.
fn first_shaped_run(line: &dyn TextLine) -> Option<Rc<ShapedTextRun>> {
    line.text_runs().iter().find_map(|run| run.clone().downcast_rc::<ShapedTextRun>())
}

/// Local copy of the FixedRunsTextSource pattern used in
/// TextLineTests — that class is private, so duplicate here.
struct FixedRunsTextSource {
    text_runs: Vec<Rc<dyn TextRun>>,
}

impl FixedRunsTextSource {
    fn new(text_runs: Vec<Rc<dyn TextRun>>) -> Self {
        Self { text_runs }
    }
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
