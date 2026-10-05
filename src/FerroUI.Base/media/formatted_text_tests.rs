//! Tests of the formatted text on the text test harness (fixed advance fonts:
//! 6 per glyph at an em size of 12, a line height of 13.2).
//!
//! Additions: upstream has no unit tests for this class (it is only drawn by a
//! render test, which needs a real renderer).

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

use super::*;
use crate::media::text_formatting::testing::{
    advance, line_height, DrawCall, RecordingDrawingSink, TextTestScope, CJK_FAMILY, DEFAULT_FAMILY,
};
use crate::media::text_formatting::{TextLayout, TextLayoutOptions};
use crate::media::{Brushes, FontFeature, TextDecorations};

const EM: f64 = 12.0;

fn black() -> Option<Rc<dyn IBrush>> {
    let brush: Rc<dyn IBrush> = Brushes::black();

    Some(brush)
}

fn formatted(text: &str) -> FormattedText {
    FormattedText::new(
        text,
        CultureInfo::invariant_culture(),
        FlowDirection::LeftToRight,
        Typeface::default_typeface(),
        EM,
        black(),
    )
}

/// The format runs: length and properties of every span.
fn runs(formatted_text: &FormattedText) -> Vec<(i32, Rc<dyn TextRunProperties>)> {
    let format_runs = formatted_text.format_runs.borrow();

    let mut enumerator = format_runs.get_enumerator();

    let mut result = Vec::new();

    while enumerator.move_next() {
        let span = enumerator.current();

        result.push((span.length, span.element.clone().expect("every span has properties")));
    }

    result
}

fn run_lengths(formatted_text: &FormattedText) -> Vec<i32> {
    runs(formatted_text).iter().map(|(length, _)| *length).collect()
}

/// The glyph runs a draw produces: text and origin.
fn drawn_glyph_runs(formatted_text: &FormattedText, origin: Point) -> Vec<(String, Point)> {
    let mut sink = RecordingDrawingSink::new();

    formatted_text.draw(&mut sink, origin);

    assert_eq!(sink.transform_depth(), 0);

    sink.calls()
        .into_iter()
        .filter_map(|call| match call {
            DrawCall::GlyphRun { text, origin } => Some((text, origin)),
            _ => None,
        })
        .collect()
}

fn drawn_text(formatted_text: &FormattedText) -> String {
    drawn_glyph_runs(formatted_text, Point::default()).into_iter().map(|(text, _)| text).collect()
}

#[track_caller]
fn assert_panics(action: impl FnOnce()) {
    let result = catch_unwind(AssertUnwindSafe(action));

    assert!(result.is_err(), "expected a panic");
}

#[test]
fn construction_has_upstream_defaults() {
    let _scope = TextTestScope::new();

    let text = formatted("abc");

    assert_eq!(text.flow_direction(), FlowDirection::LeftToRight);
    assert_eq!(text.text_alignment(), TextAlignment::Left);
    assert_eq!(text.line_height(), 0.0);
    assert_eq!(text.max_text_width(), f64::INFINITY);
    assert_eq!(text.max_text_height(), f64::INFINITY);
    assert_eq!(text.max_line_count(), i32::MAX);
    assert!(text.get_max_text_widths().is_empty());
    assert!(Rc::ptr_eq(&text.trimming(), &<dyn TextTrimming>::word_ellipsis()));

    let runs = runs(&text);

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].0, 3);

    let properties = &runs[0].1;

    assert_eq!(*properties.typeface(), Typeface::default_typeface());
    assert_eq!(properties.font_rendering_em_size(), EM);
    assert!(properties.text_decorations().is_none());
    assert!(properties.background_brush().is_none());
    assert!(properties.font_features().is_none());
    assert_eq!(properties.culture_info(), Some(&CultureInfo::invariant_culture()));
    assert!(Rc::ptr_eq(properties.foreground_brush().unwrap(), &black().unwrap()));

    assert_eq!(FormattedText::DEFAULT_REAL_TO_IDEAL, 300.0);
    assert_eq!(FormattedText::IDEAL_INFINITE_WIDTH, 0x3FFFFFFE);
    assert_eq!(FormattedText::GREATEST_MULTIPLIER_OF_EM, 100.0);
    assert_eq!(FormattedText::REAL_INFINITE_WIDTH, 0x3FFFFFFE as f64 * FormattedText::DEFAULT_IDEAL_TO_REAL);
}

#[test]
fn single_line_metrics_match_the_text_layout() {
    let _scope = TextTestScope::new();

    let text = formatted("abc def  ");

    let layout = TextLayout::new(
        "abc def  ",
        Typeface::default_typeface(),
        TextLayoutOptions { font_size: EM, ..Default::default() },
    );

    assert_eq!(text.width(), 7.0 * advance(EM));
    assert_eq!(text.width_including_trailing_whitespace(), 9.0 * advance(EM));
    assert_eq!(text.height(), line_height(EM));

    assert_eq!(text.width(), layout.width());
    assert_eq!(text.width_including_trailing_whitespace(), layout.width_including_trailing_whitespace());
    assert_eq!(text.height(), layout.height());
    assert_eq!(text.baseline(), layout.baseline());

    // The black box metrics of one line are the line's.
    let line = &layout.text_lines()[0];

    assert_eq!(text.extent(), line.extent());
    assert_eq!(text.overhang_after(), line.overhang_after());
    assert_eq!(text.overhang_leading(), line.overhang_leading());
    assert_eq!(text.overhang_trailing(), line.overhang_trailing());
}

#[test]
fn black_box_metrics_are_computed_on_demand() {
    let _scope = TextTestScope::new();

    let text = formatted("abc");

    assert!(text.metrics.get().is_none());

    let _ = text.height();

    // The plain metrics leave the black box metrics unknown.
    assert!(text.metrics.get().unwrap().extent.is_nan());

    let extent = text.extent();

    assert!(!extent.is_nan());
    assert_eq!(text.metrics.get().unwrap().extent, extent);
    assert_eq!(text.height(), line_height(EM));
}

#[test]
fn empty_text_has_zero_metrics_and_draws_nothing() {
    let _scope = TextTestScope::new();

    let text = formatted("");

    assert_eq!(text.width(), 0.0);
    assert_eq!(text.width_including_trailing_whitespace(), 0.0);
    assert_eq!(text.height(), 0.0);
    assert_eq!(text.baseline(), 0.0);
    assert_eq!(text.extent(), 0.0);
    assert_eq!(text.overhang_after(), 0.0);
    assert_eq!(text.overhang_leading(), 0.0);
    assert_eq!(text.overhang_trailing(), 0.0);

    assert!(drawn_glyph_runs(&text, Point::default()).is_empty());
    assert!(text.build_highlight_geometry(Point::default()).is_none());

    // Formatting an empty range of an empty text is valid.
    text.set_font_size_range(20.0, 0, 0);
    text.set_font_size(20.0);
}

#[test]
fn line_breaks_give_lines() {
    let _scope = TextTestScope::new();

    let text = formatted("ab\ncdef\ng");

    assert_eq!(text.height(), 3.0 * line_height(EM));
    assert_eq!(text.width(), 4.0 * advance(EM));
    assert_eq!(text.width_including_trailing_whitespace(), 4.0 * advance(EM));

    let single = formatted("ab");

    assert_eq!(text.baseline(), single.baseline());
    assert!((text.extent() - (2.0 * line_height(EM) + single.extent())).abs() < 1e-9);

    let glyph_runs = drawn_glyph_runs(&text, Point::new(5.0, 7.0));

    assert_eq!(
        glyph_runs,
        [
            ("ab\n".to_owned(), Point::new(5.0, 7.0)),
            ("cdef\n".to_owned(), Point::new(5.0, 7.0 + line_height(EM))),
            ("g".to_owned(), Point::new(5.0, 7.0 + 2.0 * line_height(EM))),
        ]
    );
}

#[test]
fn set_font_size_formats_a_range_and_invalidates_the_metrics() {
    let _scope = TextTestScope::new();

    let text = formatted("abcdef");

    assert_eq!(text.width(), 6.0 * advance(EM));

    text.set_font_size_range(24.0, 0, 3);

    assert!(text.metrics.get().is_none());

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [3, 3]);
    assert_eq!(format_runs[0].1.font_rendering_em_size(), 24.0);
    assert_eq!(format_runs[1].1.font_rendering_em_size(), 12.0);

    assert_eq!(text.width(), 3.0 * advance(24.0) + 3.0 * advance(12.0));
    assert_eq!(text.height(), line_height(24.0));

    // The same value again changes nothing.
    text.set_font_size_range(24.0, 1, 2);

    assert_eq!(run_lengths(&text), [3, 3]);
    assert!(text.metrics.get().is_some());
    assert!(Rc::ptr_eq(&runs(&text)[0].1, &format_runs[0].1));

    // Extending the range merges with the equal neighbour.
    text.set_font_size_range(24.0, 2, 2);

    assert_eq!(run_lengths(&text), [4, 2]);

    // A range in the middle splits a run in three.
    text.set_font_size_range(6.0, 1, 2);

    assert_eq!(run_lengths(&text), [1, 2, 1, 2]);
    assert_eq!(text.width(), 2.0 * advance(24.0) + 2.0 * advance(6.0) + 2.0 * advance(12.0));

    // The whole text collapses the runs again.
    text.set_font_size(12.0);

    assert_eq!(run_lengths(&text), [6]);
    assert_eq!(text.width(), 6.0 * advance(EM));
    assert_eq!(text.height(), line_height(EM));
}

#[test]
fn typeface_setters_format_ranges() {
    let _scope = TextTestScope::new();

    let text = formatted("abcdef");

    text.set_font_weight_range(FontWeight::Bold, 0, 2);
    text.set_font_style_range(FontStyle::Italic, 1, 2);

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [1, 1, 1, 3]);

    let typefaces: Vec<(FontWeight, FontStyle)> = format_runs
        .iter()
        .map(|(_, properties)| (properties.typeface().weight(), properties.typeface().style()))
        .collect();

    assert_eq!(
        typefaces,
        [
            (FontWeight::Bold, FontStyle::Normal),
            (FontWeight::Bold, FontStyle::Italic),
            (FontWeight::Normal, FontStyle::Italic),
            (FontWeight::Normal, FontStyle::Normal),
        ]
    );

    for (_, properties) in &format_runs {
        assert!(*properties.typeface().font_family() == FontFamily::default_family());
        assert_eq!(properties.font_rendering_em_size(), EM);
    }

    // The family keeps weight and style.
    text.set_font_family_name_range(CJK_FAMILY, 0, 2);

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [1, 1, 1, 3]);
    assert_eq!(format_runs[0].1.typeface().font_family().name(), CJK_FAMILY);
    assert_eq!(format_runs[0].1.typeface().weight(), FontWeight::Bold);
    assert_eq!(format_runs[1].1.typeface().font_family().name(), CJK_FAMILY);
    assert_eq!(format_runs[1].1.typeface().style(), FontStyle::Italic);
    assert!(*format_runs[2].1.typeface().font_family() == FontFamily::default_family());

    // The same family again is not a change.
    text.set_font_family_range(FontFamily::new(CJK_FAMILY), 0, 2);

    assert!(Rc::ptr_eq(&runs(&text)[0].1, &format_runs[0].1));

    // The whole text with one typeface is one run again.
    text.set_font_typeface(Typeface::from_name(CJK_FAMILY));

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [6]);
    assert_eq!(*format_runs[0].1.typeface(), Typeface::from_name(CJK_FAMILY));

    text.set_font_family(FontFamily::new(DEFAULT_FAMILY));
    text.set_font_weight(FontWeight::Bold);
    text.set_font_style(FontStyle::Italic);
    text.set_font_family_name(DEFAULT_FAMILY);

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [6]);
    assert_eq!(
        *format_runs[0].1.typeface(),
        Typeface::with_style(FontFamily::new(DEFAULT_FAMILY), FontStyle::Italic, FontWeight::Bold, FontStretch::Normal)
    );

    text.set_font_typeface_range(Typeface::default_typeface(), 2, 2);

    assert_eq!(run_lengths(&text), [2, 2, 2]);
}

#[test]
fn appearance_setters_format_ranges_without_invalidating_the_metrics() {
    let _scope = TextTestScope::new();

    let text = formatted("abcdef");

    let _ = text.width();

    assert!(text.metrics.get().is_some());

    let red: Rc<dyn IBrush> = Brushes::red();

    text.set_foreground_brush_range(Some(red.clone()), 1, 2);

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [1, 2, 3]);
    assert!(Rc::ptr_eq(format_runs[1].1.foreground_brush().unwrap(), &red));
    assert!(Rc::ptr_eq(format_runs[0].1.foreground_brush().unwrap(), &black().unwrap()));

    // The same brush again is not a change.
    text.set_foreground_brush_range(Some(red.clone()), 1, 2);

    assert!(Rc::ptr_eq(&runs(&text)[1].1, &format_runs[1].1));

    text.set_foreground_brush_range(None, 4, 2);

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [1, 2, 1, 2]);
    assert!(format_runs[3].1.foreground_brush().is_none());

    text.set_foreground_brush(red.clone());

    assert_eq!(run_lengths(&text), [6]);
    assert!(Rc::ptr_eq(runs(&text)[0].1.foreground_brush().unwrap(), &red));

    let underline = TextDecorations::underline();

    text.set_text_decorations_range(underline.clone(), 0, 3);

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [3, 3]);
    assert!(format_runs[0].1.text_decorations() == Some(&underline));
    assert!(format_runs[1].1.text_decorations().is_none());

    text.set_text_decorations(underline.clone());

    assert_eq!(run_lengths(&text), [6]);

    let features = FontFeatureCollection::from_items([FontFeature::parse("kern")]);

    text.set_font_features_range(Some(features.clone()), 2, 2);

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [2, 2, 2]);
    assert_eq!(format_runs[1].1.font_features().unwrap().to_vec(), features.to_vec());
    assert!(format_runs[0].1.font_features().is_none());

    // An equal sequence of features is not a change.
    text.set_font_features_range(Some(FontFeatureCollection::from_items([FontFeature::parse("kern")])), 2, 2);

    assert!(Rc::ptr_eq(&runs(&text)[1].1, &format_runs[1].1));

    text.set_font_features(None);

    assert_eq!(run_lengths(&text), [6]);
    assert!(runs(&text)[0].1.font_features().is_none());

    // None of these setters invalidates the metrics (as upstream).
    assert!(text.metrics.get().is_some());
}

#[test]
fn set_culture_formats_a_range() {
    let _scope = TextTestScope::new();

    let text = formatted("abcdef");

    let _ = text.width();

    let culture = CultureInfo::get_culture_info("de-DE");

    text.set_culture_range(culture.clone(), 3, 3);

    assert!(text.metrics.get().is_none());

    let format_runs = runs(&text);

    assert_eq!(run_lengths(&text), [3, 3]);
    assert_eq!(format_runs[0].1.culture_info(), Some(&CultureInfo::invariant_culture()));
    assert_eq!(format_runs[1].1.culture_info(), Some(&culture));

    text.set_culture(culture.clone());

    assert_eq!(run_lengths(&text), [6]);
    assert_eq!(runs(&text)[0].1.culture_info(), Some(&culture));

    assert_eq!(text.width(), 6.0 * advance(EM));
}

#[test]
fn paragraph_setters_invalidate_the_metrics() {
    let _scope = TextTestScope::new();

    let text = formatted("ab");

    let _ = text.width();

    text.set_flow_direction(FlowDirection::RightToLeft);

    assert!(text.metrics.get().is_none());
    assert_eq!(text.flow_direction(), FlowDirection::RightToLeft);

    text.set_flow_direction(FlowDirection::LeftToRight);

    let _ = text.width();

    text.set_text_alignment(TextAlignment::Right);

    assert!(text.metrics.get().is_none());
    assert_eq!(text.text_alignment(), TextAlignment::Right);

    let _ = text.width();

    text.set_line_height(30.0);

    assert!(text.metrics.get().is_none());
    assert_eq!(text.line_height(), 30.0);
    assert_eq!(text.height(), 30.0);

    text.set_line_height(0.0);

    assert_eq!(text.height(), line_height(EM));

    // Right aligned text is drawn at the right edge of the text width.
    text.set_max_text_width(10.0 * advance(EM));

    assert!(text.metrics.get().is_none());

    let glyph_runs = drawn_glyph_runs(&text, Point::new(1.0, 2.0));

    assert_eq!(glyph_runs, [("ab".to_owned(), Point::new(1.0 + 8.0 * advance(EM), 2.0))]);

    assert_eq!(text.width(), 2.0 * advance(EM));
}

#[test]
fn max_text_width_wraps() {
    let _scope = TextTestScope::new();

    let text = formatted("aaa bbb ccc");

    assert_eq!(text.height(), line_height(EM));
    assert_eq!(text.width(), 11.0 * advance(EM));

    text.set_max_text_width(7.0 * advance(EM));

    assert_eq!(text.max_text_width(), 7.0 * advance(EM));
    assert_eq!(text.height(), 2.0 * line_height(EM));
    assert_eq!(text.width(), 7.0 * advance(EM));
    assert_eq!(text.width_including_trailing_whitespace(), 8.0 * advance(EM));

    let glyph_runs = drawn_glyph_runs(&text, Point::default());

    assert_eq!(
        glyph_runs,
        [("aaa bbb ".to_owned(), Point::new(0.0, 0.0)), ("ccc".to_owned(), Point::new(0.0, line_height(EM)))]
    );

    text.set_max_text_width(f64::INFINITY);

    assert_eq!(text.height(), line_height(EM));
}

#[test]
fn max_text_widths_apply_to_the_lines_in_turn() {
    let _scope = TextTestScope::new();

    let text = formatted("aaa bbb ccc ddd");

    text.set_max_text_width(1000.0);

    let widths = [3.0 * advance(EM), 7.0 * advance(EM)];

    text.set_max_text_widths(&widths);

    assert_eq!(text.get_max_text_widths(), widths);

    // The widths override the max text width; the last one is reused.
    let glyph_runs = drawn_glyph_runs(&text, Point::default());

    let texts: Vec<&str> = glyph_runs.iter().map(|(text, _)| text.as_str()).collect();

    assert_eq!(texts, ["aaa ", "bbb ccc ", "ddd"]);
    assert_eq!(text.height(), 3.0 * line_height(EM));
}

#[test]
fn max_line_count_trims_the_last_line() {
    let _scope = TextTestScope::new();

    let text = formatted("aaa bbb ccc ddd eee");

    text.set_max_text_width(7.0 * advance(EM));

    assert_eq!(text.height(), 3.0 * line_height(EM));

    text.set_max_line_count(2);

    assert_eq!(text.max_line_count(), 2);
    assert_eq!(text.height(), 2.0 * line_height(EM));

    // The default trimming is a word ellipsis.
    let drawn = drawn_text(&text);

    assert_eq!(drawn, "aaa bbb ccc\u{2026}");
    assert!(text.width() <= 7.0 * advance(EM));

    // The temporary non wrapping of the trimmed line is undone.
    assert_eq!(text.default_para_props.text_wrapping(), TextWrapping::WrapWithOverflow);

    text.set_trimming(<dyn TextTrimming>::character_ellipsis());

    assert!(Rc::ptr_eq(&text.trimming(), &<dyn TextTrimming>::character_ellipsis()));

    let drawn = drawn_text(&text);

    // Six characters and the ellipsis fit the seven character column of the last line.
    assert_eq!(drawn, "aaa bbb ccc dd\u{2026}");

    // Without trimming the remaining text is just not shown.
    text.set_trimming(<dyn TextTrimming>::none());

    assert_eq!(text.default_para_props.text_wrapping(), TextWrapping::Wrap);

    assert_eq!(drawn_text(&text), "aaa bbb ccc ddd ");
    assert_eq!(text.height(), 2.0 * line_height(EM));

    text.set_max_line_count(1);

    assert_eq!(drawn_text(&text), "aaa bbb ");
    assert_eq!(text.height(), line_height(EM));
}

#[test]
fn max_text_height_limits_the_lines() {
    let _scope = TextTestScope::new();

    let text = formatted("aaa bbb ccc ddd eee");

    text.set_max_text_width(7.0 * advance(EM));
    text.set_max_text_height(2.5 * line_height(EM));

    assert_eq!(text.max_text_height(), 2.5 * line_height(EM));
    assert_eq!(text.height(), 2.0 * line_height(EM));

    let drawn = drawn_text(&text);

    assert!(drawn.starts_with("aaa bbb "), "{drawn:?}");
    assert!(drawn.ends_with('\u{2026}'), "{drawn:?}");

    // Not even the first line fits: nothing is shown.
    text.set_max_text_height(0.5 * line_height(EM));

    assert_eq!(text.height(), 0.0);
    assert_eq!(text.width(), 0.0);
    assert!(drawn_glyph_runs(&text, Point::default()).is_empty());
}

#[test]
fn drawing_computes_the_metrics_and_repeats() {
    let _scope = TextTestScope::new();

    let text = formatted("ab\ncd");

    text.set_text_decorations_range(TextDecorations::underline(), 0, 2);

    let mut sink = RecordingDrawingSink::new();

    // The first draw computes the metrics (black box metrics included) on the way.
    text.draw(&mut sink, Point::new(3.0, 4.0));

    let metrics = text.metrics.get().expect("drawing caches the metrics");

    assert!(!metrics.extent.is_nan());
    assert_eq!(metrics.height, 2.0 * line_height(EM));
    assert_eq!(metrics.width, 2.0 * advance(EM));

    let first_calls = sink.calls();

    // "ab" (underlined), its line break and "cd".
    assert_eq!(first_calls.iter().filter(|call| matches!(call, DrawCall::GlyphRun { .. })).count(), 3);
    assert_eq!(first_calls.iter().filter(|call| matches!(call, DrawCall::Line(..))).count(), 1);

    // The second draw goes through the plain line loop and draws the same.
    let mut sink = RecordingDrawingSink::new();

    text.draw(&mut sink, Point::new(3.0, 4.0));

    assert_eq!(sink.calls(), first_calls);
    assert_eq!(sink.transform_depth(), 0);
}

#[test]
fn highlight_geometry_surrounds_the_text() {
    let _scope = TextTestScope::new();

    let text = formatted("abcdef");

    let geometry = text.build_highlight_geometry(Point::new(10.0, 20.0)).expect("a geometry");

    assert_eq!(geometry.bounds(), Rect::new(10.0, 20.0, 6.0 * advance(EM), line_height(EM)));

    let geometry = text.build_highlight_geometry_range(Point::new(10.0, 20.0), 2, 3).expect("a geometry");

    assert_eq!(geometry.bounds(), Rect::new(10.0 + 2.0 * advance(EM), 20.0, 3.0 * advance(EM), line_height(EM)));

    // An empty range has no geometry.
    assert!(text.build_highlight_geometry_range(Point::default(), 2, 0).is_none());

    // The lines of a range are combined.
    let text = formatted("ab\ncdef");

    let geometry = text.build_highlight_geometry_range(Point::new(1.0, 2.0), 1, 4).expect("a geometry");

    assert_eq!(geometry.bounds(), Rect::new(1.0, 2.0, 2.0 * advance(EM), 2.0 * line_height(EM)));

    // Right to left: the rectangles are mirrored in the paragraph width.
    let text = formatted("ab");

    text.set_flow_direction(FlowDirection::RightToLeft);
    text.set_max_text_width(10.0 * advance(EM));

    let geometry = text.build_highlight_geometry(Point::default()).expect("a geometry");

    assert_eq!(geometry.bounds().width, 2.0 * advance(EM));
    assert_eq!(geometry.bounds().height, line_height(EM));
}

#[test]
fn invalid_arguments_panic() {
    let _scope = TextTestScope::new();

    let new_with_size = |em_size: f64| {
        FormattedText::new(
            "abc",
            CultureInfo::invariant_culture(),
            FlowDirection::LeftToRight,
            Typeface::default_typeface(),
            em_size,
            None,
        )
    };

    assert_panics(|| drop(new_with_size(0.0)));
    assert_panics(|| drop(new_with_size(-1.0)));
    assert_panics(|| drop(new_with_size(f64::NAN)));
    assert_panics(|| drop(new_with_size(FormattedText::MAX_FONT_EM_SIZE * 2.0)));

    drop(new_with_size(FormattedText::MAX_FONT_EM_SIZE));

    let text = formatted("abc");

    // The start index.
    assert_panics(|| text.set_font_size_range(20.0, -1, 1));
    assert_panics(|| text.set_font_size_range(20.0, 4, 0));

    // The count.
    assert_panics(|| text.set_font_size_range(20.0, 0, -1));
    assert_panics(|| text.set_font_size_range(20.0, 0, 4));
    assert_panics(|| text.set_font_size_range(20.0, 2, 2));
    assert_panics(|| text.set_font_size_range(20.0, 1, i32::MAX));

    // The end of the text is a valid (empty) range.
    text.set_font_size_range(20.0, 3, 0);

    assert_panics(|| text.set_font_weight_range(FontWeight::Bold, 0, 4));
    assert_panics(|| text.set_font_style_range(FontStyle::Italic, 0, 4));
    assert_panics(|| text.set_font_typeface_range(Typeface::default_typeface(), 0, 4));
    assert_panics(|| text.set_font_family_name_range(DEFAULT_FAMILY, 0, 4));
    assert_panics(|| text.set_culture_range(CultureInfo::invariant_culture(), 0, 4));
    assert_panics(|| text.set_foreground_brush_range(None, 0, 4));
    assert_panics(|| text.set_text_decorations_range(TextDecorations::underline(), 0, 4));
    assert_panics(|| text.set_font_features_range(None, 0, 4));
    assert_panics(|| drop(text.build_highlight_geometry_range(Point::default(), 0, 4)));

    assert_panics(|| text.set_font_size(0.0));
    assert_panics(|| text.set_line_height(-1.0));
    assert_panics(|| text.set_max_text_width(-1.0));
    assert_panics(|| text.set_max_text_height(0.0));
    assert_panics(|| text.set_max_text_height(f64::NAN));
    assert_panics(|| text.set_max_line_count(0));
    assert_panics(|| text.set_max_text_widths(&[]));

    // Nothing was changed by the rejected calls.
    assert_eq!(run_lengths(&text), [3]);
    assert_eq!(runs(&text)[0].1.font_rendering_em_size(), EM);
    assert_eq!(text.width(), 3.0 * advance(EM));
}
