/// Controls the performance and quality of bitmap scaling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BitmapInterpolationMode {
    #[default]
    Unspecified = 0,
    None = 1,
    LowQuality = 2,
    MediumQuality = 3,
    HighQuality = 4,
}
