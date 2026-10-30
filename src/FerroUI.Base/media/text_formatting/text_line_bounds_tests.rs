//! The box of a line under each `TextLineBounds` (a FerroUI addition after WinUI's
//! `TextLineBounds`, docs/porting/DEVIATIONS.md, Text block), on the test fonts: an ascent of
//! 800, a descent of 200 and a line gap of 100 in 1000 units per em, no OS/2 table and no
//! outlines, so the cap height is seven tenths of the ascent (560).

use std::rc::Rc;

use crate::media::text_formatting::testing::{
    format_line, line_height, run_properties, SingleBufferTextSource, TextTestScope, ASCENDER, DESCENDER,
    DESIGN_EM_HEIGHT, LINE_GAP,
};
use crate::media::text_formatting::{
    GenericTextParagraphProperties, TextLine, TextLineMetricsMode, TextParagraphProperties, TextRunProperties,
};
use crate::media::{FlowDirection, GlyphTypeface, TextAlignment, TextLineBounds, TextWrapping, Typeface};

const EM: f64 = 12.0;

fn scale() -> f64 {
    EM / DESIGN_EM_HEIGHT as f64
}

fn ascent() -> f64 {
    ASCENDER as f64 * scale()
}

fn descent() -> f64 {
    -(DESCENDER as f64) * scale()
}

fn half_gap() -> f64 {
    LINE_GAP as f64 * scale() / 2.0
}

fn cap_height() -> f64 {
    (ASCENDER as f64 * 0.7).round() * scale()
}

fn format(text_line_bounds: TextLineBounds, line_height: f64) -> Rc<dyn TextLine> {
    let default_properties: Rc<dyn TextRunProperties> = run_properties(EM);
    let properties = GenericTextParagraphProperties::with_all(
        FlowDirection::LeftToRight,
        TextAlignment::Left,
        true,
        false,
        default_properties.clone(),
        TextWrapping::NoWrap,
        line_height,
        0.0,
        0.0,
    );
    properties.set_text_line_bounds(text_line_bounds);
    let properties: Rc<dyn TextParagraphProperties> = Rc::new(properties);
    let text_source = SingleBufferTextSource::new("Hxg", default_properties);

    format_line(&text_source, 0, f64::INFINITY, &properties, None).unwrap()
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < 1e-9, "{what}: {actual} != {expected}");
}

#[test]
fn the_cap_height_falls_back_to_seven_tenths_of_the_ascent() {
    let _scope = TextTestScope::new();

    let glyph_typeface: Rc<GlyphTypeface> = Typeface::default_typeface().glyph_typeface();

    assert_eq!(glyph_typeface.metrics().cap_height, 560);
}

#[test]
fn full_is_the_box_of_the_reference() {
    let _scope = TextTestScope::new();

    let line = format(TextLineBounds::Full, f64::NAN);

    assert_close(line.height(), line_height(EM), "height");
    assert_close(line.baseline(), ascent() + half_gap(), "baseline");
}

#[test]
fn trim_to_cap_height_puts_the_top_at_the_cap_height() {
    let _scope = TextTestScope::new();

    let line = format(TextLineBounds::TrimToCapHeight, f64::NAN);

    assert_close(line.baseline(), cap_height(), "baseline");
    assert_close(line.height(), cap_height() + descent() + half_gap(), "height");
}

#[test]
fn trim_to_baseline_puts_the_bottom_at_the_baseline() {
    let _scope = TextTestScope::new();

    let line = format(TextLineBounds::TrimToBaseline, f64::NAN);

    assert_close(line.baseline(), ascent() + half_gap(), "baseline");
    assert_close(line.height(), line.baseline(), "height");
}

#[test]
fn tight_runs_from_the_cap_height_to_the_baseline() {
    let _scope = TextTestScope::new();

    let line = format(TextLineBounds::Tight, f64::NAN);

    assert_close(line.baseline(), cap_height(), "baseline");
    assert_close(line.height(), cap_height(), "height");
}

#[test]
fn a_line_height_centres_the_trimmed_box() {
    let _scope = TextTestScope::new();

    let line = format(TextLineBounds::Tight, 20.0);

    assert_close(line.height(), 20.0, "height");
    // The capitals are in the middle of the line.
    assert_close(line.baseline() - cap_height() / 2.0, 10.0, "middle of the capitals");

    let full = format(TextLineBounds::Full, 20.0);

    // The reference centres the ascent and the descent.
    assert_close(full.baseline(), ascent() + (20.0 - ascent() - descent()) / 2.0, "baseline of Full");
}

#[test]
fn a_smaller_line_height_clamps_the_trimmed_box() {
    let _scope = TextTestScope::new();

    let line = format(TextLineBounds::TrimToCapHeight, 5.0);

    assert_close(line.height(), 5.0, "height");
    assert_close(line.baseline(), cap_height(), "baseline");
}

#[test]
fn full_is_the_default_of_paragraph_properties() {
    let _scope = TextTestScope::new();

    let properties = GenericTextParagraphProperties::new(run_properties(EM));

    assert_eq!(properties.text_line_bounds(), TextLineBounds::Full);

    properties.set_text_line_bounds(TextLineBounds::Tight);
    let copy = GenericTextParagraphProperties::from_properties(&properties);

    assert_eq!(copy.text_line_bounds(), TextLineBounds::Tight);
}

#[test]
fn paragraph_copy_preserves_tabs_and_line_bounds() {
    use crate::media::text_formatting::TextTabStops;

    let _scope = TextTestScope::new();
    let properties = GenericTextParagraphProperties::new(run_properties(EM)).with_tabs(
        TextTabStops::parse("48,Left;96,Decimal,Dots").unwrap(),
        24.0,
        7.0,
    );
    properties.set_text_line_bounds(TextLineBounds::Tight);

    let copy = GenericTextParagraphProperties::from_properties(&properties);

    assert_eq!(copy.tab_stops(), properties.tab_stops());
    assert_eq!(copy.default_incremental_tab(), 24.0);
    assert_eq!(copy.tab_origin(), 7.0);
    assert_eq!(copy.text_line_bounds(), TextLineBounds::Tight);
    properties.set_text_line_bounds(TextLineBounds::Full);
    assert_eq!(copy.text_line_bounds(), TextLineBounds::Tight);
}

/// Tab layout is opt-in: a copy of paragraph properties without tab stops
/// keeps upstream's values (no tab stops, the default tab width of the copy).
#[test]
fn paragraph_copy_without_tab_stops_keeps_the_upstream_defaults() {
    struct WithIncrementalTab(Rc<dyn TextRunProperties>);
    impl TextParagraphProperties for WithIncrementalTab {
        fn flow_direction(&self) -> FlowDirection { FlowDirection::LeftToRight }
        fn text_alignment(&self) -> TextAlignment { TextAlignment::Left }
        fn line_height(&self) -> f64 { f64::NAN }
        fn first_line_in_paragraph(&self) -> bool { true }
        fn default_text_run_properties(&self) -> &Rc<dyn TextRunProperties> { &self.0 }
        fn text_wrapping(&self) -> TextWrapping { TextWrapping::NoWrap }
        fn indent(&self) -> f64 { 0.0 }
        fn default_incremental_tab(&self) -> f64 { 40.0 }
    }

    let _scope = TextTestScope::new();
    let source = WithIncrementalTab(run_properties(EM));
    let copy = GenericTextParagraphProperties::from_properties(&source);

    assert_eq!(copy.tab_stops(), None);
    assert_eq!(copy.default_incremental_tab(), 0.0);
    assert_eq!(copy.tab_origin(), 0.0);
    assert_eq!(copy.line_metrics_mode(), TextLineMetricsMode::GlyphTypeface);
}
