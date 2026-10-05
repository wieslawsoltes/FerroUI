use crate::media::imaging::bitmap_memory::BitmapMemory;
use crate::media::imaging::pixel_format_readers::PixelFormatReader;
use crate::media::imaging::{Bitmap, BitmapEncoderOptions, BitmapInterpolationMode, IBitmap};
use crate::media::{DrawingContext, IImage, IImageBrushSource};
use crate::platform::{self, AlphaFormat, IBitmapImpl, ILockedFramebuffer, IWriteableBitmapImpl, PixelFormat};
use crate::utilities::RefCounted;
use crate::{PixelRect, PixelSize, Rect, Size, Vector};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::ops::Deref;
use std::rc::Rc;

/// Holds a writeable bitmap image.
pub struct WriteableBitmap {
    base: Bitmap,
    /// Holds a buffer with a pixel format that requires transcoding.
    pixel_format_memory: Option<Rc<RefCell<BitmapMemory>>>,
}

impl Deref for WriteableBitmap {
    type Target = Bitmap;

    #[inline]
    fn deref(&self) -> &Bitmap {
        &self.base
    }
}

impl WriteableBitmap {
    /// Creates a bitmap of the given size, in pixels, and DPI. The formats
    /// default to the backend's default formats.
    ///
    /// Panics when the size is not positive or the pixel format is not
    /// supported.
    pub fn new(
        size: PixelSize,
        dpi: Vector,
        format: Option<PixelFormat>,
        alpha_format: Option<AlphaFormat>,
    ) -> WriteableBitmap {
        let (platform_impl, pixel_format_memory) = Self::create_platform_impl(size, dpi, format, alpha_format);
        Self::from_impl(platform_impl, pixel_format_memory)
    }

    fn from_impl(platform_impl: Rc<dyn IWriteableBitmapImpl>, pixel_format_memory: Option<BitmapMemory>) -> Self {
        Self {
            base: Bitmap::from_impl(platform_impl),
            pixel_format_memory: pixel_format_memory.map(|memory| Rc::new(RefCell::new(memory))),
        }
    }

    /// Creates a writeable bitmap from a pixel data copy: `data` holds rows
    /// of `format` pixels, `stride` bytes apart.
    ///
    /// Panics when `stride` is too small for a row.
    pub fn from_pixels(
        format: PixelFormat,
        alpha_format: AlphaFormat,
        data: &[u8],
        size: PixelSize,
        dpi: Vector,
        stride: i32,
    ) -> WriteableBitmap {
        let result = Self::new(size, dpi, Some(format), Some(alpha_format));

        let min_stride = (format.bits_per_pixel() as i32 * size.width + 7) / 8;
        if min_stride > stride {
            panic!("stride is out of range: {stride}");
        }

        let locked = result.lock();
        let row_bytes = locked.row_bytes();
        locked.with_data(&mut |dest| {
            let min_stride = min_stride as usize;
            for y in 0..size.height {
                let dst = (row_bytes as i64 * y as i64) as usize;
                let src = (stride as i64 * y as i64) as usize;
                dest[dst..dst + min_stride].copy_from_slice(&data[src..src + min_stride]);
            }
        });
        locked.dispose();

        result
    }

    /// The pixel format of the bitmap.
    pub fn format(&self) -> Option<PixelFormat> {
        match &self.pixel_format_memory {
            Some(memory) => Some(memory.borrow().format()),
            None => self.base.format(),
        }
    }

    /// Locks the pixel data of the bitmap and returns a framebuffer that can
    /// be used to read and write the pixels. The changes are committed when
    /// the framebuffer is disposed.
    pub fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        let platform_impl = self.base.platform_impl().item();
        let Some(memory) = &self.pixel_format_memory else {
            return platform_impl.as_readable_bitmap().expect("the platform bitmap is writeable").lock();
        };

        let (size, row_bytes, format, alpha_format) = {
            let memory = memory.borrow();
            (memory.size(), memory.row_bytes(), memory.format(), memory.alpha_format())
        };

        Rc::new(MemoryLockedFramebuffer {
            memory: memory.clone(),
            platform_impl,
            size,
            row_bytes,
            dpi: self.dpi(),
            format,
            alpha_format,
            disposed: Cell::new(false),
        })
    }

    /// Copies the pixels of `source_rect` into `buffer`, whose rows are
    /// `stride` bytes apart. An empty `source_rect` selects the whole
    /// bitmap.
    pub fn copy_pixels(&self, source_rect: PixelRect, buffer: &mut [u8], stride: i32) {
        let fb = self.lock();
        self.base.copy_pixels_core(source_rect, buffer, stride, &*fb);
        fb.dispose();
    }

    /// Loads a writeable bitmap from a stream.
    pub fn decode(stream: &mut dyn Read) -> std::io::Result<WriteableBitmap> {
        let ri = platform::render_interface();
        Ok(Self::from_impl(ri.load_writeable_bitmap(stream)?, None))
    }

    /// Loads a writeable bitmap from a stream, decoded to the specified
    /// width maintaining aspect ratio.
    pub fn decode_to_width(
        stream: &mut dyn Read,
        width: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<WriteableBitmap> {
        let ri = platform::render_interface();
        Ok(Self::from_impl(ri.load_writeable_bitmap_to_width(stream, width, interpolation_mode)?, None))
    }

    /// Loads a writeable bitmap from a stream, decoded to the specified
    /// height maintaining aspect ratio.
    pub fn decode_to_height(
        stream: &mut dyn Read,
        height: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<WriteableBitmap> {
        let ri = platform::render_interface();
        Ok(Self::from_impl(ri.load_writeable_bitmap_to_height(stream, height, interpolation_mode)?, None))
    }

    fn create_platform_impl(
        size: PixelSize,
        dpi: Vector,
        format: Option<PixelFormat>,
        alpha_format: Option<AlphaFormat>,
    ) -> (Rc<dyn IWriteableBitmapImpl>, Option<BitmapMemory>) {
        if size.width <= 0 || size.height <= 0 {
            panic!("Size should be >= (1,1)");
        }

        let ri = platform::render_interface();

        let final_format = format.unwrap_or_else(|| ri.default_pixel_format());
        let mut final_alpha_format = alpha_format.unwrap_or_else(|| ri.default_alpha_format());

        if ri.is_supported_bitmap_pixel_format(final_format) {
            return (ri.create_writeable_bitmap(size, dpi, final_format, final_alpha_format), None);
        }

        if !PixelFormatReader::supports_format(final_format) {
            panic!("Pixel format {final_format} is not supported");
        }

        final_alpha_format = if final_format.has_alpha() { final_alpha_format } else { AlphaFormat::Opaque };

        let platform_impl = ri.create_writeable_bitmap(size, dpi, PixelFormat::RGBA8888, final_alpha_format);
        (platform_impl, Some(BitmapMemory::new(final_format, final_alpha_format, size)))
    }
}

/// A framebuffer over the transcoding buffer of a [`WriteableBitmap`]: when
/// it is disposed, the pixels are converted into the platform bitmap.
struct MemoryLockedFramebuffer {
    memory: Rc<RefCell<BitmapMemory>>,
    platform_impl: Rc<dyn IBitmapImpl>,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    disposed: Cell<bool>,
}

impl ILockedFramebuffer for MemoryLockedFramebuffer {
    fn address(&self) -> *mut u8 {
        self.memory.borrow_mut().data_mut().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(self.memory.borrow_mut().data_mut());
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
        if self.disposed.replace(true) {
            return;
        }
        let inner = self.platform_impl.as_readable_bitmap().expect("the platform bitmap is writeable").lock();
        let (alpha_format, row_bytes) = (inner.alpha_format(), inner.row_bytes());
        let memory = self.memory.borrow();
        inner.with_data(&mut |dest| memory.copy_to_rgba(alpha_format, dest, row_bytes));
        inner.dispose();
    }
}

impl IImage for WriteableBitmap {
    fn size(&self) -> Size {
        self.base.size()
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source_rect: Rect, dest_rect: Rect) {
        IImage::draw(&self.base, context, source_rect, dest_rect)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_bitmap(&self) -> Option<&dyn IBitmap> {
        Some(self)
    }
}

impl IBitmap for WriteableBitmap {
    fn dpi(&self) -> Vector {
        self.base.dpi()
    }

    fn pixel_size(&self) -> PixelSize {
        self.base.pixel_size()
    }

    fn platform_impl(&self) -> &RefCounted<dyn IBitmapImpl> {
        self.base.platform_impl()
    }

    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> std::io::Result<()> {
        self.base.save(stream, options)
    }

    fn dispose(&self) {
        self.base.dispose()
    }
}

impl IImageBrushSource for WriteableBitmap {
    fn bitmap(&self) -> Option<&RefCounted<dyn IBitmapImpl>> {
        IImageBrushSource::bitmap(&self.base)
    }
}
