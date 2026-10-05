/// Controls the way bitmaps are drawn together.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BitmapBlendingMode {
    #[default]
    Unspecified = 0,
    SourceOver = 1,
    Source = 2,
    Destination = 3,
    DestinationOver = 4,
    SourceIn = 5,
    DestinationIn = 6,
    SourceOut = 7,
    DestinationOut = 8,
    SourceAtop = 9,
    DestinationAtop = 10,
    Xor = 11,
    Plus = 12,
    Screen = 13,
    Overlay = 14,
    Darken = 15,
    Lighten = 16,
    ColorDodge = 17,
    ColorBurn = 18,
    HardLight = 19,
    SoftLight = 20,
    Difference = 21,
    Exclusion = 22,
    Multiply = 23,
    Hue = 24,
    Saturation = 25,
    Color = 26,
    Luminosity = 27,
}
