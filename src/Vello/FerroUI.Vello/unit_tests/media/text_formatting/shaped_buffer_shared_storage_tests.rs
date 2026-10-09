//! Port of upstream's `Media/TextFormatting/ShapedBufferSharedStorageTests.cs`
//! of the Skia unit tests.
//!
//! Exercises the shared storage and the per-write generation counter that
//! `ShapedBuffer::split` and `ShapedBuffer::with_bidi_level` rely on. Each
//! test starts from a freshly shaped buffer (so the glyph and cluster arrays
//! are shared) and checks that:
//!
//! * aliases keep working after their source is disposed,
//! * `ShapedBuffer::dispose` is idempotent,
//! * indexer mutations propagate to siblings via the generation bump (i.e.
//!   nobody is left with a stale cluster cache).
//!
//! Upstream's `using var` declarations are explicit `dispose` calls at the end
//! of the scope, in reverse declaration order.

use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::text_formatting::{GlyphInfo, ShapedBuffer, TextShaper, TextShaperOptions};
use ferroui_base::media::Typeface;
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

const ASCII_TEXT: &str = "The quick brown fox";

#[test]
fn dispose_is_idempotent() {
    let _scope = start();

    let buffer = shape_ascii(ASCII_TEXT);
    let _ = buffer.total_glyph_advance(); // prime cluster cache so refs are non-null.

    buffer.dispose();
    buffer.dispose();
    buffer.dispose();
}

#[test]
fn split_children_survive_parent_disposal() {
    let _scope = start();

    let parent = shape_ascii(ASCII_TEXT);
    let total_before = parent.total_glyph_advance();

    let split = parent.split(parent.text().len() as i32 / 2);
    let first = split.first.unwrap();
    let second = split.second.unwrap();

    // Prime sibling caches via the parent's cluster-cache reference
    // (children inherit the parent's prefix sums by ref).
    let _ = first.total_glyph_advance();
    let _ = second.total_glyph_advance();

    // Release the parent's references first; the shared holders
    // must keep the arrays alive for the surviving children.
    parent.dispose();

    assert_equal_precision(total_before, first.total_glyph_advance() + second.total_glyph_advance(), 3);
    assert_eq!(parent.text().len() / 2, first.text().len());
    assert!(first.length() > 0);
    assert!(second.length() > 0);

    first.dispose();
    second.dispose();
}

#[test]
fn split_parent_survives_children_disposal() {
    let _scope = start();

    let parent = shape_ascii(ASCII_TEXT);
    let total_before = parent.total_glyph_advance();

    let split = parent.split(parent.text().len() as i32 / 2);
    split.first.unwrap().dispose();
    split.second.unwrap().dispose();

    // Parent's own refs must still hold the arrays alive.
    assert_equal_precision(total_before, parent.total_glyph_advance(), 6);

    parent.dispose();
}

#[test]
fn with_bidi_level_alias_survives_original_disposal() {
    let _scope = start();

    let original = shape_ascii(ASCII_TEXT);
    let total_before = original.total_glyph_advance();
    assert_eq!(0, original.bidi_level());

    let alias = original.with_bidi_level(2);

    original.dispose();

    assert_equal_precision(total_before, alias.total_glyph_advance(), 6);
    assert_eq!(2, alias.bidi_level());
    assert_eq!(original.text().len(), alias.text().len());

    alias.dispose();
}

#[test]
fn with_bidi_level_returns_same_instance_when_level_matches() {
    let _scope = start();

    let original = shape_ascii(ASCII_TEXT);
    let alias = original.with_bidi_level(original.bidi_level());
    assert!(Rc::ptr_eq(&original, &alias));

    original.dispose();
}

#[test]
fn indexer_mutation_invalidates_own_cluster_cache() {
    let _scope = start();

    let buffer = shape_ascii(ASCII_TEXT);

    let advance_before = buffer.total_glyph_advance();
    let original = buffer.get(0);

    // Mutate the leading glyph's advance via the indexer setter.
    const DELTA: f64 = 50.0;
    buffer.set(
        0,
        GlyphInfo::with_offset(
            original.glyph_index,
            original.glyph_cluster,
            original.glyph_advance + DELTA,
            original.glyph_offset,
        ),
    );

    // The cache must be rebuilt against the new glyph data.
    assert_equal_precision(advance_before + DELTA, buffer.total_glyph_advance(), 3);

    buffer.dispose();
}

#[test]
fn indexer_mutation_on_parent_invalidates_sibling_caches_after_split() {
    let _scope = start();

    let parent = shape_ascii(ASCII_TEXT);

    let split_index = parent.text().len() as i32 / 2;
    let split = parent.split(split_index);
    let first = split.first.unwrap();
    let second = split.second.unwrap();

    // Prime both children's views of the inherited cluster cache.
    let first_before = first.total_glyph_advance();
    let second_before = second.total_glyph_advance();

    // Mutate parent[0]: this lives in the first child's slice but
    // bumps the generation counter on the shared glyph holder so
    // the second child also sees the change (and rebuilds if needed).
    const DELTA: f64 = 25.0;
    let first0 = parent.get(0);
    parent.set(
        0,
        GlyphInfo::with_offset(first0.glyph_index, first0.glyph_cluster, first0.glyph_advance + DELTA, first0.glyph_offset),
    );

    assert_equal_precision(first_before + DELTA, first.total_glyph_advance(), 3);
    // Second child's range doesn't include glyph 0 so its advance is unchanged,
    // but its cache must still have been invalidated/rebuilt without error.
    assert_equal_precision(second_before, second.total_glyph_advance(), 3);

    second.dispose();
    first.dispose();
    parent.dispose();
}

#[test]
fn indexer_mutation_on_child_invalidates_parent_and_sibling() {
    let _scope = start();

    let parent = shape_ascii(ASCII_TEXT);

    let split_index = parent.text().len() as i32 / 2;
    let split = parent.split(split_index);
    let first = split.first.unwrap();
    let second = split.second.unwrap();

    let parent_before = parent.total_glyph_advance();
    let _ = first.total_glyph_advance();
    let _ = second.total_glyph_advance();

    const DELTA: f64 = 17.0;
    let glyph = first.get(0);
    first.set(
        0,
        GlyphInfo::with_offset(glyph.glyph_index, glyph.glyph_cluster, glyph.glyph_advance + DELTA, glyph.glyph_offset),
    );

    assert_equal_precision(parent_before + DELTA, parent.total_glyph_advance(), 3);

    second.dispose();
    first.dispose();
    parent.dispose();
}

#[test]
fn indexer_mutation_invalidates_with_bidi_level_alias_cache() {
    let _scope = start();

    let original = shape_ascii(ASCII_TEXT);
    let alias = original.with_bidi_level(2);

    let alias_before = alias.total_glyph_advance();

    const DELTA: f64 = 33.0;
    let glyph = original.get(0);
    original.set(
        0,
        GlyphInfo::with_offset(glyph.glyph_index, glyph.glyph_cluster, glyph.glyph_advance + DELTA, glyph.glyph_offset),
    );

    assert_equal_precision(alias_before + DELTA, alias.total_glyph_advance(), 3);

    alias.dispose();
    original.dispose();
}

// Regression: when a buffer is mutated *before* it is Split / aliased,
// the parent's cache generation is already > 0 by the time the alias
// inherits the cluster cache. The alias constructor must copy that
// generation onto the child; otherwise the child's stamp stays at 0,
// the cluster cache sees a mismatch on first access and rebuilds a
// fresh cache instead of reusing the parent's arrays — silently
// defeating the cached-split fast path.
#[test]
fn split_children_share_parent_cluster_cache_when_parent_was_mutated_before_split() {
    let _scope = start();

    let parent = shape_ascii(ASCII_TEXT);

    // Mutate first so the shared glyph holder's generation is
    // non-zero before the cluster cache is built.
    let first0 = parent.get(0);
    parent.set(
        0,
        GlyphInfo::with_offset(first0.glyph_index, first0.glyph_cluster, first0.glyph_advance + 10.0, first0.glyph_offset),
    );

    // Prime the parent's cluster cache against the bumped generation.
    let _ = parent.total_glyph_advance();
    let parent_prefix = parent.cluster_prefix();
    assert!(parent_prefix.is_some());
    let parent_prefix = parent_prefix.unwrap();

    let split = parent.split(parent.text().len() as i32 / 2);
    let first = split.first.unwrap();
    let second = split.second.unwrap();

    // Touching the child's metrics must reuse the parent's prefix
    // array, not rebuild a fresh one.
    let _ = first.total_glyph_advance();
    let _ = second.total_glyph_advance();

    assert!(first.cluster_prefix().is_some_and(|prefix| Rc::ptr_eq(&parent_prefix, &prefix)));
    assert!(second.cluster_prefix().is_some_and(|prefix| Rc::ptr_eq(&parent_prefix, &prefix)));

    second.dispose();
    first.dispose();
    parent.dispose();
}

#[test]
fn with_bidi_level_alias_shares_cluster_cache_when_original_was_mutated_before_alias() {
    let _scope = start();

    let original = shape_ascii(ASCII_TEXT);

    let first0 = original.get(0);
    original.set(
        0,
        GlyphInfo::with_offset(first0.glyph_index, first0.glyph_cluster, first0.glyph_advance + 7.0, first0.glyph_offset),
    );

    let _ = original.total_glyph_advance();
    let original_prefix = original.cluster_prefix();
    assert!(original_prefix.is_some());
    let original_prefix = original_prefix.unwrap();

    let alias = original.with_bidi_level(2);

    let _ = alias.total_glyph_advance();

    assert!(alias.cluster_prefix().is_some_and(|prefix| Rc::ptr_eq(&original_prefix, &prefix)));

    alias.dispose();
    original.dispose();
}

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
    TextShaper::current().shape_text(&ReadOnlyMemory::from_str(text), &options)
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
