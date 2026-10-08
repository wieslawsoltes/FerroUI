//! Port of upstream's `Media/TextFormatting/TextLineTests.cs` of the Skia
//! unit tests.
//!
//! Upstream's `TextTestHelper.GetStartCharIndex` (internal to the headless
//! platform, `HeadlessPlatformStubs.cs`) and the `ListTextSource` of
//! `TextFormatterTests.cs` are ported here as local helpers. Upstream's
//! `Assert.Equal(double, double, precision)` is [`assert_equal_precision`].

use super::SingleBufferTextSource;
use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::text_formatting::{
    DrawableTextRun, GenericTextParagraphProperties, GenericTextRunProperties, ITextDrawingSink, ITextSource,
    ShapedTextRun, TextBounds, TextCharacters, TextEndOfParagraph, TextFormatter, TextFormatterImpl, TextLine,
    TextMetrics, TextParagraphProperties, TextRun, TextShaper, TextShaperOptions,
};
use ferroui_base::media::{
    CharacterHit, FlowDirection, FontFamily, TextAlignment, TextCollapsingCreateInfo, TextPathSegmentTrimming,
    TextTrimming, TextWrapping, Typeface,
};
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory};
use ferroui_base::{Point, Rect, Size};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::any::Any;
use std::rc::Rc;

const S_MULTI_LINE_TEXT: &str = "012345678\r\r0123456789";

/// C# `string.Length`: the length in UTF-16 code units.
fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// xUnit's `Assert.Equal(double expected, double actual, int precision)`:
/// both values rounded to `precision` decimal places (`Math.Round`, to even).
fn assert_equal_precision(expected: f64, actual: f64, precision: i32) {
    let factor = 10f64.powi(precision);
    let expected_rounded = (expected * factor).round_ties_even() / factor;
    let actual_rounded = (actual * factor).round_ties_even() / factor;

    assert_eq!(
        expected_rounded, actual_rounded,
        "Values differ at precision {precision}: expected {expected}, actual {actual}"
    );
}

fn default_paragraph(default_properties: &Rc<GenericTextRunProperties>) -> Rc<dyn TextParagraphProperties> {
    Rc::new(GenericTextParagraphProperties::new(default_properties.clone()))
}

#[allow(clippy::too_many_arguments)]
fn paragraph(
    flow_direction: FlowDirection,
    text_alignment: TextAlignment,
    first_line_in_paragraph: bool,
    always_collapsible: bool,
    default_properties: &Rc<GenericTextRunProperties>,
    text_wrapping: TextWrapping,
    line_height: f64,
    indent: f64,
    letter_spacing: f64,
) -> Rc<dyn TextParagraphProperties> {
    Rc::new(GenericTextParagraphProperties::with_all(
        flow_direction,
        text_alignment,
        first_line_in_paragraph,
        always_collapsible,
        default_properties.clone(),
        text_wrapping,
        line_height,
        indent,
        letter_spacing,
    ))
}

fn shaped(run: &Rc<dyn TextRun>) -> &ShapedTextRun {
    run.downcast_ref::<ShapedTextRun>().expect("a ShapedTextRun")
}

/// Upstream's `TextTestHelper.GetStartCharIndex` (`HeadlessPlatformStubs.cs`):
/// the start of the text in the string it is a slice of.
fn get_start_char_index(text: &ReadOnlyMemory<u16>) -> i32 {
    text.offset_in_owner() as i32
}

fn same_run(a: &Rc<dyn TextRun>, b: &Rc<dyn TextRun>) -> bool {
    std::ptr::eq(Rc::as_ptr(a) as *const (), Rc::as_ptr(b) as *const ())
}

fn sum_of_widths(text_bounds: &[TextBounds]) -> f64 {
    text_bounds.iter().map(|x| x.rectangle().width).sum()
}

#[test]
fn should_get_first_character_hit() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new(S_MULTI_LINE_TEXT, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let mut current_index = 0;

    while current_index < utf16_len(S_MULTI_LINE_TEXT) {
        let text_line = formatter
            .format_line(&text_source, current_index, f64::INFINITY, &default_paragraph(&default_properties), None)
            .expect("a text line");

        let first_character_hit = text_line.get_previous_caret_character_hit(CharacterHit::new(i32::MIN));

        assert_eq!(text_line.first_text_source_index(), first_character_hit.first_character_index());

        current_index += text_line.length();
    }
}

#[test]
fn should_get_last_character_hit() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new(S_MULTI_LINE_TEXT, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let mut current_index = 0;

    while current_index < utf16_len(S_MULTI_LINE_TEXT) {
        let text_line = formatter
            .format_line(&text_source, current_index, f64::INFINITY, &default_paragraph(&default_properties), None)
            .expect("a text line");

        let last_character_hit = text_line.get_next_caret_character_hit(CharacterHit::new(i32::MAX));

        assert_eq!(
            text_line.first_text_source_index() + text_line.length(),
            last_character_hit.first_character_index() + last_character_hit.trailing_length()
        );

        current_index += text_line.length();
    }
}

/// The glyph clusters of the line in logical order (runs ordered by the
/// start of their text, the clusters of a right to left run reversed).
fn logical_clusters(text_line: &dyn TextLine) -> Vec<i32> {
    let text_runs = text_line.text_runs();

    let mut ordered: Vec<&Rc<dyn TextRun>> = text_runs.iter().collect();

    ordered.sort_by_key(|x| get_start_char_index(&x.text()));

    let mut clusters = Vec::new();

    for text_run in ordered {
        let shaped_run = shaped(text_run);
        let run_offset = get_start_char_index(&shaped_run.text());

        let mut run_clusters: Vec<i32> =
            shaped_run.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster + run_offset).collect();

        if !shaped_run.shaped_buffer().is_left_to_right() {
            run_clusters.reverse();
        }

        clusters.extend(run_clusters);
    }

    clusters
}

#[test]
fn should_get_next_caret_character_hit_bidi() {
    let text = "אבג 1 ABC";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let clusters = logical_clusters(&*text_line);

    let mut next_character_hit = CharacterHit::with_trailing_length(0, clusters[1] - clusters[0]);

    for &cluster in &clusters {
        assert_eq!(cluster, next_character_hit.first_character_index());

        next_character_hit = text_line.get_next_caret_character_hit(next_character_hit);
    }

    let last_character_hit = next_character_hit;

    next_character_hit = text_line.get_next_caret_character_hit(last_character_hit);

    assert_eq!(last_character_hit.first_character_index(), next_character_hit.first_character_index());

    assert_eq!(last_character_hit.trailing_length(), next_character_hit.trailing_length());
}

#[test]
fn should_get_previous_caret_character_hit_bidi() {
    let text = "אבג 1 ABC";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut clusters = logical_clusters(&*text_line);

    clusters.reverse();

    let mut next_character_hit = CharacterHit::new(utf16_len(text) - 1);

    for &cluster in &clusters {
        let current_caret_index = next_character_hit.first_character_index() + next_character_hit.trailing_length();

        assert_eq!(cluster, current_caret_index);

        next_character_hit = text_line.get_previous_caret_character_hit(next_character_hit);
    }

    let last_character_hit = next_character_hit;

    next_character_hit = text_line.get_previous_caret_character_hit(last_character_hit);

    assert_eq!(last_character_hit.first_character_index(), next_character_hit.first_character_index());

    assert_eq!(last_character_hit.trailing_length(), next_character_hit.trailing_length());
}

#[test]
fn should_get_next_caret_character_hit() {
    for text in ["𐐷𐐷𐐷𐐷𐐷", "01234567🎉\n", "𐐷1234"] {
        let _scope = start();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

        let formatter = TextFormatterImpl::new();

        let text_line = formatter
            .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
            .expect("a text line");

        let clusters = build_glyph_clusters(&*text_line);

        let mut next_character_hit = CharacterHit::new(0);

        for &expected_cluster in &clusters {
            let actual_cluster = next_character_hit.first_character_index() + next_character_hit.trailing_length();

            assert_eq!(expected_cluster, actual_cluster, "{text:?}");

            next_character_hit = text_line.get_next_caret_character_hit(next_character_hit);
        }

        let mut last_character_hit = next_character_hit;

        next_character_hit = text_line.get_next_caret_character_hit(last_character_hit);

        assert_eq!(last_character_hit.first_character_index(), next_character_hit.first_character_index(), "{text:?}");

        assert_eq!(last_character_hit.trailing_length(), next_character_hit.trailing_length(), "{text:?}");

        next_character_hit = CharacterHit::with_trailing_length(0, clusters[1] - clusters[0]);

        for &cluster in &clusters {
            assert_eq!(cluster, next_character_hit.first_character_index(), "{text:?}");

            next_character_hit = text_line.get_next_caret_character_hit(next_character_hit);
        }

        last_character_hit = next_character_hit;

        next_character_hit = text_line.get_next_caret_character_hit(last_character_hit);

        assert_eq!(last_character_hit.first_character_index(), next_character_hit.first_character_index(), "{text:?}");

        assert_eq!(last_character_hit.trailing_length(), next_character_hit.trailing_length(), "{text:?}");
    }
}

#[test]
fn should_get_previous_caret_character_hit() {
    for text in ["𐐷𐐷𐐷𐐷𐐷", "01234567🎉\n", "𐐷1234"] {
        let _scope = start();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

        let formatter = TextFormatterImpl::new();

        let text_line = formatter
            .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
            .expect("a text line");

        let clusters: Vec<i32> = text_line
            .text_runs()
            .iter()
            .map(shaped)
            .flat_map(|run| {
                let run_offset = get_start_char_index(&run.text());

                run.shaped_buffer()
                    .glyph_infos()
                    .iter()
                    .map(|glyph| glyph.glyph_cluster + run_offset)
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut previous_character_hit = CharacterHit::new(utf16_len(text));

        for i in (0..clusters.len()).rev() {
            previous_character_hit = text_line.get_previous_caret_character_hit(previous_character_hit);

            assert_eq!(
                clusters[i],
                previous_character_hit.first_character_index() + previous_character_hit.trailing_length(),
                "{text:?}"
            );
        }

        let first_character_hit = previous_character_hit;

        previous_character_hit = text_line.get_previous_caret_character_hit(first_character_hit);

        assert_eq!(
            first_character_hit.first_character_index(),
            previous_character_hit.first_character_index(),
            "{text:?}"
        );

        assert_eq!(0, previous_character_hit.trailing_length(), "{text:?}");

        let last = clusters[clusters.len() - 1];

        previous_character_hit = CharacterHit::with_trailing_length(last, utf16_len(text) - last);

        for i in (1..clusters.len()).rev() {
            previous_character_hit = text_line.get_previous_caret_character_hit(previous_character_hit);

            assert_eq!(
                clusters[i],
                previous_character_hit.first_character_index() + previous_character_hit.trailing_length(),
                "{text:?}"
            );
        }
    }
}

#[test]
fn should_get_distance_from_character_hit() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new(S_MULTI_LINE_TEXT, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut current_distance = 0.0;

    for run in text_line.text_runs().iter() {
        let text_run = shaped(run);

        let glyph_run = text_run.glyph_run();

        let glyph_infos = glyph_run.glyph_infos();

        for glyph_info in glyph_infos.borrow().iter() {
            let cluster = glyph_info.glyph_cluster;

            let advance = glyph_info.glyph_advance;

            let distance = text_line.get_distance_from_character_hit(CharacterHit::new(cluster));

            assert_eq!(current_distance, distance);

            current_distance += advance;
        }
    }

    let actual_distance = text_line.get_distance_from_character_hit(CharacterHit::new(utf16_len(S_MULTI_LINE_TEXT)));

    assert_eq!(current_distance, actual_distance);
}

#[test]
fn should_get_character_hit_from_distance() {
    for text in [
        // LeftToRight
        "ABC012345",
        // RightToLeft
        "זה כיף סתם לשמוע איך תנצח קרפד עץ טוב בגן",
    ] {
        let _scope = start();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

        let formatter = TextFormatterImpl::new();

        let text_line = formatter
            .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
            .expect("a text line");

        let is_right_to_left = is_right_to_left(&*text_line);
        let rects = build_rects(&*text_line);
        let glyph_clusters = build_glyph_clusters(&*text_line);

        for i in 0..rects.len() {
            let cluster = glyph_clusters[i];
            let rect = rects[i];

            let character_hit = text_line.get_character_hit_from_distance(rect.left());

            assert_eq!(
                if is_right_to_left { cluster + 1 } else { cluster },
                character_hit.first_character_index() + character_hit.trailing_length(),
                "{text:?} at {i}"
            );
        }
    }
}

/// Upstream's `CollapsingData`.
fn collapsing_data() -> Vec<(&'static str, f64, Rc<dyn TextTrimming>, &'static str)> {
    vec![
        ("01234 01234 01234", 120.0, <dyn TextTrimming>::prefix_character_ellipsis(), "01234 01\u{2026}4 01234"),
        ("01234 01234", 58.0, <dyn TextTrimming>::character_ellipsis(), "01234 0\u{2026}"),
        ("01234 01234", 58.0, <dyn TextTrimming>::word_ellipsis(), "01234\u{2026}"),
        ("01234", 9.0, <dyn TextTrimming>::character_ellipsis(), "\u{2026}"),
        ("01234", 2.0, <dyn TextTrimming>::character_ellipsis(), ""),
    ]
}

#[test]
fn should_collapse_line() {
    for (text, width, trimming, expected) in collapsing_data() {
        let _scope = start();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

        let formatter = TextFormatterImpl::new();

        let text_line = formatter
            .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
            .expect("a text line");

        assert!(!text_line.has_collapsed());

        let collapsing_properties = trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
            width,
            default_properties.clone(),
            FlowDirection::LeftToRight,
        ));

        let collapsed_line = text_line.collapse(&[Some(collapsing_properties)]);

        assert!(collapsed_line.has_collapsed(), "{text:?} at {width} with {trimming}");

        let trimmed_text: Vec<u16> =
            collapsed_line.text_runs().iter().flat_map(|x| x.text().to_string_lossy().encode_utf16().collect::<Vec<_>>()).collect();

        let expected: Vec<u16> = expected.encode_utf16().collect();

        assert_eq!(expected.len(), trimmed_text.len(), "{text:?} at {width} with {trimming}");

        for i in 0..expected.len() {
            assert_eq!(expected[i], trimmed_text[i], "{text:?} at {width} with {trimming}");
        }
    }
}

#[test]
fn should_get_next_character_hit_for_drawable_runs() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = DrawableRunTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    assert_eq!(4, text_line.text_runs().len());

    let mut current_hit = text_line.get_next_caret_character_hit(CharacterHit::new(0));

    assert_eq!(1, current_hit.first_character_index());
    assert_eq!(0, current_hit.trailing_length());

    current_hit = text_line.get_next_caret_character_hit(current_hit);

    assert_eq!(2, current_hit.first_character_index());
    assert_eq!(0, current_hit.trailing_length());

    current_hit = text_line.get_next_caret_character_hit(current_hit);

    assert_eq!(3, current_hit.first_character_index());
    assert_eq!(0, current_hit.trailing_length());

    current_hit = text_line.get_next_caret_character_hit(current_hit);

    assert_eq!(4, current_hit.first_character_index());
    assert_eq!(0, current_hit.trailing_length());
}

#[test]
fn should_get_previous_character_hit_for_drawable_runs() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = DrawableRunTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    assert_eq!(4, text_line.text_runs().len());

    let mut current_hit = text_line.get_previous_caret_character_hit(CharacterHit::with_trailing_length(3, 1));

    assert_eq!(3, current_hit.first_character_index());
    assert_eq!(0, current_hit.trailing_length());

    current_hit = text_line.get_previous_caret_character_hit(current_hit);

    assert_eq!(2, current_hit.first_character_index());
    assert_eq!(0, current_hit.trailing_length());

    current_hit = text_line.get_previous_caret_character_hit(current_hit);

    assert_eq!(1, current_hit.first_character_index());
    assert_eq!(0, current_hit.trailing_length());

    current_hit = text_line.get_previous_caret_character_hit(current_hit);

    assert_eq!(0, current_hit.first_character_index());
    assert_eq!(0, current_hit.trailing_length());
}

#[test]
fn should_get_character_hit_from_distance_for_drawable_runs() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = DrawableRunTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut character_hit = text_line.get_character_hit_from_distance(50.0);

    assert_eq!(5, character_hit.first_character_index());
    assert_eq!(1, character_hit.trailing_length());

    character_hit = text_line.get_character_hit_from_distance(32.0);

    assert_eq!(3, character_hit.first_character_index());
    assert_eq!(0, character_hit.trailing_length());
}

#[test]
fn should_get_distance_from_character_hit_drawable_runs() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = DrawableRunTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut distance = text_line.get_distance_from_character_hit(CharacterHit::new(1));

    assert_eq!(14.0, distance);

    distance = text_line.get_distance_from_character_hit(CharacterHit::new(2));

    assert!(distance > 14.0);
}

#[test]
fn should_get_distance_from_character_hit_mixed_text_buffer() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = MixedTextBufferTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut distance = text_line.get_distance_from_character_hit(CharacterHit::new(10));

    assert_eq!(72.01171875, distance);

    distance = text_line.get_distance_from_character_hit(CharacterHit::new(20));

    assert_eq!(144.0234375, distance);

    distance = text_line.get_distance_from_character_hit(CharacterHit::new(30));

    assert_eq!(216.03515625, distance);

    distance = text_line.get_distance_from_character_hit(CharacterHit::new(40));

    assert_eq!(text_line.width_including_trailing_whitespace(), distance);
}

#[test]
fn should_get_text_bounds_from_mixed_text_buffer() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = MixedTextBufferTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut text_bounds = text_line.get_text_bounds(0, 10);

    assert_eq!(1, text_bounds.len());

    assert_eq!(72.01171875, text_bounds[0].rectangle().width);

    text_bounds = text_line.get_text_bounds(0, 20);

    assert_eq!(1, text_bounds.len());

    assert_eq!(144.0234375, sum_of_widths(&text_bounds));

    text_bounds = text_line.get_text_bounds(0, 30);

    assert_eq!(1, text_bounds.len());

    assert_eq!(216.03515625, sum_of_widths(&text_bounds));

    text_bounds = text_line.get_text_bounds(0, 40);

    assert_eq!(1, text_bounds.len());

    assert_eq!(text_line.width_including_trailing_whitespace(), sum_of_widths(&text_bounds));
}

/// C# `Environment.NewLine`.
#[cfg(windows)]
const NEW_LINE: &str = "\r\n";
#[cfg(not(windows))]
const NEW_LINE: &str = "\n";

#[test]
fn should_get_text_bounds_for_line_break() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = SingleBufferTextSource::new(NEW_LINE, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(0, utf16_len(NEW_LINE));

    assert_eq!(1, text_bounds.len());

    assert_eq!(1, text_bounds[0].text_run_bounds().len());

    assert_eq!(utf16_len(NEW_LINE), text_bounds[0].text_run_bounds()[0].length());
}

#[test]
fn should_get_text_range() {
    let text = "שדגככעיחדגכAישדגשדגחייטYDASYWIWחיחלדשSAטויליHUHIUHUIDWKLאא'ק'קחליק/'וקןגגגלךשף'/קפוכדגכשדגשיח'/קטאגשד";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_runs: Vec<Rc<dyn TextRun>> = text_line.text_runs().to_vec();

    let line_width = text_line.width_including_trailing_whitespace();

    let text_bounds = text_line.get_text_bounds(0, utf16_len(text));

    let mut last_bounds: Option<&TextBounds> = None;

    let run_bounds: Vec<_> = text_bounds.iter().flat_map(|x| x.text_run_bounds().iter()).collect();

    assert_eq!(text_runs.len(), run_bounds.len());

    for i in 0..text_runs.len() {
        let run = &text_runs[i];
        let bounds = run_bounds[i];

        assert_eq!(get_start_char_index(&shaped(run).text()), bounds.text_source_character_index());
        assert!(same_run(run, bounds.text_run()));
        assert_equal_precision(shaped(run).size().width, bounds.rectangle().width, 2);
    }

    for current_bounds in &text_bounds {
        if let Some(last_bounds) = last_bounds {
            assert_equal_precision(last_bounds.rectangle().right(), current_bounds.rectangle().left(), 2);
        }

        let sum_of_run_width: f64 = current_bounds.text_run_bounds().iter().map(|x| x.rectangle().width).sum();

        assert_equal_precision(sum_of_run_width, current_bounds.rectangle().width, 2);

        last_bounds = Some(current_bounds);
    }

    let sum_of_bounds_width = sum_of_widths(&text_bounds);

    assert_equal_precision(line_width, sum_of_bounds_width, 2);
}

#[test]
fn should_get_character_hit_for_distance_with_text_end_of_line() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new("Hello World", default_properties.clone(), true);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, 1000.0, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let character_hit = text_line.get_character_hit_from_distance(1000.0);

    assert_eq!(10, character_hit.first_character_index());
    assert_eq!(1, character_hit.trailing_length());
}

#[test]
fn should_get_next_caret_character_hit_from_mixed_text_buffer() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = MixedTextBufferTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut character_hit = text_line.get_next_caret_character_hit(CharacterHit::with_trailing_length(9, 1));

    assert_eq!(10, character_hit.first_character_index());

    assert_eq!(1, character_hit.trailing_length());

    character_hit = text_line.get_next_caret_character_hit(character_hit);

    assert_eq!(11, character_hit.first_character_index());

    assert_eq!(1, character_hit.trailing_length());

    character_hit = text_line.get_next_caret_character_hit(CharacterHit::with_trailing_length(19, 1));

    assert_eq!(20, character_hit.first_character_index());

    assert_eq!(1, character_hit.trailing_length());

    character_hit = text_line.get_next_caret_character_hit(CharacterHit::new(10));

    assert_eq!(11, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());

    character_hit = text_line.get_next_caret_character_hit(character_hit);

    assert_eq!(12, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());

    character_hit = text_line.get_next_caret_character_hit(CharacterHit::new(20));

    assert_eq!(21, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());
}

#[test]
fn should_get_previous_caret_character_hit_from_mixed_text_buffer() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = MixedTextBufferTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut character_hit = text_line.get_previous_caret_character_hit(CharacterHit::with_trailing_length(20, 1));

    assert_eq!(20, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());

    character_hit = text_line.get_previous_caret_character_hit(CharacterHit::with_trailing_length(10, 1));

    assert_eq!(10, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());

    character_hit = text_line.get_previous_caret_character_hit(character_hit);

    assert_eq!(9, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());

    character_hit = text_line.get_previous_caret_character_hit(CharacterHit::new(21));

    assert_eq!(20, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());

    character_hit = text_line.get_previous_caret_character_hit(CharacterHit::new(11));

    assert_eq!(10, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());

    character_hit = text_line.get_previous_caret_character_hit(character_hit);

    assert_eq!(9, character_hit.first_character_index());

    assert_eq!(0, character_hit.trailing_length());
}

#[test]
fn should_get_character_hit_from_distance_from_mixed_text_buffer() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = MixedTextBufferTextSource;

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 20, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let character_hit = text_line.get_character_hit_from_distance(f64::INFINITY);

    assert_eq!(40, character_hit.first_character_index() + character_hit.trailing_length());
}

#[test]
#[should_panic(expected = "textLength ('0') must be a non-zero value.")]
fn should_throw_argument_out_of_range_exception_for_zero_text_length() {
    let _scope = start();

    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = CustomTextBufferTextSource::new(vec![Rc::new(TextCharacters::from_str(
        "1234",
        default_properties.clone(),
    ))]);
    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    // Upstream: Assert.Throws<ArgumentOutOfRangeException>; the port panics.
    text_line.get_text_bounds(0, 0);
}

#[test]
fn should_get_text_bounds_for_negative_text_length() {
    let _scope = start();

    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = CustomTextBufferTextSource::new(vec![Rc::new(TextCharacters::from_str(
        "1234",
        default_properties.clone(),
    ))]);
    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(0, -1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(first_bounds.text_run_bounds().is_empty());

    assert_eq!(0.0, first_bounds.rectangle().width);

    assert_eq!(0.0, first_bounds.rectangle().left());
}

#[test]
fn should_get_text_bounds_for_exceeding_text_length() {
    let _scope = start();

    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = CustomTextBufferTextSource::new(vec![Rc::new(TextCharacters::from_str(
        "1234",
        default_properties.clone(),
    ))]);
    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(10, 1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(first_bounds.text_run_bounds().is_empty());

    assert_eq!(0.0, first_bounds.rectangle().width);

    assert_eq!(text_line.width_including_trailing_whitespace(), first_bounds.rectangle().right());
}

const MANROPE: &str = "resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia#Manrope";

#[test]
fn should_get_text_bounds_for_mixed_hidden_runs_with_ligature() {
    let _scope = start();

    let typeface = Typeface::new(FontFamily::parse(MANROPE).unwrap());

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = CustomTextBufferTextSource::new(vec![
        Rc::new(TextHidden::new(1)),
        Rc::new(TextCharacters::from_str("Authenti", default_properties.clone())),
        Rc::new(TextHidden::new(1)),
        Rc::new(TextHidden::new(1)),
        Rc::new(TextCharacters::from_str("ff", default_properties.clone())),
        Rc::new(TextHidden::new(1)),
        Rc::new(TextHidden::new(1)),
    ]);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(12, 1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(!first_bounds.text_run_bounds().is_empty());

    let first_run = &first_bounds.text_run_bounds()[0];

    assert_eq!(12, first_run.text_source_character_index());
}

#[test]
fn should_get_text_bounds_for_mixed_hidden_runs() {
    let _scope = start();

    let typeface = Typeface::new(FontFamily::parse(MANROPE).unwrap());

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = CustomTextBufferTextSource::new(vec![
        Rc::new(TextHidden::new(1)),
        Rc::new(TextCharacters::from_str("Authenti", default_properties.clone())),
        Rc::new(TextHidden::new(1)),
        Rc::new(TextHidden::new(1)),
        Rc::new(TextEndOfParagraph::with_length(1)),
    ]);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(8, 1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(!first_bounds.text_run_bounds().is_empty());

    let first_run = &first_bounds.text_run_bounds()[0];

    assert_eq!(8, first_run.text_source_character_index());
}

#[test]
#[cfg_attr(not(windows), ignore = "Windows font")]
fn should_get_text_bounds_within_cluster() {
    let _scope = start();

    let typeface = Typeface::from_name("Segoe UI Emoji");

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source =
        CustomTextBufferTextSource::new(vec![Rc::new(TextCharacters::from_str("🙈", default_properties.clone()))]);
    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut text_bounds = text_line.get_text_bounds(0, 1);

    assert!(!text_bounds.is_empty());

    let mut run_bounds = &text_bounds[0].text_run_bounds()[0];

    assert_eq!(0, run_bounds.text_source_character_index());

    text_bounds = text_line.get_text_bounds(1, 1);

    assert!(!text_bounds.is_empty());

    run_bounds = &text_bounds[0].text_run_bounds()[0];

    assert_eq!(1, run_bounds.text_source_character_index());

    let text_bounds = text_line.get_text_bounds(2, 1);

    assert!(!text_bounds.is_empty());

    assert!(text_bounds[0].text_run_bounds().is_empty());
}

#[test]
#[cfg_attr(not(windows), ignore = "Windows font")]
fn should_get_text_bounds_after_last_index() {
    let _scope = start();

    let typeface = Typeface::from_name("Segoe UI Emoji");

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source =
        CustomTextBufferTextSource::new(vec![Rc::new(TextCharacters::from_str("🙈", default_properties.clone()))]);
    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(2, 1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert_eq!(text_line.width(), first_bounds.rectangle().right());

    assert!(first_bounds.text_run_bounds().is_empty());
}

#[test]
fn should_get_run_bounds() {
    let _scope = start();

    let typeface = Typeface::new(FontFamily::parse(MANROPE).unwrap());
    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = CustomTextBufferTextSource::new(vec![
        Rc::new(TextCharacters::from_str("He", default_properties.clone())),
        Rc::new(TextCharacters::from_str("Wo", default_properties.clone())),
        Rc::new(TextCharacters::from_str("ff", default_properties.clone())),
    ]);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let mut text_bounds = text_line.get_text_bounds(1, 1);

    assert!(!text_bounds.is_empty());

    text_bounds = text_line.get_text_bounds(2, 1);

    assert!(!text_bounds.is_empty());

    text_bounds = text_line.get_text_bounds(4, 1);

    assert!(!text_bounds.is_empty());
}

#[test]
fn should_handle_new_line_in_rtl_text() {
    let _scope = start();

    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));

    let text_source = SingleBufferTextSource::new("test\r\n", default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::RightToLeft,
                TextAlignment::Right,
                true,
                true,
                &default_properties,
                TextWrapping::Wrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    assert_ne!(text_line.new_line_length(), 0);
}

#[test]
fn should_set_new_line_length_for_crlf_in_rtl_text() {
    for text in ["hello\r\nworld", "مرحباً\r\nبالعالم", "hello مرحباً\r\nworld بالعالم", "مرحباً hello\r\nبالعالم nworld"] {
        let _scope = start();

        let typeface = Typeface::default_typeface();
        let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

        let formatter = TextFormatterImpl::new();

        let text_line = formatter
            .format_line(
                &text_source,
                0,
                f64::INFINITY,
                &paragraph(
                    FlowDirection::RightToLeft,
                    TextAlignment::Right,
                    true,
                    true,
                    &default_properties,
                    TextWrapping::Wrap,
                    0.0,
                    0.0,
                    0.0,
                ),
                None,
            )
            .expect("a text line");

        assert_ne!(0, text_line.new_line_length(), "{text:?}");
    }
}

#[test]
fn should_get_text_bounds_with_trailing_zero_advance() {
    let df7_font = "resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia#DF7segHMI";

    let _scope = start();

    let typeface = Typeface::from_name(df7_font);
    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = SingleBufferTextSource::new("3,47-=?:#", default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(0, 2);

    assert!(!text_bounds.is_empty());

    let text_run_bounds = text_bounds.first().unwrap().text_run_bounds();

    assert!(!text_bounds.is_empty());

    let first = text_run_bounds.first().unwrap();

    assert_eq!(0, first.text_source_character_index());
    assert_eq!(2, first.length());
}

#[test]
fn should_get_in_cluster_backspace_hit() {
    let _scope = start();

    let typeface = Typeface::new(FontFamily::parse(MANROPE).unwrap());
    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = SingleBufferTextSource::new("ff", default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let backspace_hit = text_line.get_backspace_caret_character_hit(CharacterHit::with_trailing_length(1, 1));

    assert_eq!(1, backspace_hit.first_character_index());
}

struct TextHidden {
    length: i32,
}

impl TextHidden {
    fn new(length: i32) -> Self {
        Self { length }
    }
}

impl TextRun for TextHidden {
    fn length(&self) -> i32 {
        self.length
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

struct CustomTextBufferTextSource {
    text_runs: Vec<Rc<dyn TextRun>>,
}

impl CustomTextBufferTextSource {
    fn new(text_runs: Vec<Rc<dyn TextRun>>) -> Self {
        Self { text_runs }
    }
}

impl ITextSource for CustomTextBufferTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let mut pos = 0;

        for current_run in &self.text_runs {
            if pos + current_run.length() > text_source_index {
                return Some(current_run.clone());
            }

            pos += current_run.length();
        }

        None
    }
}

struct MixedTextBufferTextSource;

impl ITextSource for MixedTextBufferTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let text = match text_source_index {
            0 => "aaaaaaaaaa",
            10 => "bbbbbbbbbb",
            20 => "cccccccccc",
            30 => "dddddddddd",
            _ => return None,
        };

        Some(Rc::new(TextCharacters::from_str(
            text,
            Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
        )))
    }
}

struct DrawableRunTextSource;

impl DrawableRunTextSource {
    const TEXT: &'static str = "_A_A";
}

impl ITextSource for DrawableRunTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        match text_source_index {
            0 => Some(Rc::new(CustomDrawableRun)),
            1 => Some(Rc::new(TextCharacters::from_str(
                Self::TEXT,
                Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
            ))),
            5 => Some(Rc::new(CustomDrawableRun)),
            6 => Some(Rc::new(TextCharacters::from_str(
                Self::TEXT,
                Rc::new(GenericTextRunProperties::new(Typeface::default_typeface())),
            ))),
            _ => None,
        }
    }
}

struct CustomDrawableRun;

impl TextRun for CustomDrawableRun {
    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl DrawableTextRun for CustomDrawableRun {
    fn size(&self) -> Size {
        Size::new(14.0, 14.0)
    }

    fn baseline(&self) -> f64 {
        14.0
    }

    fn draw(&self, _drawing_context: &mut dyn ITextDrawingSink, _origin: Point) {}
}

fn is_right_to_left(text_line: &dyn TextLine) -> bool {
    text_line.text_runs().iter().any(|x| !shaped(x).shaped_buffer().is_left_to_right())
}

fn build_glyph_clusters(text_line: &dyn TextLine) -> Vec<i32> {
    let mut glyph_clusters = Vec::new();

    let shaped_text_runs: Vec<Rc<dyn TextRun>> = text_line.text_runs().to_vec();

    let mut last_cluster = -1;

    for text_run in shaped_text_runs.iter().map(shaped) {
        let run_offset = get_start_char_index(&text_run.text());
        let shaped_buffer = text_run.shaped_buffer();

        let current_clusters: Vec<i32> =
            shaped_buffer.glyph_infos().iter().map(|glyph| glyph.glyph_cluster + run_offset).collect();

        for current_cluster in current_clusters {
            if last_cluster == current_cluster {
                continue;
            }

            glyph_clusters.push(current_cluster);

            last_cluster = current_cluster;
        }
    }

    glyph_clusters
}

fn build_rects(text_line: &dyn TextLine) -> Vec<Rect> {
    let mut rects: Vec<Rect> = Vec::new();
    let height = text_line.height();

    let mut current_x = 0.0;

    let mut last_cluster = -1;

    let shaped_text_runs: Vec<Rc<dyn TextRun>> = text_line.text_runs().to_vec();

    for text_run in shaped_text_runs.iter().map(shaped) {
        // Glyph clusters are relative to the run's own text, so they only line up across a
        // multi-run line once the run's start is added - same as build_glyph_clusters.
        let run_offset = get_start_char_index(&text_run.text());
        let shaped_buffer = text_run.shaped_buffer();

        for index in 0..shaped_buffer.length() {
            let current_cluster = shaped_buffer.get(index).glyph_cluster + run_offset;

            let advance = shaped_buffer.get(index).glyph_advance;

            if last_cluster != current_cluster {
                rects.push(Rect::new(current_x, 0.0, advance, height));
            } else {
                // Another glyph of the cluster that produced the last rect: widen it.
                let rect = rects[rects.len() - 1];

                let last = rects.len() - 1;
                rects[last] = rect.with_width(rect.width + advance);
            }

            current_x += advance;

            last_cluster = current_cluster;
        }
    }

    rects
}

#[test]
fn should_get_text_bounds_mixed() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text = "0123";
    let shaper_option = TextShaperOptions::with_all(
        Typeface::default_typeface().glyph_typeface(),
        10.0,
        0,
        Some(CultureInfo::current_culture()),
        0.0,
        0.0,
        None,
    );

    let first_run: Rc<dyn TextRun> = Rc::new(ShapedTextRun::new(
        TextShaper::current().shape_str(text, &shaper_option),
        default_properties.clone(),
    ));

    let text_runs: Vec<Rc<dyn TextRun>> = vec![
        Rc::new(CustomDrawableRun),
        first_run.clone(),
        Rc::new(CustomDrawableRun),
        Rc::new(ShapedTextRun::new(TextShaper::current().shape_str(text, &shaper_option), default_properties.clone())),
        Rc::new(CustomDrawableRun),
        Rc::new(ShapedTextRun::new(TextShaper::current().shape_str(text, &shaper_option), default_properties.clone())),
    ];

    let text_source = FixedRunsTextSource::new(text_runs);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let first_run_width = shaped(&first_run).size().width;

    let mut text_bounds = text_line.get_text_bounds(0, text_line.length());

    assert_eq!(1, text_bounds.len());
    assert_eq!(text_line.width_including_trailing_whitespace(), sum_of_widths(&text_bounds));

    text_bounds = text_line.get_text_bounds(0, 1);

    assert_eq!(1, text_bounds.len());
    assert_eq!(14.0, text_bounds[0].rectangle().width);

    text_bounds = text_line.get_text_bounds(0, first_run.length() + 1);

    assert_eq!(1, text_bounds.len());
    assert_eq!(first_run_width + 14.0, sum_of_widths(&text_bounds));

    text_bounds = text_line.get_text_bounds(1, first_run.length());

    assert_eq!(1, text_bounds.len());
    assert_eq!(first_run_width, text_bounds[0].rectangle().width);

    text_bounds = text_line.get_text_bounds(0, 1 + first_run.length());

    assert_eq!(1, text_bounds.len());
    assert_eq!(first_run_width + 14.0, sum_of_widths(&text_bounds));
}

#[test]
fn should_get_text_bounds_bi_di_left_to_right() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text = "אאא AAA";
    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            200.0,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let mut text_bounds = text_line.get_text_bounds(0, 3);

    let first_run_width = shaped(&text_line.text_runs()[0]).size().width;

    assert_eq!(1, text_bounds.len());
    assert_eq!(first_run_width, sum_of_widths(&text_bounds));

    text_bounds = text_line.get_text_bounds(3, 4);

    let second_run_width = shaped(&text_line.text_runs()[1]).size().width;

    assert_eq!(1, text_bounds.len());
    assert_eq!(second_run_width, sum_of_widths(&text_bounds));

    text_bounds = text_line.get_text_bounds(0, 4);

    assert_eq!(2, text_bounds.len());

    assert_eq!(first_run_width, text_bounds[0].rectangle().width);

    assert_eq!(7.201171875, text_bounds[1].rectangle().width);

    assert_eq!(first_run_width, text_bounds[1].rectangle().left());

    text_bounds = text_line.get_text_bounds(0, utf16_len(text));

    assert_eq!(2, text_bounds.len());
    assert_eq!(text_line.width_including_trailing_whitespace(), sum_of_widths(&text_bounds));
}

#[test]
fn should_get_text_bounds_bi_di_right_to_left() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text = "אאא AAA";
    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            200.0,
            &paragraph(
                FlowDirection::RightToLeft,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    // Runs come in visual order: the Latin word sits leftmost, then the space, then the
    // Hebrew word. The space belongs to the primary font, so it is a run of its own.
    let (latin_run_width, space_run_width, hebrew_run_width) = {
        let text_runs = text_line.text_runs();

        (shaped(&text_runs[0]).size().width, shaped(&text_runs[1]).size().width, shaped(&text_runs[2]).size().width)
    };

    let hebrew_and_space_width = hebrew_run_width + space_run_width;

    let run_lengths = |text_bounds: &[TextBounds]| -> i32 {
        text_bounds.iter().map(|x| x.text_run_bounds().iter().map(|x| x.length()).sum::<i32>()).sum()
    };

    let mut text_bounds = text_line.get_text_bounds(0, 4);

    assert_eq!(1, text_bounds.len());
    assert_eq!(hebrew_and_space_width, sum_of_widths(&text_bounds));

    text_bounds = text_line.get_text_bounds(4, 3);

    assert_eq!(1, text_bounds.len());

    assert_eq!(3, text_bounds[0].text_run_bounds().iter().map(|x| x.length()).sum::<i32>());
    assert_eq!(latin_run_width, sum_of_widths(&text_bounds));

    text_bounds = text_line.get_text_bounds(0, 5);

    assert_eq!(2, text_bounds.len());
    assert_eq!(5, run_lengths(&text_bounds));

    assert_eq!(hebrew_and_space_width, text_bounds[1].rectangle().width);
    assert_eq!(7.201171875, text_bounds[0].rectangle().width);

    assert_equal_precision(text_line.start() + 7.201171875, text_bounds[0].rectangle().right(), 2);
    assert_equal_precision(text_line.start() + latin_run_width, text_bounds[1].rectangle().left(), 2);

    text_bounds = text_line.get_text_bounds(0, utf16_len(text));

    assert_eq!(2, text_bounds.len());
    assert_eq!(7, run_lengths(&text_bounds));
    assert_equal_precision(text_line.width_including_trailing_whitespace(), sum_of_widths(&text_bounds), 2);
}

#[test]
fn should_get_text_bounds_with_end_of_paragraph_right_to_left() {
    let text = "لوحة المفاتيح العربية";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), true);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(0, 1);

    assert_eq!(1, text_bounds.len());

    let first_bounds = text_bounds.first().unwrap();

    assert!(!first_bounds.text_run_bounds().is_empty());
}

#[test]
fn should_get_text_bounds_with_end_of_paragraph() {
    let text = "abc";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), true);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(3, 1);

    assert_eq!(1, text_bounds.len());

    let first_bounds = text_bounds.first().unwrap();

    assert!(!first_bounds.text_run_bounds().is_empty());
}

#[test]
fn should_get_text_bounds_not_infinite_loop() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let shaper_option = TextShaperOptions::with_all(
        Typeface::default_typeface().glyph_typeface(),
        10.0,
        0,
        Some(CultureInfo::current_culture()),
        0.0,
        0.0,
        None,
    );
    let shaper_option2 = TextShaperOptions::with_all(
        Typeface::default_typeface().glyph_typeface(),
        11.0,
        0,
        Some(CultureInfo::current_culture()),
        0.0,
        0.0,
        None,
    );

    let text_runs: Vec<Rc<dyn TextRun>> = vec![
        Rc::new(ShapedTextRun::new(TextShaper::current().shape_str("قرأ ", &shaper_option), default_properties.clone())),
        Rc::new(ShapedTextRun::new(
            TextShaper::current().shape_str("Wikipedia\u{2122}", &shaper_option),
            default_properties.clone(),
        )),
        Rc::new(ShapedTextRun::new(
            TextShaper::current().shape_str("\u{200e} ", &shaper_option2),
            default_properties.clone(),
        )),
        Rc::new(ShapedTextRun::new(
            TextShaper::current().shape_str("طوال اليوم", &shaper_option),
            default_properties.clone(),
        )),
        Rc::new(ShapedTextRun::new(TextShaper::current().shape_str(".", &shaper_option), default_properties.clone())),
    ];

    let text_source = FixedRunsTextSource::new(text_runs);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    text_line.get_text_bounds(4, 11);
}

#[test]
fn should_get_text_bounds_bidi() {
    let text = "אבגדה 12345 ABCDEF אבגדה";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), true);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let mut bounds = text_line.get_text_bounds(6, 1);

    assert_eq!(1, bounds.len());

    assert_eq!(0.0, bounds[0].rectangle().left());

    bounds = text_line.get_text_bounds(5, 1);

    assert_eq!(1, bounds.len());

    // Layout bounds depend on the order in which floating-point glyph
    // advances are summed inside ShapedBuffer; that order changed when
    // the cluster-width cache landed, so this value moved by one ULP
    // (36.005859374999993 -> 36.005859375). Both round to the same
    // sub-pixel position; use a tolerant compare to capture intent.
    assert_equal_precision(36.005859375, bounds[0].rectangle().left(), 5);

    bounds = text_line.get_text_bounds(0, 1);

    assert_eq!(1, bounds.len());

    // The space between the Hebrew word and the digits is drawn with the primary font
    // rather than the Hebrew fallback, which is 4.08 wider at this size, so everything
    // laid out after it sits that much further right.
    assert_eq!(75.247031249999992, bounds[0].rectangle().right());

    bounds = text_line.get_text_bounds(11, 1);

    assert_eq!(1, bounds.len());

    assert_eq!(75.247031249999992, bounds[0].rectangle().left());

    bounds = text_line.get_text_bounds(0, 25);

    assert_eq!(4, bounds.len());

    assert_eq!(text_line.width_including_trailing_whitespace(), bounds.last().unwrap().rectangle().right());
}

#[test]
fn should_get_text_bounds_bidi_2() {
    let text = "אבג ABC אבג 123";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), true);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let bounds = text_line.get_text_bounds(0, utf16_len(text));

    assert_eq!(4, bounds.len());

    let right = bounds.last().unwrap().rectangle().right();

    assert_eq!(text_line.width_including_trailing_whitespace(), right);
}

#[test]
fn should_get_previous_character_hit_non_trailing() {
    let text = "123.45.67.•";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), true);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let _character_hit = text_line.get_previous_caret_character_hit(CharacterHit::with_trailing_length(10, 1));
}

#[test]
fn should_ignore_null_terminator() {
    for (text, width) in [("\0", 0.0), ("\0\0\0", 0.0), ("\0A\0\0", 7.201171875), ("\0AA\0AA\0", 28.8046875)] {
        let _scope = start();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), true);

        let formatter = TextFormatterImpl::new();

        let text_line = formatter
            .format_line(
                &text_source,
                0,
                f64::INFINITY,
                &paragraph(
                    FlowDirection::LeftToRight,
                    TextAlignment::Left,
                    true,
                    true,
                    &default_properties,
                    TextWrapping::NoWrap,
                    0.0,
                    0.0,
                    0.0,
                ),
                None,
            )
            .expect("a text line");

        assert_eq!(width, text_line.width(), "{text:?}");
    }
}

#[test]
fn should_get_text_bounds_for_clustered_zero_width_characters() {
    let text = "\r\n";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = ListTextSource::new(vec![
        Rc::new(TextHidden::new(1)),
        Rc::new(TextCharacters::from_str(text, default_properties.clone())),
    ]);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(2, 1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(!first_bounds.text_run_bounds().is_empty());

    let first_run_bounds = &first_bounds.text_run_bounds()[0];

    assert_eq!(2, first_run_bounds.text_source_character_index());

    assert_eq!(1, first_run_bounds.length());
}

#[test]
#[cfg_attr(not(windows), ignore = "Values depend on the Skia platform backend")]
fn should_produce_overhang() {
    let symbols_font = "resm:FerroUI.Skia.UnitTests.Assets?assembly=ferroui-skia#Source Serif";

    for (text, leading, trailing, after) in
        [("y", -8.0, -1.304, -5.44), ("f", -12.0, -11.824, -4.44), ("a", 1.0, -0.232, -20.44)]
    {
        let _scope = start();

        let typeface = Typeface::new(FontFamily::parse(symbols_font).unwrap());

        let default_properties = Rc::new(GenericTextRunProperties::with_font_size(typeface, 64.0));

        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

        let formatter = TextFormatterImpl::new();

        let text_line = formatter
            .format_line(
                &text_source,
                0,
                f64::INFINITY,
                &paragraph(
                    FlowDirection::LeftToRight,
                    TextAlignment::Left,
                    true,
                    true,
                    &default_properties,
                    TextWrapping::NoWrap,
                    0.0,
                    0.0,
                    0.0,
                ),
                None,
            )
            .expect("a text line");

        assert_equal_precision(leading, text_line.overhang_leading(), 2);
        assert_equal_precision(trailing, text_line.overhang_trailing(), 2);
        assert_equal_precision(after, text_line.overhang_after(), 2);
    }
}

#[test]
fn should_get_text_bounds_for_multiple_text_runs() {
    let text = "Test👩🏽‍🚒";

    let _scope = start();

    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::with_font_size(typeface, 12.0));

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let result = text_line.get_text_bounds(0, 11);

    assert_eq!(1, result.len());

    let first_bounds = &result[0];

    assert!(!first_bounds.text_run_bounds().is_empty());

    assert_equal_precision(text_line.width_including_trailing_whitespace(), first_bounds.rectangle().width, 2);
}

#[test]
fn should_get_text_bounds_within_cluster_2() {
    let text = "Test👩🏽‍🚒";

    let _scope = start();

    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::with_font_size(typeface, 12.0));

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    let mut text_position = 0;

    while text_position < utf16_len(text) {
        let bounds = text_line.get_text_bounds(text_position, 1);

        assert_eq!(1, bounds.len(), "at {text_position}");

        let first_bounds = &bounds[0];

        assert_eq!(1, first_bounds.text_run_bounds().len(), "at {text_position}");

        let first_run_bounds = &first_bounds.text_run_bounds()[0];

        assert_eq!(text_position, first_run_bounds.text_source_character_index());

        let expected_distance = first_run_bounds.rectangle().left();

        let character_hit = CharacterHit::new(text_position);

        let distance = text_line.get_distance_from_character_hit(character_hit);

        assert_equal_precision(expected_distance, distance, 2);

        let next_character_hit = text_line.get_next_caret_character_hit(character_hit);

        let expected_next_position = text_position + first_run_bounds.length();

        let next_position = next_character_hit.first_character_index() + next_character_hit.trailing_length();

        assert_eq!(expected_next_position, next_position, "at {text_position}");

        let previous_character_hit = text_line.get_previous_caret_character_hit(next_character_hit);

        assert_eq!(character_hit, previous_character_hit, "at {text_position}");

        text_position += first_run_bounds.length();
    }
}

#[test]
fn should_get_text_bounds_with_mixed_runs_within_cluster() {
    let _scope = start();

    let manrope_font = MANROPE;

    let typeface = Typeface::from_name(manrope_font);

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface.clone()));
    let text = "Fotografin";
    let shaper_option = TextShaperOptions::new(typeface.glyph_typeface());

    let first_run: Rc<dyn TextRun> = Rc::new(ShapedTextRun::new(
        TextShaper::current().shape_str(text, &shaper_option),
        default_properties.clone(),
    ));

    let text_runs: Vec<Rc<dyn TextRun>> =
        vec![Rc::new(CustomDrawableRun), Rc::new(CustomDrawableRun), first_run, Rc::new(CustomDrawableRun)];

    let text_source = FixedRunsTextSource::new(text_runs);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(10, 1);

    assert_eq!(1, text_bounds.len());

    let first_bounds = &text_bounds[0];

    assert!(!first_bounds.text_run_bounds().is_empty());

    let first_run_bounds = &first_bounds.text_run_bounds()[0];

    assert_eq!(1, first_run_bounds.length());
}

#[test]
fn should_get_text_bounds_with_glue() {
    let _scope = start();

    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text = "a\u{202C}\u{202C}\u{202C}\u{202C}b";

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(1, 1);

    assert!(!text_bounds.is_empty());

    let first_text_bounds = &text_bounds[0];

    assert!(!first_text_bounds.text_run_bounds().is_empty());

    let first_run_bounds = &first_text_bounds.text_run_bounds()[0];

    assert_eq!(1, first_run_bounds.text_source_character_index());
    assert_eq!(1, first_run_bounds.length());
}

#[test]
fn should_get_text_bounds_tamil() {
    let text = "எடுத்துக்காட்டு வழி வினவல்";

    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), true);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &paragraph(
                FlowDirection::LeftToRight,
                TextAlignment::Left,
                true,
                true,
                &default_properties,
                TextWrapping::NoWrap,
                0.0,
                0.0,
                0.0,
            ),
            None,
        )
        .expect("a text line");

    assert!(!text_line.text_runs().is_empty());

    let first_run: Rc<dyn TextRun> = text_line.text_runs()[0].clone();

    let first_run = first_run.downcast_ref::<ShapedTextRun>().expect("a ShapedTextRun");

    let mut cluster_width = Vec::new();
    let mut distances = Vec::new();
    let mut clusters = Vec::new();
    let mut last_cluster = -1;
    let mut current_distance = 0.0;
    let mut current_advance = 0.0;

    for glyph_info in first_run.shaped_buffer().glyph_infos().iter() {
        if last_cluster != glyph_info.glyph_cluster {
            cluster_width.push(current_advance);
            distances.push(current_distance);
            clusters.push(glyph_info.glyph_cluster);

            current_advance = 0.0;
        }

        last_cluster = glyph_info.glyph_cluster;
        current_distance += glyph_info.glyph_advance;
        current_advance += glyph_info.glyph_advance;
    }

    cluster_width.remove(0);

    cluster_width.push(current_advance);

    for i in 6..clusters.len() {
        let cluster = clusters[i];
        let expected_distance = distances[i];
        let expected_width = cluster_width[i];

        let actual_distance = text_line.get_distance_from_character_hit(CharacterHit::new(cluster));

        assert_equal_precision(expected_distance, actual_distance, 2);

        let character_hit = text_line.get_character_hit_from_distance(expected_distance);

        let text_position = character_hit.first_character_index() + character_hit.trailing_length();

        assert_eq!(cluster, text_position, "at cluster {i}");

        let bounds = text_line.get_text_bounds(cluster, 1);

        assert!(!bounds.is_empty());

        let first_bounds = &bounds[0];

        assert!(!first_bounds.text_run_bounds().is_empty());

        let first_run_bounds = &first_bounds.text_run_bounds()[0];

        assert_eq!(cluster, first_run_bounds.text_source_character_index());

        let width = first_run_bounds.rectangle().width;

        assert_equal_precision(expected_width, width, 2);
    }
}

#[test]
fn should_get_text_bounds_trailing_zero_width() {
    let text = "dasdsad\r\n";

    let _scope = start();

    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface.clone()));
    let _shaper_option = TextShaperOptions::new(typeface.glyph_typeface());

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_bounds = text_line.get_text_bounds(7, 3);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(!first_bounds.text_run_bounds().is_empty());

    let first_run_bounds = &first_bounds.text_run_bounds()[0];

    assert_eq!(7, first_run_bounds.text_source_character_index());

    assert_eq!(2, first_run_bounds.length());
}

const INTER: &str = "resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia#Inter";

#[test]
fn should_add_half_line_gap_to_baseline() {
    let _scope = start();

    let typeface = Typeface::from_name(INTER);
    let default_properties = Rc::new(GenericTextRunProperties::new(typeface.clone()));

    let text_source = SingleBufferTextSource::new("F", default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let text_metrics = TextMetrics::new(&typeface.glyph_typeface(), 12.0);

    let expected_baseline = -text_metrics.ascent + text_metrics.line_gap / 2.0;

    assert_eq!(expected_baseline, text_line.baseline());
}

#[test]
fn should_clamp_baseline_when_line_height_is_smaller_than_natural() {
    let _scope = start();

    let typeface = Typeface::from_name(INTER);
    let default_properties = Rc::new(GenericTextRunProperties::new(typeface.clone()));

    let text_source = SingleBufferTextSource::new("F", default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_metrics = TextMetrics::new(&typeface.glyph_typeface(), 12.0);
    let natural = -text_metrics.ascent + text_metrics.descent + text_metrics.line_gap;

    let smaller_line_height = natural - 2.0;

    // Force a smaller line height than ascent+descent+lineGap
    let paragraph_props: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties.clone(),
        TextAlignment::Left,
        TextWrapping::NoWrap,
        smaller_line_height,
        0.0,
    ));

    let text_line =
        formatter.format_line(&text_source, 0, f64::INFINITY, &paragraph_props, None).expect("a text line");

    // In this case, baseline should equal -Ascent (lineGap ignored)
    let expected_baseline = -text_metrics.ascent;

    assert_eq!(expected_baseline, text_line.baseline());
    assert_eq!(paragraph_props.line_height(), text_line.height());
}

#[test]
fn should_distribute_extra_space_when_line_height_is_larger_than_natural() {
    let _scope = start();

    let typeface = Typeface::from_name(INTER);
    let default_properties = Rc::new(GenericTextRunProperties::new(typeface.clone()));

    let text_source = SingleBufferTextSource::new("F", default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_metrics = TextMetrics::new(&typeface.glyph_typeface(), 12.0);
    let natural = -text_metrics.ascent + text_metrics.descent + text_metrics.line_gap;

    let larger_line_height = natural + 50.0;

    let paragraph_props: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties.clone(),
        TextAlignment::Left,
        TextWrapping::NoWrap,
        larger_line_height,
        0.0,
    ));

    let text_line =
        formatter.format_line(&text_source, 0, f64::INFINITY, &paragraph_props, None).expect("a text line");

    // Extra space is distributed evenly above and below
    let extra = larger_line_height - (text_metrics.descent - text_metrics.ascent);
    let expected_baseline = -text_metrics.ascent + extra / 2.0;

    assert_equal_precision(expected_baseline, text_line.baseline(), 5);
    assert_equal_precision(larger_line_height, text_line.height(), 5);
}

#[test]
fn backspace_should_treat_crlf_as_a_unit() {
    let _scope = start();

    let typeface = Typeface::new(FontFamily::parse(MANROPE).unwrap());
    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));
    let text_source = SingleBufferTextSource::new("one\r\n", default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let backspace_hit = text_line.get_backspace_caret_character_hit(CharacterHit::new(5));

    assert_eq!(3, backspace_hit.first_character_index());
}

/// The text of `text` collapsed with a path segment trimming of `*` at `width`.
fn collapse_path_segment(text: &str, width: f64) -> String {
    let typeface = Typeface::default_typeface();

    let default_properties = Rc::new(GenericTextRunProperties::new(typeface));

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(&text_source, 0, f64::INFINITY, &default_paragraph(&default_properties), None)
        .expect("a text line");

    let trimming = TextPathSegmentTrimming::new("*");

    let collapsing_properties = trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
        width,
        default_properties.clone(),
        FlowDirection::LeftToRight,
    ));

    let collapsed_line = text_line.collapse(&[Some(collapsing_properties)]);

    extract_text_from_runs(&*collapsed_line)
}

#[test]
fn should_collapse_with_text_path_segment_trimming_without_path_segment() {
    let text = "foo";

    let _scope = start();

    let result = collapse_path_segment(text, 15.0);

    assert_eq!("*o", result);
}

#[test]
fn should_collapse_with_text_path_segment_trimming_no_space() {
    let text = "foo";

    let _scope = start();

    let result = collapse_path_segment(text, 8.0);

    assert_eq!("*", result);
}

#[test]
fn truncate_path_path_ending_with_slash_returns_non_empty() {
    for path in ["somedirectory\\", "somedirectory/"] {
        let _scope = start();

        let result = collapse_path_segment(path, 50.0);

        assert!(result.contains("ory"), "{path:?}: {result:?}");
    }
}

#[test]
fn should_collapse_with_ellipsis() {
    for path in ["directory\\file.txt", "directory/file.txt"] {
        let _scope = start();

        let result = collapse_path_segment(path, 8.0);

        assert_eq!("*", result, "{path:?}");
    }
}

#[test]
fn should_trim_path_at_the_end() {
    let text = "verylongdirectory\\file.txt";

    let _scope = start();

    let result = collapse_path_segment(text, 40.0);

    assert_eq!("*.txt", result);
}

pub fn extract_text_from_runs(text_line: &dyn TextLine) -> String {
    // Only extract text for ShapedTextRun instances.
    text_line
        .text_runs()
        .iter()
        .filter_map(|r| r.downcast_ref::<ShapedTextRun>())
        .map(|r| r.text().to_string_lossy())
        .collect()
}

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
        let mut current_position = 0;

        for text_run in &self.text_runs {
            if current_position == text_source_index {
                return Some(text_run.clone());
            }

            current_position += text_run.length();
        }

        None
    }
}

/// Upstream's `TextFormatterTests.ListTextSource` (`TextFormatterTests.cs`).
struct ListTextSource {
    runs: Vec<Rc<dyn TextRun>>,
}

impl ListTextSource {
    fn new(runs: Vec<Rc<dyn TextRun>>) -> Self {
        Self { runs }
    }
}

impl ITextSource for ListTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let mut off = 0;

        for run in &self.runs {
            if text_source_index >= off && text_source_index - off < run.length() {
                if run.length() == 1 {
                    return Some(run.clone());
                }

                let chars = run.downcast_ref::<TextCharacters>().expect("a TextCharacters run");

                return Some(Rc::new(TextCharacters::new(
                    chars.text().slice_from((text_source_index - off) as usize),
                    chars.properties().expect("text characters have properties").clone(),
                )));
            }

            off += run.length();
        }

        None
    }
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface()
            .with_render_interface(Rc::new(PlatformRenderInterface::new(None, None)))
            .with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    )
}
