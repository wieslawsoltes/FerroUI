//! A block of pixel memory with a known format.


use super::pixel_format_transcoder::PixelFormatTranscoder;
use crate::platform::{AlphaFormat, PixelFormat};
use crate::PixelSize;

/// A zero-initialised block of pixel memory with rows padded to 4 bytes.
pub(crate) struct BitmapMemory {
    data: Vec<u8>,
    size: PixelSize,
    row_bytes: i32,
    format: PixelFormat,
    alpha_format: AlphaFormat,
}

impl BitmapMemory {
    /// Allocates memory for `size` pixels of `format`.
    ///
    /// Panics when the size is negative or too large.
    pub fn new(format: PixelFormat, alpha_format: AlphaFormat, size: PixelSize) -> Self {
        assert!(size.width >= 0 && size.height >= 0, "invalid bitmap size {}x{}", size.width, size.height);

        let bytes_per_pixel = (format.bits_per_pixel() as i32 + 7) / 8;

        let row_bytes = size
            .width
            .checked_mul(bytes_per_pixel)
            .and_then(|bytes| bytes.checked_add(3))
            .map(|bytes| 4 * (bytes / 4))
            .unwrap_or_else(|| panic!("bitmap too wide: {} pixels of {format}", size.width));

        let memory_size = row_bytes
            .checked_mul(size.height)
            .unwrap_or_else(|| panic!("bitmap too large: {}x{} pixels of {format}", size.width, size.height));

        Self { data: vec![0; memory_size as usize], size, row_bytes, format, alpha_format }
    }

    /// Releases the pixel memory; the accessors return empty slices afterwards.
    pub fn dispose(&mut self) {
        self.data = Vec::new();
    }

    /// The pixel memory: `row_bytes() * size().height` bytes.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// The pixel memory: `row_bytes() * size().height` bytes.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// The size in pixels.
    pub fn size(&self) -> PixelSize {
        self.size
    }

    /// The number of bytes per row.
    pub fn row_bytes(&self) -> i32 {
        self.row_bytes
    }

    /// The pixel format.
    pub fn format(&self) -> PixelFormat {
        self.format
    }

    /// The alpha format.
    pub fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    /// Copies the pixels into `buffer` as RGBA8888 with the given alpha format,
    /// rows `row_bytes` bytes apart.
    pub fn copy_to_rgba(&self, alpha_format: AlphaFormat, buffer: &mut [u8], row_bytes: i32) {
        PixelFormatTranscoder::transcode(
            &self.data,
            self.size,
            self.row_bytes,
            self.format,
            self.alpha_format,
            buffer,
            row_bytes,
            PixelFormat::RGBA8888,
            alpha_format,
        );
    }
}

// Additional coverage (the reference suite has no tests dedicated to this file).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PixelFormats;

    #[test]
    fn rows_are_padded_to_four_bytes() {
        let cases = [
            (PixelFormats::BLACK_WHITE, 10, 12),
            (PixelFormats::GRAY2, 3, 4),
            (PixelFormats::GRAY8, 5, 8),
            (PixelFormats::GRAY16, 3, 8),
            (PixelFormats::RGB24, 3, 12),
            (PixelFormats::RGB24, 5, 16),
            (PixelFormats::BGRA8888, 3, 12),
            (PixelFormats::RGBA64, 3, 24),
            (PixelFormats::RGBA8888, 0, 0),
        ];

        for (format, width, row_bytes) in cases {
            let memory = BitmapMemory::new(format, AlphaFormat::Premul, PixelSize::new(width, 3));

            assert_eq!(row_bytes, memory.row_bytes(), "{format} x {width}");
            assert_eq!(row_bytes as usize * 3, memory.data().len());
            assert!(memory.data().iter().all(|&b| b == 0));
            assert_eq!(format, memory.format());
            assert_eq!(AlphaFormat::Premul, memory.alpha_format());
            assert_eq!(PixelSize::new(width, 3), memory.size());
        }
    }

    #[test]
    fn copy_to_rgba_transcodes() {
        let mut memory = BitmapMemory::new(PixelFormats::BGR24, AlphaFormat::Opaque, PixelSize::new(1, 2));
        assert_eq!(4, memory.row_bytes());
        memory.data_mut().copy_from_slice(&[1, 2, 3, 0, 4, 5, 6, 0]);

        let mut buffer = [0u8; 12];
        memory.copy_to_rgba(AlphaFormat::Opaque, &mut buffer, 8);

        assert_eq!(buffer, [3, 2, 1, 255, 0, 0, 0, 0, 6, 5, 4, 255]);
    }

    #[test]
    fn copy_to_rgba_unpremultiplies() {
        let mut memory = BitmapMemory::new(PixelFormats::BGRA8888, AlphaFormat::Premul, PixelSize::new(1, 1));
        memory.data_mut().copy_from_slice(&[0, 64, 128, 128]);

        let mut buffer = [0u8; 4];
        memory.copy_to_rgba(AlphaFormat::Unpremul, &mut buffer, 4);

        // Single precision: 128 * (1 / (128 / 255)) = 254.99998 -> 254 (truncated).
        assert_eq!(buffer, [254, 127, 0, 128]);
    }

    #[test]
    fn dispose_releases_memory() {
        let mut memory = BitmapMemory::new(PixelFormats::GRAY8, AlphaFormat::Opaque, PixelSize::new(4, 4));
        memory.dispose();

        assert!(memory.data().is_empty());
    }
}
