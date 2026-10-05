/// The vertical metrics of one glyph, in font design units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct VerticalGlyphMetric {
    pub advance_height: u16,
    pub top_side_bearing: i16,
}

impl VerticalGlyphMetric {
    #[inline]
    pub const fn new(advance_height: u16, top_side_bearing: i16) -> Self {
        Self { advance_height, top_side_bearing }
    }
}
