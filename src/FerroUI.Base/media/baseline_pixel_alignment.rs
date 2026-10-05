/// Specifies the baseline pixel alignment options for rendering text or graphics.
///
/// Use this enumeration to control whether the baseline of rendered content is
/// aligned to the pixel grid, which can affect visual crispness and
/// positioning. The value may influence rendering quality, especially at small
/// font sizes or when precise alignment is required.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BaselinePixelAlignment {
    /// The baseline pixel alignment is unspecified.
    #[default]
    Unspecified,
    /// The baseline is aligned to the pixel grid.
    Aligned,
    /// The baseline is not aligned to the pixel grid.
    Unaligned,
}
