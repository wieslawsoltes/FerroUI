use crate::{
    drawing::{DrawBatch, DrawResource, DrawingContext},
    ffi::{self, Owned},
};
use ferroui_base::{media::imaging::BitmapEncoderOptions, platform::*, PixelSize, Rect, Vector};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    io::{self, Read, Write},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Mutex, MutexGuard, PoisonError,
    },
};
/// A lock that a panic of another holder does not poison: the state under these locks is whole
/// after every statement.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
struct Surface {
    context: Owned,
}
enum Recovery {
    Encoded(Vec<u8>),
    Pixels(Vec<u8>),
}
pub struct BitmapInner {
    surface: Mutex<Surface>,
    size: PixelSize,
    dpi: Vector,
    version: AtomicI32,
    active: AtomicBool,
    /// The surface was replaced after a device loss and its pixels could not be restored, or it
    /// could not be created: what was drawn is gone, and the owner has to draw it again.
    content_lost: AtomicBool,
    recovery: Mutex<Option<Recovery>>,
    pending: Mutex<DrawBatch>,
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
    /// A bitmap of `size`. A size that is not positive, a size over the device limit and a
    /// device that cannot be created give a bitmap without a surface: nothing is drawn into it,
    /// it draws nothing, and [`IDrawingContextLayerImpl::is_corrupted`] reports it.
    pub fn new(size: PixelSize, dpi: Vector) -> Self {
        Self::try_new(size, dpi)
            .unwrap_or_else(|_| Self::from_context(size, dpi, Owned::default()))
    }
    fn from_context(size: PixelSize, dpi: Vector, context: Owned) -> Self {
        Self(Arc::new(BitmapInner {
            surface: Mutex::new(Surface { context }),
            size: PixelSize::new(size.width.max(0), size.height.max(0)),
            dpi,
            version: AtomicI32::new(0),
            active: AtomicBool::new(false),
            content_lost: AtomicBool::new(false),
            recovery: Mutex::new(None),
            pending: Mutex::new(DrawBatch::default()),
        }))
    }
    /// A bitmap of `size`, or the reason there is no surface for it.
    pub fn try_new(size: PixelSize, dpi: Vector) -> io::Result<Self> {
        if size.width <= 0 || size.height <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "A Direct2D surface needs positive dimensions",
            ));
        }
        let context =
            unsafe { Owned::try_new(ffi::d2_surface(size.width as usize, size.height as usize)) }
                .ok_or_else(|| {
                    io::Error::other(
                        "Direct2D surface allocation failed or exceeds the device limit",
                    )
                })?;
        Ok(Self::from_context(size, dpi, context))
    }
    fn byte_length(&self) -> usize {
        self.size.width as usize * self.size.height as usize * 4
    }
    /// A surface in place of one the device lost, with the pixels that can be restored: the
    /// encoded image or the written pixels of the bitmap. Without a surface when the hardware
    /// device cannot be created again or the allocation fails.
    fn replacement(&self) -> Owned {
        if self.size.width <= 0 || self.size.height <= 0 || crate::device_generation() == 0 {
            return Owned::default();
        }
        let recovery = lock(&self.recovery);
        let replacement = unsafe {
            match &*recovery {
                Some(Recovery::Encoded(bytes)) => {
                    Owned::try_new(ffi::d2_decode(bytes.as_ptr(), bytes.len()))
                }
                pixels => {
                    let replacement = Owned::try_new(ffi::d2_surface(
                        self.size.width as usize,
                        self.size.height as usize,
                    ));
                    if let (Some(replacement), Some(Recovery::Pixels(bytes))) =
                        (&replacement, pixels)
                    {
                        ffi::d2_write_pixels(replacement.0, bytes.as_ptr(), bytes.len());
                    }
                    replacement
                }
            }
        };
        if recovery.is_none() || replacement.is_none() {
            self.content_lost.store(true, Ordering::Release);
        }
        replacement.unwrap_or_default()
    }
    /// Runs `f` with the native surface, after the commands queued for it. A bitmap without a
    /// surface drops the queued commands and answers with the default of `R`.
    pub fn context<R: Default>(&self, f: impl FnOnce(ffi::Ptr) -> R) -> R {
        let mut surface = lock(&self.surface);
        // Check driver health at drawing-session boundaries. EndDraw records
        // mid-session failures; native draws still validate device generation.
        if !self.active.load(Ordering::Acquire)
            && (surface.context.0.is_null()
                || unsafe { ffi::d2_surface_lost(surface.context.0) } != 0)
        {
            surface.context = self.replacement();
        }
        let mut batch = lock(&self.pending);
        if surface.context.0.is_null() {
            batch.commands.clear();
            batch.glyphs.clear();
            batch.paths.clear();
            return R::default();
        }
        if !batch.commands.is_empty() {
            unsafe {
                ffi::d2_draw_batch(
                    surface.context.0,
                    batch.commands.as_ptr(),
                    batch.commands.len(),
                )
            };
            batch.commands.clear();
            batch.glyphs.clear();
            batch.paths.clear();
        }
        drop(batch);
        f(surface.context.0)
    }
    pub(crate) fn queue(&self, command: ffi::DrawCommand, resource: Option<DrawResource>) {
        let flush = {
            let mut batch = lock(&self.pending);
            batch.commands.push(command);
            match resource {
                Some(DrawResource::Glyphs(glyphs)) => batch.glyphs.push(glyphs),
                Some(DrawResource::Path(path)) => batch.paths.push(path),
                None => {}
            }
            batch.commands.len() >= 4096
        };
        if flush {
            self.context(|_| {});
        }
    }
    /// An immutable copy of the content, null for a bitmap without a surface.
    pub fn image(&self) -> Owned {
        self.context(|p| unsafe {
            if !self.active.load(Ordering::Acquire) {
                ffi::d2_finish(p);
            }
            Owned::try_new(ffi::d2_snapshot(p)).unwrap_or_default()
        })
    }
    /// A reference to the surface itself, null for a bitmap without a surface.
    pub fn retained_surface(&self) -> Owned {
        self.context(|p| unsafe {
            if !self.active.load(Ordering::Acquire) {
                ffi::d2_finish(p);
            }
            Owned::try_new(ffi::d2_retain(p)).unwrap_or_default()
        })
    }
    /// Records that the content the bitmap was made for could not be produced (an external
    /// texture that was not ready): the bitmap stays blank and reports itself corrupted.
    pub(crate) fn mark_content_lost(&self) {
        self.content_lost.store(true, Ordering::Release);
    }
    /// The pixels, or `None` when the bitmap has no surface or the device could not read them.
    fn read_pixels(&self) -> Option<Vec<u8>> {
        let mut bytes = vec![0; self.byte_length()];
        let read =
            self.context(|p| unsafe { ffi::d2_read_pixels(p, bytes.as_mut_ptr(), bytes.len()) });
        (read != 0).then_some(bytes)
    }
    /// The pixels as premultiplied BGRA, transparent black when they cannot be read.
    pub fn pixels(&self) -> Vec<u8> {
        self.read_pixels()
            .unwrap_or_else(|| vec![0; self.byte_length()])
    }
    pub fn write(&self, bytes: &[u8]) {
        assert_eq!(bytes.len(), self.byte_length());
        self.context(|p| unsafe { ffi::d2_write_pixels(p, bytes.as_ptr(), bytes.len()) });
        *lock(&self.recovery) = Some(Recovery::Pixels(bytes.to_vec()));
        // Every pixel is written: nothing of the lost content is left to draw again.
        self.content_lost.store(false, Ordering::Release);
        self.version.fetch_add(1, Ordering::Relaxed);
    }
    pub fn draw_to(&self, target: &Bitmap, src: Rect, dst: Rect, alpha: f64) {
        let source = self.retained_surface();
        if source.0.is_null() {
            return;
        }
        target.context(|c| unsafe {
            ffi::d2_gpu_draw_surface(
                c, source.0, src.x, src.y, src.width, src.height, dst.x, dst.y, dst.width,
                dst.height, alpha,
            );
        });
    }
    pub fn drawing(&self, scale: bool, finished: Option<Box<dyn FnOnce()>>) -> DrawingContext {
        self.context(|_| {});
        lock(&self.recovery).take();
        assert!(
            !self.active.swap(true, Ordering::Acquire),
            "Two drawing contexts cannot draw into the same Direct2D bitmap"
        );
        DrawingContext::new(self.clone(), scale, finished)
    }
    pub fn finished(&self) {
        self.context(|p| unsafe { ffi::d2_finish(p) });
        self.version.fetch_add(1, Ordering::Relaxed);
        self.active.store(false, Ordering::Release)
    }
    pub fn decode(stream: &mut dyn Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes)?;
        unsafe {
            let p = ffi::d2_decode(bytes.as_ptr(), bytes.len());
            if p.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "WIC could not decode this image",
                ));
            }
            let image = Owned::new(p);
            let size = PixelSize::new(
                ffi::d2_image_width(p) as i32,
                ffi::d2_image_height(p) as i32,
            );
            let bitmap = Self::try_new(size, Vector::new(96., 96.))?;
            bitmap.context(|c| {
                ffi::d2_draw_image(
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
            *lock(&bitmap.recovery) = Some(Recovery::Encoded(bytes));
            Ok(bitmap)
        }
    }
    pub fn resized(
        source: &dyn IBitmapImpl,
        size: PixelSize,
        mode: ferroui_base::media::imaging::BitmapInterpolationMode,
    ) -> Self {
        let src = Self::from_impl(source).expect("Direct2D bitmap");
        let out = Self::new(size, source.dpi());
        out.context(|p| unsafe { ffi::d2_gpu_render_options(p, 0, 0, mode as i32) });
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
/// Framebuffers and Direct2D surfaces use premultiplied BGRA; conversion never retains a borrowed address.
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
        "The Direct2D backend supports BGRA8888, RGBA8888 and RGB565 framebuffers"
    );
    assert!(
        size.width > 0 && size.height > 0,
        "Bitmap dimensions must be positive"
    );
    let (w, h) = (size.width as usize, size.height as usize);
    let pixel_bytes = if format == PixelFormat::RGB565 { 2 } else { 4 };
    let row = w.checked_mul(pixel_bytes).expect("Bitmap row overflow");
    let required = (h - 1)
        .checked_mul(stride)
        .and_then(|v| v.checked_add(row))
        .expect("Bitmap buffer overflow");
    assert!(
        stride >= row && data.len() >= required,
        "Bitmap stride or source buffer too small"
    );
    let mut out = vec![0; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let p = &mut out[(y * w + x) * 4..][..4];
            if format == PixelFormat::RGB565 {
                let s = &data[y * stride + x * 2..][..2];
                let v = u16::from_le_bytes([s[0], s[1]]);
                let (r, g, b) = (((v >> 11) & 31) as u32, ((v >> 5) & 63) as u32, (v & 31) as u32);
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
        // WIC chooses the PNG compression: `PngEncoderOptions::compression_level` has no
        // counterpart in its encoder options and is not applied.
        let (jpeg, quality) = match options {
            BitmapEncoderOptions::Jpeg(o) => {
                if !(0..=100).contains(&o.quality) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Unknown JPEG quality",
                    ));
                }
                (1, o.quality as f64 / 100.)
            }
            _ => (0, 1.),
        };
        let image = self.image();
        if image.0.is_null() {
            return Err(io::Error::other("The bitmap has no Direct2D surface to encode"));
        }
        let encoded = unsafe { Owned::try_new(ffi::d2_encode(image.0, jpeg, quality)) }
            .ok_or_else(|| io::Error::other("WIC encoder failed"))?;
        stream.write_all(&encoded.bytes())
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
        // The pixels are read before the bitmap is marked active, so that a surface the device
        // lost is replaced first. The lock is exclusive: nothing draws into the bitmap while its
        // pixels are out, and the pixels go back only when they were changed.
        assert!(
            !self.active.load(Ordering::Acquire),
            "Cannot lock an active Direct2D drawing surface"
        );
        let bytes = self.pixels();
        assert!(
            !self.active.swap(true, Ordering::Acquire),
            "Cannot lock an active Direct2D drawing surface"
        );
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
        let source = self.retained_surface();
        if !source.0.is_null() {
            copy.context(|p| unsafe { ffi::d2_gpu_copy_surface(p, source.0) });
        }
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
        if self.content_lost.load(Ordering::Acquire) {
            return true;
        }
        let surface = lock(&self.surface);
        if surface.context.0.is_null() {
            // A bitmap of no size has nothing to lose; one that should have a surface lost it.
            return self.size.width > 0 && self.size.height > 0;
        }
        unsafe { ffi::d2_surface_lost(surface.context.0) != 0 }
    }
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(self.drawing(false, None))
    }
}
struct BitmapLock {
    bitmap: Bitmap,
    original: Vec<u8>,
    bytes: RefCell<Vec<u8>>,
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
