//! Tests of `ShapedBuffer`: the glyph index view, and the shared storage and
//! per-write generation counter that `split` and `with_bidi_level` rely on.
//!
//! Upstream runs these against a real font and shaper; they only depend on
//! the buffer mechanics, so they run unchanged on the test harness.

use std::rc::Rc;

use crate::media::text_formatting::testing::{utf16, TextTestScope};
use crate::media::text_formatting::{GlyphInfo, ShapedBuffer, TextShaper, TextShaperOptions};
use crate::media::{GlyphTypeface, Typeface};
use crate::utilities::CultureInfo;

fn load_typeface() -> Rc<GlyphTypeface> {
    Typeface::default_typeface().glyph_typeface()
}

#[test]
fn glyph_indices_length_matches_buffer_length() {
    let _scope = TextTestScope::new();
    let buffer = ShapedBuffer::new(utf16("hello"), 5, load_typeface(), 16.0, 0);

    assert_eq!(buffer.glyph_indices().len(), 5);
    assert_eq!(buffer.length(), 5);

    buffer.dispose();
}

#[test]
fn indexer_set_syncs_glyph_indices() {
    let _scope = TextTestScope::new();
    let buffer = ShapedBuffer::new(utf16("ABC"), 3, load_typeface(), 16.0, 0);

    buffer.set(0, GlyphInfo::new(42, 0, 10.0));
    buffer.set(1, GlyphInfo::new(99, 1, 11.0));
    buffer.set(2, GlyphInfo::new(7, 2, 12.0));

    assert_eq!(*buffer.glyph_indices(), [42, 99, 7]);

    // And the glyph accessor still works.
    assert_eq!(buffer.get(0).glyph_index, 42);
    assert_eq!(buffer.get(1).glyph_index, 99);
    assert_eq!(buffer.get(2).glyph_index, 7);

    buffer.dispose();
}

#[test]
fn indexer_set_overwrite_updates_glyph_indices() {
    let _scope = TextTestScope::new();
    let buffer = ShapedBuffer::new(utf16("X"), 1, load_typeface(), 16.0, 0);

    buffer.set(0, GlyphInfo::new(10, 0, 1.0));
    assert_eq!(buffer.glyph_indices()[0], 10);

    // Mutating the entry (as the inter word justification does) keeps the parallel array in sync.
    buffer.set(0, GlyphInfo::new(10, 0, 2.0));
    assert_eq!(buffer.glyph_indices()[0], 10);

    buffer.set(0, GlyphInfo::new(33, 0, 1.0));
    assert_eq!(buffer.glyph_indices()[0], 33);

    buffer.dispose();
}

#[test]
fn split_ascending_preserves_glyph_indices_alignment() {
    let _scope = TextTestScope::new();

    // LTR (bidi 0): clusters ascending. Five 1-char clusters.
    let buffer = ShapedBuffer::new(utf16("ABCDE"), 5, load_typeface(), 16.0, 0);

    for (i, glyph) in [10u16, 20, 30, 40, 50].into_iter().enumerate() {
        buffer.set(i, GlyphInfo::new(glyph, i as i32, 5.0));
    }

    let split = buffer.split(2);

    let first = split.first.unwrap();
    let second = split.second.unwrap();

    assert_eq!(*first.glyph_indices(), [10, 20]);
    assert_eq!(*second.glyph_indices(), [30, 40, 50]);

    // Both halves stay in lockstep with the glyph accessor.
    assert_eq!(first.get(0).glyph_index, first.glyph_indices()[0]);
    assert_eq!(second.get(1).glyph_index, second.glyph_indices()[1]);
}

/// Addition: the same for a right to left buffer (descending clusters), which
/// upstream only covers through shaped text.
#[test]
fn split_descending_keeps_the_logical_first_half_at_the_end() {
    let _scope = TextTestScope::new();

    let buffer = ShapedBuffer::new(utf16("ABCDE"), 5, load_typeface(), 16.0, 1);

    for (i, glyph) in [50u16, 40, 30, 20, 10].into_iter().enumerate() {
        buffer.set(i, GlyphInfo::new(glyph, 4 - i as i32, 5.0));
    }

    let split = buffer.split(2);

    let first = split.first.unwrap();
    let second = split.second.unwrap();

    assert_eq!(first.text().to_string_lossy(), "AB");
    assert_eq!(second.text().to_string_lossy(), "CDE");
    assert_eq!(*first.glyph_indices(), [20, 10]);
    assert_eq!(*second.glyph_indices(), [50, 40, 30]);
    assert_eq!(first.total_glyph_advance(), 10.0);
    assert_eq!(second.total_glyph_advance(), 15.0);
}

#[test]
fn split_at_zero_yields_empty_leading_with_empty_indices() {
    let _scope = TextTestScope::new();
    let buffer = ShapedBuffer::new(utf16("ABC"), 3, load_typeface(), 16.0, 0);

    buffer.set(0, GlyphInfo::new(10, 0, 5.0));
    buffer.set(1, GlyphInfo::new(20, 1, 5.0));
    buffer.set(2, GlyphInfo::new(30, 2, 5.0));

    let split = buffer.split(0);

    let first = split.first.unwrap();

    assert_eq!(first.glyph_indices().len(), 0);
    assert_eq!(first.length(), 0);

    assert_eq!(split.second.unwrap().glyph_indices().len(), 3);
}

#[test]
fn dispose_clears_glyph_indices_view() {
    let _scope = TextTestScope::new();
    let buffer = ShapedBuffer::new(utf16("AB"), 2, load_typeface(), 16.0, 0);

    buffer.set(0, GlyphInfo::new(1, 0, 1.0));
    buffer.set(1, GlyphInfo::new(2, 1, 1.0));

    assert_eq!(buffer.glyph_indices().len(), 2);

    buffer.dispose();

    // After dispose the views are reset.
    assert_eq!(buffer.glyph_indices().len(), 0);
    assert_eq!(buffer.length(), 0);
    assert_eq!(buffer.glyph_infos().len(), 0);
}

// ── shared storage ──

const ASCII_TEXT: &str = "The quick brown fox";

fn shape_ascii(text: &str) -> Rc<ShapedBuffer> {
    let options = TextShaperOptions::with_all(
        Typeface::default_typeface().glyph_typeface(),
        12.0,
        0,
        Some(CultureInfo::current_culture()),
        0.0,
        0.0,
        None,
    );

    TextShaper::current().shape_str(text, &options)
}

fn assert_close(expected: f64, actual: f64) {
    assert!((expected - actual).abs() < 0.0005, "expected {expected}, got {actual}");
}

fn with_advance_delta(glyph: GlyphInfo, delta: f64) -> GlyphInfo {
    GlyphInfo::with_offset(glyph.glyph_index, glyph.glyph_cluster, glyph.glyph_advance + delta, glyph.glyph_offset)
}

#[test]
fn dispose_is_idempotent() {
    let _scope = TextTestScope::new();
    let buffer = shape_ascii(ASCII_TEXT);

    let _ = buffer.total_glyph_advance(); // prime cluster cache.

    buffer.dispose();
    buffer.dispose();
    buffer.dispose();
}

#[test]
fn split_children_survive_parent_disposal() {
    let _scope = TextTestScope::new();
    let parent = shape_ascii(ASCII_TEXT);

    let total_before = parent.total_glyph_advance();
    let split = parent.split(parent.text().len() as i32 / 2);

    let first = split.first.unwrap();
    let second = split.second.unwrap();

    // Prime sibling caches via the parent's cluster-cache reference
    // (children inherit the parent's prefix sums by reference).
    let _ = first.total_glyph_advance();
    let _ = second.total_glyph_advance();

    // Release the parent first; the shared storage must stay alive for the surviving children.
    parent.dispose();

    assert_close(total_before, first.total_glyph_advance() + second.total_glyph_advance());
    assert_eq!(first.text().len(), parent.text().len() / 2);
    assert!(first.length() > 0);
    assert!(second.length() > 0);

    first.dispose();
    second.dispose();
}

#[test]
fn split_parent_survives_children_disposal() {
    let _scope = TextTestScope::new();
    let parent = shape_ascii(ASCII_TEXT);

    let total_before = parent.total_glyph_advance();
    let split = parent.split(parent.text().len() as i32 / 2);

    split.first.unwrap().dispose();
    split.second.unwrap().dispose();

    // The parent's own view must still be alive.
    assert_close(total_before, parent.total_glyph_advance());
    assert_eq!(parent.length(), ASCII_TEXT.len());
}

#[test]
fn with_bidi_level_alias_survives_original_disposal() {
    let _scope = TextTestScope::new();
    let original = shape_ascii(ASCII_TEXT);
    let total_before = original.total_glyph_advance();

    assert_eq!(original.bidi_level(), 0);

    let alias = original.with_bidi_level(2);

    original.dispose();

    assert_close(total_before, alias.total_glyph_advance());
    assert_eq!(alias.bidi_level(), 2);
    assert_eq!(alias.text().len(), original.text().len());
}

#[test]
fn with_bidi_level_returns_same_instance_when_level_matches() {
    let _scope = TextTestScope::new();
    let original = shape_ascii(ASCII_TEXT);

    let alias = original.with_bidi_level(original.bidi_level());

    assert!(Rc::ptr_eq(&original, &alias));
}

#[test]
fn indexer_mutation_invalidates_own_cluster_cache() {
    let _scope = TextTestScope::new();
    let buffer = shape_ascii(ASCII_TEXT);

    let advance_before = buffer.total_glyph_advance();

    // Mutate the leading glyph's advance via the setter.
    buffer.set(0, with_advance_delta(buffer.get(0), 50.0));

    // The cache must be rebuilt against the new glyph data.
    assert_close(advance_before + 50.0, buffer.total_glyph_advance());
}

#[test]
fn indexer_mutation_on_parent_invalidates_sibling_caches_after_split() {
    let _scope = TextTestScope::new();
    let parent = shape_ascii(ASCII_TEXT);
    let split = parent.split(parent.text().len() as i32 / 2);

    let first = split.first.unwrap();
    let second = split.second.unwrap();

    // Prime both children's views of the inherited cluster cache.
    let first_before = first.total_glyph_advance();
    let second_before = second.total_glyph_advance();

    // Mutate the parent's first glyph: this lives in the first child's slice but
    // bumps the generation counter of the shared storage so
    // the second child also sees the change (and rebuilds if needed).
    parent.set(0, with_advance_delta(parent.get(0), 25.0));

    assert_close(first_before + 25.0, first.total_glyph_advance());

    // The second child's range doesn't include glyph 0 so its advance is unchanged,
    // but its cache must still have been invalidated/rebuilt without error.
    assert_close(second_before, second.total_glyph_advance());
}

#[test]
fn indexer_mutation_on_child_invalidates_parent_and_sibling() {
    let _scope = TextTestScope::new();
    let parent = shape_ascii(ASCII_TEXT);
    let split = parent.split(parent.text().len() as i32 / 2);

    let first = split.first.unwrap();
    let second = split.second.unwrap();

    let parent_before = parent.total_glyph_advance();
    let _ = first.total_glyph_advance();
    let _ = second.total_glyph_advance();

    first.set(0, with_advance_delta(first.get(0), 17.0));

    assert_close(parent_before + 17.0, parent.total_glyph_advance());
}

#[test]
fn indexer_mutation_invalidates_with_bidi_level_alias_cache() {
    let _scope = TextTestScope::new();
    let original = shape_ascii(ASCII_TEXT);
    let alias = original.with_bidi_level(2);

    let alias_before = alias.total_glyph_advance();

    original.set(0, with_advance_delta(original.get(0), 33.0));

    assert_close(alias_before + 33.0, alias.total_glyph_advance());
}

// Regression: when a buffer is mutated *before* it is split / aliased,
// the shared generation is already > 0 by the time the alias
// inherits the cluster cache. The alias must copy that
// generation; otherwise its stamp stays at 0, the cache sees a mismatch on
// first access and rebuilds instead of reusing the parent's arrays — silently
// defeating the cached-split fast path.
#[test]
fn split_children_share_parent_cluster_cache_when_parent_was_mutated_before_split() {
    let _scope = TextTestScope::new();
    let parent = shape_ascii(ASCII_TEXT);

    // Mutate first so the shared generation is non-zero before the cluster cache is built.
    parent.set(0, with_advance_delta(parent.get(0), 10.0));

    // Prime the parent's cluster cache against the bumped generation.
    let _ = parent.total_glyph_advance();

    let parent_prefix = parent.cluster_prefix().unwrap();

    let split = parent.split(parent.text().len() as i32 / 2);

    let first = split.first.unwrap();
    let second = split.second.unwrap();

    // Touching the child's metrics must reuse the parent's prefix array, not rebuild a fresh one.
    let _ = first.total_glyph_advance();
    let _ = second.total_glyph_advance();

    assert!(Rc::ptr_eq(&parent_prefix, &first.cluster_prefix().unwrap()));
    assert!(Rc::ptr_eq(&parent_prefix, &second.cluster_prefix().unwrap()));
}

#[test]
fn with_bidi_level_alias_shares_cluster_cache_when_original_was_mutated_before_alias() {
    let _scope = TextTestScope::new();
    let original = shape_ascii(ASCII_TEXT);

    original.set(0, with_advance_delta(original.get(0), 7.0));

    let _ = original.total_glyph_advance();

    let original_prefix = original.cluster_prefix().unwrap();

    let alias = original.with_bidi_level(2);

    let _ = alias.total_glyph_advance();

    assert!(Rc::ptr_eq(&original_prefix, &alias.cluster_prefix().unwrap()));
}
