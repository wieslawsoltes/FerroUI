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

/// The Vello render backend bound to software rendering (scenes are
/// rendered into memory) or to the graphics device of a platform, whose
/// surfaces the hybrid and the GPU mode draw to: it holds the GPU as the
/// context of the Skia backend holds its GPU.
pub struct VelloContext {
    rendering_modes: Vec<VelloRenderingMode>,
    #[cfg(any(feature = "hybrid", feature = "gpu"))]
    gpu: Option<Rc<dyn crate::gpu::IVelloGpu>>,
}

impl VelloContext {
    /// Creates the backend context of software rendering. Scenes are drawn
    /// in the first of `rendering_modes` that is available.
    pub fn new(rendering_modes: Vec<VelloRenderingMode>) -> Self {
        Self {
            rendering_modes,
            #[cfg(any(feature = "hybrid", feature = "gpu"))]
            gpu: None,
        }
    }

    /// Creates the backend context of a graphics device: the surfaces of
    /// the platform the GPU knows are drawn on the device, in the first of
    /// `rendering_modes` it runs.
    #[cfg(any(feature = "hybrid", feature = "gpu"))]
    pub fn with_gpu(gpu: Rc<dyn crate::gpu::IVelloGpu>, rendering_modes: Vec<VelloRenderingMode>) -> Self {
        Self { rendering_modes, gpu: Some(gpu) }
    }

    /// The GPU of the context, when it has one.
    #[cfg(any(feature = "hybrid", feature = "gpu"))]
    pub fn gpu(&self) -> Option<&Rc<dyn crate::gpu::IVelloGpu>> {
        self.gpu.as_ref()
    }
}

impl IOptionalFeatureProvider for VelloContext {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformRenderInterfaceContext for VelloContext {
    fn create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        #[cfg(any(feature = "hybrid", feature = "gpu"))]
        if let Some(gpu) = &self.gpu {
            if let Some(target) = gpu.clone().try_create_render_target(surfaces, &self.rendering_modes) {
                return target;
            }
        }

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
        // With a graphics device whose window is drawn in a mode that
        // composes over a texture, an offscreen target is a texture of the
        // device too: what is drawn into it is drawn from without leaving
        // the device.
        #[cfg(any(feature = "hybrid", feature = "gpu"))]
        if let Some(gpu) = &self.gpu {
            let window_mode = self.rendering_modes.iter().copied().find(|mode| match mode {
                VelloRenderingMode::Cpu => true,
                VelloRenderingMode::Hybrid => cfg!(feature = "hybrid"),
                VelloRenderingMode::Gpu => cfg!(feature = "gpu") && gpu.device().supports_compute(),
            });
            if let Some(mode) = window_mode
                .filter(|mode| crate::gpu::DeviceSurfaceRenderTarget::is_available(*mode, gpu.device()))
            {
                return Rc::new(crate::gpu::DeviceSurfaceRenderTarget::new(
                    gpu.device().clone(),
                    mode,
                    pixel_size,
                    scaling * 96.0,
                    self.rendering_modes.clone(),
                    false,
                ));
            }
        }

        Rc::new(SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
            width: pixel_size.width,
            height: pixel_size.height,
            dpi: scaling * 96.0,
            rendering_modes: self.rendering_modes.clone(),
            use_scaled_drawing: false,
        }))
    }

    fn is_lost(&self) -> bool {
        // A lost device draws nothing: the compositor makes a new context
        // and new render targets.
        #[cfg(any(feature = "hybrid", feature = "gpu"))]
        if let Some(gpu) = &self.gpu {
            return gpu.device().is_lost();
        }

        false
    }

    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        // The renderers of the Vello project count pixels in 16 bits.
        Some(PixelSize::new(u16::MAX as i32, u16::MAX as i32))
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> bool {
        #[cfg(any(feature = "hybrid", feature = "gpu"))]
        if let Some(gpu) = &self.gpu {
            if gpu.is_ready_to_create_render_target(surfaces) {
                return true;
            }
        }

        for surface in surfaces {
            if surface.as_framebuffer_surface().is_some() {
                return surface.is_ready();
            }
        }

        false
    }

    fn dispose(&self) {
        crate::perf::print_summary();

        #[cfg(any(feature = "hybrid", feature = "gpu"))]
        if let Some(gpu) = &self.gpu {
            gpu.dispose();
        }
    }
}
