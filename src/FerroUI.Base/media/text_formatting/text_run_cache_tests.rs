//! Port of upstream's run cache tests onto the test harness. The two tests
//! that go through `TextLayout` are in `text_layout_tests.rs`.

use std::rc::Rc;

use crate::media::text_formatting::testing::{
    line_text, paragraph_properties, run_properties, SingleBufferTextSource, TextTestScope,
};
use crate::media::text_formatting::{
    ITextSource, ShapedTextRun, TextFormatter, TextLine, TextLineBreak, TextParagraphProperties, TextRunCache,
    TextRunProperties,
};
use crate::media::TextWrapping;

fn default_properties() -> Rc<dyn TextRunProperties> {
    run_properties(12.0)
}

fn format_line(
    text_source: &dyn ITextSource,
    first_text_source_index: i32,
    paragraph_width: f64,
    paragraph_properties: &Rc<dyn TextParagraphProperties>,
    cache: Option<&TextRunCache>,
) -> Rc<dyn TextLine> {
    <dyn TextFormatter>::current()
        .format_line_with_cache(text_source, first_text_source_index, paragraph_width, paragraph_properties, None, cache)
        .expect("a line")
}

fn format_all_lines(
    text_source: &dyn ITextSource,
    text_length: i32,
    paragraph_width: f64,
    paragraph_properties: &Rc<dyn TextParagraphProperties>,
    cache: Option<&TextRunCache>,
) -> Vec<Rc<dyn TextLine>> {
    let formatter = <dyn TextFormatter>::current();

    let mut lines: Vec<Rc<dyn TextLine>> = Vec::new();
    let mut current_position = 0;
    let mut previous_line_break: Option<Rc<TextLineBreak>> = None;

    while current_position < text_length {
        let Some(line) = formatter.format_line_with_cache(
            text_source,
            current_position,
            paragraph_width,
            paragraph_properties,
            previous_line_break.as_ref(),
            cache,
        ) else {
            break;
        };

        current_position += line.length();
        previous_line_break = line.text_line_break();
        lines.push(line);
    }

    lines
}

fn assert_same_glyphs(expected: &dyn TextLine, actual: &dyn TextLine) {
    assert_eq!(actual.length(), expected.length());

    let expected_runs = expected.text_runs();
    let actual_runs = actual.text_runs();

    assert_eq!(actual_runs.len(), expected_runs.len());

    for (expected_run, actual_run) in expected_runs.iter().zip(actual_runs.iter()) {
        let (Some(expected_run), Some(actual_run)) =
            (expected_run.downcast_ref::<ShapedTextRun>(), actual_run.downcast_ref::<ShapedTextRun>())
        else {
            continue;
        };

        assert_eq!(actual_run.bidi_level(), expected_run.bidi_level());

        let expected_clusters: Vec<i32> =
            expected_run.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster).collect();
        let actual_clusters: Vec<i32> =
            actual_run.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster).collect();

        assert!(!expected_clusters.is_empty());
        assert_eq!(actual_clusters, expected_clusters);

        if !expected_run.shaped_buffer().is_left_to_right() && expected_clusters.len() > 1 {
            // For right to left runs the shaper produces glyphs in descending cluster order.
            assert!(expected_clusters[0] >= expected_clusters[expected_clusters.len() - 1]);
        }
    }
}

#[test]
fn cache_hit_produces_identical_layout() {
    let scope = TextTestScope::new();

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new("Hello World", default_properties.clone());

    let cache = TextRunCache::new();

    // First call: cache miss, populates cache.
    let line1 = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_eq!(scope.shaper().shape_call_count(), 1);

    // Second call: cache hit, different paragraph width.
    let line2 = format_line(&text_source, 0, 200.0, &paragraph_properties, Some(&cache));

    assert_eq!(line1.length(), line2.length());
    assert_eq!(line1.text_runs().len(), line2.text_runs().len());

    // Addition: the hit did not shape again and reuses the shaped run.
    assert_eq!(scope.shaper().shape_call_count(), 1);
    assert!(std::ptr::addr_eq(Rc::as_ptr(&line1.text_runs()[0]), Rc::as_ptr(&line2.text_runs()[0])));

    cache.dispose();
}

#[test]
fn full_invalidation_clears_cache() {
    let scope = TextTestScope::new();

    let text = "Hello World";

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let cache = TextRunCache::new();

    // Populate cache.
    format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    // Invalidate.
    cache.invalidate();

    assert!(cache.try_get_shaped_runs(0).is_none());

    // Verify cache miss: should produce a valid line.
    let line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_eq!(line.length(), text.len() as i32);
    assert_eq!(scope.shaper().shape_call_count(), 2);

    cache.dispose();
}

#[test]
fn partial_invalidation_preserves_earlier_entries() {
    let scope = TextTestScope::new();

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new("First paragraph\nSecond paragraph", default_properties.clone());

    let cache = TextRunCache::new();

    // Format first paragraph (populates cache at index 0).
    let line1 = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    let first_line_length = line1.length();

    assert_eq!(first_line_length, 16);

    // Format second paragraph (populates cache at the first line's length).
    let line2 = format_line(&text_source, first_line_length, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert!(cache.has_entries_map());
    assert_eq!(scope.shaper().shape_call_count(), 2);

    // Invalidate from the second paragraph index.
    cache.invalidate_from(first_line_length);

    assert!(cache.try_get_shaped_runs(0).is_some());
    assert!(cache.try_get_shaped_runs(first_line_length).is_none());

    // First paragraph should still be cached (cache hit).
    let line1_again = format_line(&text_source, 0, 200.0, &paragraph_properties, Some(&cache));

    assert_eq!(line1.length(), line1_again.length());
    assert_eq!(scope.shaper().shape_call_count(), 2);

    // Second paragraph should be re-shaped (cache miss then re-populated).
    let line2_again =
        format_line(&text_source, first_line_length, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_eq!(line2.length(), line2_again.length());
    assert_eq!(scope.shaper().shape_call_count(), 3);

    cache.dispose();
}

#[test]
fn text_wrapping_with_cache_produces_correct_lines() {
    let _scope = TextTestScope::new();

    let text = "The quick brown fox jumps over the lazy dog";

    let default_properties = default_properties();
    let wrapping_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    // Format without cache.
    let lines_without = format_all_lines(&text_source, text.len() as i32, 100.0, &wrapping_properties, None);

    // 100 fits sixteen glyphs of 6.
    assert_eq!(
        lines_without.iter().map(|line| line_text(&**line)).collect::<Vec<_>>(),
        ["The quick brown ", "fox jumps over ", "the lazy dog"]
    );

    // Format with cache (first pass: cache miss).
    let cache = TextRunCache::new();

    let lines_with = format_all_lines(&text_source, text.len() as i32, 100.0, &wrapping_properties, Some(&cache));

    // Format with cache again (second pass: cache hit).
    let lines_cache_hit = format_all_lines(&text_source, text.len() as i32, 100.0, &wrapping_properties, Some(&cache));

    for lines in [&lines_with, &lines_cache_hit] {
        assert_eq!(lines.len(), lines_without.len());

        for (line, expected) in lines.iter().zip(&lines_without) {
            assert_eq!(line.length(), expected.length());
            assert_eq!(line_text(&**line), line_text(&**expected));
            assert_eq!(line.width(), expected.width());
        }
    }

    cache.dispose();
}

#[test]
fn wrapping_with_different_width_from_cache() {
    let _scope = TextTestScope::new();

    let text = "The quick brown fox jumps over the lazy dog";

    let default_properties = default_properties();
    let wrapping_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let cache = TextRunCache::new();

    // First format: wide (cache miss, populates).
    let wide_lines = format_all_lines(&text_source, text.len() as i32, 500.0, &wrapping_properties, Some(&cache));

    // Second format: narrow (cache hit, different wrapping).
    let narrow_lines = format_all_lines(&text_source, text.len() as i32, 80.0, &wrapping_properties, Some(&cache));

    // Narrow should produce more lines.
    assert_eq!(wide_lines.len(), 1);
    assert_eq!(narrow_lines.len(), 4);

    // Total characters should be the same.
    let wide_total: i32 = wide_lines.iter().map(|line| line.length()).sum();
    let narrow_total: i32 = narrow_lines.iter().map(|line| line.length()).sum();

    assert_eq!(wide_total, narrow_total);

    cache.dispose();
}

#[test]
fn bidi_text_with_cache_produces_correct_results() {
    let _scope = TextTestScope::new();

    // Mixed LTR/RTL text.
    let text = "Hello \u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629} World";

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    // Without cache.
    let line_without = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None);

    // With cache.
    let cache = TextRunCache::new();

    let line_with = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_same_glyphs(&*line_without, &*line_with);

    // Cache hit should also produce correct results.
    let line_cache_hit = format_line(&text_source, 0, 200.0, &paragraph_properties, Some(&cache));

    assert_same_glyphs(&*line_without, &*line_cache_hit);

    cache.dispose();
}

#[test]
fn bidi_cache_hit_does_not_double_reorder_rtl_glyph_clusters() {
    let _scope = TextTestScope::new();

    // LTR paragraph containing an Arabic RTL island followed by Latin text.
    let text = "Hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627} World";

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let cache = TextRunCache::new();

    // Cache miss: shapes and caches.
    let line_miss = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    // Cache hit: must not reverse the RTL run's glyph buffer again.
    let line_hit = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_eq!(line_miss.text_runs().len(), 3);

    assert_same_glyphs(&*line_miss, &*line_hit);

    cache.dispose();
}

#[test]
fn bidi_cache_hit_matches_no_cache_for_pure_rtl_paragraph() {
    let _scope = TextTestScope::new();

    // All text is Arabic so the resolved flow direction is RTL.
    let text = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627} \u{0628}\u{0627}\u{0644}\u{0639}\u{0627}\u{0644}\u{0645}";

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    // Reference: formatted without any cache.
    let line_no_cache = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None);

    let cache = TextRunCache::new();

    let line_miss = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));
    let line_hit = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_same_glyphs(&*line_no_cache, &*line_miss);
    assert_same_glyphs(&*line_no_cache, &*line_hit);

    cache.dispose();
}

#[test]
fn bidi_cache_hit_with_wrapping_does_not_double_reorder() {
    let _scope = TextTestScope::new();

    // Wrapped bidi text so the wrap path through the cache is exercised.
    let text = "Hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627} World and more text to force wrapping";

    let default_properties = default_properties();
    let wrapping_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let text_length = text.encode_utf16().count() as i32;

    let cache = TextRunCache::new();

    // Cache miss pass.
    let miss_lines = format_all_lines(&text_source, text_length, 80.0, &wrapping_properties, Some(&cache));

    // Cache hit pass.
    let hit_lines = format_all_lines(&text_source, text_length, 80.0, &wrapping_properties, Some(&cache));

    assert!(miss_lines.len() > 1);
    assert_eq!(miss_lines.len(), hit_lines.len());

    for (miss_line, hit_line) in miss_lines.iter().zip(&hit_lines) {
        assert_same_glyphs(&**miss_line, &**hit_line);
    }

    cache.dispose();
}

#[test]
fn dispose_releases_cache_entries() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new("Hello World", default_properties.clone());

    let cache = TextRunCache::new();

    // Populate cache.
    let line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache));

    let run = line.text_runs()[0].clone();
    let shaped_run = run.downcast_ref::<ShapedTextRun>().unwrap();

    // Dispose should not fail.
    cache.dispose();

    // Addition: the line still owns its reference to the shaped run ...
    assert_eq!(shaped_run.shaped_buffer().length(), 11);

    // ... and releasing the line releases the buffer.
    line.dispose();

    assert_eq!(shaped_run.shaped_buffer().length(), 0);

    // After dispose, another cache still works (re-creates entries).
    let cache2 = TextRunCache::new();

    let line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, Some(&cache2));

    assert_eq!(line.length(), 11);

    cache2.dispose();
}

#[test]
fn cache_with_multiple_paragraphs() {
    let scope = TextTestScope::new();

    let text = "First line\nSecond line\nThird line";

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let cache = TextRunCache::new();

    // Format all paragraphs.
    let lines = format_all_lines(&text_source, text.len() as i32, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_eq!(lines.len(), 3);
    assert_eq!(scope.shaper().shape_call_count(), 3);

    // Format again from cache.
    let lines2 = format_all_lines(&text_source, text.len() as i32, f64::INFINITY, &paragraph_properties, Some(&cache));

    assert_eq!(lines.len(), lines2.len());

    for (line, line2) in lines.iter().zip(&lines2) {
        assert_eq!(line.length(), line2.length());
    }

    assert_eq!(scope.shaper().shape_call_count(), 3);

    cache.dispose();
}

/// Addition: adding an entry for a key that is already cached releases the old entry.
#[test]
fn replacing_an_entry_releases_the_old_runs() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::NoWrap);
    let text_source = SingleBufferTextSource::new("Hello", default_properties.clone());

    let line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None);

    let runs: Rc<[Rc<dyn crate::media::text_formatting::TextRun>]> = Rc::from(line.text_runs().to_vec());

    let cache = TextRunCache::new();

    let result = crate::media::text_formatting::CachedShapingResult::new(
        runs.clone(),
        crate::media::FlowDirection::LeftToRight,
        None,
        5,
    );

    cache.add(0, result.clone());
    cache.add(0, result.clone());

    // Line + one cache reference: releasing the line keeps the buffer alive.
    line.dispose();

    let shaped_run = runs[0].downcast_ref::<ShapedTextRun>().unwrap();

    assert_eq!(shaped_run.shaped_buffer().length(), 5);

    cache.invalidate();

    assert_eq!(shaped_run.shaped_buffer().length(), 0);
}
