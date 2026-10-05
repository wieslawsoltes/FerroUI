use std::fmt;

/// The horizontal metrics of one glyph, in font design units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct HorizontalGlyphMetric {
    pub advance_width: u16,
    pub left_side_bearing: i16,
}

impl HorizontalGlyphMetric {
    #[inline]
    pub const fn new(advance_width: u16, left_side_bearing: i16) -> Self {
        Self { advance_width, left_side_bearing }
    }
}

impl fmt::Display for HorizontalGlyphMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Advance={}, LSB={}", self.advance_width, self.left_side_bearing)
    }
}
