use crate::media::imaging::{JpegBitmapEncoderOptions, PngBitmapEncoderOptions};

/// Options that select and configure the encoder used to save a bitmap.
///
/// The set of encoders is closed: one variant per encoder options type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitmapEncoderOptions {
    /// Encodes as JPEG.
    Jpeg(JpegBitmapEncoderOptions),
    /// Encodes as PNG.
    Png(PngBitmapEncoderOptions),
}

impl From<JpegBitmapEncoderOptions> for BitmapEncoderOptions {
    fn from(value: JpegBitmapEncoderOptions) -> Self {
        Self::Jpeg(value)
    }
}

impl From<PngBitmapEncoderOptions> for BitmapEncoderOptions {
    fn from(value: PngBitmapEncoderOptions) -> Self {
        Self::Png(value)
    }
}
