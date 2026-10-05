/// Represents the metrics for a single glyph in design units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlyphMetrics {
    /// Distance from the x-origin to the left extremum of the glyph.
    pub x_bearing: i32,
    /// Distance from the top extremum of the glyph to the y-origin.
    pub y_bearing: i32,
    /// Distance from the left extremum of the glyph to the right extremum.
    pub width: u16,
    /// Distance from the top extremum of the glyph to the bottom extremum.
    pub height: u16,
    /// The horizontal advance of the glyph.
    pub advance_width: u16,
    /// The vertical advance of the glyph.
    pub advance_height: u16,
    /// The horizontal offset of the glyph.
    pub x_offset: u16,
    /// The vertical offset of the glyph.
    pub y_offset: u16,
    /// The x coordinate of the vertical origin.
    pub vertical_origin_x: u16,
    /// The y coordinate of the vertical origin.
    pub vertical_origin_y: u16,
}
