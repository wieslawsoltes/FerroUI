use super::{AlphaFormat, PixelFormat};
use crate::{PixelSize, Vector};

/// A locked framebuffer: a block of pixel memory that can be read and
/// written until it is disposed, at which point its content is presented or
/// committed.
pub trait ILockedFramebuffer {
    /// The address of the first pixel.
    ///
    /// The memory is `row_bytes() * size().height` bytes long and stays valid
    /// until [`dispose`](Self::dispose) is called.
    fn address(&self) -> *mut u8;

    /// Gives `access` the pixel memory as a slice of
    /// `row_bytes() * size().height` bytes.
    ///
    /// This is the safe counterpart of [`address`](Self::address). The
    /// framebuffer must not be accessed again from within `access`.
    fn with_data(&self, access: &mut dyn FnMut(&mut [u8]));

    /// The framebuffer size in device pixels.
    fn size(&self) -> PixelSize;

    /// The number of bytes per row.
    fn row_bytes(&self) -> i32;

    /// The DPI of the underlying screen.
    fn dpi(&self) -> Vector;

    /// The pixel format.
    fn format(&self) -> PixelFormat;

    /// The alpha format.
    fn alpha_format(&self) -> AlphaFormat;

    /// Unlocks the framebuffer.
    fn dispose(&self);
}
