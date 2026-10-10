//! A framebuffer over pixels this crate owns: what the reference makes
//! with `new LockedFramebuffer(address, size, rowBytes, dpi, format,
//! alphaFormat, null)` over memory of its own, to have a bitmap copied
//! into it in a format it chooses.

use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use ferroui_base::{PixelSize, Vector};
use std::cell::RefCell;

/// Premultiplied BGRA pixels at 96 DPI, in rows without padding.
pub(crate) struct PixelBuffer {
    data: RefCell<Vec<u8>>,
    size: PixelSize,
}

impl PixelBuffer {
    pub(crate) fn new(size: PixelSize) -> Self {
        let length = size.width.max(0) as usize * size.height.max(0) as usize * 4;
        Self { data: RefCell::new(vec![0; length]), size }
    }

    /// The pixels as 32 bit values in the byte order of the machine, which
    /// is how the server and the cursor library take them.
    pub(crate) fn into_pixels(self) -> Vec<u32> {
        bytes_to_pixels(&self.data.into_inner())
    }
}

/// Reads rows of four bytes a pixel as 32 bit values in the byte order of
/// the machine.
pub(crate) fn bytes_to_pixels(data: &[u8]) -> Vec<u32> {
    data.chunks_exact(4).map(|pixel| u32::from_ne_bytes([pixel[0], pixel[1], pixel[2], pixel[3]])).collect()
}

impl ILockedFramebuffer for PixelBuffer {
    fn address(&self) -> *mut u8 {
        self.data.borrow_mut().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.data.borrow_mut());
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.size.width * 4
    }

    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn format(&self) -> PixelFormat {
        PixelFormat::BGRA8888
    }

    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn dispose(&self) {}
}

#[cfg(test)]
mod tests {
    // Not from the reference: this type stands for a constructor of it.
    use super::*;

    #[test]
    fn the_buffer_has_four_bytes_a_pixel_and_reads_back_as_pixels() {
        let buffer = PixelBuffer::new(PixelSize::new(3, 2));
        assert_eq!(buffer.row_bytes(), 12);
        buffer.with_data(&mut |data| {
            assert_eq!(data.len(), 24);
            data[..4].copy_from_slice(&0x8040_2010u32.to_ne_bytes());
            data[20..].copy_from_slice(&0xffff_ffffu32.to_ne_bytes());
        });
        let pixels = buffer.into_pixels();
        assert_eq!(pixels.len(), 6);
        assert_eq!(pixels[0], 0x8040_2010);
        assert_eq!(pixels[5], 0xffff_ffff);
        assert_eq!(pixels[1], 0);
    }

    #[test]
    fn an_empty_size_has_no_pixels() {
        assert!(PixelBuffer::new(PixelSize::new(0, 5)).into_pixels().is_empty());
    }
}
