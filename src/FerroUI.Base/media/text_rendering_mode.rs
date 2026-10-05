/// Specifies how text glyphs are rendered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TextRenderingMode {
    #[default]
    Unspecified = 0,
    SubpixelAntialias = 1,
    Antialias = 2,
    Alias = 3,
}
