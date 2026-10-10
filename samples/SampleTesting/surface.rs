//! A window surface rendered to in memory, whose frames the tests read.

use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
    IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat, PixelFormats, RenderTargetSceneInfo, RetainedFramebuffer};
use ferroui_base::{PixelSize, Vector};
use std::any::Any;
use std::cell::RefCell;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// The pixels of a frame: RGBA, premultiplied, rows of `width * 4` bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl Frame {
    /// The pixel at `(x, y)` as `[r, g, b, a]`.
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let at = (y * self.width + x) * 4;
        [self.data[at], self.data[at + 1], self.data[at + 2], self.data[at + 3]]
    }

    /// The rectangle `(left, top, right, bottom)` clipped to the frame.
    fn clip(&self, rect: (f64, f64, f64, f64)) -> (usize, usize, usize, usize) {
        let clamp = |value: f64, max: usize| (value.max(0.0).round() as usize).min(max);
        (clamp(rect.0, self.width), clamp(rect.1, self.height), clamp(rect.2, self.width), clamp(rect.3, self.height))
    }

    /// The bounds `(left, top, right, bottom)`, exclusive at the right and the bottom, of the
    /// pixels inside `rect` (`(left, top, right, bottom)` in pixels) that are not `background`:
    /// what was drawn there. `None` when every pixel of the rectangle is the background.
    pub fn content_bounds(&self, rect: (f64, f64, f64, f64), background: [u8; 4]) -> Option<(usize, usize, usize, usize)> {
        let (left, top, right, bottom) = self.clip(rect);
        let mut bounds: Option<(usize, usize, usize, usize)> = None;
        for y in top..bottom {
            for x in left..right {
                if self.pixel(x, y) != background {
                    bounds = Some(match bounds {
                        None => (x, y, x + 1, y + 1),
                        Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x + 1), b.max(y + 1)),
                    });
                }
            }
        }
        bounds
    }

    /// The number of pixels inside `rect` that are `color`, each channel within `tolerance`.
    pub fn count(&self, rect: (f64, f64, f64, f64), color: [u8; 4], tolerance: u8) -> usize {
        let (left, top, right, bottom) = self.clip(rect);
        let mut count = 0;
        for y in top..bottom {
            for x in left..right {
                let pixel = self.pixel(x, y);
                if (0..4).all(|channel| pixel[channel].abs_diff(color[channel]) <= tolerance) {
                    count += 1;
                }
            }
        }
        count
    }

    /// The number of distinct colours inside `rect`.
    pub fn colors(&self, rect: (f64, f64, f64, f64)) -> usize {
        let (left, top, right, bottom) = self.clip(rect);
        let mut colors = std::collections::BTreeSet::new();
        for y in top..bottom {
            for x in left..right {
                colors.insert(self.pixel(x, y));
            }
        }
        colors.len()
    }

    /// The number of pixels inside `rect` that differ between this frame and `other`.
    pub fn difference(&self, other: &Frame, rect: (f64, f64, f64, f64)) -> usize {
        assert_eq!((self.width, self.height), (other.width, other.height), "the frames have the same size");
        let (left, top, right, bottom) = self.clip(rect);
        let mut count = 0;
        for y in top..bottom {
            for x in left..right {
                if self.pixel(x, y) != other.pixel(x, y) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Writes the frame as a binary PPM picture (the colours as they are stored, without the
    /// alpha channel), for looking at a frame of a failing test.
    pub fn save_ppm(&self, path: &Path) -> std::io::Result<()> {
        let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
        write!(file, "P6\n{} {}\n255\n", self.width, self.height)?;
        for pixel in self.data.chunks_exact(4) {
            file.write_all(&pixel[..3])?;
        }
        file.flush()
    }
}

/// A window surface rendered to in memory, as the software surface of the browser is. The
/// last frame the compositor drew is kept for the test.
pub struct FrameSurface {
    frames: Arc<AtomicU32>,
    last: Arc<Mutex<Option<Frame>>>,
}

impl FrameSurface {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { frames: Arc::new(AtomicU32::new(0)), last: Arc::new(Mutex::new(None)) })
    }

    /// The number of frames the surface received.
    pub fn frames(&self) -> u32 {
        self.frames.load(Ordering::SeqCst)
    }

    /// The last frame the surface received.
    pub fn last_frame(&self) -> Option<Frame> {
        self.last.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl IPlatformRenderSurface for FrameSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for FrameSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        Rc::new(FrameTarget { framebuffer: RefCell::new(None), frames: self.frames.clone(), last: self.last.clone() })
    }
}

/// The render target of a [`FrameSurface`]: an object of the server side, used under the
/// compositor lock. It keeps the pixels between the frames (the compositor draws the dirty
/// rectangles only) and copies them for the test when a frame is unlocked.
struct FrameTarget {
    framebuffer: RefCell<Option<(PixelSize, Rc<RetainedFramebuffer>)>>,
    frames: Arc<AtomicU32>,
    last: Arc<Mutex<Option<Frame>>>,
}

impl IPlatformRenderSurfaceRenderTarget for FrameTarget {}

impl IFramebufferRenderTarget for FrameTarget {
    fn lock(&self, scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        let size = PixelSize::new(scene_info.size.width.max(1), scene_info.size.height.max(1));
        let mut framebuffer = self.framebuffer.borrow_mut();
        if framebuffer.as_ref().is_some_and(|(current, _)| *current != size) {
            if let Some((_, old)) = framebuffer.take() {
                old.dispose();
            }
        }
        let (_, framebuffer) =
            framebuffer.get_or_insert_with(|| (size, RetainedFramebuffer::new(size, PixelFormats::RGBA8888, AlphaFormat::Premul)));
        let dpi = 96.0 * scene_info.scaling;
        let inner = framebuffer.lock(Vector::new(dpi, dpi), |_| {});
        let locked = Rc::new(CopiedFramebuffer { inner, frames: self.frames.clone(), last: self.last.clone() });
        (locked, FramebufferLockProperties::default())
    }

    fn dispose(&self) {
        if let Some((_, framebuffer)) = self.framebuffer.borrow_mut().take() {
            framebuffer.dispose();
        }
    }
}

/// A locked framebuffer that hands a copy of its pixels to the surface when it is unlocked.
struct CopiedFramebuffer {
    inner: Rc<dyn ILockedFramebuffer>,
    frames: Arc<AtomicU32>,
    last: Arc<Mutex<Option<Frame>>>,
}

impl ILockedFramebuffer for CopiedFramebuffer {
    fn address(&self) -> *mut u8 {
        self.inner.address()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        self.inner.with_data(access)
    }

    fn size(&self) -> PixelSize {
        self.inner.size()
    }

    fn row_bytes(&self) -> i32 {
        self.inner.row_bytes()
    }

    fn dpi(&self) -> Vector {
        self.inner.dpi()
    }

    fn format(&self) -> PixelFormat {
        self.inner.format()
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.inner.alpha_format()
    }

    fn dispose(&self) {
        let size = self.inner.size();
        let (width, height, row_bytes) = (size.width as usize, size.height as usize, self.inner.row_bytes() as usize);
        let mut data = Vec::with_capacity(width * height * 4);
        self.inner.with_data(&mut |pixels| {
            for row in pixels.chunks(row_bytes).take(height) {
                data.extend_from_slice(&row[..width * 4]);
            }
        });
        if data.len() == width * height * 4 {
            *self.last.lock().unwrap_or_else(|e| e.into_inner()) = Some(Frame { width, height, data });
        }
        self.frames.fetch_add(1, Ordering::SeqCst);
        self.inner.dispose();
    }
}
