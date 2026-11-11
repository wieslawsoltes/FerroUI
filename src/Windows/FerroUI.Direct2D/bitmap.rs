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
        atomic::{AtomicI32, Ordering},
        Arc, Mutex,
    },
};
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
    active: std::sync::atomic::AtomicBool,
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
    pub fn new(size: PixelSize, dpi: Vector) -> Self {
        assert!(size.width > 0 && size.height > 0);
        unsafe {
            Self(Arc::new(BitmapInner {
                surface: Mutex::new(Surface {
                    context: Owned::new(ffi::d2_surface(size.width as usize, size.height as usize)),
                }),
                size,
                dpi,
                version: AtomicI32::new(0),
                active: std::sync::atomic::AtomicBool::new(false),
                recovery: Mutex::new(None),
                pending: Mutex::new(DrawBatch::default()),
            }))
        }
    }
    pub fn context<R>(&self, f: impl FnOnce(ffi::Ptr) -> R) -> R {
        let mut surface = self.surface.lock().unwrap();
        // Check driver health at drawing-session boundaries. EndDraw records
        // mid-session failures; native draws still validate device generation.
        if !self.active.load(Ordering::Acquire)
            && unsafe { ffi::d2_surface_lost(surface.context.0) } != 0
        {
            assert_ne!(
                crate::device_generation(),
                0,
                "Cannot recreate the hardware D3D11 device"
            );
            let recovery = self.recovery.lock().unwrap();
            surface.context = unsafe {
                match &*recovery {
                    Some(Recovery::Encoded(bytes)) => {
                        Owned::new(ffi::d2_decode(bytes.as_ptr(), bytes.len()))
                    }
                    pixels => {
                        let replacement = Owned::new(ffi::d2_surface(
                            self.size.width as usize,
                            self.size.height as usize,
                        ));
                        if let Some(Recovery::Pixels(bytes)) = pixels {
                            ffi::d2_write_pixels(replacement.0, bytes.as_ptr(), bytes.len());
                        }
                        replacement
                    }
                }
            };
        }
        {
            let mut batch = self.pending.lock().unwrap();
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
        }
        f(surface.context.0)
    }
    pub(crate) fn queue(&self, command: ffi::DrawCommand, resource: Option<DrawResource>) {
        let flush = {
            let mut batch = self.pending.lock().unwrap();
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
    pub fn image(&self) -> Owned {
        self.context(|p| unsafe {
            if !self.active.load(Ordering::Acquire) {
                ffi::d2_finish(p);
            }
            Owned::new(ffi::d2_snapshot(p))
        })
    }
    #[cfg(feature = "gpu")]
    pub fn retained_surface(&self) -> Owned {
        self.context(|p| unsafe {
            if !self.active.load(Ordering::Acquire) {
                ffi::d2_finish(p);
            }
            Owned::new(ffi::d2_retain(p))
        })
    }
    pub fn pixels(&self) -> Vec<u8> {
        let mut bytes = vec![0; self.size.width as usize * self.size.height as usize * 4];
        self.context(|p| unsafe { ffi::d2_read_pixels(p, bytes.as_mut_ptr(), bytes.len()) });
        bytes
    }
    pub fn write(&self, bytes: &[u8]) {
        assert_eq!(
            bytes.len(),
            self.size.width as usize * self.size.height as usize * 4
        );
        self.context(|p| unsafe { ffi::d2_write_pixels(p, bytes.as_ptr(), bytes.len()) });
        *self.recovery.lock().unwrap() = Some(Recovery::Pixels(bytes.to_vec()));
        self.version.fetch_add(1, Ordering::Relaxed);
    }
    pub fn draw_to(&self, target: &Bitmap, src: Rect, dst: Rect, alpha: f64) {
        #[cfg(feature = "gpu")]
        let source = self.retained_surface();
        #[cfg(not(feature = "gpu"))]
        let source = self.image();
        target.context(|c| unsafe {
            #[cfg(feature = "gpu")]
            ffi::d2_gpu_draw_surface(
                c, source.0, src.x, src.y, src.width, src.height, dst.x, dst.y, dst.width,
                dst.height, alpha,
            );
            #[cfg(not(feature = "gpu"))]
            ffi::d2_draw_image(
                c, source.0, src.x, src.y, src.width, src.height, dst.x, dst.y, dst.width,
                dst.height, alpha,
            );
        });
    }
    pub fn drawing(&self, scale: bool, finished: Option<Box<dyn FnOnce()>>) -> DrawingContext {
        self.context(|_| {});
        self.recovery.lock().unwrap().take();
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
            let bitmap = Self::new(size, Vector::new(96., 96.));
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
            *bitmap.recovery.lock().unwrap() = Some(Recovery::Encoded(bytes));
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
        #[cfg(feature = "gpu")]
        out.context(|p| unsafe { ffi::d2_gpu_render_options(p, 0, 0, mode as i32) });
        #[cfg(not(feature = "gpu"))]
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
        matches!(format, PixelFormat::BGRA8888 | PixelFormat::RGBA8888),
        "Direct2D prototype supports BGRA8888 and RGBA8888 framebuffers"
    );
    let (w, h) = (size.width as usize, size.height as usize);
    assert!(stride >= w * 4 && data.len() >= stride * h);
    let mut out = vec![0; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let s = &data[y * stride + x * 4..][..4];
            let p = &mut out[(y * w + x) * 4..][..4];
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
        let image = self.image();
        let (jpeg, quality) = match options {
            BitmapEncoderOptions::Jpeg(o) => (1, o.quality as f64 / 100.),
            _ => (0, 1.),
        };
        unsafe {
            let p = ffi::d2_encode(image.0, jpeg, quality);
            if p.is_null() {
                return Err(io::Error::other("WIC encoder failed"));
            }
            stream.write_all(&Owned::new(p).bytes())
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
            !self.active.load(Ordering::Acquire),
            "Cannot lock an active Direct2D drawing surface"
        );
        Rc::new(BitmapLock {
            bitmap: self.clone(),
            bytes: RefCell::new(self.pixels()),
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
        #[cfg(feature = "gpu")]
        {
            let source = self.retained_surface();
            copy.context(|p| unsafe { ffi::d2_gpu_copy_surface(p, source.0) });
        }
        #[cfg(not(feature = "gpu"))]
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
        let surface = self.surface.lock().unwrap();
        unsafe { ffi::d2_surface_lost(surface.context.0) != 0 }
    }
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(self.drawing(false, None))
    }
}
struct BitmapLock {
    bitmap: Bitmap,
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
            self.bitmap.write(&self.bytes.borrow())
        }
    }
}
impl Drop for BitmapLock {
    fn drop(&mut self) {
        self.dispose()
    }
}
