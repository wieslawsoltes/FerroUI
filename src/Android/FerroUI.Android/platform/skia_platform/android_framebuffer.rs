//! The buffer of the native window, locked for a frame drawn in software.
//!
//! The declarations of the NDK the reference has in this file are in
//! `interop/ndk.rs`.

use ferroui_base::platform::{AlphaFormat, PixelFormat};

/// The formats of the buffer of a window (`WINDOW_FORMAT_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
#[allow(dead_code)]
pub(crate) enum AndroidPixelFormat {
    Rgba8888 = 1,
    Rgbx8888 = 2,
    Rgb565 = 4,
}

/// The pixel format, the alpha format and the bytes of a row of a locked
/// buffer of the given window format and stride (in pixels).
pub(crate) fn buffer_layout(format: i32, stride: i32) -> (PixelFormat, AlphaFormat, i32) {
    if format == AndroidPixelFormat::Rgb565 as i32 {
        (PixelFormat::RGB565, AlphaFormat::Opaque, stride * 2)
    } else {
        (PixelFormat::RGBA8888, AlphaFormat::Premul, stride * 4)
    }
}

#[cfg(target_os = "android")]
pub(crate) use imp::{AndroidFramebuffer, DiscardedFramebuffer};

#[cfg(target_os = "android")]
mod imp {
    use super::buffer_layout;
    use crate::interop::ndk::{LockedBuffer, NativeWindow};
    use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
    use ferroui_base::{PixelSize, Vector};
    use std::cell::RefCell;

    /// The locked buffer of a native window. Disposing it posts the buffer
    /// to the screen.
    pub(crate) struct AndroidFramebuffer {
        buffer: RefCell<Option<LockedBuffer>>,
        size: PixelSize,
        row_bytes: i32,
        dpi: Vector,
        format: PixelFormat,
        alpha_format: AlphaFormat,
    }

    impl AndroidFramebuffer {
        /// Locks the window; `None` when the window cannot be locked.
        pub fn new(window: &NativeWindow, scaling: f64) -> Option<AndroidFramebuffer> {
            let size = PixelSize::new(window.width(), window.height());
            let buffer = window.lock()?;

            let (format, alpha_format, row_bytes) = buffer_layout(buffer.format, buffer.stride);
            // The buffer the system hands out can be smaller than the size the window reported
            // a moment ago (the surface is being resized): the frame is the smaller of the two,
            // so that every row written is inside the buffer.
            let size = PixelSize::new(size.width.min(buffer.width), size.height.min(buffer.height));

            Some(AndroidFramebuffer {
                buffer: RefCell::new(Some(buffer)),
                size,
                row_bytes,
                dpi: Vector::new(96.0, 96.0) * scaling,
                format,
                alpha_format,
            })
        }
    }

    impl ILockedFramebuffer for AndroidFramebuffer {
        fn address(&self) -> *mut u8 {
            self.buffer.borrow().as_ref().map_or(std::ptr::null_mut(), LockedBuffer::bits)
        }

        fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
            let buffer = self.buffer.borrow();
            let Some(buffer) = buffer.as_ref() else {
                access(&mut []);
                return;
            };
            let length = self.row_bytes.max(0) as usize * self.size.height.max(0) as usize;
            // SAFETY: the locked buffer has `stride` pixels a row and at least
            // `size.height` rows (the height is capped by the height of the buffer in
            // `new`), `row_bytes` is the stride in bytes, and the buffer stays locked
            // while the borrow of the cell lives; nothing else reads or writes it.
            let data = unsafe { std::slice::from_raw_parts_mut(buffer.bits(), length) };
            access(data);
        }

        fn size(&self) -> PixelSize {
            self.size
        }

        fn row_bytes(&self) -> i32 {
            self.row_bytes
        }

        fn dpi(&self) -> Vector {
            self.dpi
        }

        fn format(&self) -> PixelFormat {
            self.format
        }

        fn alpha_format(&self) -> AlphaFormat {
            self.alpha_format
        }

        fn dispose(&self) {
            // Unlocks and posts.
            let buffer = self.buffer.borrow_mut().take();
            drop(buffer);
        }
    }

    /// A frame that goes nowhere: what a render target locks when the
    /// surface is gone (it was destroyed between the frame being scheduled
    /// and drawn). The reference throws there, on the render thread; a
    /// frame for a surface that no longer exists is drawn into memory here
    /// and dropped.
    pub(crate) struct DiscardedFramebuffer {
        data: RefCell<Vec<u8>>,
        dpi: Vector,
    }

    impl DiscardedFramebuffer {
        pub fn new(scaling: f64) -> Self {
            Self { data: RefCell::new(vec![0; 4]), dpi: Vector::new(96.0, 96.0) * scaling }
        }
    }

    impl ILockedFramebuffer for DiscardedFramebuffer {
        fn address(&self) -> *mut u8 {
            self.data.borrow_mut().as_mut_ptr()
        }

        fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
            access(&mut self.data.borrow_mut());
        }

        fn size(&self) -> PixelSize {
            PixelSize::new(1, 1)
        }

        fn row_bytes(&self) -> i32 {
            4
        }

        fn dpi(&self) -> Vector {
            self.dpi
        }

        fn format(&self) -> PixelFormat {
            PixelFormat::RGBA8888
        }

        fn alpha_format(&self) -> AlphaFormat {
            AlphaFormat::Premul
        }

        fn dispose(&self) {}
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the framebuffer.
    use super::*;

    #[test]
    fn a_565_window_is_opaque_with_two_bytes_a_pixel() {
        assert_eq!(
            buffer_layout(AndroidPixelFormat::Rgb565 as i32, 1088),
            (PixelFormat::RGB565, AlphaFormat::Opaque, 2176)
        );
    }

    #[test]
    fn every_other_window_is_premultiplied_rgba() {
        assert_eq!(
            buffer_layout(AndroidPixelFormat::Rgba8888 as i32, 1088),
            (PixelFormat::RGBA8888, AlphaFormat::Premul, 4352)
        );
        assert_eq!(
            buffer_layout(AndroidPixelFormat::Rgbx8888 as i32, 720),
            (PixelFormat::RGBA8888, AlphaFormat::Premul, 2880)
        );
    }
}
