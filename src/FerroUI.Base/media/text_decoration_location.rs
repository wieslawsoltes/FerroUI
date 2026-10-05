/// Specifies the vertical position of a text decoration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextDecorationLocation {
    /// The underline position.
    #[default]
    Underline = 0,
    /// The overline position.
    Overline = 1,
    /// The strikethrough position.
    Strikethrough = 2,
    /// The baseline position.
    Baseline = 3,
}
