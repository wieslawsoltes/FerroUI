use crate::helpers::image_saving_helper::{self, decode_image, load_error};
use crate::helpers::pixel_format_helper::{scene_size, to_image, to_premul_rgba};
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::scene::{IVelloSceneSink, VelloCpuSceneSink, VelloSceneBrush, VelloSceneImage, VelloScenePaint};
use crate::helpers::mipmap_helper;
use crate::vello_extensions::{rect_path, to_sampling};
use ferroui_base::media::imaging::{BitmapEncoderOptions, BitmapInterpolationMode};
use ferroui_base::platform::{AlphaFormat, IBitmapImpl, ILockedFramebuffer, IReadableBitmapImpl, PixelFormat};
use ferroui_base::{PixelSize, Rect, Vector};
use kurbo::Affine;
use peniko::{BlendMode, Extend, Fill, ImageData};
use std::any::Any;
use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::rc::Rc;
use std::sync::Mutex;

/// Decodes an encoded image at its natural size.
pub(crate) fn decode_bitmap(stream: &mut dyn Read) -> io::Result<ImageData> {
    let (rgba, width, height) = decode_image(stream)?;
    let size = PixelSize::new(i32::try_from(width).map_err(|_| load_error())?, i32::try_from(height).map_err(|_| load_error())?);

    Ok(to_image(rgba, size))
}

/// Decodes an encoded image so that its width (`horizontal`) or height is
/// `decode_size`, keeping the aspect ratio.
pub(crate) fn decode_bitmap_to_size(
    stream: &mut dyn Read,
    decode_size: i32,
    horizontal: bool,
    interpolation_mode: BitmapInterpolationMode,
) -> io::Result<ImageData> {
    let image = decode_bitmap(stream)?;
    if decode_size < 1 || image.width == 0 || image.height == 0 {
        return Err(load_error());
    }

    let real_scale = if horizontal {
        image.height as f64 / image.width as f64
    } else {
        image.width as f64 / image.height as f64
    };

    let desired = if horizontal {
        PixelSize::new(decode_size, (real_scale * decode_size as f64) as i32)
    } else {
        PixelSize::new((real_scale * decode_size as f64) as i32, decode_size)
    };

    if desired.width as u32 != image.width || desired.height as u32 != image.height {
        return scale_image(&image, desired, interpolation_mode);
    }

    Ok(image)
}

/// Scales an image to another size.
///
/// The image is drawn scaled by the CPU renderer, which every build of the
/// backend has: a bitmap is resized the same way in every rendering mode,
/// and on the thread that asks.
pub(crate) fn scale_image(
    image: &ImageData,
    destination_size: PixelSize,
    interpolation_mode: BitmapInterpolationMode,
) -> io::Result<ImageData> {
    if destination_size.width < 1
        || destination_size.height < 1
        || destination_size.width > u16::MAX as i32
        || destination_size.height > u16::MAX as i32
        || image.width == 0
        || image.height == 0
    {
        return Err(load_error());
    }

    let (width, height) = scene_size(destination_size);
    let mut sink = VelloCpuSceneSink::new(width, height);

    let destination = Rect::new(0.0, 0.0, destination_size.width as f64, destination_size.height as f64);
    let is_upscaling =
        destination_size.width as u32 > image.width || destination_size.height as u32 > image.height;
    let (quality, mipmaps) = to_sampling(interpolation_mode, is_upscaling);

    let transform =
        Affine::scale_non_uniform(destination.width / image.width as f64, destination.height / image.height as f64);

    // An image that is reduced in a mode with mipmaps is sampled from the
    // two levels of its mipmap that are nearest to the new size.
    match mipmaps.then(|| mipmap_helper::levels(image, transform)).flatten() {
        Some(levels) => mipmap_helper::fill_with_levels(
            &mut sink,
            &rect_path(destination),
            Affine::IDENTITY,
            transform,
            &levels,
            1.0,
            false,
        ),
        None => {
            let paint = VelloScenePaint {
                brush: VelloSceneBrush::Image(VelloSceneImage {
                    image: image.clone(),
                    x_extend: Extend::Pad,
                    y_extend: Extend::Pad,
                    quality,
                    alpha: 1.0,
                }),
                transform,
            };
            sink.fill(&rect_path(destination), Fill::NonZero, Affine::IDENTITY, &paint, BlendMode::default(), false);
        }
    }

    let mut rgba = vec![0u8; width as usize * height as usize * 4];
    sink.render_to_pixels(&mut rgba);

    Ok(to_image(rgba, destination_size))
}

/// Immutable bitmap: pixels that never change, premultiplied RGBA.
pub struct ImmutableBitmap {
    image: Mutex<Option<ImageData>>,
    dpi: Vector,
    pixel_size: PixelSize,
}

impl ImmutableBitmap {
    /// Wraps an image of premultiplied RGBA pixels.
    pub fn from_image(image: ImageData) -> Self {
        let pixel_size = PixelSize::new(image.width as i32, image.height as i32);

        Self { image: Mutex::new(Some(image)), dpi: Vector::new(96.0, 96.0), pixel_size }
    }

    /// Creates an immutable bitmap from the given stream.
    pub fn from_stream(stream: &mut dyn Read) -> io::Result<Self> {
        // The decoder has no API for DPI.
        Ok(Self::from_image(decode_bitmap(stream)?))
    }

    /// Creates an immutable bitmap from a stream, decoded so that its width
    /// (`horizontal`) or height is `decode_size`.
    pub fn from_stream_to_size(
        stream: &mut dyn Read,
        decode_size: i32,
        horizontal: bool,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Self> {
        Ok(Self::from_image(decode_bitmap_to_size(stream, decode_size, horizontal, interpolation_mode)?))
    }

    /// Creates a resized copy of `src`.
    pub fn resized(
        src: &ImmutableBitmap,
        destination_size: PixelSize,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Self> {
        let image = src.image().ok_or_else(load_error)?;

        Ok(Self::from_image(scale_image(&image, destination_size, interpolation_mode)?))
    }

    /// Creates an immutable bitmap from raw pixel data: `stride` bytes a
    /// row, `size.height` rows, in the given formats.
    pub fn from_pixels(
        size: PixelSize,
        dpi: Vector,
        stride: i32,
        format: PixelFormat,
        alpha_format: AlphaFormat,
        data: &[u8],
    ) -> io::Result<Self> {
        if size.width < 1 || size.height < 1 || stride < size.width * 4 {
            return Err(load_error());
        }
        if data.len() < stride as usize * (size.height as usize - 1) + size.width as usize * 4 {
            return Err(load_error());
        }

        let rgba = to_premul_rgba(data, stride as usize, size, format, alpha_format);

        Ok(Self { image: Mutex::new(Some(to_image(rgba, size))), dpi, pixel_size: size })
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
        let image = self.image().ok_or_else(|| io::Error::other("ImmutableBitmap has been disposed"))?;
        image_saving_helper::save_image(&image, stream, options)
    }

    fn dispose(&self) {
        self.image.lock().unwrap().take();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IDrawableBitmapImpl for ImmutableBitmap {
    fn image(&self) -> Option<ImageData> {
        self.image.lock().unwrap().clone()
    }
}

impl IReadableBitmapImpl for ImmutableBitmap {
    fn format(&self) -> Option<PixelFormat> {
        Some(PixelFormat::RGBA8888)
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(AlphaFormat::Premul)
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        let image = self.image().unwrap_or_else(|| panic!("ImmutableBitmap has been disposed"));

        // The pixels of the bitmap do not change: the framebuffer is over a
        // copy of them, and what is written to it is dropped with it.
        Rc::new(PixelsFramebuffer {
            pixels: RefCell::new(image.data.data().to_vec()),
            size: self.pixel_size,
            dpi: self.dpi,
        })
    }
}

/// A framebuffer over pixels of its own: premultiplied RGBA without padding.
pub(crate) struct PixelsFramebuffer {
    pub pixels: RefCell<Vec<u8>>,
    pub size: PixelSize,
    pub dpi: Vector,
}

impl ILockedFramebuffer for PixelsFramebuffer {
    fn address(&self) -> *mut u8 {
        self.pixels.borrow_mut().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.pixels.borrow_mut());
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.size.width * 4
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
