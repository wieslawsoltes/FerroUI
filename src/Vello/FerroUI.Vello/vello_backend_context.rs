use crate::framebuffer_render_target::FramebufferRenderTarget;
use crate::surface_render_target::{SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextLayerImpl, IOptionalFeatureProvider, IPlatformRenderInterfaceContext, IRenderTarget,
};
use ferroui_base::{PixelSize, Vector};
use std::any::{Any, TypeId};
use std::rc::Rc;

/// The Vello render backend bound to software rendering: scenes are
/// rendered into memory.
///
/// The context of a graphics device (the hybrid and the GPU mode drawing
/// into the surface of a window) is stages 7 and 8 of the design document;
/// it will hold the device and the queue as the context of the Skia backend
/// holds its GPU.
pub struct VelloContext {
    rendering_modes: Vec<VelloRenderingMode>,
}

impl VelloContext {
    /// Creates the backend context of software rendering. Scenes are drawn
    /// in the first of `rendering_modes` that is available.
    pub fn new(rendering_modes: Vec<VelloRenderingMode>) -> Self {
        Self { rendering_modes }
    }
}

impl IOptionalFeatureProvider for VelloContext {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformRenderInterfaceContext for VelloContext {
    fn create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        for surface in surfaces {
            if let Some(framebuffer_surface) = surface.as_framebuffer_surface() {
                return Rc::new(FramebufferRenderTarget::new(framebuffer_surface, false, self.rendering_modes.clone()));
            }
        }

        panic!("Don't know how to create a Vello render target from any of provided surfaces");
    }

    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        scaling: Vector,
        _enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
            width: pixel_size.width,
            height: pixel_size.height,
            dpi: scaling * 96.0,
            rendering_modes: self.rendering_modes.clone(),
            use_scaled_drawing: false,
        }))
    }

    fn is_lost(&self) -> bool {
        false
    }

    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        // The renderers of the Vello project count pixels in 16 bits.
        Some(PixelSize::new(u16::MAX as i32, u16::MAX as i32))
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> bool {
        for surface in surfaces {
            if surface.as_framebuffer_surface().is_some() {
                return surface.is_ready();
            }
        }

        false
    }

    fn dispose(&self) {}
}
