use std::cmp::Ordering;

use crate::Vector;

/// Represents a single glyph.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlyphInfo {
    /// The index of the glyph in the associated font.
    pub glyph_index: u16,
    /// The index (UTF-16 code unit, relative to the shaped text) of the first
    /// character that was used to create this glyph.
    pub glyph_cluster: i32,
    /// The glyph advance.
    pub glyph_advance: f64,
    /// The glyph offset.
    pub glyph_offset: Vector,
}

impl GlyphInfo {
    #[inline]
    pub const fn new(glyph_index: u16, glyph_cluster: i32, glyph_advance: f64) -> Self {
        Self { glyph_index, glyph_cluster, glyph_advance, glyph_offset: Vector::new(0.0, 0.0) }
    }

    #[inline]
    pub const fn with_offset(glyph_index: u16, glyph_cluster: i32, glyph_advance: f64, glyph_offset: Vector) -> Self {
        Self { glyph_index, glyph_cluster, glyph_advance, glyph_offset }
    }

    /// Orders glyphs by ascending cluster.
    pub(crate) fn cluster_ascending_comparer(x: &GlyphInfo, y: &GlyphInfo) -> Ordering {
        x.glyph_cluster.cmp(&y.glyph_cluster)
    }

    /// Orders glyphs by descending cluster.
    pub(crate) fn cluster_descending_comparer(x: &GlyphInfo, y: &GlyphInfo) -> Ordering {
        y.glyph_cluster.cmp(&x.glyph_cluster)
    }
}
