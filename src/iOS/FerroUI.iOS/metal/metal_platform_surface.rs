//! The Metal render surface of a view: its Metal layer.

use super::{FrameCapture, MetalDevice, MetalRenderTarget};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::PixelSize;
use ferroui_metal::{IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget};
use objc2::rc::Retained;
use objc2_quartz_core::CAMetalLayer;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

/// The Metal layer of a view, for the thread that renders.
pub(crate) struct SharedLayer(pub(crate) Retained<CAMetalLayer>);

// SAFETY: the layer is created by UIKit for a view of the main thread and
// handed to the thread that renders, as the reference does. Through this
// type that thread calls `setDevice:`, `setDrawableSize:`,
// `setFramebufferOnly:` and `nextDrawable` and nothing else. A Metal layer
// is made to be drawn to from a thread that is not the main thread
// (`nextDrawable` blocks, and is documented for a render thread), and the
// properties of a Core Animation layer may be set from any thread: a change
// becomes part of the implicit transaction of the calling thread. The main
// thread does not call any of the four after the view is created. Retaining
// and releasing an Objective-C object is thread-safe.
unsafe impl Send for SharedLayer {}
// SAFETY: see `Send`.
unsafe impl Sync for SharedLayer {}

/// What a view shares with its surface and the render target of the
/// surface: the reference keeps the first as a property the view sets on
/// the render target; the port keeps it where both threads reach it under
/// a lock (DEVIATIONS.md, iOS backend). The other two are for diagnostics.
pub(crate) struct SurfaceShared {
    /// The size in pixels and the scaling of the last layout of the view.
    pending_layout: Mutex<(PixelSize, f64)>,
    /// The number of frames that were presented.
    frames_presented: AtomicU64,
    /// The number of frames that were begun.
    frames_begun: AtomicU64,
    /// Who waits for the pixels of the next frame.
    capture: Mutex<Option<Box<dyn FnOnce(FrameCapture) + Send>>>,
}

impl SurfaceShared {
    pub(crate) fn new() -> Arc<SurfaceShared> {
        Arc::new(SurfaceShared {
            pending_layout: Mutex::new((PixelSize::new(1, 1), 1.0)),
            frames_presented: AtomicU64::new(0),
            frames_begun: AtomicU64::new(0),
            capture: Mutex::new(None),
        })
    }

    pub(crate) fn pending_layout(&self) -> (PixelSize, f64) {
        *self.pending_layout.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn set_pending_layout(&self, value: (PixelSize, f64)) {
        *self.pending_layout.lock().unwrap_or_else(PoisonError::into_inner) = value;
    }

    pub(crate) fn frames_presented(&self) -> u64 {
        self.frames_presented.load(Ordering::SeqCst)
    }

    pub(crate) fn frames_begun(&self) -> u64 {
        self.frames_begun.load(Ordering::SeqCst)
    }

    pub(crate) fn frame_begun(&self) {
        self.frames_begun.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn frame_presented(&self) {
        self.frames_presented.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn request_capture(&self, callback: Box<dyn FnOnce(FrameCapture) + Send>) {
        *self.capture.lock().unwrap_or_else(PoisonError::into_inner) = Some(callback);
    }

    pub(crate) fn capture_requested(&self) -> bool {
        self.capture.lock().unwrap_or_else(PoisonError::into_inner).is_some()
    }

    pub(crate) fn take_capture(&self) -> Option<Box<dyn FnOnce(FrameCapture) + Send>> {
        self.capture.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

/// The Metal render surface of a view.
pub struct MetalPlatformSurface {
    weak_self: Weak<MetalPlatformSurface>,
    layer: Arc<SharedLayer>,
    shared: Arc<SurfaceShared>,
}

impl MetalPlatformSurface {
    pub(crate) fn new(layer: Retained<CAMetalLayer>, shared: Arc<SurfaceShared>) -> Arc<MetalPlatformSurface> {
        Arc::new_cyclic(|weak_self| MetalPlatformSurface {
            weak_self: weak_self.clone(),
            layer: Arc::new(SharedLayer(layer)),
            shared,
        })
    }
}

impl IPlatformRenderSurface for MetalPlatformSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IMetalPlatformSurface>() {
            // The contract hands out an `Rc`, and the surface lives in an
            // `Arc`: the view forwards to the surface.
            let this: Rc<dyn IMetalPlatformSurface> = Rc::new(MetalPlatformSurfaceView(self.weak_self.upgrade()?));
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalPlatformSurface for MetalPlatformSurface {
    /// # Panics
    /// Panics when `device` is not a device of this backend.
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        let Some(dev) = device.as_any().downcast_ref::<MetalDevice>() else {
            panic!("The Metal device belongs to a different platform backend.");
        };
        self.layer.0.setDevice(Some(dev.mtl_device()));

        // The reference hands the target to the view, which gives it the
        // layout it knows; here the target reads the layout the view shares
        // with the surface.
        Rc::new(MetalRenderTarget::new(self.layer.clone(), device, self.shared.clone()))
    }
}

/// The Metal surface as the Metal contract hands it out
/// (`Rc<dyn IMetalPlatformSurface>`); it forwards to the shared surface.
struct MetalPlatformSurfaceView(Arc<MetalPlatformSurface>);

impl IPlatformRenderSurface for MetalPlatformSurfaceView {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        self.0.try_get_surface_kind(kind)
    }

    fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }
}

impl IMetalPlatformSurface for MetalPlatformSurfaceView {
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        self.0.create_metal_render_target(device)
    }
}
