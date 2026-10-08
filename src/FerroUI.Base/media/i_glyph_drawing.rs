use crate::media::{DrawingContext, GlyphDrawingType};
use crate::{Point, Rect};

/// Represents a glyph that knows how to draw itself into a [`DrawingContext`].
///
/// Implementations are produced by the per-format renderers (color layers from
/// COLR/CPAL, bitmap strikes from sbix/CBDT, etc.) and shielded behind this contract
/// so callers can render any color glyph without caring which font-table format
/// produced it. For plain outline glyphs, use `GlyphTypeface::get_glyph_outline`
/// instead: outlines are returned as a geometry rather than via this interface.
pub trait IGlyphDrawing {
    /// Gets the format this drawing was produced from. Callers can use this to
    /// branch on rendering behaviour without downcasting.
    fn type_(&self) -> GlyphDrawingType;

    /// Gets the axis-aligned bounding rectangle of the drawing, in drawing-space
    /// coordinates (Y-down).
    ///
    /// The rectangle is relative to the drawing's local origin. To get bounds at a
    /// specific paint location, translate by the origin passed to [`IGlyphDrawing::draw`].
    /// Implementations are expected to compute this once (e.g. from the font's clip
    /// box or layer extents) and cache it.
    fn bounds(&self) -> Rect;

    /// Draws the glyph into `context` at `origin`.
    ///
    /// `origin` is the drawing-space point (Y-down) at which the glyph's local origin
    /// should land. For text rendering this is typically the pen position on the
    /// baseline. Implementations apply the Y-flip from font-space (Y-up) internally, so
    /// callers don't need to flip `origin` themselves.
    fn draw(&self, context: &mut DrawingContext<'_>, origin: Point);
}
