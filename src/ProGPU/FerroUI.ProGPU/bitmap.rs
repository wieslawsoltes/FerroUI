//! The bitmaps of the backend: pixels in memory (writeable bitmaps, bitmaps made from pixels,
//! snapshots of layers, render-target bitmaps). A scene draws one as an image of its stream.
//!
//! Decoding and encoding image files is the image module of ProGPU (step P5 of
//! docs/porting/progpu-backend.md): until then loading and saving report `Unsupported`.

use crate::drawing_context::ProGpuDrawingContext;
use crate::gpu::{default_gpu, GpuTexture};
use crate::notes;
use crate::scene::SceneBuilder;
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, ILockedFramebuffer, IReadableBitmapImpl, IRenderTargetBitmapImpl,
    IWriteableBitmapImpl, PixelFormat,
};
use ferroui_base::{PixelSize, Rect, Vector};
use std::any::Any;
use std::io;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// The pixels of a bitmap, shared with its locks.
struct Pixels {
    data: Mutex<Vec<u8>>,
    version: AtomicI32,
}

impl Pixels {
    fn data(&self) -> MutexGuard<'_, Vec<u8>> {
        self.data.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
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
            pixels: Arc::new(Pixels { data: Mutex::new(vec![0; width * height * 4]), version: AtomicI32::new(0) }),
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

    /// Reads the pixels.
    pub fn with_pixels<R>(&self, read: impl FnOnce(&[u8]) -> R) -> R {
        read(&self.pixels.data())
    }

    /// Replaces the pixels (as many bytes as the bitmap has) and advances the version.
    pub(crate) fn set_pixels(&self, data: &[u8]) {
        let mut pixels = self.pixels.data();
        if pixels.len() == data.len() {
            pixels.copy_from_slice(data);
            self.pixels.version.fetch_add(1, Ordering::Relaxed);
        }
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

    fn save(&self, _stream: &mut dyn io::Write, _options: &BitmapEncoderOptions) -> io::Result<()> {
        Err(unsupported_codec("encoding a bitmap"))
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

/// The error of the image codecs the backend does not have yet, noted once.
pub(crate) fn unsupported_codec(what: &str) -> io::Error {
    let message = format!("{what} needs the image module of ProGPU, which the ProGPU backend does not use yet");
    notes::note_once("image-codec", &message);
    io::Error::new(io::ErrorKind::Unsupported, message)
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
        let (pixels, size, row_bytes, format, alpha_format) = (self.pixels.clone(), self.size, self.row_bytes, self.format, self.alpha_format);
        let target = gpu.clone();
        let mut context = ProGpuDrawingContext::with_gpu(
            SceneBuilder::new(),
            gpu,
            Some(Box::new(move |mut recording| {
                let Some(gpu) = target else { return };
                let Some(engine) = gpu.offscreen() else { return };
                let texture = GpuTexture::new(gpu.device(), width, height);
                match recording.render(engine, texture.view(), width, height, false) {
                    Ok(()) => {
                        let frame = texture.read();
                        let mut data = pixels.data();
                        if write_frame(&frame, size.width.max(0) as usize, size.height.max(0) as usize, &mut data, row_bytes, format, alpha_format) {
                            pixels.version.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(error) => notes::failure(&format!("a render-target bitmap was not drawn: {error}")),
                }
            })),
        );
        // What was drawn before stays under what is drawn now.
        if self.version() != 0 {
            let all = Rect::new(0.0, 0.0, self.size.width as f64, self.size.height as f64);
            context.draw_bitmap(self, 1.0, all, all);
        }
        Box::new(context)
    }
}
