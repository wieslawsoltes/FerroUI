use crate::gpu::{drawable_image, needs_mipmaps, ISkiaGrContext};
use crate::helpers::image_saving_helper;
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::locked_framebuffer::LockedFramebuffer;
use crate::skia_sharp_extensions::{
    to_alpha_format, to_pixel_format_opt, to_sk_alpha_type, to_sk_color_type_or_panic, to_sk_sampling_options_scaled,
};
use ferroui_base::media::imaging::{BitmapEncoderOptions, BitmapInterpolationMode};
use ferroui_base::platform::{AlphaFormat, IBitmapImpl, ILockedFramebuffer, IReadableBitmapImpl, PixelFormat};
use ferroui_base::{PixelSize, Vector};
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    AlphaType, Bitmap, Canvas, Codec, ColorType, Data, Image, ImageInfo, Paint, Rect, SamplingOptions,
};
use std::any::Any;
use std::io::{self, Read, Write};
use std::rc::Rc;
use std::sync::Mutex;

pub(crate) fn load_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Unable to load bitmap from provided data")
}

fn read_stream(stream: &mut dyn Read) -> io::Result<Data> {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    Ok(Data::new_copy(&bytes))
}

/// Allocates a bitmap for `info` and decodes the codec's image into it.
fn decode_with_info(codec: &mut Codec, info: &ImageInfo) -> Option<Bitmap> {
    let mut bitmap = Bitmap::new();
    if !bitmap.try_alloc_pixels_info(info, None) {
        return None;
    }

    let row_bytes = bitmap.row_bytes();
    let mut pixmap = bitmap.peek_pixels()?;
    let pixels = pixmap.bytes_mut()?;

    match codec.get_pixels_with_options(info, pixels, row_bytes, None) {
        skia_safe::codec::Result::Success | skia_safe::codec::Result::IncompleteInput => Some(bitmap),
        _ => None,
    }
}

/// Decodes an encoded image at its natural size.
pub(crate) fn decode_bitmap(stream: &mut dyn Read) -> io::Result<Bitmap> {
    let data = read_stream(stream)?;
    let mut codec = Codec::from_data(data).ok_or_else(load_error)?;

    let mut info = codec.info();
    if info.alpha_type() == AlphaType::Unpremul {
        info = info.with_alpha_type(AlphaType::Premul);
    }
    let info = info.with_color_space(None);

    decode_with_info(&mut codec, &info).ok_or_else(load_error)
}

/// Decodes an encoded image so that its width (`horizontal`) or height is
/// `decode_size`, keeping the aspect ratio.
pub(crate) fn decode_bitmap_to_size(
    stream: &mut dyn Read,
    decode_size: i32,
    horizontal: bool,
    interpolation_mode: BitmapInterpolationMode,
) -> io::Result<Bitmap> {
    let data = read_stream(stream)?;
    let mut codec = Codec::from_data(data).ok_or_else(load_error)?;
    let info = codec.info();

    // Get the scale that is nearest to what we want (e.g. a jpg returns 512).
    let supported_scale = codec.get_scaled_dimensions(if horizontal {
        decode_size as f32 / info.width() as f32
    } else {
        decode_size as f32 / info.height() as f32
    });

    // Decode the bitmap at the nearest size.
    let nearest = ImageInfo::new_n32_premul((supported_scale.width, supported_scale.height), None);
    let bitmap = decode_with_info(&mut codec, &nearest).ok_or_else(load_error)?;

    // Now scale that to the size that we want.
    let real_scale = if horizontal {
        info.height() as f64 / info.width() as f64
    } else {
        info.width() as f64 / info.height() as f64
    };

    let desired = if horizontal {
        ImageInfo::new_n32_premul((decode_size, (real_scale * decode_size as f64) as i32), None)
    } else {
        ImageInfo::new_n32_premul(((real_scale * decode_size as f64) as i32, decode_size), None)
    };

    if bitmap.width() != desired.width() || bitmap.height() != desired.height() {
        let is_upscaling = desired.width() > bitmap.width() || desired.height() > bitmap.height();

        let mut scaled = Bitmap::new();
        if !scaled.try_alloc_pixels_info(&desired, None) {
            return Err(load_error());
        }

        let mut dst = scaled.peek_pixels().ok_or_else(load_error)?;
        let src = bitmap.peek_pixels().ok_or_else(load_error)?;
        if !src.scale_pixels(&mut dst, to_sk_sampling_options_scaled(interpolation_mode, is_upscaling)) {
            return Err(load_error());
        }

        return Ok(scaled);
    }

    Ok(bitmap)
}

/// A Skia bitmap held by an object that is shared between threads.
pub(crate) struct SendBitmap(pub Bitmap);

// SAFETY: a Skia bitmap owns its pixels through an atomically counted
// reference and has no affinity to the thread that created it; its holders
// keep it under a lock, so it is used by one thread at a time.
unsafe impl Send for SendBitmap {}

impl std::ops::Deref for SendBitmap {
    type Target = Bitmap;

    fn deref(&self) -> &Bitmap {
        &self.0
    }
}

/// Immutable Skia bitmap.
pub struct ImmutableBitmap {
    image: Mutex<Option<Image>>,
    bitmap: Mutex<Option<SendBitmap>>,
    custom_image_dispose: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    dpi: Vector,
    pixel_size: PixelSize,
}

impl ImmutableBitmap {
    fn from_bitmap(mut bitmap: Bitmap, pixel_size: Option<PixelSize>, dpi: Vector) -> io::Result<Self> {
        bitmap.set_immutable();
        let image = skia_safe::images::raster_from_bitmap(&bitmap).ok_or_else(load_error)?;
        let pixel_size = pixel_size.unwrap_or_else(|| PixelSize::new(image.width(), image.height()));

        Ok(Self {
            image: Mutex::new(Some(image)),
            bitmap: Mutex::new(Some(SendBitmap(bitmap))),
            custom_image_dispose: Mutex::new(None),
            dpi,
            pixel_size,
        })
    }

    /// Creates an immutable bitmap from the given stream.
    pub fn from_stream(stream: &mut dyn Read) -> io::Result<Self> {
        // Skia doesn't have an API for DPI.
        Self::from_bitmap(decode_bitmap(stream)?, None, Vector::new(96.0, 96.0))
    }

    /// Wraps a Skia image. `custom_image_dispose` replaces releasing the
    /// image when the bitmap is disposed.
    pub fn from_image(image: Image, custom_image_dispose: Option<Box<dyn FnOnce() + Send>>) -> Self {
        let pixel_size = PixelSize::new(image.width(), image.height());

        Self {
            image: Mutex::new(Some(image)),
            bitmap: Mutex::new(None),
            custom_image_dispose: Mutex::new(custom_image_dispose),
            dpi: Vector::new(96.0, 96.0),
            pixel_size,
        }
    }

    /// Creates a resized copy of `src`.
    pub fn resized(
        src: &ImmutableBitmap,
        destination_size: PixelSize,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Self> {
        let is_upscaling =
            destination_size.width > src.pixel_size.width || destination_size.height > src.pixel_size.height;
        let info = ImageInfo::new(
            (destination_size.width, destination_size.height),
            ColorType::BGRA8888,
            AlphaType::Premul,
            None,
        );

        let mut bitmap = Bitmap::new();
        if !bitmap.try_alloc_pixels_info(&info, None) {
            return Err(load_error());
        }

        let pixmap = bitmap.peek_pixels().ok_or_else(load_error)?;
        if !src.image().scale_pixels(&pixmap, to_sk_sampling_options_scaled(interpolation_mode, is_upscaling), None)
        {
            return Err(load_error());
        }

        Self::from_bitmap(bitmap, None, Vector::new(96.0, 96.0))
    }

    /// Creates an immutable bitmap from a stream, decoded so that its width
    /// (`horizontal`) or height is `decode_size`.
    pub fn from_stream_to_size(
        stream: &mut dyn Read,
        decode_size: i32,
        horizontal: bool,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Self> {
        let bitmap = decode_bitmap_to_size(stream, decode_size, horizontal, interpolation_mode)?;
        Self::from_bitmap(bitmap, None, Vector::new(96.0, 96.0))
    }

    /// Creates an immutable bitmap from raw pixel data, which is copied.
    ///
    /// `data` holds `|stride| * size.height` bytes. A negative stride means
    /// the rows are stored bottom-up: the first row of the image is the last
    /// row of `data`.
    pub fn from_pixels(
        size: PixelSize,
        dpi: Vector,
        stride: i32,
        format: PixelFormat,
        alpha_format: AlphaFormat,
        data: &[u8],
    ) -> io::Result<Self> {
        let create_error =
            || io::Error::new(io::ErrorKind::InvalidInput, "Unable to create bitmap from provided data");

        let info = ImageInfo::new(
            (size.width, size.height),
            to_sk_color_type_or_panic(format),
            to_sk_alpha_type(alpha_format),
            None,
        );

        let mut bitmap = Bitmap::new();
        if !bitmap.try_alloc_pixels_info(&info, None) {
            return Err(create_error());
        }

        let row_bytes = bitmap.row_bytes();
        let height = size.height.max(0) as usize;
        let abs_stride = stride.unsigned_abs() as usize;
        let copy_bytes = info.min_row_bytes().min(abs_stride);

        if data.len() < abs_stride * height.saturating_sub(1) + if height > 0 { copy_bytes } else { 0 } {
            return Err(create_error());
        }

        if height > 0 {
            let mut pixmap = bitmap.peek_pixels().ok_or_else(create_error)?;
            let pixels = pixmap.bytes_mut().ok_or_else(create_error)?;

            for row in 0..height {
                let source_row = if stride < 0 { height - 1 - row } else { row };
                let source = &data[source_row * abs_stride..source_row * abs_stride + copy_bytes];
                pixels[row * row_bytes..row * row_bytes + copy_bytes].copy_from_slice(source);
            }
        }

        Self::from_bitmap(bitmap, Some(size), dpi)
    }

    /// The Skia image.
    ///
    /// # Panics
    /// Panics when the bitmap has been disposed.
    pub fn image(&self) -> Image {
        self.image.lock().unwrap().clone().expect("ImmutableBitmap has been disposed")
    }
}

impl IBitmapImpl for ImmutableBitmap {
    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn pixel_size(&self) -> PixelSize {
        self.pixel_size
    }

    fn version(&self) -> i32 {
        1
    }

    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
        image_saving_helper::save_image(&self.image(), stream, options)
    }

    fn dispose(&self) {
        let image = self.image.lock().unwrap().take();
        if let Some(custom_image_dispose) = self.custom_image_dispose.lock().unwrap().take() {
            custom_image_dispose();
        }
        drop(image);
        self.bitmap.lock().unwrap().take();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IDrawableBitmapImpl for ImmutableBitmap {
    fn draw(
        &self,
        gr_context: Option<&dyn ISkiaGrContext>,
        canvas: &Canvas,
        source_rect: &Rect,
        dest_rect: &Rect,
        sampling_options: SamplingOptions,
        paint: &Paint,
    ) {
        canvas.draw_image_rect_with_sampling_options(
            drawable_image(gr_context, self.image(), needs_mipmaps(&sampling_options)),
            Some((source_rect, SrcRectConstraint::Fast)),
            dest_rect,
            sampling_options,
            paint,
        );
    }
}

impl IReadableBitmapImpl for ImmutableBitmap {
    fn format(&self) -> Option<PixelFormat> {
        self.bitmap.lock().unwrap().as_ref().and_then(|bitmap| to_pixel_format_opt(bitmap.color_type()))
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        self.bitmap.lock().unwrap().as_ref().map(|bitmap| to_alpha_format(bitmap.alpha_type()))
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        let bitmap = self.bitmap.lock().unwrap();
        let bitmap = bitmap.as_ref().unwrap_or_else(|| panic!("A bitmap is needed for locking"));

        let format = to_pixel_format_opt(bitmap.color_type())
            .unwrap_or_else(|| panic!("Unsupported format {:?}", bitmap.color_type()));
        let alpha_format = to_alpha_format(bitmap.alpha_type());

        Rc::new(LockedFramebuffer::new(
            bitmap.pixmap().addr() as *mut u8,
            self.pixel_size,
            bitmap.row_bytes() as i32,
            self.dpi,
            format,
            alpha_format,
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constructor_from_pixels_copies_source_data(width: i32, height: i32, negative_stride: bool) {
        let size = PixelSize::new(width, height);
        let row_bytes = (width * 4) as usize;
        let abs_stride = row_bytes;
        let byte_size = abs_stride * height as usize;

        // Logical pixel byte: deterministic function of (row, byte index within row).
        let expected = |row: usize, x: usize| ((row * row_bytes + x) * 7 + 1) as u8;

        // Lay the logical rows out in physical memory. For a negative stride
        // the rows are stored bottom-up.
        let mut source = vec![0u8; byte_size];
        for row in 0..height as usize {
            let physical_row = if negative_stride { height as usize - 1 - row } else { row };
            for x in 0..row_bytes {
                source[physical_row * abs_stride + x] = expected(row, x);
            }
        }

        let stride = if negative_stride { -(abs_stride as i32) } else { abs_stride as i32 };

        let bitmap = ImmutableBitmap::from_pixels(
            size,
            Vector::new(96.0, 96.0),
            stride,
            PixelFormat::BGRA8888,
            AlphaFormat::Premul,
            &source,
        )
        .unwrap();

        // The constructor must take its own copy: corrupting the source
        // afterwards must not affect the bitmap's pixels.
        source.fill(0xCD);
        drop(source);

        assert_eq!(size, bitmap.pixel_size());

        let locked = bitmap.lock();
        assert_eq!(size, locked.size());

        let locked_row_bytes = locked.row_bytes() as usize;
        // SAFETY: the locked framebuffer is `row_bytes * height` bytes long
        // and stays valid while `bitmap` is alive.
        let dst = unsafe { std::slice::from_raw_parts(locked.address(), locked_row_bytes * height as usize) };

        for row in 0..height as usize {
            for x in 0..row_bytes {
                assert_eq!(expected(row, x), dst[row * locked_row_bytes + x]);
            }
        }

        locked.dispose();
        bitmap.dispose();
    }

    #[test]
    fn constructor_from_pixels_copies_source_data_cases() {
        for (width, height) in [(1, 1), (3, 5), (64, 64)] {
            constructor_from_pixels_copies_source_data(width, height, false);
            constructor_from_pixels_copies_source_data(width, height, true);
        }
    }
}
