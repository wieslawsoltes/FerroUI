/// The font metrics is holding information about a font's ascent, descent, etc.
/// in design em units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FontMetrics {
    /// The font design units per em.
    pub design_em_height: u16,
    /// Whether all glyphs have the same advance.
    pub is_fixed_pitch: bool,
    /// The recommended distance above the baseline in design em size.
    pub ascent: i32,
    /// The recommended distance under the baseline in design em size.
    pub descent: i32,
    /// The recommended additional space between two lines of text in design em size.
    pub line_gap: i32,
    /// A value that indicates the distance of the underline from the baseline in design em size.
    pub underline_position: i32,
    /// A value that indicates the thickness of the underline in design em size.
    pub underline_thickness: i32,
    /// A value that indicates the distance of the strikethrough from the baseline in design em size.
    pub strikethrough_position: i32,
    /// A value that indicates the thickness of the strikethrough in design em size.
    pub strikethrough_thickness: i32,
}

impl FontMetrics {
    /// The line spacing in the design em size (`descent - ascent + line_gap`).
    #[inline]
    pub fn line_spacing(&self) -> i32 {
        self.descent - self.ascent + self.line_gap
    }
}
