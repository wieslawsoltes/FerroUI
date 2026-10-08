//! The render surface of a headless window: the part of
//! `HeadlessWindowImpl.cs` that a frame uses (`Lock`,
//! `CreateFramebufferRenderTarget`, `_lastRenderedFrame` and its lock).
//!
//! Upstream implements the framebuffer surface on the window implementation
//! itself. Here the window implementation is an object of the UI thread,
//! while a frame may be rendered by the render thread, so what a frame needs
//! lives in this object, which both threads share.

use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use ferroui_base::{PixelSize, Size, Vector};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

/// What a framebuffer is created with: the window implementation writes it
/// on the UI thread, the thread that renders reads it.
#[derive(Clone, Copy)]
struct FramebufferMetrics {
    client_size: Size,
    render_scaling: f64,
}

/// The pixels of a rendered frame.
struct RenderedFrame {
    data: Vec<u8>,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
    format: PixelFormat,
}

/// The framebuffer surface of a headless window.
pub(crate) struct HeadlessWindowSurface {
    this: Weak<HeadlessWindowSurface>,
    frame_buffer_format: PixelFormat,
    metrics: Mutex<FramebufferMetrics>,
    /// `_lastRenderedFrame`, guarded by `_sync` upstream.
    last_rendered_frame: Mutex<Option<RenderedFrame>>,
}

/// Locks a mutex of the surface. A panic of the other thread leaves plain
/// values behind, which stay usable.
fn guard<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl HeadlessWindowSurface {
    pub(crate) fn new(frame_buffer_format: PixelFormat, client_size: Size, render_scaling: f64) -> Arc<Self> {
        Arc::new_cyclic(|this| HeadlessWindowSurface {
            this: this.clone(),
            frame_buffer_format,
            metrics: Mutex::new(FramebufferMetrics { client_size, render_scaling }),
            last_rendered_frame: Mutex::new(None),
        })
    }

    pub(crate) fn client_size(&self) -> Size {
        guard(&self.metrics).client_size
    }

    pub(crate) fn set_client_size(&self, client_size: Size) {
        guard(&self.metrics).client_size = client_size;
    }

    pub(crate) fn render_scaling(&self) -> f64 {
        guard(&self.metrics).render_scaling
    }

    pub(crate) fn set_render_scaling(&self, render_scaling: f64) {
        guard(&self.metrics).render_scaling = render_scaling;
    }

    /// `Lock`: a new framebuffer of the size of the window, which becomes
    /// the last rendered frame when it is disposed.
    ///
    /// # Panics
    /// Panics when the window has no pixels (the bitmap upstream creates
    /// here rejects such a size).
    fn lock(self: &Arc<Self>) -> Rc<dyn ILockedFramebuffer> {
        let metrics = *guard(&self.metrics);
        let size = PixelSize::from_size(metrics.client_size, metrics.render_scaling);
        if size.width <= 0 || size.height <= 0 {
            panic!("Size should be >= (1,1)");
        }

        let format = self.frame_buffer_format;
        let bytes_per_pixel = (format.bits_per_pixel() as i32 + 7) / 8;
        let row_bytes = size
            .width
            .checked_mul(bytes_per_pixel)
            .and_then(|bytes| bytes.checked_add(3))
            .map(|bytes| 4 * (bytes / 4))
            .unwrap_or_else(|| panic!("framebuffer too wide: {} pixels of {format}", size.width));
        let memory_size = row_bytes
            .checked_mul(size.height)
            .unwrap_or_else(|| panic!("framebuffer too large: {}x{} pixels of {format}", size.width, size.height));

        Rc::new(HeadlessWindowFramebuffer {
            surface: self.clone(),
            memory: RefCell::new(Some(vec![0u8; memory_size as usize])),
            size,
            row_bytes,
            dpi: Vector::new(96.0, 96.0) * metrics.render_scaling,
            format,
        })
    }

    /// `GetLastRenderedFrame`: a copy of the last rendered frame, if a frame
    /// was rendered. Called on the UI thread; the bitmap is created with the
    /// render interface of that thread.
    pub(crate) fn get_last_rendered_frame(&self) -> Option<WriteableBitmap> {
        let last_rendered_frame = guard(&self.last_rendered_frame);
        let frame = last_rendered_frame.as_ref()?;

        Some(WriteableBitmap::from_pixels(
            frame.format,
            AlphaFormat::Opaque,
            &frame.data,
            frame.size,
            frame.dpi,
            frame.row_bytes,
        ))
    }

    /// The part of `Dispose` that releases the last rendered frame.
    pub(crate) fn dispose_last_rendered_frame(&self) {
        let last_rendered_frame = guard(&self.last_rendered_frame).take();
        drop(last_rendered_frame);
    }
}

impl IPlatformRenderSurface for HeadlessWindowSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for HeadlessWindowSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let surface = self.this.upgrade().expect("the surface is alive while it is used");
        Rc::new(FuncFramebufferRenderTarget::new(move || surface.lock()))
    }
}

/// The framebuffer of one frame (`FramebufferProxy` and the bitmap it locks
/// upstream). It belongs to the thread that renders the frame.
struct HeadlessWindowFramebuffer {
    surface: Arc<HeadlessWindowSurface>,
    /// The pixels, until the framebuffer is disposed.
    memory: RefCell<Option<Vec<u8>>>,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
    format: PixelFormat,
}

impl ILockedFramebuffer for HeadlessWindowFramebuffer {
    fn address(&self) -> *mut u8 {
        match self.memory.borrow_mut().as_mut() {
            Some(memory) => memory.as_mut_ptr(),
            None => std::ptr::null_mut(),
        }
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        match self.memory.borrow_mut().as_mut() {
            Some(memory) => access(memory),
            None => access(&mut []),
        }
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.row_bytes
    }

    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn format(&self) -> PixelFormat {
        self.format
    }

    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn dispose(&self) {
        let Some(data) = self.memory.borrow_mut().take() else {
            return;
        };

        let frame =
            RenderedFrame { data, size: self.size, row_bytes: self.row_bytes, dpi: self.dpi, format: self.format };
        let old = guard(&self.surface.last_rendered_frame).replace(frame);
        drop(old);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use ferroui_base::platform::{PixelFormats, RenderTargetSceneInfo};
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn the_surface_is_shared_between_the_threads() {
        assert_send_sync::<HeadlessWindowSurface>();
    }

    #[test]
    fn a_frame_rendered_on_another_thread_becomes_the_last_rendered_frame() {
        let surface = HeadlessWindowSurface::new(PixelFormats::RGBA8888, Size::new(4.0, 2.0), 2.0);
        assert!(guard(&surface.last_rendered_frame).is_none());

        let shared: Arc<dyn IPlatformRenderSurface> = surface.clone();
        std::thread::spawn(move || {
            let target = shared.as_framebuffer_surface().unwrap().create_framebuffer_render_target();
            let scene_info =
                RenderTargetSceneInfo::new(PixelSize::new(8, 4), 2.0, CompositionTransparencyLevel::default());
            let (framebuffer, _) = target.lock(&scene_info);
            assert_eq!(PixelSize::new(8, 4), framebuffer.size());
            assert_eq!(32, framebuffer.row_bytes());
            assert_eq!(Vector::new(192.0, 192.0), framebuffer.dpi());
            framebuffer.with_data(&mut |data| data.fill(7));
            framebuffer.dispose();
            // A second dispose does not replace the frame.
            framebuffer.dispose();
            target.dispose();
        })
        .join()
        .unwrap();

        {
            let frame = guard(&surface.last_rendered_frame);
            let frame = frame.as_ref().unwrap();
            assert_eq!(PixelSize::new(8, 4), frame.size);
            assert_eq!(vec![7u8; 32 * 4], frame.data);
        }

        surface.dispose_last_rendered_frame();
        assert!(guard(&surface.last_rendered_frame).is_none());
    }
}
