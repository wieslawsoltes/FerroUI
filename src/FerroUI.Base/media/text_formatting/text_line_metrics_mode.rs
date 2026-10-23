/// Chooses the font metrics that contribute to a paragraph's line boxes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextLineMetricsMode {
    /// Use each actual glyph typeface, splitting its line gap above and below
    /// the baseline. This is the default behavior of the shared formatter.
    #[default]
    GlyphTypeface,
    /// Use the regular face of each originally requested font family. Glyph
    /// fallback does not change its line box; its line gap lies below the
    /// baseline, and TextLineBounds is applied before combining runs. Drawable
    /// objects contribute their full height and baseline independently.
    FontFamily,
}
