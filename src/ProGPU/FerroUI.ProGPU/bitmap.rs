//! The bitmaps of the backend: pixels in memory (writeable bitmaps, bitmaps made from pixels,
//! snapshots of layers, render-target bitmaps). A scene draws one as an image of its stream.
//!
//! Image files are decoded by the image module of ProGPU, which reads PNG, JPEG, GIF (the first
//! frame) and BMP (a file of another format reports `Unsupported`), and bitmaps are saved by it
//! as PNG or JPEG.

use crate::drawing_context::ProGpuDrawingContext;
use crate::device::ProGpuDeviceError;
use crate::gpu::{default_gpu, GpuTexture};
use crate::notes;
use crate::scene::SceneBuilder;
use ferroui_base::media::imaging::{BitmapEncoderOptions, CompressionLevel};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, ILockedFramebuffer, IReadableBitmapImpl, IRenderTargetBitmapImpl,
    IWriteableBitmapImpl, PixelFormat,
};
use ferroui_base::{PixelSize, Rect, Vector};
use std::any::Any;
use std::io;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// The pixels of a bitmap, shared with its locks.
struct Pixels {
    data: Mutex<Vec<u8>>,
    version: AtomicI32,
    /// The reads from the GPU that have not delivered their frame yet (in the browser).
    pending_reads: AtomicU32,
    /// The last read from the GPU failed: the pixels are not what was rendered.
    read_failed: AtomicBool,
}

impl Pixels {
    fn new(data: Vec<u8>) -> Self {
        Self { data: Mutex::new(data), version: AtomicI32::new(0), pending_reads: AtomicU32::new(0), read_failed: AtomicBool::new(false) }
    }

    fn data(&self) -> MutexGuard<'_, Vec<u8>> {
        self.data.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether the pixels are what was last rendered into the bitmap.
    fn ready(&self) -> bool {
        self.pending_reads.load(Ordering::Acquire) == 0 && !self.read_failed.load(Ordering::Acquire)
    }

    /// Writes a frame read from the GPU, or records that the read failed.
    fn deliver(&self, frame: Result<Vec<u8>, ProGpuDeviceError>, layout: FrameLayout) {
        match frame {
            Ok(frame) => {
                let written = {
                    let mut data = self.data();
                    write_frame(&frame, layout.width, layout.height, &mut data, layout.row_bytes, layout.format, layout.alpha_format)
                };
                self.read_failed.store(!written, Ordering::Release);
                if written {
                    self.version.fetch_add(1, Ordering::Relaxed);
                }
            }
            Err(error) => {
                self.read_failed.store(true, Ordering::Release);
                notes::failure(&format!("the pixels of a bitmap were not read: {error}"));
            }
        }
    }

    /// Brings the frame in `texture` into the pixels: at once on the desktop, and in the browser
    /// through the mapping of a buffer, until which the bitmap says that it is not ready.
    #[cfg(not(target_os = "emscripten"))]
    fn store_frame(self: &Arc<Self>, texture: &Rc<GpuTexture>, layout: FrameLayout) {
        self.deliver(texture.read(), layout);
    }

    /// The texture stays the content of the bitmap until its pixels are back, so a frame drawn
    /// meanwhile, or a drawing into the bitmap, finds the content on the GPU: no frame goes
    /// without it, and none has to be drawn again when the pixels arrive.
    #[cfg(target_os = "emscripten")]
    fn store_frame(self: &Arc<Self>, texture: &Rc<GpuTexture>, layout: FrameLayout) {
        let key = Arc::as_ptr(self) as usize;
        KEPT_FRAMES.with(|frames| frames.borrow_mut().insert(key, texture.clone()));
        self.pending_reads.fetch_add(1, Ordering::AcqRel);
        let pixels = self.clone();
        texture.read_async(Box::new(move |frame| {
            pixels.deliver(frame, layout);
            pixels.pending_reads.fetch_sub(1, Ordering::AcqRel);
            // After a failed read the texture is still the only copy of the content.
            if pixels.ready() {
                KEPT_FRAMES.with(|frames| frames.borrow_mut().remove(&key));
            }
        }));
    }

    #[cfg(target_os = "emscripten")]
    fn kept_frame(self: &Arc<Self>) -> Option<Rc<GpuTexture>> {
        KEPT_FRAMES.with(|frames| frames.borrow().get(&(Arc::as_ptr(self) as usize)).cloned())
    }

    #[cfg(not(target_os = "emscripten"))]
    fn kept_frame(self: &Arc<Self>) -> Option<Rc<GpuTexture>> {
        None
    }
}

#[cfg(target_os = "emscripten")]
ferroui_base::ferro_thread_local! {
    /// The frames of the bitmaps whose pixels are on their way back from the GPU, by the address
    /// of the pixels. The canvas and these textures belong to the thread of the page.
    static KEPT_FRAMES: std::cell::RefCell<std::collections::HashMap<usize, Rc<GpuTexture>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// How a frame read from the GPU is written into the pixels of a bitmap.
#[derive(Clone, Copy)]
struct FrameLayout {
    width: usize,
    height: usize,
    row_bytes: usize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
}

/// A bitmap of pixels in memory: `RGBA8888` or `BGRA8888`, four bytes per pixel.
pub struct ProGpuBitmap {
    size: PixelSize,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    row_bytes: usize,
    pixels: Arc<Pixels>,
}

/// Whether the backend keeps pixels of `format` (the formats a scene samples).
pub(crate) fn is_supported(format: PixelFormat) -> bool {
    format == PixelFormat::RGBA8888 || format == PixelFormat::BGRA8888
}

impl ProGpuBitmap {
    /// A transparent bitmap. A format the backend does not keep becomes `RGBA8888`.
    pub fn new(size: PixelSize, dpi: Vector, format: PixelFormat, alpha_format: AlphaFormat) -> Self {
        let format = if is_supported(format) { format } else { PixelFormat::RGBA8888 };
        let (width, height) = (size.width.max(0) as usize, size.height.max(0) as usize);
        ProGpuBitmap {
            size,
            dpi,
            format,
            alpha_format,
            row_bytes: width * 4,
            pixels: Arc::new(Pixels::new(vec![0; width * height * 4])),
        }
    }

    /// A bitmap of a copy of `data` (rows of `stride` bytes), or `None` for a format the backend
    /// does not keep or for too few bytes.
    pub fn from_pixels(format: PixelFormat, alpha_format: AlphaFormat, data: &[u8], size: PixelSize, dpi: Vector, stride: i32) -> Option<Self> {
        if !is_supported(format) {
            return None;
        }
        let bitmap = Self::new(size, dpi, format, alpha_format);
        let (height, row_bytes, stride) = (size.height.max(0) as usize, bitmap.row_bytes, stride.max(0) as usize);
        if stride < row_bytes || (height > 0 && data.len() < stride * (height - 1) + row_bytes) {
            return None;
        }
        {
            let mut pixels = bitmap.pixels.data();
            for row in 0..height {
                pixels[row * row_bytes..(row + 1) * row_bytes].copy_from_slice(&data[row * stride..row * stride + row_bytes]);
            }
        }
        Some(bitmap)
    }

    /// The bytes of a row.
    pub fn row_bytes(&self) -> usize {
        self.row_bytes
    }

    /// The format of the pixels.
    pub fn pixel_format(&self) -> PixelFormat {
        self.format
    }

    /// The alpha format of the pixels.
    pub fn pixel_alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    /// The identity of the pixels of the bitmap: their address, and a handle that keeps the
    /// address from being reused while it is held.
    pub(crate) fn pixels_identity(&self) -> (usize, std::sync::Weak<dyn Any + Send + Sync>) {
        let weak: std::sync::Weak<dyn Any + Send + Sync> = {
            let strong: Arc<dyn Any + Send + Sync> = self.pixels.clone();
            Arc::downgrade(&strong)
        };
        (Arc::as_ptr(&self.pixels) as usize, weak)
    }

    /// Reads the pixels.
    pub fn with_pixels<R>(&self, read: impl FnOnce(&[u8]) -> R) -> R {
        read(&self.pixels.data())
    }

    /// Replaces the pixels (as many bytes as the bitmap has) and advances the version.
    #[cfg(test)]
    pub(crate) fn set_pixels(&self, data: &[u8]) {
        let mut pixels = self.pixels.data();
        if pixels.len() == data.len() {
            pixels.copy_from_slice(data);
            self.pixels.version.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// As a read from the GPU that failed.
    #[cfg(test)]
    pub(crate) fn fail_read(&self) {
        self.pixels.deliver(Err(ProGpuDeviceError::ReadBackFailed), self.frame_layout());
    }

    /// Whether the pixels are what was last rendered: not while a read from the GPU is on its
    /// way (in the browser), nor after one failed.
    pub(crate) fn pixels_ready(&self) -> bool {
        self.pixels.ready()
    }

    fn frame_layout(&self) -> FrameLayout {
        FrameLayout {
            width: self.size.width.max(0) as usize,
            height: self.size.height.max(0) as usize,
            row_bytes: self.row_bytes,
            format: self.format,
            alpha_format: self.alpha_format,
        }
    }

    /// Brings the premultiplied content of a texture of the size of the bitmap into its pixels.
    /// The bitmap takes the texture as its content until they arrive: the texture must not be
    /// drawn into afterwards.
    pub(crate) fn store_frame(&self, texture: &Rc<GpuTexture>) {
        self.pixels.store_frame(texture, self.frame_layout());
    }

    /// The texture that holds the content while the pixels are not back from the GPU.
    pub(crate) fn kept_frame(&self) -> Option<Rc<GpuTexture>> {
        self.pixels.kept_frame()
    }
}

impl IBitmapImpl for ProGpuBitmap {
    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn pixel_size(&self) -> PixelSize {
        self.size
    }

    fn version(&self) -> i32 {
        self.pixels.version.load(Ordering::Relaxed)
    }

    fn save(&self, stream: &mut dyn io::Write, options: &BitmapEncoderOptions) -> io::Result<()> {
        if !self.pixels.ready() {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "the pixels of the bitmap have not been read from the GPU"));
        }
        let source = SavedPixels {
            width: self.size.width.max(0) as u32,
            height: self.size.height.max(0) as u32,
            row_bytes: self.row_bytes,
            bgra: self.format == PixelFormat::BGRA8888,
            alpha: self.alpha_format,
        };
        let pixels = self.pixels.data();
        save_pixels(&pixels, &source, stream, options)
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// None while the pixels are not what was rendered: a reader gets no bitmap rather than an
    /// empty one.
    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        self.pixels.ready().then_some(self as &dyn IReadableBitmapImpl)
    }
}

/// How the pixels a bitmap or a layer saves are laid out.
pub(crate) struct SavedPixels {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) row_bytes: usize,
    pub(crate) bgra: bool,
    pub(crate) alpha: AlphaFormat,
}

/// The pixels in the order red, green, blue, alpha, in rows without padding. For a PNG alpha is
/// not premultiplied; for a JPEG, which has no alpha, the colours are those over black.
fn rgba_to_save(pixels: &[u8], source: &SavedPixels, for_jpeg: bool) -> io::Result<Vec<u8>> {
    let (width, height) = (source.width as usize, source.height as usize);
    let row = width * 4;
    let needed = if height == 0 { 0 } else { source.row_bytes * (height - 1) + row };
    if width == 0 || height == 0 || source.row_bytes < row || pixels.len() < needed {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "a bitmap without pixels cannot be saved"));
    }
    let mut out = Vec::with_capacity(row * height);
    for y in 0..height {
        let start = y * source.row_bytes;
        for pixel in pixels[start..start + row].chunks_exact(4) {
            let (r, g, b, a) = if source.bgra { (pixel[2], pixel[1], pixel[0], pixel[3]) } else { (pixel[0], pixel[1], pixel[2], pixel[3]) };
            let [r, g, b, a] = match (source.alpha, for_jpeg) {
                (AlphaFormat::Opaque, _) => [r, g, b, 255],
                (AlphaFormat::Premul, true) => [r, g, b, 255],
                (AlphaFormat::Unpremul, false) => [r, g, b, a],
                (AlphaFormat::Unpremul, true) => {
                    let over_black = |c: u8| ((u32::from(c) * u32::from(a) + 127) / 255) as u8;
                    [over_black(r), over_black(g), over_black(b), 255]
                }
                (AlphaFormat::Premul, false) if a == 0 => [0, 0, 0, 0],
                (AlphaFormat::Premul, false) => {
                    let straight = |c: u8| ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8;
                    [straight(r), straight(g), straight(b), a]
                }
            };
            out.extend_from_slice(&[r, g, b, a]);
        }
    }
    Ok(out)
}

/// Writes the pixels as the file `options` asks for, with the image module of ProGPU.
pub(crate) fn save_pixels(
    pixels: &[u8],
    source: &SavedPixels,
    stream: &mut dyn io::Write,
    options: &BitmapEncoderOptions,
) -> io::Result<()> {
    use progpu_native_sys::image::*;
    let for_jpeg = matches!(options, BitmapEncoderOptions::Jpeg(_));
    let rgba = rgba_to_save(pixels, source, for_jpeg)?;
    let (mut output, mut size, mut error) = (std::ptr::null_mut(), 0usize, 0u32);
    // SAFETY: `rgba` is a live slice of the size given, and the out pointers are live locals.
    let written = unsafe {
        match options {
            BitmapEncoderOptions::Png(png) => {
                let level = match png.compression_level {
                    CompressionLevel::NoCompression => PROGPU_NATIVE_IMAGE_PNG_NO_COMPRESSION,
                    CompressionLevel::Fastest => PROGPU_NATIVE_IMAGE_PNG_FASTEST,
                    CompressionLevel::Optimal => PROGPU_NATIVE_IMAGE_PNG_OPTIMAL,
                    CompressionLevel::SmallestSize => PROGPU_NATIVE_IMAGE_PNG_SMALLEST,
                };
                progpu_native_image_encode_png(rgba.as_ptr(), rgba.len(), source.width, source.height, level, &mut output, &mut size, &mut error)
            }
            BitmapEncoderOptions::Jpeg(jpeg) => {
                let quality = jpeg.quality.clamp(1, 100) as u32;
                progpu_native_image_encode_jpeg(rgba.as_ptr(), rgba.len(), source.width, source.height, quality, &mut output, &mut size, &mut error)
            }
        }
    };
    if written == 0 {
        return Err(io::Error::other(format!("the image could not be encoded (image error {error})")));
    }
    // SAFETY: on success the encoder handed out `size` bytes at `output`; they are released once,
    // after the write, whatever its result.
    let result = stream.write_all(unsafe { std::slice::from_raw_parts(output, size) });
    unsafe { progpu_native_image_free(output) };
    result
}

/// The error of what the image module of ProGPU does not do, noted once.
pub(crate) fn unsupported_codec(what: &str) -> io::Error {
    let message = format!("{what} is not in the image module of ProGPU");
    notes::note_once("image-codec", &message);
    io::Error::new(io::ErrorKind::Unsupported, message)
}

/// The most bytes the pixels of a decoded image may take: a file that asks for more is refused
/// before anything is allocated.
const DECODED_BYTES_LIMIT: usize = 1 << 30;

impl ProGpuBitmap {
    /// The bitmap of an image file: `RGBA8888` pixels that are not premultiplied, at 96 dpi.
    /// The image module of ProGPU decodes it: PNG, JPEG, GIF (the first frame) or BMP. Another
    /// format, or a variant the module does not read, is `Unsupported`; a damaged file is
    /// `InvalidData`.
    pub fn decode(file: &[u8]) -> io::Result<Self> {
        use progpu_native_sys::image::*;
        let failed = |error: u32| match error {
            PROGPU_NATIVE_IMAGE_ERROR_INVALID_SIGNATURE | PROGPU_NATIVE_IMAGE_ERROR_UNSUPPORTED_FORMAT => {
                unsupported_codec("decoding an image of a format the image module does not read")
            }
            _ => io::Error::new(io::ErrorKind::InvalidData, format!("the image could not be decoded (image error {error})")),
        };
        let (mut format, mut width, mut height, mut size, mut error) = (0u32, 0u32, 0u32, 0usize, 0u32);
        // SAFETY: `file` is a live slice and every out pointer is a live local.
        let read = unsafe {
            progpu_native_image_read_size(file.as_ptr(), file.len(), &mut format, &mut width, &mut height, &mut size, &mut error)
        };
        if read == 0 {
            return Err(failed(error));
        }
        if width > i32::MAX as u32 || height > i32::MAX as u32 || size > DECODED_BYTES_LIMIT || size != width as usize * height as usize * 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("an image of {width} x {height} pixels is too large")));
        }
        let bitmap = Self::new(PixelSize::new(width as i32, height as i32), Vector::new(96.0, 96.0), PixelFormat::RGBA8888, AlphaFormat::Unpremul);
        {
            let mut pixels = bitmap.pixels.data();
            // SAFETY: `file` is a live slice, and the pixels of the bitmap are `size` bytes, which
            // the decoder is told.
            if unsafe { progpu_native_image_decode_rgba(file.as_ptr(), file.len(), pixels.as_mut_ptr(), pixels.len(), &mut error) } == 0 {
                return Err(failed(error));
            }
        }
        Ok(bitmap)
    }
}

/// A lock of the pixels of a bitmap. Disposing it advances the version of the bitmap.
struct BitmapLock {
    pixels: Arc<Pixels>,
    size: PixelSize,
    row_bytes: usize,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
}

impl ILockedFramebuffer for BitmapLock {
    fn address(&self) -> *mut u8 {
        // The buffer of the bitmap is never reallocated: the address stays valid while the lock
        // (which shares the pixels) is alive.
        self.pixels.data().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.pixels.data());
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

    fn dispose(&self) {
        self.pixels.version.fetch_add(1, Ordering::Relaxed);
    }
}

impl IReadableBitmapImpl for ProGpuBitmap {
    fn format(&self) -> Option<PixelFormat> {
        Some(self.format)
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(self.alpha_format)
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        Rc::new(BitmapLock {
            pixels: self.pixels.clone(),
            size: self.size,
            row_bytes: self.row_bytes,
            dpi: self.dpi,
            format: self.format,
            alpha_format: self.alpha_format,
        })
    }
}

impl IWriteableBitmapImpl for ProGpuBitmap {}

/// Premultiplied `RGBA` pixels of a frame written into pixels of `format` and `alpha_format`
/// (rows of `row_bytes`). False for a format the backend does not write.
pub(crate) fn write_frame(
    frame: &[u8],
    width: usize,
    height: usize,
    target: &mut [u8],
    row_bytes: usize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
) -> bool {
    if !is_supported(format) || row_bytes < width * 4 || frame.len() < width * height * 4 {
        return false;
    }
    if height > 0 && target.len() < row_bytes * (height - 1) + width * 4 {
        return false;
    }
    let bgra = format == PixelFormat::BGRA8888;
    for row in 0..height {
        let source = &frame[row * width * 4..(row + 1) * width * 4];
        let destination = &mut target[row * row_bytes..row * row_bytes + width * 4];
        for (pixel, out) in source.chunks_exact(4).zip(destination.chunks_exact_mut(4)) {
            let (mut r, mut g, mut b, mut a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
            match alpha_format {
                AlphaFormat::Premul => {}
                AlphaFormat::Unpremul => {
                    if a != 0 && a != 255 {
                        let straight = |value: u8| ((value as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
                        (r, g, b) = (straight(r), straight(g), straight(b));
                    }
                }
                AlphaFormat::Opaque => a = 255,
            }
            if bgra {
                out.copy_from_slice(&[b, g, r, a]);
            } else {
                out.copy_from_slice(&[r, g, b, a]);
            }
        }
    }
    true
}

impl IRenderTargetBitmapImpl for ProGpuBitmap {
    /// A context drawing over the content of the bitmap: the frame is rendered on the device of
    /// the calling thread and read back into the pixels.
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        let gpu = default_gpu();
        let (width, height) = (self.size.width.max(1) as u32, self.size.height.max(1) as u32);
        let (pixels, layout) = (self.pixels.clone(), self.frame_layout());
        let target = gpu.clone();
        let mut context = ProGpuDrawingContext::with_gpu(
            SceneBuilder::new(),
            gpu,
            Some(Box::new(move |mut recording| {
                let Some(gpu) = target else { return };
                let Some(engine) = gpu.offscreen() else { return };
                let texture = GpuTexture::new(gpu.device(), width, height);
                match recording.render(engine, texture.view(), width, height, false) {
                    Ok(()) => pixels.store_frame(&texture, layout),
                    Err(error) => notes::failure(&format!("a render-target bitmap was not drawn: {error}")),
                }
            })),
        );
        // What was drawn before stays under what is drawn now, also while its pixels are on
        // their way back from the GPU.
        if self.version() != 0 || self.kept_frame().is_some() {
            let all = Rect::new(0.0, 0.0, self.size.width as f64, self.size.height as f64);
            context.draw_bitmap(self, 1.0, all, all);
        }
        Box::new(context)
    }
}
