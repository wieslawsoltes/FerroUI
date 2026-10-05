/// The kind of data a glyph is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GlyphDrawingType {
    /// glyf / CFF / CFF2 outlines.
    Outline,
    /// COLR/CPAL color layers.
    ColorLayers,
    /// The SVG table.
    Svg,
    /// sbix / CBDT / EBDT bitmaps.
    Bitmap,
}
