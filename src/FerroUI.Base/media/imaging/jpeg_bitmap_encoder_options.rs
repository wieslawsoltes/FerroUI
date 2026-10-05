/// Options for the JPEG bitmap encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JpegBitmapEncoderOptions {
    /// The quality of the encoded image, from 0 to 100.
    pub quality: i32,
}

impl JpegBitmapEncoderOptions {
    /// The default options: full quality.
    pub const DEFAULT: JpegBitmapEncoderOptions = JpegBitmapEncoderOptions { quality: 100 };
}

impl Default for JpegBitmapEncoderOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}
