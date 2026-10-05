//! Converts bitmaps between pixel formats.


use super::pixel_format_readers::{PixelFormatReader, Rgba8888Pixel};
use super::pixel_format_writer::PixelFormatWriter;
use crate::platform::{AlphaFormat, PixelFormat};
use crate::PixelSize;

/// Converts bitmaps between pixel formats.
pub(crate) struct PixelFormatTranscoder;

impl PixelFormatTranscoder {
    /// Converts `src_size` pixels of `source` (`src_format` / `src_alpha_format`,
    /// rows `src_stride` bytes apart) into `dest` (`dest_format` /
    /// `dest_alpha_format`, rows `dest_stride` bytes apart), going through an
    /// intermediate RGBA8888 buffer.
    ///
    /// Panics when either slice is too small for the bitmap.
    #[allow(clippy::too_many_arguments)]
    pub fn transcode(
        source: &[u8],
        src_size: PixelSize,
        src_stride: i32,
        src_format: PixelFormat,
        src_alpha_format: AlphaFormat,
        dest: &mut [u8],
        dest_stride: i32,
        dest_format: PixelFormat,
        dest_alpha_format: AlphaFormat,
    ) {
        let pixel_count = src_size.width as i64 * src_size.height as i64;
        assert!(pixel_count >= 0, "invalid bitmap size {}x{}", src_size.width, src_size.height);

        let mut pixels = vec![Rgba8888Pixel::default(); pixel_count as usize];

        PixelFormatReader::read(&mut pixels, source, src_size, src_stride, src_format);

        PixelFormatWriter::write(&pixels, dest, src_size, dest_stride, dest_format, dest_alpha_format, src_alpha_format);
    }
}

#[cfg(test)]
mod tests {
    use super::super::bitmap_memory::BitmapMemory;
    use super::super::pixel_format_readers::{Bgra8888PixelFormatReader, IPixelFormatReader};
    use super::super::pixel_format_writer::{IPixelFormatWriter, Rgba8888PixelFormatWriter};
    use super::*;
    use crate::platform::PixelFormats;

    #[test]
    fn should_transcode() {
        let source_memory = create_bitmap_memory();

        let mut dest_memory = BitmapMemory::new(PixelFormats::BGRA8888, AlphaFormat::Opaque, source_memory.size());

        let dest_row_bytes = dest_memory.row_bytes();
        let dest_format = dest_memory.format();
        let dest_alpha_format = dest_memory.alpha_format();

        PixelFormatTranscoder::transcode(
            source_memory.data(),
            source_memory.size(),
            source_memory.row_bytes(),
            source_memory.format(),
            source_memory.alpha_format(),
            dest_memory.data_mut(),
            dest_row_bytes,
            dest_format,
            dest_alpha_format,
        );

        let mut reader = Bgra8888PixelFormatReader::default();

        reader.reset(0);

        assert_eq!(Rgba8888Pixel::new(255, 0, 0, 0), reader.read_next(dest_memory.data()));
        assert_eq!(Rgba8888Pixel::new(0, 255, 0, 0), reader.read_next(dest_memory.data()));
        assert_eq!(Rgba8888Pixel::new(0, 0, 255, 0), reader.read_next(dest_memory.data()));
    }

    fn create_bitmap_memory() -> BitmapMemory {
        let mut bitmap_memory = BitmapMemory::new(PixelFormats::RGBA8888, AlphaFormat::Opaque, PixelSize::new(3, 1));

        let mut source_writer = Rgba8888PixelFormatWriter::default();

        source_writer.reset(0);

        source_writer.write_next(bitmap_memory.data_mut(), Rgba8888Pixel { r: 255, ..Default::default() });
        source_writer.write_next(bitmap_memory.data_mut(), Rgba8888Pixel { g: 255, ..Default::default() });
        source_writer.write_next(bitmap_memory.data_mut(), Rgba8888Pixel { b: 255, ..Default::default() });

        bitmap_memory
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn transcodes_rows_with_different_strides_and_alpha() {
        // 1x2 unpremultiplied Bgra8888, stride 6 -> premultiplied Rgba8888, stride 4.
        let source = [0, 128, 255, 128, 77, 77, 10, 20, 30, 255];
        let mut dest = [0u8; 8];

        PixelFormatTranscoder::transcode(
            &source,
            PixelSize::new(1, 2),
            6,
            PixelFormats::BGRA8888,
            AlphaFormat::Unpremul,
            &mut dest,
            4,
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
        );

        assert_eq!(dest, [128, 64, 0, 128, 30, 20, 10, 255]);
    }

    #[test]
    fn empty_bitmap_touches_nothing() {
        let mut dest = [7u8; 4];

        PixelFormatTranscoder::transcode(
            &[],
            PixelSize::new(0, 3),
            4,
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
            &mut dest,
            4,
            PixelFormats::BGRA8888,
            AlphaFormat::Premul,
        );

        assert_eq!(dest, [7; 4]);
    }

    #[test]
    #[should_panic(expected = "source buffer too small")]
    fn short_source_panics() {
        PixelFormatTranscoder::transcode(
            &[0u8; 7],
            PixelSize::new(2, 1),
            8,
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
            &mut [0u8; 8],
            8,
            PixelFormats::BGRA8888,
            AlphaFormat::Premul,
        );
    }

    #[test]
    #[should_panic(expected = "destination buffer too small")]
    fn short_destination_panics() {
        PixelFormatTranscoder::transcode(
            &[0u8; 8],
            PixelSize::new(2, 1),
            8,
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
            &mut [0u8; 7],
            8,
            PixelFormats::BGRA8888,
            AlphaFormat::Premul,
        );
    }
}
