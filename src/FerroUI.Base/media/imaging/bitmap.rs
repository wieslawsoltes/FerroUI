use crate::media::imaging::bitmap_memory::BitmapMemory;
use crate::media::imaging::pixel_format_transcoder::PixelFormatTranscoder;
use crate::media::imaging::{BitmapEncoderOptions, BitmapInterpolationMode, IBitmap};
use crate::media::{DrawingContext, IImage, IImageBrushSource};
use crate::platform::{self, AlphaFormat, IBitmapImpl, ILockedFramebuffer, PixelFormat};
use crate::utilities::{RefCountable, RefCounted};
use crate::{PixelRect, PixelSize, Rect, Size, Vector};
use std::any::Any;
use std::io::{Read, Write};

/// Holds a bitmap image.
///
/// The types deriving from it ([`WriteableBitmap`](super::WriteableBitmap),
/// [`RenderTargetBitmap`](super::RenderTargetBitmap)) dereference to
/// `Bitmap`; the members they override are reached through the deriving type
/// or through [`IBitmap`]/[`IImage`] handles.
pub struct Bitmap {
    is_transcoded: bool,
    platform_impl: RefCounted<crate::platform::SharedBitmapImpl>,
}

/// Wraps a platform bitmap in a counted reference that disposes it when the
/// last reference is released.
pub(crate) fn create_bitmap_ref(platform_impl: std::sync::Arc<crate::platform::SharedBitmapImpl>) -> RefCounted<crate::platform::SharedBitmapImpl> {
    let item = platform_impl.clone();
    RefCountable::create(platform_impl, move || item.dispose())
}

impl Bitmap {
    /// Loads a bitmap from a stream, decoded to the specified width
    /// maintaining aspect ratio.
    ///
    /// `interpolation_mode` is used should a resize be necessary; upstream
    /// defaults it to high quality.
    pub fn decode_to_width(
        stream: &mut dyn Read,
        width: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Bitmap> {
        Ok(Bitmap::from_impl(platform::render_interface().load_bitmap_to_width(stream, width, interpolation_mode)?))
    }

    /// Loads a bitmap from a stream, decoded to the specified height
    /// maintaining aspect ratio.
    pub fn decode_to_height(
        stream: &mut dyn Read,
        height: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Bitmap> {
        Ok(Bitmap::from_impl(platform::render_interface().load_bitmap_to_height(stream, height, interpolation_mode)?))
    }

    /// Creates a bitmap scaled to the specified size from the current
    /// bitmap.
    pub fn create_scaled_bitmap(
        &self,
        destination_size: PixelSize,
        interpolation_mode: BitmapInterpolationMode,
    ) -> Bitmap {
        Bitmap::from_impl(platform::render_interface().resize_bitmap(
            &*self.platform_impl.item(),
            destination_size,
            interpolation_mode,
        ))
    }

    /// Loads a bitmap from a file.
    pub fn from_file(file_name: &str) -> std::io::Result<Bitmap> {
        Ok(Bitmap::from_impl(platform::render_interface().load_bitmap_from_file(file_name)?))
    }

    /// Loads a bitmap from a stream.
    pub fn from_stream(stream: &mut dyn Read) -> std::io::Result<Bitmap> {
        Ok(Bitmap::from_impl(platform::render_interface().load_bitmap(stream)?))
    }

    /// Creates a bitmap sharing a platform bitmap: takes another reference
    /// to it.
    pub fn from_ref(platform_impl: &RefCounted<crate::platform::SharedBitmapImpl>) -> Bitmap {
        Bitmap { is_transcoded: false, platform_impl: platform_impl.clone_ref() }
    }

    /// Creates a bitmap owning a platform bitmap.
    pub fn from_impl(platform_impl: std::sync::Arc<crate::platform::SharedBitmapImpl>) -> Bitmap {
        Bitmap { is_transcoded: false, platform_impl: create_bitmap_ref(platform_impl) }
    }

    /// Creates a bitmap from pixel data: `stride * size.height` bytes of
    /// `format`/`alpha_format` pixels.
    ///
    /// A format the backend cannot hold is converted to RGBA8888.
    pub fn from_pixels(
        format: PixelFormat,
        alpha_format: AlphaFormat,
        data: &[u8],
        size: PixelSize,
        dpi: Vector,
        stride: i32,
    ) -> Bitmap {
        let factory = platform::render_interface();
        if factory.is_supported_bitmap_pixel_format(format) {
            Bitmap::from_impl(factory.load_bitmap_from_pixels(format, alpha_format, data, size, dpi, stride))
        } else {
            let mut transcoded = BitmapMemory::new(PixelFormat::RGBA8888, AlphaFormat::Unpremul, size);
            let transcoded_alpha_format = if format.has_alpha() { alpha_format } else { AlphaFormat::Opaque };
            let (row_bytes, transcoded_format) = (transcoded.row_bytes(), transcoded.format());

            PixelFormatTranscoder::transcode(
                data,
                size,
                stride,
                format,
                alpha_format,
                transcoded.data_mut(),
                row_bytes,
                transcoded_format,
                transcoded_alpha_format,
            );

            let platform_impl = factory.load_bitmap_from_pixels(
                PixelFormat::RGBA8888,
                transcoded_alpha_format,
                transcoded.data(),
                size,
                dpi,
                row_bytes,
            );
            transcoded.dispose();

            Bitmap { is_transcoded: true, platform_impl: create_bitmap_ref(platform_impl) }
        }
    }

    /// Releases the bitmap's reference to the platform bitmap.
    pub fn dispose(&self) {
        self.platform_impl.dispose();
    }

    /// The dots per inch (DPI) of the image.
    pub fn dpi(&self) -> Vector {
        self.platform_impl.item().dpi()
    }

    /// The size of the image, in device independent pixels.
    pub fn size(&self) -> Size {
        self.platform_impl.item().pixel_size().to_size_with_dpi_vector(self.dpi())
    }

    /// The size of the bitmap, in device pixels.
    pub fn pixel_size(&self) -> PixelSize {
        self.platform_impl.item().pixel_size()
    }

    /// The platform-specific bitmap implementation.
    pub fn platform_impl(&self) -> &RefCounted<crate::platform::SharedBitmapImpl> {
        &self.platform_impl
    }

    /// Saves the bitmap to a file.
    pub fn save_to_file(&self, file_name: &str, options: &BitmapEncoderOptions) -> std::io::Result<()> {
        let mut stream = std::fs::File::create(file_name)?;
        self.save(&mut stream, options)
    }

    /// Saves the bitmap to a stream.
    pub fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> std::io::Result<()> {
        self.platform_impl.item().save(stream, options)
    }

    /// The pixel format of the bitmap, if its pixels can be read.
    pub fn format(&self) -> Option<PixelFormat> {
        self.platform_impl.item().as_readable_bitmap().and_then(|readable| readable.format())
    }

    /// The alpha format of the bitmap, if its pixels can be read.
    pub fn alpha_format(&self) -> Option<AlphaFormat> {
        self.platform_impl.item().as_readable_bitmap().and_then(|readable| readable.alpha_format())
    }

    fn validate_source_rect(&self, mut source_rect: PixelRect) -> PixelRect {
        let pixel_size = self.pixel_size();

        if (source_rect.width <= 0 || source_rect.height <= 0) && (source_rect.x != 0 || source_rect.y != 0) {
            panic!("source_rect is out of range: {source_rect}");
        }

        if source_rect.x < 0 || source_rect.y < 0 {
            panic!("source_rect is out of range: {source_rect}");
        }

        if source_rect.width <= 0 {
            source_rect = source_rect.with_width(pixel_size.width);
        }
        if source_rect.height <= 0 {
            source_rect = source_rect.with_height(pixel_size.height);
        }

        if source_rect.right() > pixel_size.width || source_rect.bottom() > pixel_size.height {
            panic!("source_rect is out of range: {source_rect}");
        }

        source_rect
    }

    /// Copies the rows of `source_rect` from `source` (rows
    /// `source_row_bytes` apart, pixels of `source_format`) into `buffer`
    /// (rows `stride` apart).
    ///
    /// Panics when `stride` is too small for a row or `buffer` too small for
    /// the rectangle.
    pub(crate) fn copy_pixels_between(
        source_rect: PixelRect,
        source: &[u8],
        source_row_bytes: i32,
        source_format: PixelFormat,
        buffer: &mut [u8],
        stride: i32,
    ) {
        let bits_per_pixel = source_format.bits_per_pixel() as i32;
        let min_stride = source_rect
            .width
            .checked_mul(bits_per_pixel)
            .and_then(|bits| bits.checked_add(7))
            .expect("arithmetic overflow")
            / 8;
        if stride < min_stride {
            panic!("stride is out of range: {stride}");
        }

        // 64-bit to avoid overflowing the guard for very large
        // strides/heights.
        let min_buffer_size = stride as i64 * source_rect.height as i64;
        if min_buffer_size > buffer.len() as i64 {
            panic!("buffer size is out of range: {}", buffer.len());
        }

        let offset_x =
            source_rect.x.checked_mul(bits_per_pixel).and_then(|bits| bits.checked_add(7)).expect("arithmetic overflow")
                / 8;

        // Fast-path: when the source and destination layouts are identical,
        // tightly-packed and forward (no row padding, no X offset, positive
        // stride), the whole region is contiguous in both buffers and can be
        // copied with a single blit. Requiring stride == min_stride also
        // guarantees the source is not read past its last row.
        if offset_x == 0 && source_row_bytes == stride && stride == min_stride {
            let start = (source_row_bytes as i64 * source_rect.y as i64) as usize;
            let length = min_buffer_size as usize;
            buffer[..length].copy_from_slice(&source[start..start + length]);
            return;
        }

        let min_stride = min_stride as usize;
        for y in 0..source_rect.height {
            let src_address =
                (source_row_bytes as i64 * (source_rect.y + y) as i64 + offset_x as i64) as usize;
            let dst_address = (stride as i64 * y as i64) as usize;
            buffer[dst_address..dst_address + min_stride]
                .copy_from_slice(&source[src_address..src_address + min_stride]);
        }
    }

    pub(crate) fn copy_pixels_core(
        &self,
        source_rect: PixelRect,
        buffer: &mut [u8],
        stride: i32,
        fb: &dyn ILockedFramebuffer,
    ) {
        let source_rect = self.validate_source_rect(source_rect);
        let (row_bytes, format) = (fb.row_bytes(), fb.format());
        fb.with_data(&mut |source| {
            Self::copy_pixels_between(source_rect, source, row_bytes, format, buffer, stride);
        });
    }

    /// Copies the pixels of `source_rect` into `buffer`, whose rows are
    /// `stride` bytes apart. An empty `source_rect` selects the whole
    /// bitmap.
    ///
    /// Panics when the pixels of the bitmap cannot be read, when the bitmap
    /// was converted from another pixel format on creation, or when the
    /// rectangle, stride or buffer are out of range.
    pub fn copy_pixels(&self, source_rect: PixelRect, buffer: &mut [u8], stride: i32) {
        let item = self.platform_impl.item();
        let readable = match (self.format(), item.as_readable_bitmap()) {
            (Some(format), Some(readable)) if Some(format) == readable.format() => readable,
            _ => panic!("CopyPixels is not supported for this bitmap type"),
        };

        if self.is_transcoded {
            panic!("CopyPixels is not supported for transcoded bitmaps");
        }

        let fb = readable.lock();
        self.copy_pixels_core(source_rect, buffer, stride, &*fb);
        fb.dispose();
    }

    /// Copies the pixels of the bitmap into a locked framebuffer, converting
    /// them to its pixel and alpha formats.
    ///
    /// Panics when the pixels of the bitmap cannot be read.
    pub fn copy_pixels_to_framebuffer(&self, buffer: &dyn ILockedFramebuffer) {
        let item = self.platform_impl.item();
        let readable = match item.as_readable_bitmap() {
            Some(readable) if readable.format().is_some() && readable.alpha_format().is_some() => readable,
            // The pixels cannot be read: the bitmap has to be drawn into a
            // compatible bitmap first, which needs a drawing context.
            _ => panic!("CopyPixels is not supported for this bitmap type"),
        };

        if Some(buffer.format()) != readable.format() || Some(buffer.alpha_format()) != readable.alpha_format() {
            let fb = readable.lock();
            let (size, row_bytes, format, alpha_format) = (fb.size(), fb.row_bytes(), fb.format(), fb.alpha_format());
            let (dest_row_bytes, dest_format, dest_alpha_format) =
                (buffer.row_bytes(), buffer.format(), buffer.alpha_format());
            fb.with_data(&mut |source| {
                buffer.with_data(&mut |dest| {
                    PixelFormatTranscoder::transcode(
                        source,
                        size,
                        row_bytes,
                        format,
                        alpha_format,
                        dest,
                        dest_row_bytes,
                        dest_format,
                        dest_alpha_format,
                    );
                });
            });
            fb.dispose();
        } else {
            let fb = readable.lock();
            let (source_rect, stride) = (PixelRect::from_size(fb.size()), fb.row_bytes());
            let buffer_size = (buffer.row_bytes() as i64 * buffer.size().height as i64).max(0) as usize;
            buffer.with_data(&mut |dest| {
                let length = buffer_size.min(dest.len());
                self.copy_pixels_core(source_rect, &mut dest[..length], stride, &*fb);
            });
            fb.dispose();
        }
    }
}

impl IImage for Bitmap {
    fn size(&self) -> Size {
        Bitmap::size(self)
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source_rect: Rect, dest_rect: Rect) {
        context.draw_bitmap(&self.platform_impl.item(), 1.0, source_rect, dest_rect);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_bitmap(&self) -> Option<&dyn IBitmap> {
        Some(self)
    }
}

impl IBitmap for Bitmap {
    fn dpi(&self) -> Vector {
        Bitmap::dpi(self)
    }

    fn pixel_size(&self) -> PixelSize {
        Bitmap::pixel_size(self)
    }

    fn platform_impl(&self) -> &RefCounted<crate::platform::SharedBitmapImpl> {
        &self.platform_impl
    }

    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> std::io::Result<()> {
        Bitmap::save(self, stream, options)
    }

    fn dispose(&self) {
        Bitmap::dispose(self)
    }
}

impl IImageBrushSource for Bitmap {
    fn bitmap(&self) -> Option<&RefCounted<crate::platform::SharedBitmapImpl>> {
        if !self.platform_impl.is_alive() {
            return None;
        }
        Some(&self.platform_impl)
    }
}

/// Bitmaps compare by identity (reference equality), so that their handles can be
/// held in untyped values.
impl PartialEq for Bitmap {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
