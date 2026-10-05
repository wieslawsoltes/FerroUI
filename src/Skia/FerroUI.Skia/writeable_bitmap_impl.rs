use crate::helpers::image_saving_helper;
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::immutable_bitmap::{decode_bitmap, decode_bitmap_to_size};
use crate::skia_platform::SkiaPlatform;
use crate::skia_sharp_extensions::{
    to_alpha_format, to_pixel_format, to_pixel_format_opt, to_sk_alpha_type, to_sk_color_type_or_panic,
};
use ferroui_base::media::imaging::{BitmapEncoderOptions, BitmapInterpolationMode};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, ILockedFramebuffer, IReadableBitmapImpl, IWriteableBitmapImpl, PixelFormat,
};
use ferroui_base::{PixelSize, Vector};
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{images, Bitmap, Canvas, Color, Data, Image, ImageInfo, Paint, Rect, SamplingOptions};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::io::{self, Read, Write};
use std::rc::{Rc, Weak};

/// Skia based writeable bitmap.
pub struct WriteableBitmapImpl {
    weak_self: Weak<WriteableBitmapImpl>,
    bitmap: RefCell<Option<Bitmap>>,
    image: RefCell<Option<Image>>,
    image_valid: Cell<bool>,
    dpi: Vector,
    pixel_size: PixelSize,
    version: Cell<i32>,
}

impl WriteableBitmapImpl {
    fn from_bitmap(bitmap: Bitmap, pixel_size: PixelSize, dpi: Vector) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            bitmap: RefCell::new(Some(bitmap)),
            image: RefCell::new(None),
            image_valid: Cell::new(false),
            dpi,
            pixel_size,
            version: Cell::new(1),
        })
    }

    /// Creates a writeable bitmap from the given stream.
    pub fn from_stream(stream: &mut dyn Read) -> io::Result<Rc<Self>> {
        let bitmap = decode_bitmap(stream)?;
        let pixel_size = PixelSize::new(bitmap.width(), bitmap.height());

        Ok(Self::from_bitmap(bitmap, pixel_size, SkiaPlatform::default_dpi()))
    }

    /// Creates a writeable bitmap from a stream, decoded so that its width
    /// (`horizontal`) or height is `decode_size`.
    pub fn from_stream_to_size(
        stream: &mut dyn Read,
        decode_size: i32,
        horizontal: bool,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Rc<Self>> {
        let bitmap = decode_bitmap_to_size(stream, decode_size, horizontal, interpolation_mode)?;
        let pixel_size = PixelSize::new(bitmap.width(), bitmap.height());

        Ok(Self::from_bitmap(bitmap, pixel_size, SkiaPlatform::default_dpi()))
    }

    /// Creates a writeable bitmap with the given size, DPI and formats. The
    /// pixels start out transparent.
    ///
    /// # Panics
    /// Panics when the pixel format is unknown to Skia or the pixels cannot
    /// be allocated.
    pub fn new(size: PixelSize, dpi: Vector, format: PixelFormat, alpha_format: AlphaFormat) -> Rc<Self> {
        let color_type = to_sk_color_type_or_panic(format);
        let alpha_type = to_sk_alpha_type(alpha_format);

        let nfo = ImageInfo::new((size.width, size.height), color_type, alpha_type, None);

        let mut bitmap = Bitmap::new();
        if !bitmap.try_alloc_pixels_info(&nfo, None) {
            panic!("Unable to allocate a {}x{} bitmap", size.width, size.height);
        }
        bitmap.erase_color(Color::TRANSPARENT);

        Self::from_bitmap(bitmap, size, dpi)
    }

    fn with_bitmap<R>(&self, f: impl FnOnce(&Bitmap) -> R) -> R {
        let bitmap = self.bitmap.borrow();
        f(bitmap.as_ref().expect("WriteableBitmapImpl has been disposed"))
    }

    /// Gets a snapshot of the bitmap: an image holding a copy of the current
    /// pixels.
    pub fn get_snapshot(&self) -> Image {
        self.with_bitmap(|bitmap| {
            let pixmap = bitmap.pixmap();
            let bytes = pixmap.bytes().unwrap_or(&[]);
            images::raster_from_data(bitmap.info(), Data::new_copy(bytes), bitmap.row_bytes())
                .unwrap_or_else(|| panic!("Unable to create an image from the bitmap pixels"))
        })
    }
}

impl IBitmapImpl for WriteableBitmapImpl {
    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn pixel_size(&self) -> PixelSize {
        self.pixel_size
    }

    fn version(&self) -> i32 {
        self.version.get()
    }

    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
        let image = self.get_snapshot();
        image_saving_helper::save_image(&image, stream, options)
    }

    fn dispose(&self) {
        self.image.borrow_mut().take();
        self.bitmap.borrow_mut().take();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IDrawableBitmapImpl for WriteableBitmapImpl {
    fn draw(
        &self,
        canvas: &Canvas,
        source_rect: &Rect,
        dest_rect: &Rect,
        sampling_options: SamplingOptions,
        paint: &Paint,
    ) {
        if self.image.borrow().is_none() || !self.image_valid.get() {
            // NOTE: this does a snapshot of the bitmap. If the canvas is not
            // GPU-backed we might want to avoid that by force-sharing the
            // pixel data with the bitmap, but that would require manual pixel
            // buffer management.
            let snapshot = self.get_snapshot();
            *self.image.borrow_mut() = Some(snapshot);
            self.image_valid.set(true);
        }

        let image = self.image.borrow();
        if let Some(image) = image.as_ref() {
            canvas.draw_image_rect_with_sampling_options(
                image,
                Some((source_rect, SrcRectConstraint::Fast)),
                dest_rect,
                sampling_options,
                paint,
            );
        }
    }
}

impl IReadableBitmapImpl for WriteableBitmapImpl {
    fn format(&self) -> Option<PixelFormat> {
        self.with_bitmap(|bitmap| to_pixel_format_opt(bitmap.color_type()))
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        self.with_bitmap(|bitmap| Some(to_alpha_format(bitmap.alpha_type())))
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        let parent = self.weak_self.upgrade().expect("WriteableBitmapImpl is alive while it is used");
        Rc::new(BitmapFramebuffer { parent: RefCell::new(Some(parent)) })
    }
}

impl IWriteableBitmapImpl for WriteableBitmapImpl {}

/// Framebuffer for a bitmap.
struct BitmapFramebuffer {
    parent: RefCell<Option<Rc<WriteableBitmapImpl>>>,
}

impl BitmapFramebuffer {
    fn with_bitmap<R>(&self, f: impl FnOnce(&WriteableBitmapImpl, &Bitmap) -> R) -> R {
        let parent = self.parent.borrow();
        let parent = parent.as_ref().expect("the framebuffer has been unlocked");
        parent.with_bitmap(|bitmap| f(parent, bitmap))
    }
}

impl ILockedFramebuffer for BitmapFramebuffer {
    fn address(&self) -> *mut u8 {
        self.with_bitmap(|_, bitmap| bitmap.pixmap().addr() as *mut u8)
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        self.with_bitmap(|_, bitmap| match bitmap.peek_pixels().as_mut().and_then(|pixmap| pixmap.bytes_mut()) {
            Some(bytes) => access(bytes),
            None => access(&mut []),
        });
    }

    fn size(&self) -> PixelSize {
        self.with_bitmap(|_, bitmap| PixelSize::new(bitmap.width(), bitmap.height()))
    }

    fn row_bytes(&self) -> i32 {
        self.with_bitmap(|_, bitmap| bitmap.row_bytes() as i32)
    }

    fn dpi(&self) -> Vector {
        self.with_bitmap(|parent, _| parent.dpi)
    }

    fn format(&self) -> PixelFormat {
        self.with_bitmap(|_, bitmap| to_pixel_format(bitmap.color_type()))
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.with_bitmap(|_, bitmap| to_alpha_format(bitmap.alpha_type()))
    }

    fn dispose(&self) {
        if let Some(parent) = self.parent.borrow_mut().take() {
            if let Some(bitmap) = parent.bitmap.borrow().as_ref() {
                bitmap.notify_pixels_changed();
            }
            parent.version.set(parent.version.get() + 1);
            parent.image_valid.set(false);
        }
    }
}
