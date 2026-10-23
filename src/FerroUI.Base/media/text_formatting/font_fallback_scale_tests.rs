//! Controlled range mappings through the shared formatter. These tests prove
//! scale transport and fallback behavior; they do not assert that a production
//! platform font provider currently supplies a non-unit scale.

use super::testing::{
    line_text, ListTextSource, SingleBufferTextSource, TestFont, TestPlatformTypeface,
    TextTestScope,
};
use super::{
    GenericTextParagraphProperties, GenericTextRunProperties, ITextDrawingSink, ITextSource,
    ShapedTextRun, TextCharacters, TextFormatter, TextLine, TextLineMetricsMode,
    TextParagraphProperties, TextRunCache, TextRunProperties,
};
use crate::media::font_fallback_mapping::FontFallbackMappingResult;
use crate::media::{
    BaselineAlignment, Brushes, CharacterHit, FlowDirection, FontFallback, FontFallbackMapping,
    FontFamily, FontFeatureCollection, FontManager, FontManagerOptions, FontSimulations, GlyphRun,
    GlyphTypeface, IBrush, IPen, TextAlignment, TextDecoration, TextDecorationCollection,
    TextDecorationLocation, TextDecorationUnit, TextLineBounds, TextWrapping, Typeface,
    UnicodeRange,
};
use crate::platform::IFontFallbackMapper;
use crate::utilities::{CultureInfo, ReadOnlyMemory};
use crate::{FerroLocator, Matrix, Point, Rect};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

type Map = dyn Fn(&[u16], &Typeface, Option<&CultureInfo>) -> Option<FontFallbackMapping>;

struct Mapper {
    calls: RefCell<Vec<Vec<u16>>>,
    map: Box<Map>,
}

impl Mapper {
    fn new(
        map: impl Fn(&[u16], &Typeface, Option<&CultureInfo>) -> Option<FontFallbackMapping> + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            calls: RefCell::new(Vec::new()),
            map: Box::new(map),
        })
    }

    fn calls(&self) -> Vec<String> {
        self.calls
            .borrow()
            .iter()
            .map(|text| String::from_utf16_lossy(text))
            .collect()
    }
}

impl IFontFallbackMapper for Mapper {
    fn try_map_characters(
        &self,
        text: &[u16],
        typeface: &Typeface,
        culture: Option<&CultureInfo>,
    ) -> Option<FontFallbackMapping> {
        self.calls.borrow_mut().push(text.to_vec());
        (self.map)(text, typeface, culture)
    }
}

fn fonts() -> Vec<TestFont> {
    vec![
        TestFont::new("Requested").with_ranges(&[(0x20, 0x7e), (0x300, 0x36f), (0x590, 0x6ff)]),
        TestFont::new("Legacy Fallback").with_ranges(&[(0x3000, 0x30ff), (0x1f600, 0x1f64f)]),
    ]
}

fn glyph(font: TestFont) -> Rc<GlyphTypeface> {
    GlyphTypeface::new(
        Rc::new(TestPlatformTypeface::new(font)),
        FontSimulations::None,
    )
    .unwrap()
}

fn mapped_face() -> Rc<GlyphTypeface> {
    glyph(
        TestFont::new("Mapped Face")
            .with_ranges(&[
                (0x20, 0x7e),
                (0x300, 0x36f),
                (0x590, 0x6ff),
                (0x3000, 0x30ff),
                (0x1f600, 0x1f64f),
            ])
            .with_advance(750)
            .with_vertical_metrics(1000, -500)
            .with_line_gap(700)
            .with_cap_height(850),
    )
}

fn run_properties() -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::from_name("Requested"),
        20.0,
    ))
}

fn paragraph(
    properties: Rc<dyn TextRunProperties>,
    mode: TextLineMetricsMode,
    bounds: TextLineBounds,
    wrapping: TextWrapping,
) -> Rc<GenericTextParagraphProperties> {
    let paragraph = Rc::new(GenericTextParagraphProperties::with_options(
        properties,
        TextAlignment::Left,
        wrapping,
        f64::NAN,
        0.0,
    ));
    paragraph.set_line_metrics_mode(mode);
    paragraph.set_text_line_bounds(bounds);
    paragraph
}

fn format(
    source: &dyn ITextSource,
    properties: Rc<GenericTextParagraphProperties>,
) -> Rc<dyn TextLine> {
    let properties: Rc<dyn TextParagraphProperties> = properties;
    <dyn TextFormatter>::current()
        .format_line(source, 0, 1000.0, &properties, None)
        .unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}

fn scales(line: &dyn TextLine) -> Vec<f64> {
    line.text_runs()
        .iter()
        .filter_map(|run| {
            run.downcast_ref::<ShapedTextRun>()
                .map(|run| run.run_properties().font_scale())
        })
        .collect()
}

#[test]
fn font_fallback_scale_mapping_requires_positive_finite_scale_and_nonempty_prefix() {
    let face = mapped_face();
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(FontFallbackMapping::new(face.clone(), 1, scale).is_none());
    }
    assert!(FontFallbackMapping::new(face.clone(), 0, 1.0).is_none());
    for scale in [0.5, 1.0, 2.0] {
        let mapping = FontFallbackMapping::new(face.clone(), 2, scale).unwrap();
        assert_eq!(mapping.mapped_length(), 2);
        assert_eq!(mapping.scale(), scale);
        assert!(Rc::ptr_eq(mapping.glyph_typeface(), &face));
    }
}

#[test]
fn font_fallback_scale_effective_em_reaches_glyphs_and_each_requested_family_bounds_mode() {
    for scale in [0.5, 2.0] {
        let face = mapped_face();
        let selected = face.clone();
        let mapper = Mapper::new(move |text, requested, _| {
            assert_eq!(requested.font_family().name(), "Requested");
            FontFallbackMapping::new(selected.clone(), text.len(), scale)
        });
        let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper);
        for (bounds, baseline, height) in [
            (TextLineBounds::Full, 16.0, 22.0),
            (TextLineBounds::TrimToCapHeight, 11.2, 17.2),
            (TextLineBounds::TrimToBaseline, 16.0, 16.0),
            (TextLineBounds::Tight, 11.2, 11.2),
        ] {
            let source_properties = run_properties();
            let source = SingleBufferTextSource::new("あい", source_properties.clone());
            let line = format(
                &source,
                paragraph(
                    source_properties.clone(),
                    TextLineMetricsMode::FontFamily,
                    bounds,
                    TextWrapping::NoWrap,
                ),
            );
            close(line.baseline(), baseline * scale);
            close(line.height(), height * scale);
            close(line.width(), 30.0 * scale);
            let runs = line.text_runs();
            let shaped = runs[0].downcast_ref::<ShapedTextRun>().unwrap();
            assert!(Rc::ptr_eq(
                &shaped.run_properties().cached_glyph_typeface(),
                &face
            ));
            assert_eq!(
                shaped.run_properties().requested_font_family().name(),
                "Requested"
            );
            close(source_properties.font_rendering_em_size(), 20.0);
            close(source_properties.font_scale(), 1.0);
            close(shaped.run_properties().font_rendering_em_size(), 20.0);
            close(
                shaped.run_properties().effective_font_rendering_em_size(),
                20.0 * scale,
            );
            close(
                shaped.shaped_buffer().font_rendering_em_size(),
                20.0 * scale,
            );
            close(shaped.glyph_run().font_rendering_em_size(), 20.0 * scale);
            close(shaped.text_metrics().line_height, 44.0 * scale);
            close(
                line.get_text_bounds(0, 2)[0].rectangle().height,
                height * scale,
            );
            assert_eq!(line_text(&*line), "あい");
        }
    }
}

#[test]
fn font_fallback_scale_default_formatter_never_queries_optional_mapper() {
    let mapper = Mapper::new(|_, _, _| panic!("the legacy formatter must not query range mapping"));
    let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
    let properties = run_properties();
    let source = SingleBufferTextSource::new("ab", properties.clone());
    let line = format(
        &source,
        paragraph(
            properties,
            TextLineMetricsMode::GlyphTypeface,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    close(line.width(), 20.0);
    close(line.height(), 22.0);
    close(line.baseline(), 17.0);
    assert_eq!(scales(&*line), [1.0]);
    assert!(mapper.calls().is_empty());
}

#[test]
fn font_fallback_scale_default_family_placeholder_resolves_before_provider_mapping() {
    let face = mapped_face();
    let mapper = Mapper::new(move |text, requested, _| {
        assert_eq!(requested.font_family().name(), "Requested");
        FontFallbackMapping::new(face.clone(), text.len(), 0.5)
    });
    let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
    let properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::default_typeface(),
        20.0,
    ));
    let line = format(
        &SingleBufferTextSource::new("ab", properties.clone()),
        paragraph(
            properties.clone(),
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert_eq!(mapper.calls(), ["ab"]);
    assert_eq!(
        properties.requested_font_family().name(),
        FontFamily::DEFAULT_FONT_FAMILY_NAME
    );
    close(line.height(), 11.0);
    close(line.width(), 15.0);
}

#[test]
fn font_fallback_scale_absent_provider_keeps_legacy_matching_at_unit_scale() {
    let _scope = TextTestScope::with_fonts(fonts());
    let properties = run_properties();
    let source = SingleBufferTextSource::new("あい", properties.clone());
    let line = format(
        &source,
        paragraph(
            properties,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    close(line.width(), 20.0);
    close(line.height(), 22.0);
    assert_eq!(scales(&*line), [1.0]);
    let runs = line.text_runs();
    assert_eq!(
        runs[0]
            .properties()
            .unwrap()
            .cached_glyph_typeface()
            .family_name(),
        "Legacy Fallback"
    );
}

#[test]
fn font_fallback_scale_different_scales_split_shaping_even_with_the_same_face() {
    let face = mapped_face();
    let mapper = Mapper::new(move |text, _, _| {
        FontFallbackMapping::new(
            face.clone(),
            1,
            if text[0] == b'a' as u16 { 0.5 } else { 2.0 },
        )
    });
    let scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
    scope.shaper().add_ligature("ab");
    let properties = run_properties();
    let line = format(
        &SingleBufferTextSource::new("ab", properties.clone()),
        paragraph(
            properties,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert_eq!(mapper.calls(), ["ab", "b"]);
    assert_eq!(scope.shaper().shape_calls(), ["a", "b"]);
    assert_eq!(scales(&*line), [0.5, 2.0]);
    close(line.width(), 37.5);
    close(line.height(), 44.0);
}

#[test]
fn font_fallback_scale_identical_face_and_scale_keep_contiguous_ligature_grouping() {
    let face = mapped_face();
    let mapper =
        Mapper::new(move |text, _, _| FontFallbackMapping::new(face.clone(), text.len(), 2.0));
    let scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper);
    scope.shaper().add_ligature("ab");
    let memory = ReadOnlyMemory::from_str("ab");
    let properties = run_properties();
    let source = ListTextSource::new(vec![
        Rc::new(TextCharacters::new(memory.slice(0, 1), properties.clone())),
        Rc::new(TextCharacters::new(memory.slice(1, 1), properties.clone())),
    ]);
    let line = format(
        &source,
        paragraph(
            properties,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert_eq!(scope.shaper().shape_calls(), ["ab"]);
    close(line.width(), 30.0);
    assert_eq!(line.length(), 2);
}

#[test]
fn font_fallback_scale_distinct_actual_faces_with_equal_metadata_do_not_merge() {
    let first = glyph(
        TestFont::new("Same Name")
            .with_ranges(&[(0x20, 0x7e)])
            .with_advance(500),
    );
    let second = glyph(
        TestFont::new("Same Name")
            .with_ranges(&[(0x20, 0x7e)])
            .with_advance(750),
    );
    let a = first.clone();
    let b = second.clone();
    let mapper = Mapper::new(move |text, _, _| {
        FontFallbackMapping::new(
            if text[0] == b'a' as u16 {
                a.clone()
            } else {
                b.clone()
            },
            1,
            1.0,
        )
    });
    let scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper);
    scope.shaper().add_ligature("ab");
    let properties = run_properties();
    let line = format(
        &SingleBufferTextSource::new("ab", properties.clone()),
        paragraph(
            properties,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert_eq!(scope.shaper().shape_calls(), ["a", "b"]);
    close(line.width(), 25.0);
    let runs = line.text_runs();
    assert!(Rc::ptr_eq(
        &runs[0].properties().unwrap().cached_glyph_typeface(),
        &first
    ));
    assert!(Rc::ptr_eq(
        &runs[1].properties().unwrap().cached_glyph_typeface(),
        &second
    ));
}

#[test]
fn font_fallback_scale_invalid_prefixes_preserve_whole_graphemes_and_later_mappings() {
    for (text, invalid_length) in [("😀b", 1), ("a\u{301}b", 1), ("ab", 99)] {
        let face = mapped_face();
        let mapper = Mapper::new(move |offered, _, _| {
            FontFallbackMapping::new(
                face.clone(),
                if offered[0] == b'b' as u16 {
                    1
                } else {
                    invalid_length
                },
                2.0,
            )
        });
        let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
        let properties = run_properties();
        let line = format(
            &SingleBufferTextSource::new(text, properties.clone()),
            paragraph(
                properties,
                TextLineMetricsMode::FontFamily,
                TextLineBounds::Full,
                TextWrapping::NoWrap,
            ),
        );
        assert_eq!(line_text(&*line), text);
        assert_eq!(line.length(), text.encode_utf16().count() as i32);
        assert_eq!(mapper.calls(), [text, "b"]);
        assert_eq!(scales(&*line), [1.0, 2.0]);
        let first_length = text.encode_utf16().count() as i32 - 1;
        let runs = line.text_runs();
        assert_eq!(runs[0].length(), first_length);
        assert_eq!(runs[1].length(), 1);
    }
}

#[test]
fn font_fallback_scale_declined_uncovered_and_overflowing_mappings_make_bounded_progress() {
    for failure in 0..3 {
        let face = mapped_face();
        let empty = glyph(TestFont::new("Uncovered"));
        let mapper = Mapper::new(move |text, _, _| {
            if text[0] == b'b' as u16 {
                return FontFallbackMapping::new(face.clone(), 1, 2.0);
            }
            match failure {
                0 => None,
                1 => FontFallbackMapping::new(empty.clone(), 1, 2.0),
                _ => FontFallbackMapping::new(face.clone(), 1, f64::MAX),
            }
        });
        let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
        let properties = run_properties();
        let line = format(
            &SingleBufferTextSource::new("ab", properties.clone()),
            paragraph(
                properties,
                TextLineMetricsMode::FontFamily,
                TextLineBounds::Full,
                TextWrapping::NoWrap,
            ),
        );
        assert_eq!(mapper.calls(), ["ab", "b"]);
        assert_eq!(line_text(&*line), "ab");
        assert_eq!(scales(&*line), [1.0, 2.0]);
        close(line.width(), 40.0);
    }
}

#[test]
fn font_fallback_scale_rtl_queries_logical_prefixes_and_keeps_utf16_range_geometry() {
    let face = mapped_face();
    let mapper = Mapper::new(move |text, _, _| {
        FontFallbackMapping::new(
            face.clone(),
            1,
            if text[0] == 'א' as u16 { 0.5 } else { 2.0 },
        )
    });
    let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
    let properties = run_properties();
    let paragraph = paragraph(
        properties.clone(),
        TextLineMetricsMode::FontFamily,
        TextLineBounds::Full,
        TextWrapping::NoWrap,
    );
    paragraph.set_flow_direction(FlowDirection::RightToLeft);
    let line = format(&SingleBufferTextSource::new("אב", properties), paragraph);
    assert_eq!(mapper.calls(), ["אב", "ב"]);
    assert_eq!(line.length(), 2);
    close(line.get_text_bounds(0, 1)[0].rectangle().width, 7.5);
    close(line.get_text_bounds(1, 1)[0].rectangle().width, 30.0);
    assert!(
        line.get_distance_from_character_hit(CharacterHit::new(0))
            > line.get_distance_from_character_hit(CharacterHit::new(2))
    );
    for run in line.text_runs().iter() {
        let shaped = run.downcast_ref::<ShapedTextRun>().unwrap();
        assert_eq!(shaped.bidi_level() % 2, 1);
    }
}

#[test]
fn font_fallback_scale_nul_and_crlf_keep_original_utf16_break_lengths() {
    let face = mapped_face();
    let mapper = Mapper::new(move |text, _, _| {
        assert_ne!(text[0], 0);
        FontFallbackMapping::new(face.clone(), text.len(), 2.0)
    });
    let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
    let properties = run_properties();
    let source = SingleBufferTextSource::new("\0A\r\nB", properties.clone());
    let paragraph: Rc<dyn TextParagraphProperties> = paragraph(
        properties,
        TextLineMetricsMode::FontFamily,
        TextLineBounds::Full,
        TextWrapping::NoWrap,
    );
    let formatter = <dyn TextFormatter>::current();
    let first = formatter
        .format_line(&source, 0, 1000.0, &paragraph, None)
        .unwrap();
    assert_eq!(first.length(), 4);
    assert_eq!(first.new_line_length(), 2);
    assert_eq!(line_text(&*first), "\u{2060}A\r\n");
    let next = formatter
        .format_line(&source, 4, 1000.0, &paragraph, first.text_line_break().as_ref())
        .unwrap();
    assert_eq!(next.length(), 1);
    assert_eq!(line_text(&*next), "B");
    assert_eq!(scales(&*next), [2.0]);
    assert!(mapper.calls().iter().all(|text| !text.contains('\0')));
}

#[test]
fn font_fallback_scale_cached_and_forked_wrapped_runs_preserve_scale_without_remapping() {
    let face = mapped_face();
    let mapper =
        Mapper::new(move |text, _, _| FontFallbackMapping::new(face.clone(), text.len(), 2.0));
    let scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
    let properties = run_properties();
    let source = SingleBufferTextSource::new("abcd", properties.clone());
    let paragraph: Rc<dyn TextParagraphProperties> = paragraph(
        properties,
        TextLineMetricsMode::FontFamily,
        TextLineBounds::Full,
        TextWrapping::Wrap,
    );
    let cache = TextRunCache::new();
    let formatter = <dyn TextFormatter>::current();
    let first = formatter
        .format_line_with_cache(&source, 0, 30.0, &paragraph, None, Some(&cache))
        .unwrap();
    assert_eq!(first.length(), 1);
    let calls = mapper.calls();
    let shaped_calls = scope.shaper().shape_call_count();
    let wider = formatter
        .format_line_with_cache(&source, 0, 60.0, &paragraph, None, Some(&cache))
        .unwrap();
    assert_eq!(wider.length(), 2);
    close(wider.width(), 60.0);
    let fork = first.text_line_break().unwrap().fork();
    let second = formatter
        .format_line(&source, 1, 30.0, &paragraph, Some(&fork))
        .unwrap();
    assert_eq!(line_text(&*second), "b");
    close(second.height(), 44.0);
    assert_eq!(scales(&*second), [2.0]);
    assert_eq!(mapper.calls(), calls);
    assert_eq!(scope.shaper().shape_call_count(), shaped_calls);
    assert!(first
        .text_line_break()
        .unwrap()
        .remaining_runs
        .borrow()
        .is_some());
    cache.dispose();
}

#[derive(Default)]
struct DecorationSink {
    glyph_em: Vec<f64>,
    thickness: Vec<f64>,
}
impl ITextDrawingSink for DecorationSink {
    fn draw_glyph_run(&mut self, _: Option<&Rc<dyn IBrush>>, run: &Rc<GlyphRun>) {
        self.glyph_em.push(run.font_rendering_em_size());
    }
    fn draw_rectangle(&mut self, _: Option<&Rc<dyn IBrush>>, _: Option<&Rc<dyn IPen>>, _: Rect) {}
    fn draw_line(&mut self, pen: &Rc<dyn IPen>, _: Point, _: Point) {
        self.thickness.push(pen.thickness());
    }
    fn push_transform(&mut self, _: Matrix) {}
    fn pop_transform(&mut self) {}
}

#[test]
fn font_fallback_scale_decorations_letter_spacing_and_copied_properties_apply_scale_once() {
    let _dispatcher = crate::threading::Dispatcher::unit_test_scope();
    let face = mapped_face();
    let chosen = face.clone();
    let mapper = Mapper::new(move |text, _, culture| {
        assert_eq!(culture.unwrap().name(), "ja-JP");
        FontFallbackMapping::new(chosen.clone(), text.len(), 2.0)
    });
    let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper);
    let decorations = TextDecorationCollection::new();
    let underline = TextDecoration::new();
    underline.set_location(TextDecorationLocation::Strikethrough);
    underline.set_stroke_thickness_unit(TextDecorationUnit::FontRenderingEmSize);
    underline.set_stroke_thickness(0.1);
    decorations.add(underline);
    let features = FontFeatureCollection::parse("kern=0,ss01=1").unwrap();
    let properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
        Typeface::from_name("Requested"),
        20.0,
        Some(decorations.clone()),
        Some(Brushes::black()),
        None,
        BaselineAlignment::Baseline,
        Some(CultureInfo::get_culture_info("ja-JP")),
        Some(features.clone()),
    ));
    let paragraph = Rc::new(GenericTextParagraphProperties::with_options(
        properties.clone(),
        TextAlignment::Left,
        TextWrapping::NoWrap,
        f64::NAN,
        3.0,
    ));
    paragraph.set_line_metrics_mode(TextLineMetricsMode::FontFamily);
    let line = format(
        &SingleBufferTextSource::new("ab", properties.clone()),
        paragraph,
    );
    close(line.width(), 72.0); // Two 30px glyphs plus two scaled 6px advances.
    let runs = line.text_runs();
    let shaped = runs[0].downcast_ref::<ShapedTextRun>().unwrap();
    let mapped = shaped.run_properties();
    assert!(mapped.font_features() == Some(&features));
    assert!(mapped.text_decorations() == Some(&decorations));
    assert_eq!(mapped.culture_info(), properties.culture_info());
    let remapped = mapped.with_font_mapping(&FontFallbackMapping::new(face, 1, 0.5).unwrap());
    close(remapped.font_rendering_em_size(), 20.0);
    close(remapped.effective_font_rendering_em_size(), 10.0);
    let changed_face = mapped.with_typeface(&Typeface::from_name("Legacy Fallback"));
    close(changed_face.effective_font_rendering_em_size(), 40.0);
    let mut sink = DecorationSink::default();
    line.draw(&mut sink, Point::new(0.0, 0.0));
    assert_eq!(sink.glyph_em, [40.0]);
    assert_eq!(sink.thickness, [4.0]);
    close(properties.font_scale(), 1.0);
    close(properties.font_rendering_em_size(), 20.0);
}

#[test]
fn font_fallback_scale_configured_fallbacks_and_aliases_retain_existing_precedence() {
    for alias in [false, true] {
        let configuration = FerroLocator::enter_scope();
        let options = if alias {
            FontManagerOptions {
                font_family_mappings: Some(HashMap::from([(
                    "Alias".to_owned(),
                    FontFamily::new("Legacy Fallback"),
                )])),
                ..Default::default()
            }
        } else {
            FontManagerOptions {
                font_fallbacks: Some(vec![FontFallback {
                    font_family: FontFamily::new("Legacy Fallback"),
                    unicode_range: UnicodeRange::default_range(),
                }]),
                ..Default::default()
            }
        };
        FerroLocator::current_mutable()
            .bind::<FontManagerOptions>()
            .to_constant(Rc::new(options));
        {
            let mapper =
                Mapper::new(|_, _, _| panic!("configured family resolution must retain priority"));
            let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
            let properties: Rc<dyn TextRunProperties> =
                Rc::new(GenericTextRunProperties::with_font_size(
                    Typeface::from_name(if alias { "Alias" } else { "Requested" }),
                    20.0,
                ));
            let line = format(
                &SingleBufferTextSource::new("あ", properties.clone()),
                paragraph(
                    properties,
                    TextLineMetricsMode::FontFamily,
                    TextLineBounds::Full,
                    TextWrapping::NoWrap,
                ),
            );
            assert_eq!(scales(&*line), [1.0]);
            assert_eq!(
                line.text_runs()[0]
                    .properties()
                    .unwrap()
                    .cached_glyph_typeface()
                    .family_name(),
                "Legacy Fallback"
            );
            assert!(mapper.calls().is_empty());
        }
        configuration.dispose();
    }
}

#[test]
fn font_fallback_scale_keyed_embedded_and_composite_families_do_not_bypass_collection_rules() {
    let mapper = Mapper::new(|_, _, _| panic!("a system mapper cannot resolve keyed collections"));
    let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
    let manager = FontManager::current();
    for family in [
        "fonts:Custom#Requested",
        "resm:Example.Font.ttf#Requested",
        "Requested, Legacy Fallback",
    ] {
        let typeface = Typeface::from_name(family);
        assert!(typeface.font_family().key().is_some());
        assert!(matches!(
            manager.map_characters(&[0x3042], &typeface, None),
            FontFallbackMappingResult::Unavailable
        ));
    }
    let properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::from_name("Requested, Legacy Fallback"),
        20.0,
    ));
    let line = format(
        &SingleBufferTextSource::new("あ", properties.clone()),
        paragraph(
            properties,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert_eq!(scales(&*line), [1.0]);
    assert_eq!(
        line.text_runs()[0]
            .properties()
            .unwrap()
            .cached_glyph_typeface()
            .family_name(),
        "Legacy Fallback"
    );
    assert!(mapper.calls().is_empty());
}

#[test]
fn font_fallback_scale_provider_reentry_can_query_fonts_and_format_an_independent_line() {
    let face = mapped_face();
    let entered = Rc::new(Cell::new(false));
    let observed = entered.clone();
    let mapper = Mapper::new(move |text, _, _| {
        assert!(FontManager::current()
            .try_get_glyph_typeface(&Typeface::from_name("Requested"))
            .is_some());
        if !observed.replace(true) {
            let properties = run_properties();
            let nested = format(
                &SingleBufferTextSource::new("nested", properties.clone()),
                paragraph(
                    properties,
                    TextLineMetricsMode::GlyphTypeface,
                    TextLineBounds::Full,
                    TextWrapping::NoWrap,
                ),
            );
            close(nested.width(), 60.0);
            assert_eq!(scales(&*nested), [1.0]);
        }
        FontFallbackMapping::new(face.clone(), text.len(), 2.0)
    });
    let _scope = TextTestScope::with_fonts_and_fallback_mapper(fonts(), mapper.clone());
    let properties = run_properties();
    let line = format(
        &SingleBufferTextSource::new("ab", properties.clone()),
        paragraph(
            properties,
            TextLineMetricsMode::FontFamily,
            TextLineBounds::Full,
            TextWrapping::NoWrap,
        ),
    );
    assert!(entered.get());
    assert_eq!(mapper.calls(), ["ab"]);
    close(line.width(), 60.0);
    close(line.height(), 44.0);
}
