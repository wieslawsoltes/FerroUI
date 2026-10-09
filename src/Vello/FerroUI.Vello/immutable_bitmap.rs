use crate::helpers::image_decoding_helper::{jpeg_scaled_dimensions, reduce_by_area, DecodedImage, EncodedImageFormat};
use crate::helpers::image_saving_helper::{self, decode_image, load_error};
use crate::helpers::pixel_format_helper::{scene_size, to_image, to_premul_rgba, try_bytes_per_pixel};
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

/// The pixels of a decoded image as an image.
fn decoded_to_image(rgba: Vec<u8>, width: u32, height: u32) -> io::Result<ImageData> {
    let size = PixelSize::new(i32::try_from(width).map_err(|_| load_error())?, i32::try_from(height).map_err(|_| load_error())?);

    Ok(to_image(rgba, size))
}

/// Decodes an encoded image at its natural size, and tells whether it has
/// no alpha.
pub(crate) fn decode_bitmap(stream: &mut dyn Read) -> io::Result<(ImageData, bool)> {
    let DecodedImage { rgba, width, height, opaque, .. } = decode_image(stream)?;

    Ok((decoded_to_image(rgba, width, height)?, opaque))
}

/// Decodes an encoded image so that its width (`horizontal`) or height is
/// `decode_size`, keeping the aspect ratio.
pub(crate) fn decode_bitmap_to_size(
    stream: &mut dyn Read,
    decode_size: i32,
    horizontal: bool,
    interpolation_mode: BitmapInterpolationMode,
) -> io::Result<ImageData> {
    let DecodedImage { mut rgba, mut width, mut height, format, .. } = decode_image(stream)?;
    if decode_size < 1 {
        return Err(load_error());
    }
    let (natural_width, natural_height) = (width, height);

    // Get the scale that is nearest to what we want: the codec of Skia
    // decodes a JPEG at a number of eighths of its size (and every other
    // format at its size), which is the mean of the pixels that fall into
    // one.
    if format == EncodedImageFormat::Jpeg {
        let scale = decode_size as f64 / if horizontal { width } else { height } as f64;
        let (scaled_width, scaled_height) = jpeg_scaled_dimensions(width, height, scale);
        if (scaled_width, scaled_height) != (width, height) {
            rgba = reduce_by_area(&rgba, width as usize, height as usize, scaled_width as usize, scaled_height as usize);
            (width, height) = (scaled_width, scaled_height);
        }
    }
    let image = decoded_to_image(rgba, width, height)?;

    // Now scale that to the size that we want.
    let real_scale = if horizontal {
        natural_height as f64 / natural_width as f64
    } else {
        natural_width as f64 / natural_height as f64
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

/// The pixels a bitmap was created from, in their own format.
struct SourcePixels {
    /// `row_bytes` bytes a row, without padding.
    data: Vec<u8>,
    row_bytes: usize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
}

/// Immutable bitmap: pixels that never change.
///
/// The bitmap is drawn from an image of premultiplied RGBA pixels, the one
/// format the renderers draw. A bitmap that was created from pixels in
/// another format keeps those as well: it reports their format and is read
/// in it, as a bitmap of the Skia backend, which draws every format it has
/// a color type for.
pub struct ImmutableBitmap {
    image: Mutex<Option<ImageData>>,
    source: Mutex<Option<SourcePixels>>,
    /// Whether the bitmap was decoded from an image without alpha.
    opaque: bool,
    dpi: Vector,
    pixel_size: PixelSize,
}

impl ImmutableBitmap {
    /// Wraps an image of premultiplied RGBA pixels.
    pub fn from_image(image: ImageData) -> Self {
        let pixel_size = PixelSize::new(image.width as i32, image.height as i32);

        Self {
            image: Mutex::new(Some(image)),
            source: Mutex::new(None),
            opaque: false,
            dpi: Vector::new(96.0, 96.0),
            pixel_size,
        }
    }

    /// Creates an immutable bitmap from the given stream.
    pub fn from_stream(stream: &mut dyn Read) -> io::Result<Self> {
        // The decoders have no API for DPI.
        let (image, opaque) = decode_bitmap(stream)?;

        Ok(Self { opaque, ..Self::from_image(image) })
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

    /// Creates an immutable bitmap from raw pixel data, which is copied.
    ///
    /// `data` holds `|stride| * size.height` bytes. A negative stride means
    /// the rows are stored bottom-up: the first row of the image is the last
    /// row of `data`.
    ///
    /// # Panics
    /// Panics when the pixel format is unknown to the backend.
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

        let pixel_bytes =
            try_bytes_per_pixel(format).unwrap_or_else(|| panic!("Unknown pixel format: {format}"));
        if size.width < 1 || size.height < 1 || size.width > u16::MAX as i32 || size.height > u16::MAX as i32 {
            return Err(create_error());
        }

        let (width, height) = (size.width as usize, size.height as usize);
        let row_bytes = width * pixel_bytes;
        let abs_stride = stride.unsigned_abs() as usize;
        let copy_bytes = row_bytes.min(abs_stride);

        if data.len() < abs_stride * (height - 1) + copy_bytes {
            return Err(create_error());
        }

        // The rows from the top down, without their padding; what a row of
        // the data lacks is zero.
        let mut pixels = vec![0u8; row_bytes * height];
        for row in 0..height {
            let source_row = if stride < 0 { height - 1 - row } else { row };
            let source = &data[source_row * abs_stride..source_row * abs_stride + copy_bytes];
            pixels[row * row_bytes..row * row_bytes + copy_bytes].copy_from_slice(source);
        }

        let rgba = to_premul_rgba(&pixels, row_bytes, size, format, alpha_format);

        // Pixels in the format the renderers draw are the image itself.
        let source = (format != PixelFormat::RGBA8888 || alpha_format != AlphaFormat::Premul)
            .then_some(SourcePixels { data: pixels, row_bytes, format, alpha_format });

        Ok(Self {
            image: Mutex::new(Some(to_image(rgba, size))),
            source: Mutex::new(source),
            opaque: false,
            dpi,
            pixel_size: size,
        })
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
        self.source.lock().unwrap().take();
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
        Some(self.source.lock().unwrap().as_ref().map_or(PixelFormat::RGBA8888, |source| source.format))
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(self.source.lock().unwrap().as_ref().map_or(self.decoded_alpha_format(), |source| source.alpha_format))
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        // The pixels of the bitmap do not change: the framebuffer is over a
        // copy of them, and what is written to it is dropped with it.
        if let Some(source) = self.source.lock().unwrap().as_ref() {
            return Rc::new(PixelsFramebuffer {
                pixels: RefCell::new(source.data.clone()),
                size: self.pixel_size,
                row_bytes: source.row_bytes,
                dpi: self.dpi,
                format: source.format,
                alpha_format: source.alpha_format,
            });
        }

        let image = self.image().unwrap_or_else(|| panic!("ImmutableBitmap has been disposed"));

        Rc::new(PixelsFramebuffer {
            pixels: RefCell::new(image.data.data().to_vec()),
            size: self.pixel_size,
            row_bytes: self.pixel_size.width.max(0) as usize * 4,
            dpi: self.dpi,
            format: PixelFormat::RGBA8888,
            alpha_format: self.decoded_alpha_format(),
        })
    }
}

impl ImmutableBitmap {
    /// The alpha format of the image of the bitmap: opaque for one that was
    /// decoded from an image without alpha, as the codec of Skia reports
    /// it.
    fn decoded_alpha_format(&self) -> AlphaFormat {
        if self.opaque {
            AlphaFormat::Opaque
        } else {
            AlphaFormat::Premul
        }
    }
}

/// A framebuffer over pixels of its own, without padding.
pub(crate) struct PixelsFramebuffer {
    pub pixels: RefCell<Vec<u8>>,
    pub size: PixelSize,
    pub row_bytes: usize,
    pub dpi: Vector,
    pub format: PixelFormat,
    pub alpha_format: AlphaFormat,
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
        self.row_bytes as i32
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

    fn dispose(&self) {}
}
