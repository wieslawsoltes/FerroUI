use crate::{
    drawing::DrawingContext,
    ffi::{self, Owned},
};
use ferroui_base::{media::imaging::BitmapEncoderOptions, platform::*, PixelSize, Rect, Vector};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    io::{self, Read, Write},
    rc::Rc,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc, Mutex,
    },
};
struct Surface {
    context: Owned,
}
pub struct BitmapInner {
    surface: Mutex<Surface>,
    size: PixelSize,
    dpi: Vector,
    version: AtomicI32,
    active: std::sync::atomic::AtomicBool,
}
#[derive(Clone)]
pub struct Bitmap(Arc<BitmapInner>);
impl std::ops::Deref for Bitmap {
    type Target = BitmapInner;
    fn deref(&self) -> &BitmapInner {
        &self.0
    }
}
impl Bitmap {
    pub fn from_impl(source: &dyn IBitmapImpl) -> Option<&Self> {
        source.as_any().downcast_ref::<Self>().or_else(|| {
            source
                .as_any()
                .downcast_ref::<crate::writeable_bitmap::WriteableBitmap>()
                .map(|b| b.bitmap())
        })
    }
    /// A GPU surface in Metal builds. Allocation failures remain transparent;
    /// presentation uses `try_new` to report them to the compositor.
    pub fn new(size: PixelSize, dpi: Vector) -> Self {
        Self::try_new(size, dpi).unwrap_or_else(|_| Self::from_context(size, dpi, Owned::default()))
    }
    pub(super) fn from_context(size: PixelSize, dpi: Vector, context: Owned) -> Self {
        Self(Arc::new(BitmapInner {
            surface: Mutex::new(Surface { context }),
            size: PixelSize::new(size.width.max(0), size.height.max(0)),
            dpi,
            version: AtomicI32::new(0),
            active: std::sync::atomic::AtomicBool::new(false),
        }))
    }
    pub fn try_new(size: PixelSize, dpi: Vector) -> io::Result<Self> {
        if size.width <= 0 || size.height <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "A Quartz surface needs positive dimensions",
            ));
        }
        let raw = unsafe { ffi::q_surface(size.width as usize, size.height as usize) };
        if raw.is_null() {
            return Err(io::Error::other(
                "Quartz surface allocation failed or exceeds the device limit",
            ));
        }
        Ok(Self::from_context(size, dpi, unsafe { Owned::new(raw) }))
    }
    pub fn context<R: Default>(&self, f: impl FnOnce(ffi::Ptr) -> R) -> R {
        let surface = self
            .surface
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if surface.context.0.is_null() {
            R::default()
        } else {
            f(surface.context.0)
        }
    }
    pub fn image(&self) -> Owned {
        self.context(|p| unsafe { Owned::new(ffi::q_snapshot(p)) })
    }
    #[cfg(feature = "metal")]
    pub fn retained_surface(&self) -> Owned {
        self.context(|p| unsafe { Owned::new(ffi::q_retain(p)) })
    }
    pub fn pixels(&self) -> Vec<u8> {
        let mut bytes = vec![0; self.size.width as usize * self.size.height as usize * 4];
        self.context(|p| unsafe { ffi::q_read_pixels(p, bytes.as_mut_ptr(), bytes.len()) });
        bytes
    }
    pub fn write(&self, bytes: &[u8]) {
        assert_eq!(
            bytes.len(),
            self.size.width as usize * self.size.height as usize * 4
        );
        self.context(|p| unsafe { ffi::q_write_pixels(p, bytes.as_ptr(), bytes.len()) });
        self.version.fetch_add(1, Ordering::Relaxed);
    }
    pub fn draw_to(&self, target: &Bitmap, src: Rect, dst: Rect, alpha: f64) {
        #[cfg(feature = "metal")]
        let source = self.retained_surface();
        #[cfg(not(feature = "metal"))]
        let source = self.image();
        target.context(|c| unsafe {
            #[cfg(feature = "metal")]
            ffi::qm_draw_surface(
                c, source.0, src.x, src.y, src.width, src.height, dst.x, dst.y, dst.width,
                dst.height, alpha,
            );
            #[cfg(not(feature = "metal"))]
            ffi::q_draw_image(
                c, source.0, src.x, src.y, src.width, src.height, dst.x, dst.y, dst.width,
                dst.height, alpha,
            );
        });
    }
    pub fn drawing(&self, scale: bool, finished: Option<Box<dyn FnOnce()>>) -> DrawingContext {
        assert!(
            !self.active.swap(true, Ordering::Acquire),
            "Two drawing contexts cannot draw into the same Quartz bitmap"
        );
        DrawingContext::new(self.clone(), scale, finished)
    }
    pub fn finished(&self) {
        self.version.fetch_add(1, Ordering::Relaxed);
        self.active.store(false, Ordering::Release)
    }
    pub fn decode(stream: &mut dyn Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes)?;
        unsafe {
            let p = ffi::q_decode(bytes.as_ptr(), bytes.len());
            if p.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ImageIO could not decode this image",
                ));
            }
            let image = Owned::new(p);
            let size = PixelSize::new(ffi::q_image_width(p) as i32, ffi::q_image_height(p) as i32);
            let bitmap = Self::try_new(size, Vector::new(96., 96.))?;
            bitmap.context(|c| {
                ffi::q_draw_image(
                    c,
                    image.0,
                    0.,
                    0.,
                    size.width as f64,
                    size.height as f64,
                    0.,
                    0.,
                    size.width as f64,
                    size.height as f64,
                    1.,
                )
            });
            Ok(bitmap)
        }
    }
    pub fn resized(
        source: &dyn IBitmapImpl,
        size: PixelSize,
        mode: ferroui_base::media::imaging::BitmapInterpolationMode,
    ) -> Self {
        let src = Self::from_impl(source).expect("Quartz bitmap");
        let out = Self::new(size, source.dpi());
        #[cfg(feature = "metal")]
        out.context(|p| unsafe { ffi::qm_render_options(p, 0, 0, mode as i32) });
        #[cfg(not(feature = "metal"))]
        let _ = mode;
        src.draw_to(
            &out,
            Rect::new(0., 0., src.size.width as f64, src.size.height as f64),
            Rect::new(0., 0., size.width as f64, size.height as f64),
            1.,
        );
        out
    }
    pub fn from_pixels(
        format: PixelFormat,
        alpha: AlphaFormat,
        data: &[u8],
        size: PixelSize,
        dpi: Vector,
        stride: i32,
    ) -> Self {
        assert!(stride > 0, "Bitmap stride must be positive");
        let bitmap = Self::new(size, dpi);
        bitmap.write(&convert(data, stride as usize, size, format, alpha));
        bitmap
    }
}
/// Framebuffers and Quartz surfaces use premultiplied BGRA; conversion never retains a borrowed address.
pub fn convert(
    data: &[u8],
    stride: usize,
    size: PixelSize,
    format: PixelFormat,
    alpha: AlphaFormat,
) -> Vec<u8> {
    assert!(
        matches!(
            format,
            PixelFormat::BGRA8888 | PixelFormat::RGBA8888 | PixelFormat::RGB565
        ),
        "Quartz supports BGRA8888, RGBA8888 and RGB565 framebuffers"
    );
    assert!(
        size.width > 0 && size.height > 0,
        "Bitmap size must be positive"
    );
    let (w, h) = (size.width as usize, size.height as usize);
    let pixel_bytes = if format == PixelFormat::RGB565 { 2 } else { 4 };
    let row = w.checked_mul(pixel_bytes).expect("Bitmap row overflow");
    let required = (h - 1)
        .checked_mul(stride)
        .and_then(|v| v.checked_add(row))
        .expect("Bitmap storage overflow");
    assert!(
        stride >= row && data.len() >= required,
        "Bitmap stride or source buffer too small"
    );
    let mut out = vec![0; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let p = &mut out[(y * w + x) * 4..][..4];
            if format == PixelFormat::RGB565 {
                let offset = y * stride + x * 2;
                let v = u16::from_le_bytes([data[offset], data[offset + 1]]);
                let r = ((v >> 11) & 31) as u32;
                let g = ((v >> 5) & 63) as u32;
                let b = (v & 31) as u32;
                p.copy_from_slice(&[
                    ((b * 255 + 15) / 31) as u8,
                    ((g * 255 + 31) / 63) as u8,
                    ((r * 255 + 15) / 31) as u8,
                    255,
                ]);
                continue;
            }
            let s = &data[y * stride + x * 4..][..4];
            p.copy_from_slice(s);
            if format == PixelFormat::RGBA8888 {
                p.swap(0, 2)
            }
            if alpha == AlphaFormat::Opaque {
                p[3] = 255
            } else if alpha == AlphaFormat::Unpremul {
                for i in 0..3 {
                    p[i] = ((p[i] as u16 * p[3] as u16 + 127) / 255) as u8
                }
            }
        }
    }
    out
}
impl IBitmapImpl for Bitmap {
    fn dpi(&self) -> Vector {
        self.dpi
    }
    fn pixel_size(&self) -> PixelSize {
        self.size
    }
    fn version(&self) -> i32 {
        self.version.load(Ordering::Relaxed)
    }
    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
        if let BitmapEncoderOptions::Jpeg(o) = options {
            if !(0..=100).contains(&o.quality) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Unknown JPEG quality",
                ));
            }
        }
        let image = self.image();
        let (jpeg, quality) = match options {
            BitmapEncoderOptions::Jpeg(o) => (1, o.quality as f64 / 100.),
            _ => (0, 1.),
        };
        unsafe {
            let p = ffi::q_encode(image.0, jpeg, quality);
            if p.is_null() {
                return Err(io::Error::other("ImageIO encoder failed"));
            }
            let bytes = Owned::new(p).bytes();
            match options {
                BitmapEncoderOptions::Png(o) => stream.write_all(
                    &crate::png_compression::recompress(&bytes, o.compression_level)?,
                ),
                BitmapEncoderOptions::Jpeg(_) => stream.write_all(&bytes),
            }
        }
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}
impl IReadableBitmapImpl for Bitmap {
    fn format(&self) -> Option<PixelFormat> {
        Some(PixelFormat::BGRA8888)
    }
    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(AlphaFormat::Premul)
    }
    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        assert!(
            !self.active.swap(true, Ordering::Acquire),
            "Cannot lock an active Quartz drawing surface"
        );
        let bytes = self.pixels();
        Rc::new(BitmapLock {
            bitmap: self.clone(),
            original: bytes.clone(),
            bytes: RefCell::new(bytes),
            done: Cell::new(false),
        })
    }
}
impl IWriteableBitmapImpl for Bitmap {}
impl IRenderTargetBitmapImpl for Bitmap {
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(self.drawing(true, None))
    }
}
impl IDrawingContextLayerImpl for Bitmap {
    fn create_shared_snapshot(&self) -> Arc<SharedBitmapImpl> {
        let copy = Bitmap::new(self.size, self.dpi);
        #[cfg(feature = "metal")]
        {
            let source = self.retained_surface();
            copy.context(|p| unsafe { ffi::qm_copy_surface(p, source.0) });
        }
        #[cfg(not(feature = "metal"))]
        copy.write(&self.pixels());
        Arc::new(copy)
    }
    fn blit(&self, c: &mut dyn IDrawingContextImpl) {
        let size = self.size;
        let r = Rect::new(0., 0., size.width as f64, size.height as f64);
        c.draw_bitmap(self, 1., r, r)
    }
    fn can_blit(&self) -> bool {
        true
    }
    fn is_corrupted(&self) -> bool {
        self.surface
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .context
            .0
            .is_null()
    }
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(self.drawing(false, None))
    }
}
struct BitmapLock {
    bitmap: Bitmap,
    bytes: RefCell<Vec<u8>>,
    original: Vec<u8>,
    done: Cell<bool>,
}
impl ILockedFramebuffer for BitmapLock {
    fn address(&self) -> *mut u8 {
        assert!(!self.done.get());
        self.bytes.borrow_mut().as_mut_ptr()
    }
    fn with_data(&self, f: &mut dyn FnMut(&mut [u8])) {
        assert!(!self.done.get());
        f(&mut self.bytes.borrow_mut())
    }
    fn size(&self) -> PixelSize {
        self.bitmap.size
    }
    fn row_bytes(&self) -> i32 {
        self.bitmap.size.width * 4
    }
    fn dpi(&self) -> Vector {
        self.bitmap.dpi
    }
    fn format(&self) -> PixelFormat {
        PixelFormat::BGRA8888
    }
    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }
    fn dispose(&self) {
        if !self.done.replace(true) {
            let bytes = self.bytes.borrow();
            if *bytes != self.original {
                self.bitmap.write(&bytes);
            }
            self.bitmap.active.store(false, Ordering::Release);
        }
    }
}
impl Drop for BitmapLock {
    fn drop(&mut self) {
        self.dispose()
    }
}
