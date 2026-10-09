//! The scene of a frame and the renderers that draw it.
//!
//! The drawing context of the backend records into an [`IVelloSceneSink`];
//! there is one implementation for each rendering mode
//! ([`VelloRenderingMode`](crate::VelloRenderingMode)): the CPU mode
//! ([`VelloCpuSceneSink`]), which is always built, and the hybrid and the
//! GPU mode (`VelloHybridSceneSink`, `VelloGpuSceneSink`), each behind a
//! feature of the crate (`hybrid`, `gpu`) and in need of a graphics device.

mod i_vello_scene_sink;
mod vello_cpu_scene_sink;
#[cfg(feature = "gpu")]
mod vello_gpu_scene_sink;
#[cfg(feature = "hybrid")]
mod vello_hybrid_scene_sink;

pub use i_vello_scene_sink::{
    IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloSceneGlyph, VelloSceneGlyphRun, VelloSceneImage,
    VelloScenePaint,
};
pub use i_vello_scene_sink::{VelloSceneFilter, VelloSceneFilterCapabilities, VelloScenePixelRect};
#[cfg(any(feature = "hybrid", feature = "gpu"))]
pub use i_vello_scene_sink::VelloSceneTexture;
pub use vello_cpu_scene_sink::VelloCpuSceneSink;
#[cfg(feature = "gpu")]
pub use vello_gpu_scene_sink::{VelloGpuAntiAliasing, VelloGpuSceneSink};
#[cfg(feature = "hybrid")]
pub use vello_hybrid_scene_sink::VelloHybridSceneSink;

use crate::vello_options::VelloRenderingMode;

/// A rendering mode that cannot draw, with the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VelloRenderingModeUnavailable {
    /// The mode that was asked for.
    pub mode: VelloRenderingMode,
    /// Why it cannot draw.
    pub reason: String,
}

impl std::fmt::Display for VelloRenderingModeUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The {:?} rendering mode of the Vello backend is not available: {}", self.mode, self.reason)
    }
}

impl std::error::Error for VelloRenderingModeUnavailable {}

/// Creates the scene of a target of the given size that is rendered into
/// memory, in one rendering mode.
///
/// The hybrid and the GPU mode fail when the crate was built without their
/// feature and when the machine has no graphics device for them (the GPU
/// mode needs one that runs compute shaders); they draw on the device of
/// the platform graphics context when there is one, and on a device of
/// their own otherwise ([`VelloWgpuDevice::shared`](crate::gpu::VelloWgpuDevice::shared)).
pub fn try_create_scene_sink(
    mode: VelloRenderingMode,
    width: u16,
    height: u16,
) -> Result<Box<dyn IVelloSceneSink>, VelloRenderingModeUnavailable> {
    match mode {
        VelloRenderingMode::Cpu => Ok(Box::new(VelloCpuSceneSink::new(width, height))),
        #[cfg(feature = "hybrid")]
        VelloRenderingMode::Hybrid => match crate::gpu::VelloWgpuDevice::shared() {
            Ok(device) => Ok(Box::new(VelloHybridSceneSink::new(device, width, height))),
            Err(error) => Err(VelloRenderingModeUnavailable { mode, reason: error.to_string() }),
        },
        #[cfg(not(feature = "hybrid"))]
        VelloRenderingMode::Hybrid => Err(VelloRenderingModeUnavailable {
            mode,
            reason: "the crate was built without its feature `hybrid`".to_string(),
        }),
        #[cfg(feature = "gpu")]
        VelloRenderingMode::Gpu => match crate::gpu::VelloWgpuDevice::shared() {
            Ok(device) if device.supports_compute() => Ok(Box::new(VelloGpuSceneSink::new(device, width, height))),
            Ok(device) => Err(VelloRenderingModeUnavailable {
                mode,
                reason: format!("the graphics device ({}) does not run compute shaders", device.adapter_info().name),
            }),
            Err(error) => Err(VelloRenderingModeUnavailable { mode, reason: error.to_string() }),
        },
        #[cfg(not(feature = "gpu"))]
        VelloRenderingMode::Gpu => Err(VelloRenderingModeUnavailable {
            mode,
            reason: "the crate was built without its feature `gpu`".to_string(),
        }),
    }
}

/// Creates the scene of a target that is rendered into memory: in the CPU
/// mode when it is among the given modes, and in the first of them that is
/// available otherwise
/// ([`VelloOptions::rendering_modes`](crate::VelloOptions::rendering_modes)).
///
/// # Panics
/// Panics when none of the modes is available, with the reason of each.
pub fn create_scene_sink(modes: &[VelloRenderingMode], width: u16, height: u16) -> Box<dyn IVelloSceneSink> {
    if modes.contains(&VelloRenderingMode::Cpu) {
        return Box::new(VelloCpuSceneSink::new(width, height));
    }

    create_first_available_scene_sink(modes, width, height)
}

/// Creates a scene in the first of the given modes that is available.
///
/// # Panics
/// Panics when none of the modes is available, with the reason of each.
pub fn create_first_available_scene_sink(
    modes: &[VelloRenderingMode],
    width: u16,
    height: u16,
) -> Box<dyn IVelloSceneSink> {
    let mut reasons = Vec::new();

    for mode in modes {
        match try_create_scene_sink(*mode, width, height) {
            Ok(sink) => return sink,
            Err(unavailable) => reasons.push(unavailable.to_string()),
        }
    }

    panic!("No rendering mode of the Vello backend is available. {}", reasons.join(" "));
}
