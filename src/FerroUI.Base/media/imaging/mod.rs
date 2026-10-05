//! Bitmaps and imaging options.

mod bitmap_blending_mode;
mod bitmap_interpolation_mode;

pub use bitmap_blending_mode::BitmapBlendingMode;
pub use bitmap_interpolation_mode::BitmapInterpolationMode;

// --- bitmaps ---

mod bitmap;
mod bitmap_encoder_options;
mod bitmap_memory;
mod cropped_bitmap;
mod i_bitmap;
mod jpeg_bitmap_encoder_options;
mod pixel_format_readers;
mod pixel_format_transcoder;
mod pixel_format_writer;
mod png_bitmap_encoder_options;
mod render_target_bitmap;
mod writeable_bitmap;

pub use bitmap::Bitmap;
pub use bitmap_encoder_options::BitmapEncoderOptions;
pub use cropped_bitmap::{CroppedBitmap, CroppedBitmapImpl, CroppedBitmapImplExt, CroppedBitmapVTable};
pub use i_bitmap::IBitmap;
pub use jpeg_bitmap_encoder_options::JpegBitmapEncoderOptions;
pub use png_bitmap_encoder_options::{CompressionLevel, PngBitmapEncoderOptions};
pub use render_target_bitmap::RenderTargetBitmap;
pub use writeable_bitmap::WriteableBitmap;

#[cfg(test)]
mod imaging_tests;
