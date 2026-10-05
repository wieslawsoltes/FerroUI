/// A glyph's bounding box in font design units, as stored in the 'glyf' glyph
/// header. Y is up-positive, so `y_min` is the bottom edge and `y_max` the top.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct GlyphBounds {
    pub x_min: i16,
    pub y_min: i16,
    pub x_max: i16,
    pub y_max: i16,
}

impl GlyphBounds {
    pub const fn new(x_min: i16, y_min: i16, x_max: i16, y_max: i16) -> Self {
        Self { x_min, y_min, x_max, y_max }
    }

    /// Width of the bounding box in design units (never negative).
    pub fn width(&self) -> i32 {
        (self.x_max as i32 - self.x_min as i32).max(0)
    }

    /// Height of the bounding box in design units (never negative).
    pub fn height(&self) -> i32 {
        (self.y_max as i32 - self.y_min as i32).max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_and_height_are_extents_for_a_well_formed_header() {
        let bounds = GlyphBounds::new(10, 20, 110, 220);

        assert_eq!(bounds.width(), 100);
        assert_eq!(bounds.height(), 200);
    }

    #[test]
    fn width_and_height_clamp_to_zero_when_max_is_below_min() {
        // A malformed glyf header (xMax < xMin / yMax < yMin) must not produce a
        // negative extent that wraps to a huge value when narrowed to 16 unsigned bits.
        let bounds = GlyphBounds::new(100, 100, 50, 40);

        assert_eq!(bounds.width(), 0);
        assert_eq!(bounds.height(), 0);
    }

    #[test]
    fn maximum_extent_for_int16_coordinates_fits_in_ushort() {
        // The widest possible extent is 65535, so narrowing the clamped value never overflows.
        let bounds = GlyphBounds::new(i16::MIN, i16::MIN, i16::MAX, i16::MAX);

        assert_eq!(bounds.width(), u16::MAX as i32);
        assert_eq!(bounds.height(), u16::MAX as i32);
    }

    #[test]
    fn extent_is_clamped() {
        assert_eq!(GlyphBounds::new(10, -5, 110, 95).width(), 100);
        assert_eq!(GlyphBounds::new(10, -5, 110, 95).height(), 100);
        assert_eq!(GlyphBounds::new(100, 100, 0, 0).width(), 0);
        assert_eq!(GlyphBounds::new(100, 100, 0, 0).height(), 0);
        assert_eq!(GlyphBounds::new(i16::MIN, 0, i16::MAX, 0).width(), 65535);
    }
}
