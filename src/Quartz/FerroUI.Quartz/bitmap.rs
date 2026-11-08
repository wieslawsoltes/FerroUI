use crate::{
    drawing::DrawingContext,
    ffi::{self, Owned},
    warn_once,
};
use ferroui_base::{media::imaging::BitmapEncoderOptions, platform::*, PixelSize, Rect, Vector};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    io::{self, Read, Write},
    rc::Rc,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc, Mutex, PoisonError,
    },
};
struct Surface {
    context: Owned,
}
pub struct BitmapInner {
    /// `None` when no native surface could be made: the bitmap is then fully transparent and
    /// drawing onto it does nothing.
    surface: Mutex<Option<Surface>>,
    size: PixelSize,
    dpi: Vector,
    /// The layout `lock` exposes. Pixels are always stored as premultiplied BGRA.
    format: PixelFormat,
    alpha: AlphaFormat,
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
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn dimensions(size: PixelSize) -> io::Result<(usize, usize)> {
    match (usize::try_from(size.width), usize::try_from(size.height)) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => Ok((w, h)),
        _ => Err(invalid("A Quartz bitmap needs a size of at least 1x1 pixels")),
    }
}
fn supported_layout(format: PixelFormat) -> io::Result<()> {
    if matches!(format, PixelFormat::BGRA8888 | PixelFormat::RGBA8888) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Quartz bitmaps hold BGRA8888 or RGBA8888 pixels",
        ))
    }
}
impl Bitmap {
    fn from_context(
        context: Option<Owned>,
        size: PixelSize,
        dpi: Vector,
        format: PixelFormat,
        alpha: AlphaFormat,
    ) -> Self {
        Self(Arc::new(BitmapInner {
            surface: Mutex::new(context.map(|context| Surface { context })),
            size: PixelSize::new(size.width.max(0), size.height.max(0)),
            dpi,
            format,
            alpha,
            version: AtomicI32::new(0),
            active: std::sync::atomic::AtomicBool::new(false),
        }))
    }
    /// A transparent premultiplied BGRA bitmap: on the GPU when a Metal renderer is available,
    /// otherwise on the CPU.
    pub fn try_new(size: PixelSize, dpi: Vector) -> io::Result<Self> {
        Self::try_with_format(size, dpi, PixelFormat::BGRA8888, AlphaFormat::Premul)
    }
    /// A bitmap whose `lock` exposes `format` and `alpha`; BGRA8888 and RGBA8888 with any
    /// alpha format are supported.
    pub fn try_with_format(
        size: PixelSize,
        dpi: Vector,
        format: PixelFormat,
        alpha: AlphaFormat,
    ) -> io::Result<Self> {
        supported_layout(format)?;
        let (w, h) = dimensions(size)?;
        // SAFETY: q_surface returns NULL or a +1 surface the bitmap then owns.
        let context = unsafe { Owned::new(ffi::q_surface(w, h)) }.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::OutOfMemory,
                "Core Graphics could not allocate the bitmap",
            )
        })?;
        Ok(Self::from_context(Some(context), size, dpi, format, alpha))
    }
    /// A GPU-only bitmap, for presentation. Fails for an empty size, a size beyond the device
    /// limit, or when no Metal renderer is available.
    #[cfg(feature = "metal")]
    pub fn try_new_gpu(size: PixelSize, dpi: Vector) -> io::Result<Self> {
        let (w, h) = dimensions(size)?;
        // SAFETY: plain queries of the native renderer.
        let max = unsafe { ffi::qm_max_texture_size() };
        if w > max || h > max {
            return Err(invalid("The surface is larger than the Metal device supports"));
        }
        // SAFETY: qm_surface returns NULL or a +1 surface the bitmap then owns.
        let context = unsafe { Owned::new(ffi::qm_surface(w, h)) }.ok_or_else(|| {
            io::Error::other(ffi::status_message(unsafe { ffi::qm_status() }))
        })?;
        Ok(Self::from_context(
            Some(context),
            size,
            dpi,
            PixelFormat::BGRA8888,
            AlphaFormat::Premul,
        ))
    }
    /// Like [`Bitmap::try_new`], but never fails: without a native surface the bitmap stays
    /// transparent and drawing onto it does nothing.
    pub fn new(size: PixelSize, dpi: Vector) -> Self {
        Self::with_format(size, dpi, PixelFormat::BGRA8888, AlphaFormat::Premul)
    }
    /// Like [`Bitmap::try_with_format`], but never fails (see [`Bitmap::new`]). An unsupported
    /// layout is stored, and exposed, as premultiplied BGRA.
    pub fn with_format(
        size: PixelSize,
        dpi: Vector,
        format: PixelFormat,
        alpha: AlphaFormat,
    ) -> Self {
        let (format, alpha) = if supported_layout(format).is_ok() {
            (format, alpha)
        } else {
            warn_once("bitmap pixel format stored as premultiplied BGRA8888");
            (PixelFormat::BGRA8888, AlphaFormat::Premul)
        };
        Self::try_with_format(size, dpi, format, alpha).unwrap_or_else(|_| {
            warn_once("a bitmap surface could not be allocated; it stays transparent");
            Self::from_context(None, size, dpi, format, alpha)
        })
    }
    fn byte_len(&self) -> usize {
        self.size.width as usize * self.size.height as usize * 4
    }
    /// Runs `f` with the native drawing context, or returns `None` for a bitmap without one.
    pub fn try_context<R>(&self, f: impl FnOnce(ffi::Ptr) -> R) -> Option<R> {
        let surface = self.surface.lock().unwrap_or_else(PoisonError::into_inner);
        surface.as_ref().map(|surface| f(surface.context.0))
    }
    /// Runs `f` with the native drawing context; a bitmap without one yields `R::default()`.
    pub fn context<R: Default>(&self, f: impl FnOnce(ffi::Ptr) -> R) -> R {
        self.try_context(f).unwrap_or_default()
    }
    /// Whether the pixels live on the GPU.
    #[cfg_attr(not(feature = "metal"), allow(dead_code))]
    pub fn is_gpu(&self) -> bool {
        // SAFETY: a type query of a live context.
        self.context(|p| unsafe { ffi::q_is_gpu(p) } != 0)
    }
    pub fn image(&self) -> Option<Owned> {
        // SAFETY: q_snapshot returns NULL or a +1 CGImage.
        self.try_context(|p| unsafe { Owned::new(ffi::q_snapshot(p)) })
            .flatten()
    }
    /// A +1 reference to the native surface, for GPU operations that read it.
    #[cfg_attr(not(feature = "metal"), allow(dead_code))]
    pub fn retained_surface(&self) -> Option<Owned> {
        // SAFETY: q_retain returns a +1 reference to a live context.
        self.try_context(|p| unsafe { Owned::new(ffi::q_retain(p)) })
            .flatten()
    }
    /// A +1 drawable source of this bitmap for `q_draw_source`.
    fn source(&self) -> Option<Owned> {
        // SAFETY: q_source returns NULL or a +1 surface or image.
        self.try_context(|p| unsafe { Owned::new(ffi::q_source(p)) })
            .flatten()
    }
    /// The pixels as tightly packed premultiplied BGRA.
    pub fn try_pixels(&self) -> io::Result<Vec<u8>> {
        let mut bytes = vec![0; self.byte_len()];
        // SAFETY: `bytes` holds exactly `bytes.len()` writable bytes.
        let status = self
            .try_context(|p| unsafe { ffi::q_read_pixels(p, bytes.as_mut_ptr(), bytes.len()) })
            .unwrap_or(ffi::STATUS_OK); // An empty bitmap is transparent.
        if status == ffi::STATUS_OK {
            Ok(bytes)
        } else {
            Err(io::Error::other(ffi::status_message(status)))
        }
    }
    /// [`Bitmap::try_pixels`], transparent when the pixels cannot be read.
    pub fn pixels(&self) -> Vec<u8> {
        self.try_pixels().unwrap_or_else(|_| {
            warn_once("bitmap pixels could not be read back");
            vec![0; self.byte_len()]
        })
    }
    /// Replaces the pixels with tightly packed premultiplied BGRA.
    pub fn try_write(&self, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() != self.byte_len() {
            return Err(invalid("The pixel data does not match the bitmap size"));
        }
        // SAFETY: `bytes` holds exactly `bytes.len()` readable bytes.
        let status = self
            .try_context(|p| unsafe { ffi::q_write_pixels(p, bytes.as_ptr(), bytes.len()) })
            .unwrap_or(ffi::STATUS_OK);
        self.version.fetch_add(1, Ordering::Relaxed);
        if status == ffi::STATUS_OK {
            Ok(())
        } else {
            Err(io::Error::other(ffi::status_message(status)))
        }
    }
    pub fn write(&self, bytes: &[u8]) {
        if self.try_write(bytes).is_err() {
            warn_once("bitmap pixels could not be written");
        }
    }
    pub fn draw_to(&self, target: &Bitmap, src: Rect, dst: Rect, alpha: f64) {
        // The source is taken before the target is locked: a bitmap may draw onto itself.
        let Some(source) = self.source() else {
            return;
        };
        target.context(|c| unsafe {
            ffi::q_draw_source(
                c, source.0, src.x, src.y, src.width, src.height, dst.x, dst.y, dst.width,
                dst.height, alpha,
            )
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
        // SAFETY: q_decode returns NULL or a +1 CGImage.
        let image = unsafe { Owned::new(ffi::q_decode(bytes.as_ptr(), bytes.len())) }.ok_or_else(
            || io::Error::new(io::ErrorKind::InvalidData, "ImageIO could not decode this image"),
        )?;
        // SAFETY: plain queries of a live CGImage.
        let (w, h) = unsafe { (ffi::q_image_width(image.0), ffi::q_image_height(image.0)) };
        let (Ok(width), Ok(height)) = (i32::try_from(w), i32::try_from(h)) else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The image is too large"));
        };
        let size = PixelSize::new(width, height);
        let bitmap = Self::try_new(size, Vector::new(96., 96.))?;
        bitmap.context(|c| unsafe {
            ffi::q_draw_image(
                c,
                image.0,
                0.,
                0.,
                w as f64,
                h as f64,
                0.,
                0.,
                w as f64,
                h as f64,
                1.,
            )
        });
        Ok(bitmap)
    }
    pub fn resized(
        source: &dyn IBitmapImpl,
        size: PixelSize,
        mode: ferroui_base::media::imaging::BitmapInterpolationMode,
    ) -> Self {
        let out = Self::new(size, source.dpi());
        let Some(src) = source.as_any().downcast_ref::<Bitmap>() else {
            warn_once("resizing a bitmap of another backend; the result is transparent");
            return out;
        };
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
        let bitmap = Self::new(size, dpi);
        match usize::try_from(stride)
            .map_err(|_| invalid("negative stride"))
            .and_then(|stride| convert(data, stride, size, format, alpha))
        {
            Ok(pixels) => bitmap.write(&pixels),
            Err(_) => warn_once("pixel data in an unsupported layout; the bitmap stays transparent"),
        }
        bitmap
    }
}
/// Converts BGRA8888/RGBA8888 rows in any alpha format into the tightly packed premultiplied
/// BGRA that Quartz surfaces hold. Conversion never retains a borrowed address.
pub fn convert(
    data: &[u8],
    stride: usize,
    size: PixelSize,
    format: PixelFormat,
    alpha: AlphaFormat,
) -> io::Result<Vec<u8>> {
    supported_layout(format)?;
    let (w, h) = (size.width.max(0) as usize, size.height.max(0) as usize);
    if h > 0 && (stride < w * 4 || data.len() < stride * (h - 1) + w * 4) {
        return Err(invalid("The pixel data is smaller than its size and stride"));
    }
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
    Ok(out)
}
/// The inverse of [`convert`]: premultiplied BGRA into tightly packed rows of `format` and
/// `alpha`.
pub fn export(premultiplied: &[u8], format: PixelFormat, alpha: AlphaFormat) -> Vec<u8> {
    let mut out = premultiplied.to_vec();
    for p in out.chunks_exact_mut(4) {
        match alpha {
            AlphaFormat::Unpremul => {
                let a = p[3] as u32;
                for c in &mut p[..3] {
                    *c = if a == 0 {
                        0
                    } else {
                        ((*c as u32 * 255 + a / 2) / a).min(255) as u8
                    };
                }
            }
            AlphaFormat::Opaque => p[3] = 255,
            _ => {}
        }
        if format == PixelFormat::RGBA8888 {
            p.swap(0, 2)
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
        let image = self
            .image()
            .ok_or_else(|| io::Error::other("The bitmap has no pixels to encode"))?;
        let (jpeg, quality) = match options {
            BitmapEncoderOptions::Jpeg(o) => (1, o.quality as f64 / 100.),
            _ => (0, 1.),
        };
        // SAFETY: q_encode returns NULL or a +1 CFData.
        let data = unsafe { Owned::new(ffi::q_encode(image.0, jpeg, quality)) }
            .ok_or_else(|| io::Error::other("ImageIO encoder failed"))?;
        stream.write_all(&data.bytes())
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
        Some(self.format)
    }
    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(self.alpha)
    }
    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        assert!(
            !self.active.load(Ordering::Acquire),
            "Cannot lock an active Quartz drawing surface"
        );
        Rc::new(BitmapLock {
            bitmap: self.clone(),
            bytes: RefCell::new(export(&self.pixels(), self.format, self.alpha)),
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
        let copy = Bitmap::with_format(self.size, self.dpi, self.format, self.alpha);
        #[cfg(feature = "metal")]
        if self.is_gpu() && copy.is_gpu() {
            if let Some(source) = self.retained_surface() {
                copy.context(|p| unsafe { ffi::qm_copy_surface(p, source.0) });
            }
            return Arc::new(copy);
        }
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
        false
    }
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(self.drawing(false, None))
    }
}
/// A locked copy of the pixels in the bitmap's exposed layout; written back on dispose.
struct BitmapLock {
    bitmap: Bitmap,
    bytes: RefCell<Vec<u8>>,
    done: Cell<bool>,
}
impl ILockedFramebuffer for BitmapLock {
    fn address(&self) -> *mut u8 {
        self.bytes.borrow_mut().as_mut_ptr()
    }
    fn with_data(&self, f: &mut dyn FnMut(&mut [u8])) {
        if !self.done.get() {
            f(&mut self.bytes.borrow_mut())
        }
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
        self.bitmap.format
    }
    fn alpha_format(&self) -> AlphaFormat {
        self.bitmap.alpha
    }
    fn dispose(&self) {
        if !self.done.replace(true) {
            let size = self.bitmap.size;
            match convert(
                &self.bytes.borrow(),
                size.width as usize * 4,
                size,
                self.bitmap.format,
                self.bitmap.alpha,
            ) {
                Ok(pixels) => self.bitmap.write(&pixels),
                Err(_) => warn_once("locked pixels could not be written back"),
            }
        }
    }
}
impl Drop for BitmapLock {
    fn drop(&mut self) {
        self.dispose()
    }
}
