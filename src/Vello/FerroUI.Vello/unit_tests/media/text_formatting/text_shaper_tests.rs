//! Port of upstream's `Media/TextFormatting/TextShaperTests.cs` of the Skia
//! unit tests.

use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::text_formatting::unicode::Codepoint;
use ferroui_base::media::text_formatting::{TextShaper, TextShaperOptions};
use ferroui_base::media::{FontFamily, FontManager, FontStretch, FontStyle, FontWeight, Typeface};
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

#[test]
fn should_form_clusters_for_break_pairs() {
    let _scope = start();

    let text = "\n\r\n";
    let options = TextShaperOptions::with_all(
        Typeface::default_typeface().glyph_typeface(),
        12.0,
        0,
        Some(CultureInfo::current_culture()),
        0.0,
        0.0,
        None,
    );
    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(text), &options);

    assert_eq!(shaped_buffer.length(), text.len());
    assert_eq!(shaped_buffer.length(), text.len());
    assert_eq!(0, shaped_buffer.get(0).glyph_cluster);
    assert_eq!(1, shaped_buffer.get(1).glyph_cluster);
    assert_eq!(1, shaped_buffer.get(2).glyph_cluster);
}

#[test]
fn should_apply_incremental_tab_width() {
    let _scope = start();

    let text = "012345\t";
    let options = TextShaperOptions::with_all(
        Typeface::default_typeface().glyph_typeface(),
        12.0,
        0,
        Some(CultureInfo::current_culture()),
        100.0,
        0.0,
        None,
    );
    let shaped_buffer = TextShaper::current().shape_text(&ReadOnlyMemory::from_str(text).slice_from(6), &options);

    assert_eq!(1, shaped_buffer.length());
    assert_eq!(100.0, shaped_buffer.get(0).glyph_advance);
}

#[test]
fn should_not_split_cluster() {
    let _scope = start();

    let typeface =
        Typeface::new(FontFamily::parse("resm:FerroUI.Vello.UnitTests.Fonts?assembly=ferroui-vello#Cascadia Code").unwrap());

    let buffer = TextShaper::current()
        .shape_text(&ReadOnlyMemory::from_str("a\"๊a"), &TextShaperOptions::new(typeface.glyph_typeface()));

    let split_result = buffer.split(1);

    assert!(split_result.first.is_some());
    assert_eq!(1, split_result.first.as_ref().unwrap().length());

    let buffer = split_result.second;

    assert!(buffer.is_some());
    let buffer = buffer.unwrap();

    //\"๊
    let split_result = buffer.split(1);

    assert!(split_result.first.is_some());
    assert_eq!(2, split_result.first.as_ref().unwrap().length());

    let buffer = split_result.second;

    assert!(buffer.is_some());
}

#[test]
fn should_not_split_right_to_left_cluster() {
    // Arabic letters carry their harakat in the same cluster, so the first cluster of this
    // text spans two characters. Splitting inside it keeps the cluster's glyphs together in
    // the leading half - the text boundary has to follow them, or the two halves disagree
    // about which characters their glyphs cover.
    const TEXT: &str = "أَبْجَدِيَّة";

    let _scope = start();

    let typeface = match_first_character(TEXT);

    assert!(typeface.is_some());
    let typeface = typeface.unwrap();

    let options = TextShaperOptions::with_all(
        typeface.glyph_typeface(),
        12.0,
        1,
        Some(CultureInfo::invariant_culture()),
        0.0,
        0.0,
        None,
    );
    let text = ReadOnlyMemory::from_str(TEXT);
    let buffer = TextShaper::current().shape_text(&text, &options);

    // Precondition: the first cluster covers the first two characters.
    assert!(!buffer.is_left_to_right());
    assert_eq!(0, buffer.get(buffer.length() - 1).glyph_cluster);
    assert_eq!(0, buffer.get(buffer.length() - 2).glyph_cluster);
    assert_eq!(2, buffer.get(buffer.length() - 3).glyph_cluster);

    let split_result = buffer.split(1);

    let first = split_result.first;
    let second = split_result.second;

    assert!(first.is_some());
    assert!(second.is_some());
    let first = first.unwrap();
    let second = second.unwrap();

    // The split snaps forward past the cluster, exactly like the left-to-right path.
    assert_eq!(2, first.text().len());
    assert_eq!(2, first.length());

    // No character and no glyph is lost or duplicated.
    assert_eq!(text.len(), first.text().len() + second.text().len());
    assert_eq!(buffer.length(), first.length() + second.length());

    // Every glyph of the trailing half belongs to the characters the trailing half owns.
    for i in 0..second.length() {
        assert!(
            second.get(i).glyph_cluster >= first.text().len() as i32,
            "Glyph {i} has cluster {}, which the leading half owns.",
            second.get(i).glyph_cluster
        );
    }
}

#[test]
fn should_split_right_to_left() {
    let text = "أَبْجَدِيَّة عَرَبِيَّة";

    let _scope = start();

    let typeface = match_first_character(text);

    assert!(typeface.is_some());
    let typeface = typeface.unwrap();

    let buffer = TextShaper::current()
        .shape_text(&ReadOnlyMemory::from_str(text), &TextShaperOptions::new(typeface.glyph_typeface()));

    let split_result = buffer.split(6);

    let first = split_result.first;

    assert!(first.is_some());
    assert_eq!(6, first.unwrap().length());
}

#[test]
fn should_split_zero_length() {
    let text = "ABC";

    let _scope = start();

    let buffer = TextShaper::current().shape_text(
        &ReadOnlyMemory::from_str(text),
        &TextShaperOptions::new(Typeface::default_typeface().glyph_typeface()),
    );

    let split_result = buffer.split(0);

    assert!(split_result.first.is_some());
    assert_eq!(0, split_result.first.as_ref().unwrap().length());

    assert!(split_result.second.is_some());

    assert_eq!(text.len(), split_result.second.as_ref().unwrap().length());
}

#[test]
fn cluster_cache_simple_mode_for_latin_text() {
    let _scope = start();

    let buffer = shape_default("ABCDEFGH");

    assert!(
        buffer.is_cluster_cache_simple(),
        "Single-codepoint LTR text should use the simple cluster-cache mode."
    );
}

#[test]
fn cluster_cache_simple_mode_measures_correctly() {
    let _scope = start();

    let buffer = shape_default("ABCDEFGH");

    assert!(buffer.is_cluster_cache_simple());

    // Sum advances linearly and compare to TotalGlyphAdvance.
    let mut expected_total = 0.0;
    for i in 0..buffer.length() {
        expected_total += buffer.get(i).glyph_advance;
    }

    assert_equal_precision(expected_total, buffer.total_glyph_advance(), 5);

    // Measure: ask for the width of the first 3 glyphs.
    let three_glyphs_width = buffer.get(0).glyph_advance + buffer.get(1).glyph_advance + buffer.get(2).glyph_advance;
    let fit = buffer.find_leading_char_count_within_width(three_glyphs_width);
    let width_consumed = buffer.get_char_range_width(0, fit);

    assert_eq!(3, fit);
    assert_equal_precision(three_glyphs_width, width_consumed, 5);

    // FirstClusterCharLength must be 1 in simple mode.
    assert_eq!(1, buffer.first_cluster_char_length());
}

#[test]
fn cluster_cache_simple_mode_survives_split() {
    let _scope = start();

    let buffer = shape_default("ABCDEFGH");

    assert!(buffer.is_cluster_cache_simple());

    let split = buffer.split(3);

    assert!(split.first.is_some());
    assert!(split.second.is_some());
    let first = split.first.unwrap();
    let second = split.second.unwrap();
    assert_eq!(3, first.length());
    assert_eq!(5, second.length());

    assert!(first.is_cluster_cache_simple(), "Split halves of a simple-mode buffer should also be simple-mode.");
    assert!(second.is_cluster_cache_simple());

    let first_width = buffer.get(0).glyph_advance + buffer.get(1).glyph_advance + buffer.get(2).glyph_advance;
    assert_equal_precision(first_width, first.total_glyph_advance(), 5);
}

#[test]
fn cluster_cache_not_simple_mode_for_complex_clusters() {
    let _scope = start();

    let typeface =
        Typeface::new(FontFamily::parse("resm:FerroUI.Vello.UnitTests.Fonts?assembly=ferroui-vello#Cascadia Code").unwrap());

    // Same text the existing Should_Not_Split_Cluster test uses: contains a
    // two-codepoint cluster that breaks the one-char-per-cluster invariant.
    let buffer = TextShaper::current()
        .shape_text(&ReadOnlyMemory::from_str("a\"๊a"), &TextShaperOptions::new(typeface.glyph_typeface()));

    assert!(
        !buffer.is_cluster_cache_simple(),
        "Multi-char clusters should fall back to the full cluster-start-chars table."
    );
}

#[test]
fn cluster_cache_simple_mode_trimming_helpers_are_correct() {
    let _scope = start();

    let buffer = shape_default("ABCDEFGH");

    assert!(buffer.is_cluster_cache_simple());

    let mut advances = vec![0.0; buffer.length()];
    for (i, advance) in advances.iter_mut().enumerate() {
        *advance = buffer.get(i).glyph_advance;
    }

    let sum = |start: usize, end: usize| -> f64 {
        let mut w = 0.0;
        for advance in &advances[start..end] {
            w += advance;
        }
        w
    };

    // GetCharRangeWidth: exact sub-range sums, including out-of-range clamping.
    // These must not throw in simple mode (the regression: _clusterStartChars is null).
    assert_equal_precision(sum(0, 3), buffer.get_char_range_width(0, 3), 5);
    assert_equal_precision(sum(2, 5), buffer.get_char_range_width(2, 5), 5);
    assert_equal_precision(sum(0, 8), buffer.get_char_range_width(-2, 100), 5); // clamped to [0, 8]
    assert_equal_precision(0.0, buffer.get_char_range_width(4, 4), 5);

    // FindLeadingCharCountWithinWidth: budget mid-way into the 4th glyph -> first 3 fit.
    let leading_budget = sum(0, 3) + advances[3] * 0.5;
    assert_eq!(3, buffer.find_leading_char_count_within_width(leading_budget));

    // FindTrailingCharCountWithinWidth: budget mid-way into glyph index 4 -> last 3 fit.
    let trailing_budget = sum(5, 8) + advances[4] * 0.5;
    let (trailing_count, consumed) = buffer.find_trailing_char_count_within_width(trailing_budget);
    assert_eq!(3, trailing_count);
    assert_equal_precision(sum(5, 8), consumed, 5);
}

#[test]
fn cluster_cache_simple_mode_trimming_helpers_survive_split() {
    let _scope = start();

    let buffer = shape_default("ABCDEFGH");
    assert!(buffer.is_cluster_cache_simple());

    let split = buffer.split(3);
    let second = split.second;
    assert!(second.is_some());
    let second = second.unwrap();
    assert!(second.is_cluster_cache_simple());
    assert_eq!(5, second.length()); // "DEFGH"

    let mut advances = vec![0.0; second.length()];
    for (i, advance) in advances.iter_mut().enumerate() {
        *advance = second.get(i).glyph_advance;
    }

    // Exercises the _clusterStartIdx offset on a simple-mode sub-buffer.
    let first_two = advances[0] + advances[1];
    assert_equal_precision(first_two, second.get_char_range_width(0, 2), 5);

    let leading_budget = first_two + advances[2] * 0.5;
    assert_eq!(2, second.find_leading_char_count_within_width(leading_budget));
}

/// `TextShaper.Current.ShapeText(text, new TextShaperOptions(Typeface.Default.GlyphTypeface))`.
fn shape_default(text: &str) -> Rc<ferroui_base::media::text_formatting::ShapedBuffer> {
    TextShaper::current().shape_text(
        &ReadOnlyMemory::from_str(text),
        &TextShaperOptions::new(Typeface::default_typeface().glyph_typeface()),
    )
}

/// `Codepoint.ReadAt(text, 0, out _)` followed by
/// `FontManager.Current.TryMatchCharacter(codepoint, Normal, Normal, Normal, null, null, out var typeface)`.
fn match_first_character(text: &str) -> Option<Typeface> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let (codepoint, _) = Codepoint::read_at(&units, 0);

    FontManager::current().try_match_character(
        codepoint.value() as i32,
        FontStyle::Normal,
        FontWeight::Normal,
        FontStretch::Normal,
        None,
        None,
    )
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface()
            .with_render_interface(Rc::new(PlatformRenderInterface::default()))
            .with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    )
}

/// xUnit's `Assert.Equal(double, double, int precision)`: both values rounded
/// to `precision` decimal places (to even) are equal.
fn assert_equal_precision(expected: f64, actual: f64, precision: i32) {
    let factor = 10f64.powi(precision);

    assert_eq!(
        (expected * factor).round_ties_even() / factor,
        (actual * factor).round_ties_even() / factor,
        "expected {expected}, actual {actual} (precision {precision})"
    );
}
