//! The backend context: the device the frames of a compositor are rendered on, and the render
//! targets and layers made on it.

use crate::gpu::Gpu;
use crate::layer::ProGpuLayer;
use crate::notes;
use crate::targets::{FramebufferRenderTarget, UnavailableRenderTarget};
use crate::device::ProGpuDevice;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextLayerImpl, IOptionalFeatureProvider, IPlatformGraphicsContext, IPlatformRenderInterfaceContext, IRenderTarget,
};
use ferroui_base::{PixelSize, Vector};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// The largest side of a target of the engine, in pixels.
const MAX_TARGET_SIDE: i32 = 16384;

/// The backend context. With the Metal device of the platform its WebGPU device is on that Metal
/// device and its command queue, and the Metal surface of a window is drawn into directly;
/// without one the device is on the default adapter, and frames are read back into the
/// framebuffers of the surfaces. Without a device nothing is drawn.
pub struct ProGpuContext {
    gpu: Option<Rc<Gpu>>,
    graphics: Option<Rc<dyn IPlatformGraphicsContext>>,
    #[cfg(target_os = "macos")]
    metal: Option<Rc<dyn ferroui_metal::IMetalDevice>>,
    disposed: Cell<bool>,
}

impl ProGpuContext {
    /// The context for the graphics context of the platform, if it has one.
    pub fn new(graphics: Option<Rc<dyn IPlatformGraphicsContext>>) -> Self {
        #[cfg(target_os = "macos")]
        let metal = graphics.as_ref().and_then(|graphics| {
            graphics
                .try_get_feature(TypeId::of::<dyn ferroui_metal::IMetalDevice>())
                .and_then(|feature| feature.downcast_ref::<Rc<dyn ferroui_metal::IMetalDevice>>().cloned())
        });

        #[cfg(target_os = "macos")]
        let device = match &metal {
            // SAFETY: the platform hands out its live Metal device and command queue, which the
            // graphics context kept by this context keeps alive.
            Some(metal) => unsafe { ProGpuDevice::from_metal(metal.device(), metal.command_queue()) },
            None => ProGpuDevice::new(),
        };
        #[cfg(not(target_os = "macos"))]
        let device = ProGpuDevice::new();

        let gpu = match device {
            Ok(device) => Some(Gpu::new(device)),
            Err(error) => {
                notes::note_once("no-device", &format!("no WebGPU device, nothing is drawn: {error}"));
                None
            }
        };
        ProGpuContext {
            gpu,
            graphics,
            #[cfg(target_os = "macos")]
            metal,
            disposed: Cell::new(false),
        }
    }

    /// Whether the context has a device to draw with.
    pub fn has_device(&self) -> bool {
        self.gpu.is_some()
    }
}

impl IOptionalFeatureProvider for ProGpuContext {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformRenderInterfaceContext for ProGpuContext {
    fn create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        let Some(gpu) = self.gpu.as_ref().filter(|_| !self.disposed.get()) else {
            return Rc::new(UnavailableRenderTarget);
        };

        #[cfg(target_os = "macos")]
        if let Some(metal) = &self.metal {
            if let Some(surface) = surfaces.iter().find_map(|surface| ferroui_metal::try_get_metal_surface(&**surface)) {
                let target = surface.create_metal_render_target(metal.clone());
                return match crate::targets::MetalRenderTarget::new(gpu.clone(), target.clone()) {
                    Some(target) => Rc::new(target),
                    None => {
                        target.dispose();
                        Rc::new(UnavailableRenderTarget)
                    }
                };
            }
        }

        if let Some(surface) = surfaces.iter().find_map(|surface| surface.as_framebuffer_surface()) {
            return Rc::new(FramebufferRenderTarget::new(gpu.clone(), surface.create_framebuffer_render_target()));
        }

        notes::note_once("no-surface", "the ProGPU backend draws to Metal surfaces and to framebuffer surfaces only");
        Rc::new(UnavailableRenderTarget)
    }

    fn create_offscreen_render_target(&self, pixel_size: PixelSize, scaling: Vector, _enable_text_antialiasing: bool) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(ProGpuLayer::new(self.gpu.clone(), pixel_size, Vector::new(96.0 * scaling.x, 96.0 * scaling.y)))
    }

    fn is_lost(&self) -> bool {
        self.graphics.as_ref().is_some_and(|graphics| graphics.is_lost())
    }

    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        Some(PixelSize::new(MAX_TARGET_SIDE, MAX_TARGET_SIDE))
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        self.gpu.is_some() && !self.disposed.get() && surfaces.iter().any(|surface| surface.is_ready())
    }

    fn dispose(&self) {
        self.disposed.set(true);
    }
}
