//! The images of custom effects (#455, E6; not from upstream;
//! docs/porting/custom-effects.md, section 3.6): a handle of an image that a
//! runtime shader effect samples besides its content (the pages of a
//! transition, a mask), held by a parameter of the effect
//! ([`EffectParameterValue::Image`](super::EffectParameterValue::Image)).
//!
//! Effector registers such an image in a registry and references it by a
//! value handle, the render thread taking a lease while it draws
//! (`SkiaShaderImageHandle.cs`, `SkiaShaderImageRegistry.cs`,
//! `SkiaShaderImageLease.cs`). Here the handle is the lease: an `Arc` of a
//! counted reference to the framework's bitmap, so the image lives while a
//! snapshot, a frame or a backend's cache entry holds a handle and is
//! released when the last one drops it, on whatever thread.
//!
//! The backends key what they make of an image (a Skia image, a texture of
//! a device) by its [identity](EffectImage::id), never reused, and keep a
//! [weak handle](EffectImageWeak) with it: an entry whose image is gone is
//! released by the thread that owns it the next time it looks at its cache
//! after [`EffectImage::release_generation`] changed, so a texture lives on
//! the render thread while the handles are dropped on the UI thread.

use crate::media::imaging::pixel_format_transcoder::PixelFormatTranscoder;
use crate::media::imaging::IBitmap;
use crate::platform::{AlphaFormat, IBitmapImpl, PixelFormat, SharedBitmapImpl};
use crate::utilities::{RefCountable, RefCounted};
use crate::PixelSize;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

/// The identity of the next image handle; 0 is the empty handle's.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// How many images were released: the backends look at their caches again
/// when it changes.
static RELEASED: AtomicU64 = AtomicU64::new(0);

/// What a handle shares.
struct EffectImageInner {
    id: u64,
    /// The counted reference to the bitmap: the bitmap is not released
    /// while the handle lives.
    bitmap: RefCounted<SharedBitmapImpl>,
    pixel_size: PixelSize,
    /// The premultiplied RGBA pixels read for the CPU references and the
    /// uploads, with the version of the bitmap they were read at.
    pixels: Mutex<Option<(i32, Option<EffectImagePixels>)>>,
}

impl Drop for EffectImageInner {
    fn drop(&mut self) {
        // The strong count is zero already: a backend that sees the new
        // generation finds the weak handle of this image dead.
        RELEASED.fetch_add(1, Ordering::Release);
    }
}

/// A handle of an image that a shader effect samples: the value of an image
/// parameter of a custom effect
/// ([`EffectParameterValue::Image`](super::EffectParameterValue::Image)).
///
/// It is cheap to clone (an `Arc`), `Send` and `Sync`, and compares by
/// identity: two handles are equal when they are clones of one handle (or
/// both empty), whatever the pixels. The default is the empty handle, an
/// image of no pixels, which a program samples as transparent.
///
/// The handle holds a counted reference to the framework's bitmap (the
/// platform bitmap of a [`Bitmap`](crate::media::imaging::Bitmap), a
/// `RenderTargetBitmap` capture of a page), so the bitmap lives while a
/// handle does. The bitmap is treated as immutable: a backend uploads it once
/// and draws the upload until the handle is released (a bitmap whose
/// [version](crate::platform::IBitmapImpl::version) changes is uploaded
/// again).
///
/// ```ignore
/// let page = Bitmap::from_stream(&mut stream)?;
/// let effect = MaskTransitionEffect { to_page: EffectImage::new(&page), progress: 0.5, ..Default::default() };
/// visual.set_effect(Some(ImmutableCustomEffect::new(effect).into()));
/// ```
#[derive(Clone, Default)]
pub struct EffectImage {
    inner: Option<Arc<EffectImageInner>>,
}

const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    let _ = assert_send_sync::<EffectImage>;
    let _ = assert_send_sync::<EffectImageWeak>;
};

impl EffectImage {
    /// The empty handle: no image; sampled as transparent.
    pub const fn empty() -> Self {
        Self { inner: None }
    }

    /// A handle of `bitmap`: another counted reference to its platform
    /// bitmap, so the bitmap lives while the handle does.
    ///
    /// # Panics
    /// Panics when the bitmap has been disposed.
    pub fn new(bitmap: &dyn IBitmap) -> Self {
        Self::from_ref(bitmap.platform_impl())
    }

    /// A handle of the platform bitmap `bitmap` refers to (another counted
    /// reference to it).
    ///
    /// # Panics
    /// Panics when the reference has been disposed.
    pub fn from_ref(bitmap: &RefCounted<SharedBitmapImpl>) -> Self {
        Self::wrap(bitmap.clone_ref())
    }

    /// A handle owning the platform bitmap `bitmap`: the bitmap is disposed
    /// when the last handle is released.
    pub fn from_impl(bitmap: Arc<SharedBitmapImpl>) -> Self {
        let item = bitmap.clone();
        Self::wrap(RefCountable::create(bitmap, move || item.dispose()))
    }

    fn wrap(bitmap: RefCounted<SharedBitmapImpl>) -> Self {
        let pixel_size = bitmap.item().pixel_size();
        Self {
            inner: Some(Arc::new(EffectImageInner {
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                bitmap,
                pixel_size,
                pixels: Mutex::new(None),
            })),
        }
    }

    /// Whether the handle is the empty one.
    pub fn is_empty(&self) -> bool {
        self.inner.is_none()
    }

    /// The identity of the handle: the same for its clones, never given to
    /// another image; 0 for the empty handle. What the backends key their
    /// uploads by.
    pub fn id(&self) -> u64 {
        self.inner.as_ref().map_or(0, |inner| inner.id)
    }

    /// The size of the image in device pixels (zero for the empty handle).
    pub fn pixel_size(&self) -> PixelSize {
        self.inner.as_ref().map_or_else(PixelSize::default, |inner| inner.pixel_size)
    }

    /// The platform bitmap; `None` for the empty handle.
    pub fn bitmap(&self) -> Option<Arc<SharedBitmapImpl>> {
        self.inner.as_ref().map(|inner| inner.bitmap.item())
    }

    /// The version of the pixels of the bitmap
    /// ([`IBitmapImpl::version`]); 0 for the empty handle.
    pub fn version(&self) -> i32 {
        self.inner.as_ref().map_or(0, |inner| inner.bitmap.item().version())
    }

    /// A weak handle, which does not keep the image: what a backend keeps
    /// with what it made of the image, to know when to release it.
    pub fn downgrade(&self) -> EffectImageWeak {
        match &self.inner {
            Some(inner) => EffectImageWeak { inner: Arc::downgrade(inner), id: inner.id },
            None => EffectImageWeak::default(),
        }
    }

    /// The pixels of the image as premultiplied RGBA, read once per version
    /// of the bitmap: what the CPU references sample and a backend that
    /// uploads pixels uploads. `None` for the empty handle and for a bitmap
    /// whose pixels cannot be read on the processor (a texture of a device).
    pub fn pixels(&self) -> Option<EffectImagePixels> {
        let inner = self.inner.as_ref()?;
        let bitmap = inner.bitmap.item();
        let version = IBitmapImpl::version(&*bitmap);
        let mut cached = inner.pixels.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((cached_version, pixels)) = &*cached {
            if *cached_version == version {
                return pixels.clone();
            }
        }
        let pixels = read_premultiplied_rgba(&*bitmap);
        *cached = Some((version, pixels.clone()));
        pixels
    }

    /// How many image handles were released so far (the last clone of each
    /// dropped). A backend that keeps uploads by identity looks for the dead
    /// [weak handles](EffectImageWeak) of its entries when this changes, on
    /// the thread that owns the uploads.
    pub fn release_generation() -> u64 {
        RELEASED.load(Ordering::Acquire)
    }
}

/// Handles compare by identity.
impl PartialEq for EffectImage {
    fn eq(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for EffectImage {}

impl Hash for EffectImage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl fmt::Debug for EffectImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            None => f.write_str("EffectImage(empty)"),
            Some(inner) => f
                .debug_struct("EffectImage")
                .field("id", &inner.id)
                .field("width", &inner.pixel_size.width)
                .field("height", &inner.pixel_size.height)
                .finish(),
        }
    }
}

/// A weak [`EffectImage`]: it does not keep the image, and tells whether
/// the image still lives.
#[derive(Clone, Debug, Default)]
pub struct EffectImageWeak {
    inner: Weak<EffectImageInner>,
    id: u64,
}

impl EffectImageWeak {
    /// Whether a handle of the image still lives. The weak handle of the
    /// empty handle is never alive.
    pub fn is_alive(&self) -> bool {
        self.inner.strong_count() > 0
    }

    /// A handle of the image, while one lives.
    pub fn upgrade(&self) -> Option<EffectImage> {
        self.inner.upgrade().map(|inner| EffectImage { inner: Some(inner) })
    }

    /// The identity of the image ([`EffectImage::id`]).
    pub fn id(&self) -> u64 {
        self.id
    }
}

/// The pixels of an [`EffectImage`]: premultiplied RGBA, eight bits a
/// channel, rows of `width * 4` bytes without padding.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectImagePixels {
    width: usize,
    height: usize,
    rgba: Arc<[u8]>,
}

impl EffectImagePixels {
    /// Pixels of `width` by `height` from `rgba` (premultiplied, `width * 4`
    /// bytes a row); `None` when the length does not match.
    pub fn new(width: usize, height: usize, rgba: impl Into<Arc<[u8]>>) -> Option<Self> {
        let rgba = rgba.into();
        (rgba.len() == width * height * 4).then_some(Self { width, height, rgba })
    }

    /// The width in pixels.
    pub fn width(&self) -> usize {
        self.width
    }

    /// The height in pixels.
    pub fn height(&self) -> usize {
        self.height
    }

    /// The premultiplied RGBA bytes.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// The texel at column `x` and row `y`; transparent beyond the image.
    pub fn texel(&self, x: i64, y: i64) -> [u8; 4] {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return [0; 4];
        }
        let at = (y as usize * self.width + x as usize) * 4;
        [self.rgba[at], self.rgba[at + 1], self.rgba[at + 2], self.rgba[at + 3]]
    }

    /// The image at `coord` (device pixels of the area of an effect whose
    /// size is `layer_size`) as the programs of the shader effects sample a
    /// child (the convention of
    /// [`ShaderEffectDescription`](super::ShaderEffectDescription)): the image
    /// stretched over the area, the texel under `coord * size / layer_size`
    /// (single precision, in that order), transparent beyond the image.
    pub fn sample_stretched(&self, coord: [f32; 2], layer_size: [f32; 2]) -> [u8; 4] {
        let layer = [layer_size[0].max(1.0), layer_size[1].max(1.0)];
        let x = (coord[0] * self.width as f32 / layer[0]).floor();
        let y = (coord[1] * self.height as f32 / layer[1]).floor();
        if !(x.is_finite() && y.is_finite()) {
            return [0; 4];
        }
        self.texel(x as i64, y as i64)
    }
}

/// Samples the image `image` for a CPU reference ([`EffectImagePixels::sample_stretched`]):
/// transparent for an empty handle or pixels that cannot be read.
pub(crate) fn sample_image(pixels: Option<&EffectImagePixels>, coord: [f32; 2], layer_size: [f32; 2]) -> [u8; 4] {
    pixels.map_or([0; 4], |pixels| pixels.sample_stretched(coord, layer_size))
}

/// The pixels of `bitmap` as premultiplied RGBA; `None` when they cannot be
/// read on the processor.
fn read_premultiplied_rgba(bitmap: &SharedBitmapImpl) -> Option<EffectImagePixels> {
    let readable = bitmap.as_readable_bitmap()?;
    readable.format()?;
    readable.alpha_format()?;
    let framebuffer = readable.lock();
    let size = framebuffer.size();
    let (format, alpha_format, row_bytes) = (framebuffer.format(), framebuffer.alpha_format(), framebuffer.row_bytes());
    let (width, height) = (size.width.max(0) as usize, size.height.max(0) as usize);
    let mut rgba = vec![0u8; width * height * 4];
    if width > 0 && height > 0 {
        let row = width * 4;
        framebuffer.with_data(&mut |source| {
            if format == PixelFormat::RGBA8888 && alpha_format == AlphaFormat::Premul && row_bytes >= 0 {
                // The pixels as they are, without the padding of the rows.
                for (y, target) in rgba.chunks_exact_mut(row).enumerate() {
                    let at = y * row_bytes as usize;
                    if let Some(line) = source.get(at..at + row) {
                        target.copy_from_slice(line);
                    }
                }
            } else {
                PixelFormatTranscoder::transcode(
                    source,
                    size,
                    row_bytes,
                    format,
                    alpha_format,
                    &mut rgba,
                    row as i32,
                    PixelFormat::RGBA8888,
                    AlphaFormat::Premul,
                );
            }
        });
    }
    framebuffer.dispose();
    EffectImagePixels::new(width, height, rgba)
}
