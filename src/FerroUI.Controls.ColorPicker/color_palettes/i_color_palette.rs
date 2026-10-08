use ferroui_base::media::Color;

/// Interface to define a color palette.
pub trait IColorPalette {
    /// Gets the total number of colors in this palette.
    /// A color is not necessarily a single value and may be composed of several shades.
    ///
    /// Represents total columns in a table.
    fn color_count(&self) -> i32;

    /// Gets the total number of shades for each color in this palette.
    /// Shades are usually a variation of the color lightening or darkening it.
    ///
    /// Represents total rows in a table.
    fn shade_count(&self) -> i32;

    /// Gets a color in the palette by index.
    ///
    /// `color_index` is the index of the color in the palette; the index must
    /// be between zero and [`color_count`](Self::color_count).
    /// `shade_index` is the index of the color shade in the palette; the
    /// index must be between zero and [`shade_count`](Self::shade_count).
    fn get_color(&self, color_index: i32, shade_index: i32) -> Color;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IColorPalette {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
