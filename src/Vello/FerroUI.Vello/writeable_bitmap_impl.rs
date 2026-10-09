use crate::helpers::image_saving_helper;
use crate::helpers::pixel_format_helper::{self, to_image, to_premul_rgba};
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::immutable_bitmap::{decode_bitmap, decode_bitmap_to_size};
use crate::vello_platform::VelloPlatform;
use ferroui_base::media::imaging::{BitmapEncoderOptions, BitmapInterpolationMode};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, ILockedFramebuffer, IReadableBitmapImpl, IWriteableBitmapImpl, PixelFormat,
};
use ferroui_base::{PixelSize, Vector};
use peniko::ImageData;
use std::any::Any;
use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, Weak};

/// A writeable bitmap: pixels in memory, in one of the formats of the
/// contract.
///
/// The UI thread writes the pixels and the render thread draws them: the
/// pixels and the image drawn from them are under one lock.
pub struct WriteableBitmapImpl {
    weak_self: Weak<WriteableBitmapImpl>,
    pixels: Mutex<Pixels>,
    dpi: Vector,
    pixel_size: PixelSize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    version: AtomicI32,
}

struct Pixels {
    /// `pixel_size.width * 4` bytes a row.
    data: Option<Vec<u8>>,
    /// The snapshot the bitmap is drawn from, until the pixels change.
    image: Option<ImageData>,
}

impl WriteableBitmapImpl {
    fn from_data(data: Vec<u8>, pixel_size: PixelSize, dpi: Vector, format: PixelFormat, alpha_format: AlphaFormat) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            pixels: Mutex::new(Pixels { data: Some(data), image: None }),
            dpi,
            pixel_size,
            format,
            alpha_format,
            version: AtomicI32::new(1),
        })
    }

    fn from_image(image: ImageData) -> Arc<Self> {
        let pixel_size = PixelSize::new(image.width as i32, image.height as i32);
        let data = image.data.data().to_vec();

        Self::from_data(data, pixel_size, VelloPlatform::default_dpi(), PixelFormat::RGBA8888, AlphaFormat::Premul)
    }

    /// Creates a writeable bitmap from the given stream.
    pub fn from_stream(stream: &mut dyn Read) -> io::Result<Arc<Self>> {
        Ok(Self::from_image(decode_bitmap(stream)?))
    }

    /// Creates a writeable bitmap from a stream, decoded so that its width
    /// (`horizontal`) or height is `decode_size`.
    pub fn from_stream_to_size(
        stream: &mut dyn Read,
        decode_size: i32,
        horizontal: bool,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<Self>> {
        Ok(Self::from_image(decode_bitmap_to_size(stream, decode_size, horizontal, interpolation_mode)?))
    }

    /// Creates a writeable bitmap with the given size, DPI and formats. The
    /// pixels start out transparent.
    ///
    /// # Panics
    /// Panics when the pixel format is not one the backend reads and writes
    /// or the size has no pixels.
    pub fn new(size: PixelSize, dpi: Vector, format: PixelFormat, alpha_format: AlphaFormat) -> Arc<Self> {
        if !pixel_format_helper::is_supported(format) {
            panic!("Unsupported pixel format: {format:?}. The Vello backend reads and writes RGBA8888 and BGRA8888.");
        }
        if size.width < 1 || size.height < 1 {
            panic!("Unable to allocate a {}x{} bitmap", size.width, size.height);
        }

        let data = vec![0u8; size.width as usize * size.height as usize * 4];

        Self::from_data(data, size, dpi, format, alpha_format)
    }

    fn with_data<R>(&self, f: impl FnOnce(&mut Vec<u8>) -> R) -> R {
        let mut pixels = self.pixels.lock().unwrap();
        f(pixels.data.as_mut().expect("WriteableBitmapImpl has been disposed"))
    }

    fn row_bytes(&self) -> usize {
        self.pixel_size.width as usize * 4
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
        self.version.load(Ordering::SeqCst)
    }

    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
        let image = self.image().ok_or_else(|| io::Error::other("WriteableBitmapImpl has been disposed"))?;
        image_saving_helper::save_image(&image, stream, options)
    }

    fn dispose(&self) {
        let mut pixels = self.pixels.lock().unwrap();
        pixels.image.take();
        pixels.data.take();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IDrawableBitmapImpl for WriteableBitmapImpl {
    fn image(&self) -> Option<ImageData> {
        let mut pixels = self.pixels.lock().unwrap();
        if pixels.image.is_none() {
            // NOTE: this does a snapshot of the bitmap, converted to the
            // format the renderers draw.
            let data = pixels.data.as_ref()?;
            let rgba = to_premul_rgba(data, self.row_bytes(), self.pixel_size, self.format, self.alpha_format);
            pixels.image = Some(to_image(rgba, self.pixel_size));
        }
        pixels.image.clone()
    }
}

impl IReadableBitmapImpl for WriteableBitmapImpl {
    fn format(&self) -> Option<PixelFormat> {
        Some(self.format)
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(self.alpha_format)
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        let parent = self.weak_self.upgrade().expect("WriteableBitmapImpl is alive while it is used");
        Rc::new(BitmapFramebuffer { parent: RefCell::new(Some(parent)) })
    }
}

impl IWriteableBitmapImpl for WriteableBitmapImpl {}

/// Framebuffer for a bitmap.
struct BitmapFramebuffer {
    parent: RefCell<Option<Arc<WriteableBitmapImpl>>>,
}

impl BitmapFramebuffer {
    fn with_parent<R>(&self, f: impl FnOnce(&WriteableBitmapImpl) -> R) -> R {
        let parent = self.parent.borrow();
        f(parent.as_ref().expect("the framebuffer has been unlocked"))
    }
}

impl ILockedFramebuffer for BitmapFramebuffer {
    fn address(&self) -> *mut u8 {
        // The pixels stay where they are until the bitmap is disposed: the
        // address outlives the lock it is read under, as the contract of a
        // framebuffer has it.
        self.with_parent(|parent| parent.with_data(|data| data.as_mut_ptr()))
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        self.with_parent(|parent| parent.with_data(|data| access(data)));
    }

    fn size(&self) -> PixelSize {
        self.with_parent(|parent| parent.pixel_size)
    }

    fn row_bytes(&self) -> i32 {
        self.with_parent(|parent| parent.row_bytes() as i32)
    }

    fn dpi(&self) -> Vector {
        self.with_parent(|parent| parent.dpi)
    }

    fn format(&self) -> PixelFormat {
        self.with_parent(|parent| parent.format)
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.with_parent(|parent| parent.alpha_format)
    }

    fn dispose(&self) {
        if let Some(parent) = self.parent.borrow_mut().take() {
            let mut pixels = parent.pixels.lock().unwrap();
            parent.version.fetch_add(1, Ordering::SeqCst);
            // The image is taken again from the new pixels when drawn.
            pixels.image = None;
        }
    }
}
