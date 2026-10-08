use crate::media::imaging::BitmapEncoderOptions;
use crate::media::IImage;
use crate::platform::IBitmapImpl;
use crate::utilities::RefCounted;
use crate::{PixelSize, Vector};
use std::io::Write;

/// Represents a bitmap image.
pub trait IBitmap: IImage {
    /// The dots per inch (DPI) of the image.
    ///
    /// Note that the backend may not support DPI information; in that case
    /// the default of 96 is reported.
    fn dpi(&self) -> Vector;

    /// The size of the bitmap, in device pixels.
    fn pixel_size(&self) -> PixelSize;

    /// The platform-specific bitmap implementation.
    fn platform_impl(&self) -> &RefCounted<crate::platform::SharedBitmapImpl>;

    /// Saves the bitmap to a stream.
    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> std::io::Result<()>;

    /// Releases the bitmap.
    fn dispose(&self);
}

/// Bitmaps compare by reference.
impl PartialEq for dyn IBitmap {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}
