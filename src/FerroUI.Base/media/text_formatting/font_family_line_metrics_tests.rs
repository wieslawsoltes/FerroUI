//! Requested-family metrics on real formatter runs with synthetic OpenType
//! fonts. Expected boxes follow the pinned family and object callbacks;
//! native shaping and pixel equivalence are separate evidence boundaries.

use super::testing::{
    line_text, CustomDrawableRun, DrawCall, ListTextSource, RecordingDrawingSink,
    SingleBufferTextSource, TestFont, TextTestScope, DEFAULT_FAMILY,
};
use super::{
    DrawableTextRun, FontFamilyLineMetrics, GenericTextParagraphProperties,
    GenericTextRunProperties, ITextSource, ShapedTextRun, TextCharacters, TextFormatter,
    TextFormatterImpl, TextLine, TextLineImpl, TextLineMetricsMode, TextParagraphProperties,
    TextRun, TextRunCache, TextRunProperties,
};
use crate::media::{
    BaselineAlignment, Brushes, FlowDirection, FontFamily, TextAlignment, TextLineBounds,
    TextWrapping, Typeface,
};
use crate::utilities::ReadOnlyMemory;
use crate::{Point, Size};
use std::rc::Rc;

const MODES: [(TextLineBounds, f64, f64); 4] = [
    (TextLineBounds::Full, 16.0, 22.0),
    (TextLineBounds::TrimToCapHeight, 11.2, 17.2),
    (TextLineBounds::TrimToBaseline, 16.0, 16.0),
    (TextLineBounds::Tight, 11.2, 11.2),
];
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}
fn run_properties(family: &str, em: f64) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::from_name(family),
        em,
    ))
}
// Drawing fixtures need a foreground; metric-only fixtures do not paint text.
fn drawing_run_properties(family: &str, em: f64) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_all(
        Typeface::from_name(family),
        em,
        None,
        Some(Brushes::black()),
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ))
}
fn paragraph(
    default: Rc<dyn TextRunProperties>,
    mode: TextLineMetricsMode,
    bounds: TextLineBounds,
    wrapping: TextWrapping,
) -> Rc<dyn TextParagraphProperties> {
    let result = GenericTextParagraphProperties::with_options(
        default,
        TextAlignment::Left,
        wrapping,
        f64::NAN,
        0.0,
    );
    result.set_line_metrics_mode(mode);
    result.set_text_line_bounds(bounds);
    Rc::new(result)
}
fn format(
    source: &dyn ITextSource,
    properties: &Rc<dyn TextParagraphProperties>,
) -> Rc<dyn TextLine> {
    <dyn TextFormatter>::current()
        .format_line(source, 0, f64::INFINITY, properties, None)
        .unwrap()
}
fn text(text: &str, properties: Rc<dyn TextRunProperties>) -> Rc<dyn TextRun> {
    Rc::new(TextCharacters::new(
        ReadOnlyMemory::from_str(text),
        properties,
    ))
}
fn fallback_scope() -> TextTestScope {
    TextTestScope::with_fonts(vec![
        TestFont::new("Family A")
            .with_ranges(&[(0x20, 0x7e)])
            .with_line_gap(100),
        TestFont::new("Family B")
            .with_ranges(&[(0x20, 0x7e)])
            .with_line_gap(500),
        TestFont::new("Fallback")
            .with_ranges(&[(0x3040, 0x30ff)])
            .with_line_gap(900),
    ])
}
fn two_family_source() -> ListTextSource {
    let buffer = ReadOnlyMemory::from_str("あい");
    ListTextSource::new(vec![
        Rc::new(TextCharacters::new(
            buffer.slice(0, 1),
            run_properties("Family A", 20.0),
        )),
        Rc::new(TextCharacters::new(
            buffer.slice(1, 1),
            run_properties("Family B", 20.0),
        )),
    ])
}

#[test]
fn font_family_line_metrics_full_and_trimmed_boxes_use_the_requested_regular_family() {
    let _scope = TextTestScope::new();
    for (bounds, baseline, height) in MODES {
        let metrics = FontFamilyLineMetrics::new(&FontFamily::new(DEFAULT_FAMILY), 20.0, bounds);
        close(metrics.baseline, baseline);
        close(metrics.height, height);
        let properties = run_properties(DEFAULT_FAMILY, 20.0);
        let line = format(
            &SingleBufferTextSource::new("Hxg", properties.clone()),
            &paragraph(
                properties,
                TextLineMetricsMode::FontFamily,
                bounds,
                TextWrapping::NoWrap,
            ),
        );
        close(line.baseline(), baseline);
        close(line.height(), height);
        let range = line.get_text_bounds(0, 3);
        assert_eq!(range.len(), 1);
        close(range[0].rectangle().height, height);
    }
}

#[test]
fn font_family_line_metrics_bounds_are_applied_per_run_before_aggregation() {
    let _scope = TextTestScope::with_fonts(vec![
        TestFont::new("Tall")
            .with_ranges(&[(0x20, 0x7e)])
            .with_vertical_metrics(900, -100)
            .with_line_gap(100)
            .with_cap_height(300),
        TestFont::new("Deep")
            .with_ranges(&[(0x20, 0x7e)])
            .with_vertical_metrics(500, -400)
            .with_line_gap(0)
            .with_cap_height(480),
    ]);
    let tall = run_properties("Tall", 10.0);
    let deep = run_properties("Deep", 10.0);
    let source = ListTextSource::new(vec![text("A", tall.clone()), text("B", deep)]);
    for (bounds, baseline, height) in [
        (TextLineBounds::Full, 9.0, 13.0),
        (TextLineBounds::TrimToCapHeight, 4.8, 8.8),
        (TextLineBounds::TrimToBaseline, 9.0, 9.0),
        (TextLineBounds::Tight, 4.8, 4.8),
    ] {
        let line = format(
            &source,
            &paragraph(
                tall.clone(),
                TextLineMetricsMode::FontFamily,
                bounds,
                TextWrapping::NoWrap,
            ),
        );
        close(line.baseline(), baseline);
        close(line.height(), height);
    }
}

#[test]
fn font_family_line_metrics_nonempty_runs_do_not_inherit_a_default_font_height_floor() {
    let _scope = TextTestScope::new();
    let default = run_properties(DEFAULT_FAMILY, 40.0);
    let source = SingleBufferTextSource::new("x", run_properties(DEFAULT_FAMILY, 10.0));
    let family = paragraph(
        default.clone(),
        TextLineMetricsMode::FontFamily,
        TextLineBounds::Full,
        TextWrapping::NoWrap,
    );
    let line = format(&source, &family);
    close(line.baseline(), 8.0);
    close(line.height(), 11.0);
    let empty = TextFormatterImpl::create_empty_text_line(0, 100.0, &family);
    close(empty.baseline(), 32.0);
    close(empty.height(), 44.0);
    let legacy = format(
        &source,
        &paragraph(
            default,
            TextLineMetricsMode::GlyphTypeface,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    close(legacy.baseline(), 34.0);
    close(legacy.height(), 44.0);
}

#[test]
fn font_family_line_metrics_empty_lines_and_break_only_runs_use_default_family() {
    let _scope = TextTestScope::new();
    for (bounds, baseline, height) in MODES {
        let properties = paragraph(
            run_properties(DEFAULT_FAMILY, 20.0),
            TextLineMetricsMode::FontFamily,
            bounds,
            TextWrapping::NoWrap,
        );
        let empty = TextFormatterImpl::create_empty_text_line(0, 100.0, &properties);
        let source = SingleBufferTextSource::new("\r\n", run_properties(DEFAULT_FAMILY, 60.0));
        let break_only = format(&source, &properties);
        for line in [&*empty as &dyn TextLine, &*break_only] {
            close(line.baseline(), baseline);
            close(line.height(), height);
        }
        assert_eq!(break_only.length(), 2);
        assert_eq!(break_only.new_line_length(), 2);
    }
}

#[test]
fn font_family_line_metrics_glyph_fallback_preserves_the_requested_family_and_glyph_font() {
    let _scope = TextTestScope::with_fonts(vec![
        TestFont::new("Requested")
            .with_ranges(&[(0x20, 0x7e)])
            .with_line_gap(100),
        TestFont::new("Fallback")
            .with_ranges(&[(0x3040, 0x30ff)])
            .with_vertical_metrics(1000, -500)
            .with_line_gap(700)
            .with_cap_height(850),
    ]);
    for (bounds, baseline, height) in MODES {
        let properties = run_properties("Requested", 20.0);
        let source = SingleBufferTextSource::new("あい", properties.clone());
        let line = format(
            &source,
            &paragraph(
                properties,
                TextLineMetricsMode::FontFamily,
                bounds,
                TextWrapping::NoWrap,
            ),
        );
        close(line.baseline(), baseline);
        close(line.height(), height);
        let runs = line.text_runs();
        let shaped = runs[0].downcast_ref::<ShapedTextRun>().unwrap();
        assert_eq!(
            shaped.run_properties().requested_font_family().name(),
            "Requested"
        );
        assert_eq!(
            shaped.run_properties().typeface().font_family().name(),
            "Fallback"
        );
        close(shaped.text_metrics().line_height, 44.0);
        close(shaped.baseline(), 27.0);
        assert_eq!(line_text(&*line), "あい");
    }
}

#[test]
fn font_family_line_metrics_composite_family_uses_first_available_face_before_character_fallback() {
    let _scope = fallback_scope();
    let properties = run_properties("Unavailable, Family A, Fallback", 20.0);
    let line = format(
        &SingleBufferTextSource::new("あ", properties.clone()),
        &paragraph(
            properties,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    close(line.baseline(), 16.0);
    close(line.height(), 22.0);
    let runs = line.text_runs();
    let shaped = runs[0].downcast_ref::<ShapedTextRun>().unwrap();
    assert_eq!(
        shaped.run_properties().requested_font_family().name(),
        "Unavailable"
    );
    assert_eq!(
        shaped
            .run_properties()
            .cached_glyph_typeface()
            .family_name(),
        "Fallback"
    );
}

#[test]
fn font_family_line_metrics_different_requested_families_keep_ligature_boundaries() {
    let scope = fallback_scope();
    scope.shaper().add_ligature("あい");
    let source = two_family_source();
    let default = run_properties("Family A", 20.0);
    let legacy = format(
        &source,
        &paragraph(
            default.clone(),
            TextLineMetricsMode::GlyphTypeface,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert_eq!(scope.shaper().shape_calls(), vec!["あい"]);
    assert_eq!(legacy.text_runs().len(), 1);
    close(legacy.height(), 38.0);
    close(legacy.width(), 10.0);
    let family = format(
        &source,
        &paragraph(
            default,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert_eq!(scope.shaper().shape_calls(), vec!["あい", "あ", "い"]);
    assert_eq!(family.text_runs().len(), 2);
    close(family.baseline(), 16.0);
    close(family.height(), 30.0);
    close(family.width(), 20.0);
    assert_eq!(family.length(), 2);
}

#[test]
fn font_family_line_metrics_same_requested_family_keeps_existing_shaping_groups() {
    let scope = fallback_scope();
    scope.shaper().add_ligature("あい");
    let buffer = ReadOnlyMemory::from_str("あい");
    let first = run_properties("Family A", 20.0);
    let second: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
        Typeface::from_name("Family A"),
        20.0,
        None,
        Some(Brushes::white()),
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ));
    let source = ListTextSource::new(vec![
        Rc::new(TextCharacters::new(buffer.slice(0, 1), first.clone())),
        Rc::new(TextCharacters::new(buffer.slice(1, 1), second)),
    ]);
    let line = format(
        &source,
        &paragraph(
            first,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert_eq!(scope.shaper().shape_calls(), vec!["あい"]);
    assert_eq!(line.length(), 2);
    close(line.width(), 10.0);
    close(line.height(), 22.0);
}

#[test]
fn font_family_line_metrics_cache_reuses_each_policy_but_rejects_cross_policy_groups() {
    let scope = fallback_scope();
    scope.shaper().add_ligature("あい");
    let cache = TextRunCache::new();
    let source = two_family_source();
    let properties = Rc::new(GenericTextParagraphProperties::new(run_properties(
        "Family A", 20.0,
    )));
    let paragraph: Rc<dyn TextParagraphProperties> = properties.clone();
    let formatter = <dyn TextFormatter>::current();
    let initial = formatter
        .format_line_with_cache(&source, 0, 100.0, &paragraph, None, Some(&cache))
        .unwrap();
    assert_eq!(scope.shaper().shape_call_count(), 1);
    close(initial.height(), 38.0);
    properties.set_line_metrics_mode(TextLineMetricsMode::FontFamily);
    let family = formatter
        .format_line_with_cache(&source, 0, 100.0, &paragraph, None, Some(&cache))
        .unwrap();
    assert_eq!(scope.shaper().shape_call_count(), 3);
    close(family.height(), 30.0);
    let repeated = formatter
        .format_line_with_cache(&source, 0, 200.0, &paragraph, None, Some(&cache))
        .unwrap();
    assert_eq!(scope.shaper().shape_call_count(), 3);
    close(repeated.height(), 30.0);
    assert!(Rc::ptr_eq(&family.text_runs()[0], &repeated.text_runs()[0]));
    properties.set_line_metrics_mode(TextLineMetricsMode::GlyphTypeface);
    let restored = formatter
        .format_line_with_cache(&source, 0, 100.0, &paragraph, None, Some(&cache))
        .unwrap();
    assert_eq!(scope.shaper().shape_call_count(), 4);
    close(restored.height(), 38.0);
    close(restored.width(), 10.0);
    cache.dispose();
}

#[test]
fn font_family_line_metrics_embedded_objects_retain_full_ascent_and_descent_after_text_trimming() {
    let _scope = TextTestScope::new();
    let properties = run_properties(DEFAULT_FAMILY, 20.0);
    for (bounds, expected_height) in [
        (TextLineBounds::Full, 31.0),
        (TextLineBounds::TrimToCapHeight, 31.0),
        (TextLineBounds::TrimToBaseline, 30.0),
        (TextLineBounds::Tight, 30.0),
    ] {
        let object = Rc::new(CustomDrawableRun::with_size(
            Size::new(10.0, 30.0),
            25.0,
            properties.clone(),
        ));
        let source = ListTextSource::new(vec![text("H", properties.clone()), object.clone()]);
        let line = format(
            &source,
            &paragraph(
                properties.clone(),
                TextLineMetricsMode::FontFamily,
                bounds,
                TextWrapping::NoWrap,
            ),
        );
        close(line.baseline(), 25.0);
        close(line.height(), expected_height);
        close(TextLineImpl::get_baseline_offset(&*line, &*object), 0.0);
        let ranges = line.get_text_bounds(1, 1);
        assert_eq!(ranges.len(), 1);
        close(ranges[0].rectangle().height, expected_height);
    }
}

#[test]
fn font_family_line_metrics_different_object_baselines_contribute_the_complete_envelope() {
    let _scope = TextTestScope::new();
    let properties = run_properties(DEFAULT_FAMILY, 20.0);
    let first = Rc::new(CustomDrawableRun::with_size(
        Size::new(10.0, 30.0),
        25.0,
        properties.clone(),
    ));
    let second = Rc::new(CustomDrawableRun::with_size(
        Size::new(10.0, 40.0),
        5.0,
        properties.clone(),
    ));
    let source = ListTextSource::new(vec![first.clone(), second.clone()]);
    for (bounds, _, _) in MODES {
        let line = format(
            &source,
            &paragraph(
                properties.clone(),
                TextLineMetricsMode::FontFamily,
                bounds,
                TextWrapping::NoWrap,
            ),
        );
        close(line.baseline(), 25.0);
        close(line.height(), 60.0);
        close(TextLineImpl::get_baseline_offset(&*line, &*first), 0.0);
        close(TextLineImpl::get_baseline_offset(&*line, &*second), 20.0);
        close(line.extent(), 60.0);
        close(
            line.as_any()
                .downcast_ref::<TextLineImpl>()
                .unwrap()
                .ink_bounds()
                .height,
            60.0,
        );
    }
}

#[test]
fn font_family_line_metrics_draw_origin_and_ink_use_the_family_baseline_with_fallback_glyphs() {
    let _scope = fallback_scope();
    for (bounds, baseline, height) in MODES {
        let properties = drawing_run_properties("Family A", 20.0);
        let source = SingleBufferTextSource::new("あ", properties.clone());
        let line = format(
            &source,
            &paragraph(
                properties,
                TextLineMetricsMode::FontFamily,
                bounds,
                TextWrapping::NoWrap,
            ),
        );
        let mut sink = RecordingDrawingSink::new();
        line.draw(&mut sink, Point::new(3.0, 7.0));
        let calls = sink.calls();
        let origin = calls
            .iter()
            .find_map(|call| match call {
                DrawCall::GlyphRun { origin, .. } => Some(*origin),
                _ => None,
            })
            .unwrap();
        // Actual fallback glyphs retain baseline 25: ascent 16 + half of gap 18.
        close(origin.y, 7.0 + baseline - 25.0);
        close(origin.x, 3.0);
        assert_eq!(sink.transform_depth(), 0);
        let ink = line
            .as_any()
            .downcast_ref::<TextLineImpl>()
            .unwrap()
            .ink_bounds();
        close(line.overhang_after(), ink.bottom() - height);
    }
}

#[test]
fn font_family_line_metrics_explicit_height_and_spacing_keep_draw_and_ink_consistent() {
    let _scope = TextTestScope::new();
    for (requested, expected_baseline) in [(6.0, 16.0), (40.0, 25.0)] {
        let run = drawing_run_properties(DEFAULT_FAMILY, 20.0);
        let properties = GenericTextParagraphProperties::with_options(
            run.clone(),
            TextAlignment::Left,
            TextWrapping::NoWrap,
            requested,
            0.0,
        );
        properties.set_line_metrics_mode(TextLineMetricsMode::FontFamily);
        properties.set_line_spacing(3.0);
        let properties: Rc<dyn TextParagraphProperties> = Rc::new(properties);
        let line = format(&SingleBufferTextSource::new("H", run), &properties);
        close(line.height(), requested + 3.0);
        close(line.baseline(), expected_baseline);
        let ink = line
            .as_any()
            .downcast_ref::<TextLineImpl>()
            .unwrap()
            .ink_bounds();
        close(line.overhang_after(), ink.bottom() - line.height());
        let mut sink = RecordingDrawingSink::new();
        line.draw(&mut sink, Point::default());
        let calls = sink.calls();
        let origin = calls
            .iter()
            .find_map(|call| match call {
                DrawCall::GlyphRun { origin, .. } => Some(*origin),
                _ => None,
            })
            .unwrap();
        close(origin.y, expected_baseline - 17.0);
    }
}

struct CustomParagraph(Rc<dyn TextRunProperties>);
impl TextParagraphProperties for CustomParagraph {
    fn flow_direction(&self) -> FlowDirection {
        FlowDirection::LeftToRight
    }
    fn text_alignment(&self) -> TextAlignment {
        TextAlignment::Left
    }
    fn line_height(&self) -> f64 {
        f64::NAN
    }
    fn first_line_in_paragraph(&self) -> bool {
        true
    }
    fn default_text_run_properties(&self) -> &Rc<dyn TextRunProperties> {
        &self.0
    }
    fn text_wrapping(&self) -> TextWrapping {
        TextWrapping::NoWrap
    }
    fn indent(&self) -> f64 {
        0.0
    }
}
#[test]
fn font_family_line_metrics_custom_and_generic_defaults_preserve_legacy_boxes_and_copy_policy() {
    let _scope = TextTestScope::new();
    let run = run_properties(DEFAULT_FAMILY, 20.0);
    let custom: Rc<dyn TextParagraphProperties> = Rc::new(CustomParagraph(run.clone()));
    assert_eq!(
        custom.line_metrics_mode(),
        TextLineMetricsMode::GlyphTypeface
    );
    let copy = GenericTextParagraphProperties::from_properties(&*custom);
    assert_eq!(copy.line_metrics_mode(), TextLineMetricsMode::GlyphTypeface);
    let line = format(&SingleBufferTextSource::new("H", run.clone()), &custom);
    close(line.baseline(), 17.0);
    close(line.height(), 22.0);
    copy.set_line_metrics_mode(TextLineMetricsMode::FontFamily);
    copy.set_text_line_bounds(TextLineBounds::TrimToCapHeight);
    let family_copy = GenericTextParagraphProperties::from_properties(&copy);
    assert_eq!(
        family_copy.line_metrics_mode(),
        TextLineMetricsMode::FontFamily
    );
    assert_eq!(
        family_copy.text_line_bounds(),
        TextLineBounds::TrimToCapHeight
    );
    let properties: Rc<dyn TextParagraphProperties> = Rc::new(family_copy);
    let line = format(&SingleBufferTextSource::new("H", run), &properties);
    close(line.baseline(), 11.2);
    close(line.height(), 17.2);
}
