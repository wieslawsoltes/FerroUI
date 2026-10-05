/// Defines how text is aligned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlignment {
    /// The text is left-aligned.
    #[default]
    Left,
    /// The text is centered.
    Center,
    /// The text is right-aligned.
    Right,
    /// The beginning of the text is aligned to the edge of the available space.
    Start,
    /// The end of the text is aligned to the edge of the available space.
    End,
    /// Text alignment is inferred from the text content.
    ///
    /// When the text alignment is calculated without taking the flow direction
    /// into account this value is treated as [`TextAlignment::Left`].
    DetectFromContent,
    /// Text is justified within the available space.
    Justify,
}
