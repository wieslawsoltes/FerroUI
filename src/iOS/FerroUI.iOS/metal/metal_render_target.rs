//! The Metal render target of a view: the drawables of its layer.

use super::{MetalDrawingSession, SharedLayer, SurfaceShared};
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::platform::PlatformGraphicsContextLostException;
use ferroui_base::PixelSize;
use ferroui_metal::{IMetalDevice, IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession};
use objc2_core_foundation::CGSize;
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
}

impl IPlatformRenderSurfaceRenderTarget for MetalRenderTarget {}

impl IMetalPlatformSurfaceRenderTarget for MetalRenderTarget {
    /// # Panics
    /// Panics with the message of a lost graphics context when the layer
    /// gives no drawable.
    fn begin_rendering(&self) -> Rc<dyn IMetalPlatformSurfaceRenderingSession> {
        self.shared.frame_begun();
        let (size, scaling) = self.pending_layout();
        if self.last_layout.get() != Some((size, scaling)) {
            self.last_layout.set(Some((size, scaling)));
            self.layer.0.setDrawableSize(CGSize { width: f64::from(size.width), height: f64::from(size.height) });
        }

        // A frame whose pixels are asked for is drawn to a texture that
        // can be read back.
        let capturable = self.shared.capture_requested();
        if capturable {
            self.layer.0.setFramebufferOnly(false);
        }

        let Some(drawable) = self.layer.0.nextDrawable() else {
            panic!("{}", PlatformGraphicsContextLostException);
        };
        Rc::new(MetalDrawingSession::new(
            self.device.clone(),
            drawable,
            size,
            scaling,
            self.shared.clone(),
            capturable,
        ))
    }

    fn dispose(&self) {}
}
