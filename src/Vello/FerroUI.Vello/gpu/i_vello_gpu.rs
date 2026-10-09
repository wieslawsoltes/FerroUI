use crate::gpu::VelloWgpuDevice;
use crate::scene::IVelloSceneSink;
use crate::vello_options::VelloRenderingMode;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{IPlatformGraphicsContext, IRenderTarget};
use std::rc::Rc;
use std::sync::Arc;

/// The graphics device of a platform as the Vello backend draws with it:
/// what `ISkiaGpu` is to the Skia backend.
///
/// An implementation makes a [`VelloWgpuDevice`] over the device of the
/// platform and render targets for the surfaces of the platform it knows
/// (`gpu/metal`: the Metal device and the surface of a top-level of the
/// macOS platform).
pub trait IVelloGpu {
    /// The `wgpu` device over the device of the platform.
    fn device(&self) -> &Arc<VelloWgpuDevice>;

    /// The graphics context of the platform this GPU draws with.
    fn platform_graphics_context(&self) -> Rc<dyn IPlatformGraphicsContext>;

    /// The render target of the first of the surfaces this GPU draws to,
    /// with scenes in the first of `rendering_modes` that the device runs.
    fn try_create_render_target(
        self: Rc<Self>,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
        rendering_modes: &[VelloRenderingMode],
    ) -> Option<Rc<dyn IRenderTarget>>;

    /// Whether a render target can be created for one of the surfaces now.
    fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool;

    /// Releases the device.
    fn dispose(&self);
}

/// Creates the scene of a frame of a window whose texture has the given
/// format: in the first of the given modes that the device runs. The CPU mode is one of them: its scene is
/// rendered into memory and copied to the texture of the window.
///
/// # Panics
/// Panics when none of the modes is available, with the reason of each.
pub fn create_window_scene_sink(
    device: &Arc<VelloWgpuDevice>,
    rendering_modes: &[VelloRenderingMode],
    width: u16,
    height: u16,
    format: wgpu::TextureFormat,
) -> Box<dyn IVelloSceneSink> {
    let _ = format;
    let mut reasons = Vec::new();

    for mode in rendering_modes {
        match mode {
            VelloRenderingMode::Cpu => return Box::new(crate::scene::VelloCpuSceneSink::new(width, height)),
            #[cfg(feature = "hybrid")]
            VelloRenderingMode::Hybrid => {
                return Box::new(crate::scene::VelloHybridSceneSink::for_format(device.clone(), width, height, format))
            }
            #[cfg(not(feature = "hybrid"))]
            VelloRenderingMode::Hybrid => reasons.push("The crate was built without its feature `hybrid`.".to_string()),
            #[cfg(feature = "gpu")]
            VelloRenderingMode::Gpu if device.supports_compute() => {
                return Box::new(crate::scene::VelloGpuSceneSink::new(device.clone(), width, height))
            }
            #[cfg(feature = "gpu")]
            VelloRenderingMode::Gpu => reasons
                .push(format!("The graphics device ({}) does not run compute shaders.", device.adapter_info().name)),
            #[cfg(not(feature = "gpu"))]
            VelloRenderingMode::Gpu => reasons.push("The crate was built without its feature `gpu`.".to_string()),
        }
    }

    panic!("No rendering mode of the Vello backend is available for a window. {}", reasons.join(" "));
}

/// The mode the window of a device is drawn in: the first of the given
/// modes that the crate was built with and that the device runs.
///
/// # Panics
/// Panics when none of the modes is available, with the reason of each.
pub fn window_rendering_mode(device: &VelloWgpuDevice, rendering_modes: &[VelloRenderingMode]) -> VelloRenderingMode {
    let mut reasons = Vec::new();

    for mode in rendering_modes {
        match mode {
            VelloRenderingMode::Cpu => return *mode,
            VelloRenderingMode::Hybrid if cfg!(feature = "hybrid") => return *mode,
            VelloRenderingMode::Hybrid => reasons.push("The crate was built without its feature `hybrid`.".to_string()),
            VelloRenderingMode::Gpu if !cfg!(feature = "gpu") => {
                reasons.push("The crate was built without its feature `gpu`.".to_string())
            }
            VelloRenderingMode::Gpu if device.supports_compute() => return *mode,
            VelloRenderingMode::Gpu => reasons
                .push(format!("The graphics device ({}) does not run compute shaders.", device.adapter_info().name)),
        }
    }

    panic!("No rendering mode of the Vello backend is available for a window. {}", reasons.join(" "));
}
