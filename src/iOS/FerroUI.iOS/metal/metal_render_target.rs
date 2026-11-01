//! The Metal render target of a view: the drawables of its layer.

use super::{MetalDevice, MetalDrawingSession, SharedLayer, SurfaceShared};
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::platform::PlatformGraphicsContextLostError;
use ferroui_base::PixelSize;
use ferroui_metal::{IMetalDevice, IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::CGSize;
use objc2_foundation::NSThread;
use objc2_metal::{MTLCommandBuffer, MTLCommandQueue};
use objc2_quartz_core::CATransaction;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// The Metal render target of a view.
pub struct MetalRenderTarget {
    layer: Arc<SharedLayer>,
    device: Rc<dyn IMetalDevice>,
    shared: Arc<SurfaceShared>,
    last_layout: Cell<Option<(PixelSize, f64)>>,
}

impl MetalRenderTarget {
    pub(crate) fn new(layer: Arc<SharedLayer>, device: Rc<dyn IMetalDevice>, shared: Arc<SurfaceShared>) -> Self {
        Self { layer, device, shared, last_layout: Cell::new(None) }
    }

    /// The layout the next frame is drawn for: the size in pixels and the
    /// scaling of the last layout of the view.
    pub fn pending_layout(&self) -> (PixelSize, f64) {
        self.shared.pending_layout()
    }

    /// Sets the layout the next frame is drawn for. The reference's view
    /// sets it on its render target; the view of the port sets it in the
    /// state it shares with its surface (`layoutSubviews`), which is where
    /// this writes too.
    pub fn set_pending_layout(&self, value: (PixelSize, f64)) {
        self.shared.set_pending_layout(value);
    }
}

impl IPlatformRenderSurfaceRenderTarget for MetalRenderTarget {}

impl IMetalPlatformSurfaceRenderTarget for MetalRenderTarget {
    /// # Panics
    /// Panics with the message of a lost graphics context when the layer
    /// gives no drawable.
    ///
    /// A frame begun on the main thread is the synchronous frame of a layout of the view
    /// (`layoutSubviews` paints): the work the render thread has queued is finished first, the
    /// layer is given the size of the new layout and the frame is presented with the current Core
    /// Animation transaction (`presentsWithTransaction`), the one that gives the view its new
    /// bounds, so the two appear together. The frames of the render thread are presented
    /// asynchronously, as before. The same as the Metal render target of macOS.
    fn begin_rendering(&self) -> Rc<dyn IMetalPlatformSurfaceRenderingSession> {
        self.shared.frame_begun();
        let (size, scaling) = self.pending_layout();
        let on_main_thread = NSThread::isMainThread_class();
        if on_main_thread {
            if let Some(buffer) = metal_queue(&self.device).and_then(|queue| queue.commandBuffer()) {
                buffer.commit();
                buffer.waitUntilCompleted();
            }
        }
        if self.last_layout.get() != Some((size, scaling)) {
            self.last_layout.set(Some((size, scaling)));
            let drawable_size = CGSize { width: f64::from(size.width), height: f64::from(size.height) };
            if on_main_thread {
                CATransaction::begin();
                CATransaction::setDisableActions(true);
                self.layer.0.setDrawableSize(drawable_size);
                CATransaction::commit();
            } else {
                self.layer.0.setDrawableSize(drawable_size);
            }
        }
        if on_main_thread {
            self.layer.0.setPresentsWithTransaction(true);
        }

        // A frame whose pixels are asked for is drawn to a texture that
        // can be read back.
        let capturable = self.shared.capture_requested();
        if capturable {
            self.layer.0.setFramebufferOnly(false);
        }

        let Some(drawable) = self.layer.0.nextDrawable() else {
            if on_main_thread {
                self.layer.0.setPresentsWithTransaction(false);
            }
            panic!("{}", PlatformGraphicsContextLostError);
        };
        Rc::new(MetalDrawingSession::new(
            self.device.clone(),
            drawable,
            size,
            scaling,
            self.shared.clone(),
            capturable,
            on_main_thread.then(|| self.layer.clone()),
        ))
    }

    fn dispose(&self) {}
}

/// The command queue of the device, when it is the device of this backend.
fn metal_queue(device: &Rc<dyn IMetalDevice>) -> Option<Retained<ProtocolObject<dyn MTLCommandQueue>>> {
    device.as_any().downcast_ref::<MetalDevice>().map(MetalDevice::queue)
}
