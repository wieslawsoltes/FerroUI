use super::{AlphaFormat, IDrawingContextImpl, ILockedFramebuffer, PixelFormat};
use crate::media::imaging::BitmapEncoderOptions;
use crate::{PixelSize, Vector};
use std::any::Any;
use std::io::Write;
use std::rc::Rc;

/// Defines the platform-specific interface for a bitmap.
pub trait IBitmapImpl {
    /// The dots per inch (DPI) of the image.
    fn dpi(&self) -> Vector;

    /// The size of the bitmap, in device pixels.
    fn pixel_size(&self) -> PixelSize;

    /// Version of the pixel data: incremented whenever the content changes.
    fn version(&self) -> i32;

    /// Saves the bitmap to a stream, encoded as selected by `options`.
    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> std::io::Result<()>;

    /// Releases the bitmap's resources.
    fn dispose(&self);

    /// Lets the backend recover its concrete type.
    fn as_any(&self) -> &dyn Any;

    /// The bitmap viewed as [`IReadableBitmapImpl`], when its pixels can be
    /// read.
    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        None
    }
}

/// A bitmap shared between the UI thread and the render thread: what a
/// bitmap object of the UI side holds and what a batch carries. (A layer of
/// a drawing context is a bitmap too, but stays on the render thread.)
pub type SharedBitmapImpl = dyn IBitmapImpl + Send + Sync;

/// A bitmap whose pixels can be read.
pub trait IReadableBitmapImpl: IBitmapImpl {
    /// The pixel format, if known.
    fn format(&self) -> Option<PixelFormat>;

    /// The alpha format, if known.
    fn alpha_format(&self) -> Option<AlphaFormat>;

    /// Locks the pixels for reading (and writing, for writeable bitmaps).
    fn lock(&self) -> Rc<dyn ILockedFramebuffer>;
}

/// Defines the platform-specific interface for a writeable bitmap.
pub trait IWriteableBitmapImpl: IReadableBitmapImpl + Send + Sync {}

/// Defines the platform-specific interface for a bitmap that can be drawn
/// into.
pub trait IRenderTargetBitmapImpl: IReadableBitmapImpl + Send + Sync {
    /// Creates a drawing context that draws into the bitmap.
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl>;
}
