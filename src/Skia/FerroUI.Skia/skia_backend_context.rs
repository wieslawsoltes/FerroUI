use crate::framebuffer_render_target::FramebufferRenderTarget;
use crate::gpu::{ISkiaGpu, SkiaGpuRenderTarget};
use crate::surface_render_target::{SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextLayerImpl, IOptionalFeatureProvider, IPlatformRenderInterfaceContext, IRenderTarget,
};
use ferroui_base::{PixelSize, Vector};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// The Skia render backend bound to a GPU (or to software rendering).
pub struct SkiaContext {
    gpu: RefCell<Option<Rc<dyn ISkiaGpu>>>,
    max_offscreen_render_target_pixel_size: Option<PixelSize>,
}

impl SkiaContext {
    /// Creates the backend context. `None` selects software rendering.
    pub fn new(gpu: Option<Rc<dyn ISkiaGpu>>) -> Self {
        let mut max_offscreen_render_target_pixel_size = None;

        if let Some(gpu) = &gpu {
            if let Some(gr) = gpu.try_get_gr_context() {
                if let Some(render_target_size) = gr.value().max_render_target_size() {
                    max_offscreen_render_target_pixel_size =
                        Some(PixelSize::new(render_target_size, render_target_size));
                }
                gr.dispose();
            }
        }

        Self { gpu: RefCell::new(gpu), max_offscreen_render_target_pixel_size }
    }

    fn gpu(&self) -> Option<Rc<dyn ISkiaGpu>> {
        self.gpu.borrow().clone()
    }
}

impl IOptionalFeatureProvider for SkiaContext {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.gpu()?.try_get_feature(feature_type)
    }
}

impl IPlatformRenderInterfaceContext for SkiaContext {
    fn create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        if let Some(gpu) = self.gpu() {
            if let Some(gpu_render_target) = gpu.try_create_render_target(surfaces) {
                return Rc::new(SkiaGpuRenderTarget::new(gpu, gpu_render_target));
            }
        }

        for surface in surfaces {
            if let Some(framebuffer_surface) = surface.as_framebuffer_surface() {
                return Rc::new(FramebufferRenderTarget::new(framebuffer_surface, false));
            }
        }

        panic!("Don't know how to create a Skia render target from any of provided surfaces");
    }

    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        scaling: Vector,
        enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        let gpu = self.gpu();
        let gr = gpu.as_ref().and_then(|gpu| gpu.try_get_gr_context());

        let create_info = SurfaceRenderTargetCreateInfo {
            width: pixel_size.width,
            height: pixel_size.height,
            dpi: scaling * 96.0,
            format: None,
            disable_text_lcd_rendering: !enable_text_antialiasing,
            gr_context: gr.as_ref().map(|gr| gr.value().clone()),
            gpu,
            disable_manual_fbo: true,
            session: None,
            use_scaled_drawing: false,
        };

        let render_target = SurfaceRenderTarget::new(create_info);

        if let Some(gr) = gr {
            gr.dispose();
        }

        Rc::new(render_target)
    }

    fn is_lost(&self) -> bool {
        self.gpu().is_some_and(|gpu| gpu.is_lost())
    }

    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        self.max_offscreen_render_target_pixel_size
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> bool {
        if let Some(gpu) = self.gpu() {
            return gpu.is_ready_to_create_render_target(surfaces);
        }

        for surface in surfaces {
            if surface.as_framebuffer_surface().is_some() {
                return surface.is_ready();
            }
        }

        false
    }

    fn dispose(&self) {
        if let Some(gpu) = self.gpu.borrow_mut().take() {
            gpu.dispose();
        }
    }
}
