/// Specifies how text glyphs are hinted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextHintingMode {
    /// Hinting mode is not explicitly specified. The hinting mode will be
    /// inherited from the parent or defaults will be used.
    #[default]
    Unspecified,
    /// Text is not hinted.
    None,
    /// Text is lightly hinted.
    Light,
    /// Text is strongly hinted.
    Strong,
}
