/// Specifies the unit type of either a text decoration offset or thickness value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextDecorationUnit {
    /// A unit value that is relative to the font used for the text decoration.
    /// If the decoration spans multiple fonts, an average recommended value is
    /// calculated. This is the default value.
    #[default]
    FontRecommended,
    /// A unit value that is relative to the em size of the font. The value of
    /// the offset or thickness is equal to the offset or thickness value
    /// multiplied by the font em size.
    FontRenderingEmSize,
    /// A unit value that is expressed in pixels.
    Pixel,
}
