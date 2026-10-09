//! Port of upstream's `Media/TextFormatting/TextRunCacheTests.cs` of the
//! Skia unit tests.
//!
//! Upstream's `using var` declarations of caches and layouts are explicit
//! `dispose()` calls at the end of the scope, in reverse order of declaration.

use crate::unit_tests::media::text_formatting::SingleBufferTextSource;
use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, ShapedTextRun, TextEndOfParagraph,
    TextFormatter, TextFormatterImpl, TextLayout, TextLayoutOptions, TextLine, TextParagraphProperties, TextRunCache,
};
use ferroui_base::media::{FlowDirection, TextAlignment, TextWrapping, Typeface};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// Upstream's `new GenericTextParagraphProperties(defaultProperties)`.
fn paragraph_properties(default_properties: &Rc<GenericTextRunProperties>) -> Rc<dyn TextParagraphProperties> {
    Rc::new(GenericTextParagraphProperties::new(default_properties.clone()))
}

/// Upstream's `new GenericTextParagraphProperties(FlowDirection.LeftToRight, TextAlignment.Left, true, false,
/// defaultProperties, TextWrapping.Wrap, 0, 0, 0)`.
fn wrapping_properties(default_properties: &Rc<GenericTextRunProperties>) -> Rc<dyn TextParagraphProperties> {
    Rc::new(GenericTextParagraphProperties::with_all(
        FlowDirection::LeftToRight,
        TextAlignment::Left,
        true,
        false,
        default_properties.clone(),
        TextWrapping::Wrap,
        0.0,
        0.0,
        0.0,
    ))
}

fn shaped(text_line: &dyn TextLine, index: usize) -> Option<Rc<ShapedTextRun>> {
    text_line.text_runs()[index].clone().downcast_rc::<ShapedTextRun>()
}

#[test]
fn cache_hit_produces_identical_layout() {
    let _scope = start();

    let text = "Hello World";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let cache = TextRunCache::new();

    // First call: cache miss, populates cache.
    let line1 =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    assert!(line1.is_some());

    // Second call: cache hit, different paragraph width.
    let line2 = formatter.format_line_with_cache(&text_source, 0, 200.0, &paragraph_properties, None, Some(&cache));

    assert!(line2.is_some());

    let (line1, line2) = (line1.unwrap(), line2.unwrap());

    // Both lines should have the same text length.
    assert_eq!(line1.length(), line2.length());

    // Both lines should have the same number of text runs.
    assert_eq!(line1.text_runs().len(), line2.text_runs().len());

    cache.dispose();
}

#[test]
fn full_invalidation_clears_cache() {
    let _scope = start();

    let text = "Hello World";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let cache = TextRunCache::new();

    // Populate cache.
    formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    // Invalidate.
    cache.invalidate();

    // Verify cache miss: should not throw and should produce a valid line.
    let line =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    assert!(line.is_some());
    assert_eq!(utf16_len(text), line.unwrap().length());

    cache.dispose();
}

#[test]
fn partial_invalidation_preserves_earlier_entries() {
    let _scope = start();

    let text = "First paragraph\nSecond paragraph";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let cache = TextRunCache::new();

    // Format first paragraph (populates cache at index 0).
    let line1 =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    assert!(line1.is_some());

    let line1 = line1.unwrap();

    let first_line_length = line1.length();

    // Format second paragraph (populates cache at firstLineLength).
    let line2 = formatter.format_line_with_cache(
        &text_source,
        first_line_length,
        f64::INFINITY,
        &paragraph_properties,
        None,
        Some(&cache),
    );

    assert!(line2.is_some());

    // Invalidate from the second paragraph index.
    cache.invalidate_from(first_line_length);

    // First paragraph should still be cached (cache hit).
    let line1_again =
        formatter.format_line_with_cache(&text_source, 0, 200.0, &paragraph_properties, None, Some(&cache));

    assert!(line1_again.is_some());
    assert_eq!(line1.length(), line1_again.unwrap().length());

    // Second paragraph should be re-shaped (cache miss then re-populated).
    let line2_again = formatter.format_line_with_cache(
        &text_source,
        first_line_length,
        f64::INFINITY,
        &paragraph_properties,
        None,
        Some(&cache),
    );

    assert!(line2_again.is_some());
    assert_eq!(line2.unwrap().length(), line2_again.unwrap().length());

    cache.dispose();
}

#[test]
fn text_wrapping_with_cache_produces_correct_lines() {
    let _scope = start();

    let text = "The quick brown fox jumps over the lazy dog";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let wrapping_properties = wrapping_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    // Format without cache.
    let lines_without = format_all_lines(&formatter, &text_source, 100.0, &wrapping_properties, None);

    // Format with cache (first pass: cache miss).
    let cache = TextRunCache::new();
    let lines_with = format_all_lines(&formatter, &text_source, 100.0, &wrapping_properties, Some(&cache));

    assert_eq!(lines_without.len(), lines_with.len());

    for i in 0..lines_without.len() {
        assert_eq!(lines_without[i].length(), lines_with[i].length());
    }

    // Format with cache again (second pass: cache hit).
    let lines_cache_hit = format_all_lines(&formatter, &text_source, 100.0, &wrapping_properties, Some(&cache));

    assert_eq!(lines_without.len(), lines_cache_hit.len());

    for i in 0..lines_without.len() {
        assert_eq!(lines_without[i].length(), lines_cache_hit[i].length());
    }

    cache.dispose();
}

#[test]
fn wrapping_with_different_width_from_cache() {
    let _scope = start();

    let text = "The quick brown fox jumps over the lazy dog";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let wrapping_properties = wrapping_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let cache = TextRunCache::new();

    // First format: wide (cache miss, populates).
    let wide_lines = format_all_lines(&formatter, &text_source, 500.0, &wrapping_properties, Some(&cache));

    // Second format: narrow (cache hit, different wrapping).
    let narrow_lines = format_all_lines(&formatter, &text_source, 80.0, &wrapping_properties, Some(&cache));

    // Narrow should produce more lines.
    assert!(narrow_lines.len() >= wide_lines.len());

    // Total characters should be the same.
    let wide_total: i32 = wide_lines.iter().map(|l| l.length()).sum();
    let narrow_total: i32 = narrow_lines.iter().map(|l| l.length()).sum();

    assert_eq!(wide_total, narrow_total);

    cache.dispose();
}

#[test]
fn bidi_text_with_cache_produces_correct_results() {
    let _scope = start();

    // Mixed LTR/RTL text.
    let text = "Hello \u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629} World";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    // Without cache.
    let line_without =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, None);

    // With cache.
    let cache = TextRunCache::new();
    let line_with =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    assert!(line_without.is_some());
    assert!(line_with.is_some());

    let (line_without, line_with) = (line_without.unwrap(), line_with.unwrap());

    assert_eq!(line_without.length(), line_with.length());
    assert_eq!(line_without.text_runs().len(), line_with.text_runs().len());

    // Cache hit should also produce correct results.
    let line_cache_hit =
        formatter.format_line_with_cache(&text_source, 0, 200.0, &paragraph_properties, None, Some(&cache));

    assert!(line_cache_hit.is_some());
    assert_eq!(line_without.length(), line_cache_hit.unwrap().length());

    cache.dispose();
}

// Regression tests for the double-reorder bug (commit 2837a287):
// When shaped runs with shared ShapedBuffer backing arrays were cached, the old
// BidiReorderer.Reverse() call on a non-owning copy mutated the same backing array
// that the cached run referenced.  On the next layout the cache returned the already-
// reversed buffer but with IsReversed=false, causing BidiReorderer to reverse it a
// second time – putting glyphs back in logical (wrong visual) order for RTL runs.

#[test]
fn bidi_cache_hit_does_not_double_reorder_rtl_glyph_clusters() {
    let _scope = start();

    // LTR paragraph containing an Arabic RTL island followed by Latin text.
    // "Hello مرحبا World" – the Arabic word is at source indices 6-10.
    let text = "Hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627} World";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let cache = TextRunCache::new();

    // Cache miss: shapes and caches.
    let line_miss =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    assert!(line_miss.is_some());

    // Cache hit: must not double-reverse the RTL run's glyph buffer.
    let line_hit =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    assert!(line_hit.is_some());

    let (line_miss, line_hit) = (line_miss.unwrap(), line_hit.unwrap());

    assert_eq!(line_miss.text_runs().len(), line_hit.text_runs().len());

    for i in 0..line_miss.text_runs().len() {
        let miss_run = shaped(&*line_miss, i);
        let hit_run = shaped(&*line_hit, i);

        let (Some(miss_run), Some(hit_run)) = (miss_run, hit_run) else {
            continue;
        };

        assert_eq!(miss_run.bidi_level(), hit_run.bidi_level());
        assert_eq!(miss_run.shaped_buffer().is_left_to_right(), hit_run.shaped_buffer().is_left_to_right());
        assert_eq!(miss_run.shaped_buffer().length(), hit_run.shaped_buffer().length());

        // For RTL runs the shaper produces glyphs in descending cluster order
        // (visual right-to-left).  Double-reversal would flip them back to
        // ascending order (logical), causing wrong rendering.
        if !miss_run.shaped_buffer().is_left_to_right() && miss_run.shaped_buffer().length() > 1 {
            let miss_first = miss_run.shaped_buffer().get(0).glyph_cluster;
            let miss_last = miss_run.shaped_buffer().get(miss_run.shaped_buffer().length() - 1).glyph_cluster;
            assert!(
                miss_first >= miss_last,
                "Cache-miss RTL run: expected descending clusters but got first={miss_first} last={miss_last}"
            );

            let hit_first = hit_run.shaped_buffer().get(0).glyph_cluster;
            let hit_last = hit_run.shaped_buffer().get(hit_run.shaped_buffer().length() - 1).glyph_cluster;
            assert!(
                hit_first >= hit_last,
                "Cache-hit RTL run: expected descending clusters but got first={hit_first} last={hit_last}"
            );

            // The individual cluster values must be identical between miss and hit.
            assert_eq!(miss_first, hit_first);
            assert_eq!(miss_last, hit_last);
        }
    }

    cache.dispose();
}

#[test]
fn bidi_cache_hit_matches_no_cache_for_pure_rtl_paragraph() {
    let _scope = start();

    // Paragraph-level RTL: all text is Arabic so the resolved flow direction is RTL.
    // "مرحبا بالعالم" (Hello World in Arabic)
    let text = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627} \u{0628}\u{0627}\u{0644}\u{0639}\u{0627}\u{0644}\u{0645}";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    // Reference: formatted without any cache.
    let line_no_cache =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, None);

    assert!(line_no_cache.is_some());

    let cache = TextRunCache::new();

    let line_miss =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    let line_hit =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    assert!(line_miss.is_some());
    assert!(line_hit.is_some());

    let (line_no_cache, line_hit) = (line_no_cache.unwrap(), line_hit.unwrap());

    assert_eq!(line_no_cache.length(), line_hit.length());
    assert_eq!(line_no_cache.text_runs().len(), line_hit.text_runs().len());

    for i in 0..line_no_cache.text_runs().len() {
        let ref_run = shaped(&*line_no_cache, i);
        let hit_run = shaped(&*line_hit, i);

        let (Some(ref_run), Some(hit_run)) = (ref_run, hit_run) else {
            continue;
        };

        assert_eq!(ref_run.bidi_level(), hit_run.bidi_level());
        assert_eq!(ref_run.shaped_buffer().is_left_to_right(), hit_run.shaped_buffer().is_left_to_right());
        assert_eq!(ref_run.shaped_buffer().length(), hit_run.shaped_buffer().length());

        // Glyph cluster values must be identical: no double-reversal allowed.
        for j in 0..ref_run.shaped_buffer().length() {
            assert_eq!(ref_run.shaped_buffer().get(j).glyph_cluster, hit_run.shaped_buffer().get(j).glyph_cluster);
        }
    }

    cache.dispose();
}

#[test]
fn text_layout_recreated_from_cache_with_bidi_has_same_glyph_order() {
    let _scope = start();

    // Mixed LTR/RTL text used for two successive TextLayout instances that share a cache.
    // Before the fix, the second instance would double-reverse RTL glyph buffers.
    let text = "Hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627} World";

    let cache = Rc::new(TextRunCache::new());

    let layout1 = TextLayout::new(
        text,
        Typeface::default_typeface(),
        TextLayoutOptions { font_size: 12.0, text_run_cache: Some(cache.clone()), ..Default::default() },
    );

    // Second layout: triggers cache-hit path – previously double-reordered RTL runs.
    let layout2 = TextLayout::new(
        text,
        Typeface::default_typeface(),
        TextLayoutOptions { font_size: 12.0, text_run_cache: Some(cache.clone()), ..Default::default() },
    );

    assert_eq!(layout1.text_lines().len(), layout2.text_lines().len());

    for line_idx in 0..layout1.text_lines().len() {
        let line1 = &layout1.text_lines()[line_idx];
        let line2 = &layout2.text_lines()[line_idx];

        assert_eq!(line1.length(), line2.length());
        assert_eq!(line1.text_runs().len(), line2.text_runs().len());

        for i in 0..line1.text_runs().len() {
            let run1 = shaped(&**line1, i);
            let run2 = shaped(&**line2, i);

            let (Some(run1), Some(run2)) = (run1, run2) else {
                continue;
            };

            assert_eq!(run1.bidi_level(), run2.bidi_level());
            assert_eq!(run1.shaped_buffer().is_left_to_right(), run2.shaped_buffer().is_left_to_right());
            assert_eq!(run1.shaped_buffer().length(), run2.shaped_buffer().length());

            for j in 0..run1.shaped_buffer().length() {
                assert_eq!(run1.shaped_buffer().get(j).glyph_cluster, run2.shaped_buffer().get(j).glyph_cluster);
            }
        }
    }

    layout2.dispose();
    layout1.dispose();
    cache.dispose();
}

#[test]
fn bidi_cache_hit_with_wrapping_does_not_double_reorder() {
    let _scope = start();

    // Wrapped bidi text so the Wrap path through FormatLineFromCache is exercised.
    let text = "Hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627} World and more text to force wrapping";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let wrapping_properties = wrapping_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let cache = TextRunCache::new();

    // Cache miss pass.
    let mis_lines = format_all_lines(&formatter, &text_source, 80.0, &wrapping_properties, Some(&cache));

    // Cache hit pass.
    let hit_lines = format_all_lines(&formatter, &text_source, 80.0, &wrapping_properties, Some(&cache));

    assert_eq!(mis_lines.len(), hit_lines.len());

    for line_idx in 0..mis_lines.len() {
        let miss_line = &mis_lines[line_idx];
        let hit_line = &hit_lines[line_idx];

        assert_eq!(miss_line.length(), hit_line.length());
        assert_eq!(miss_line.text_runs().len(), hit_line.text_runs().len());

        for i in 0..miss_line.text_runs().len() {
            let miss_run = shaped(&**miss_line, i);
            let hit_run = shaped(&**hit_line, i);

            let (Some(miss_run), Some(hit_run)) = (miss_run, hit_run) else {
                continue;
            };

            assert_eq!(miss_run.bidi_level(), hit_run.bidi_level());
            assert_eq!(miss_run.shaped_buffer().is_left_to_right(), hit_run.shaped_buffer().is_left_to_right());
            assert_eq!(miss_run.shaped_buffer().length(), hit_run.shaped_buffer().length());

            for j in 0..miss_run.shaped_buffer().length() {
                assert_eq!(miss_run.shaped_buffer().get(j).glyph_cluster, hit_run.shaped_buffer().get(j).glyph_cluster);
            }
        }
    }

    cache.dispose();
}

#[test]
fn dispose_releases_cache_entries() {
    let _scope = start();

    let text = "Hello World";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let cache = TextRunCache::new();

    // Populate cache.
    formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache));

    // Dispose should not throw.
    cache.dispose();

    // After dispose, using the cache should still work (re-creates entries).
    let cache2 = TextRunCache::new();

    let line =
        formatter.format_line_with_cache(&text_source, 0, f64::INFINITY, &paragraph_properties, None, Some(&cache2));

    assert!(line.is_some());

    cache2.dispose();
}

#[test]
fn cache_with_multiple_paragraphs() {
    let _scope = start();

    let text = "First line\nSecond line\nThird line";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(&default_properties);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let cache = TextRunCache::new();

    // Format all paragraphs.
    let lines = format_all_lines(&formatter, &text_source, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert!(lines.len() >= 3);

    // Format again from cache with different width.
    let lines2 = format_all_lines(&formatter, &text_source, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_eq!(lines.len(), lines2.len());

    for i in 0..lines.len() {
        assert_eq!(lines[i].length(), lines2[i].length());
    }

    cache.dispose();
}

#[test]
fn text_layout_with_cache_matches_without() {
    let _scope = start();

    let text = "The quick brown fox jumps over the lazy dog";

    // Layout without cache.
    let layout1 = TextLayout::new(
        text,
        Typeface::default_typeface(),
        TextLayoutOptions { font_size: 12.0, text_wrapping: TextWrapping::Wrap, max_width: 100.0, ..Default::default() },
    );

    // Layout with cache.
    let cache = Rc::new(TextRunCache::new());
    let layout2 = TextLayout::new(
        text,
        Typeface::default_typeface(),
        TextLayoutOptions {
            font_size: 12.0,
            text_wrapping: TextWrapping::Wrap,
            max_width: 100.0,
            text_run_cache: Some(cache.clone()),
            ..Default::default()
        },
    );

    assert_eq!(layout1.text_lines().len(), layout2.text_lines().len());
    assert_eq!(layout1.height(), layout2.height());
    assert_eq!(layout1.width_including_trailing_whitespace(), layout2.width_including_trailing_whitespace());

    // Second layout from cache with different width.
    let layout3 = TextLayout::new(
        text,
        Typeface::default_typeface(),
        TextLayoutOptions {
            font_size: 12.0,
            text_wrapping: TextWrapping::Wrap,
            max_width: 80.0,
            text_run_cache: Some(cache.clone()),
            ..Default::default()
        },
    );

    // Should still be valid (more lines due to narrower width).
    assert!(layout3.text_lines().len() >= layout2.text_lines().len());
    assert!(layout3.height() > 0.0);

    layout1.dispose();
    layout2.dispose();
    layout3.dispose();

    cache.dispose();
}

fn format_all_lines(
    formatter: &TextFormatterImpl,
    text_source: &dyn ITextSource,
    paragraph_width: f64,
    paragraph_properties: &Rc<dyn TextParagraphProperties>,
    cache: Option<&TextRunCache>,
) -> Vec<Rc<dyn TextLine>> {
    let mut lines: Vec<Rc<dyn TextLine>> = Vec::new();
    let mut current_index = 0;
    let mut previous_line: Option<Rc<dyn TextLine>> = None;

    loop {
        let previous_line_break = previous_line.as_ref().and_then(|line| line.text_line_break());

        let Some(line) = formatter.format_line_with_cache(
            text_source,
            current_index,
            paragraph_width,
            paragraph_properties,
            previous_line_break.as_ref(),
            cache,
        ) else {
            break;
        };

        lines.push(line.clone());
        current_index += line.length();
        previous_line = Some(line.clone());

        if line
            .text_line_break()
            .and_then(|line_break| line_break.text_end_of_line().cloned())
            .is_some_and(|text_end_of_line| text_end_of_line.is::<TextEndOfParagraph>())
        {
            break;
        }
    }

    lines
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface()
            .with_render_interface(Rc::new(PlatformRenderInterface::default()))
            .with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    )
}
