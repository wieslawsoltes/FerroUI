/// Specifies whether a compression operation emphasizes speed or size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompressionLevel {
    /// Balances compression and speed.
    #[default]
    Optimal = 0,
    /// Completes as quickly as possible, even if the result is not optimally
    /// compressed.
    Fastest = 1,
    /// Performs no compression.
    NoCompression = 2,
    /// Produces the smallest result, even if the operation takes longer.
    SmallestSize = 3,
}

/// Options for the PNG bitmap encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PngBitmapEncoderOptions {
    /// The compression level of the encoded image.
    pub compression_level: CompressionLevel,
}

impl PngBitmapEncoderOptions {
    /// The default options: optimal compression.
    pub const DEFAULT: PngBitmapEncoderOptions = PngBitmapEncoderOptions { compression_level: CompressionLevel::Optimal };
}

impl Default for PngBitmapEncoderOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}
