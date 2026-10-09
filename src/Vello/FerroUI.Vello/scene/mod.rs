//! The scene of a frame and the renderers that draw it.
//!
//! The drawing context of the backend records into an [`IVelloSceneSink`];
//! there is one implementation for each rendering mode
//! ([`VelloRenderingMode`](crate::VelloRenderingMode)): the CPU mode is
//! built ([`VelloCpuSceneSink`]), the hybrid and the GPU mode are stages 7
//! and 8 of the design document.

mod i_vello_scene_sink;
mod vello_cpu_scene_sink;

pub use i_vello_scene_sink::{
    IVelloSceneSink, VelloSceneBrush, VelloSceneCapabilities, VelloSceneImage, VelloScenePaint,
};
pub use vello_cpu_scene_sink::VelloCpuSceneSink;

use crate::vello_options::VelloRenderingMode;

/// A rendering mode that cannot draw, with the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VelloRenderingModeUnavailable {
    /// The mode that was asked for.
    pub mode: VelloRenderingMode,
    /// Why it cannot draw.
    pub reason: &'static str,
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
/// The hybrid and the GPU mode are not built yet: they fail with the stage
/// they belong to.
pub fn try_create_scene_sink(
    mode: VelloRenderingMode,
    width: u16,
    height: u16,
) -> Result<Box<dyn IVelloSceneSink>, VelloRenderingModeUnavailable> {
    match mode {
        VelloRenderingMode::Cpu => Ok(Box::new(VelloCpuSceneSink::new(width, height))),
        VelloRenderingMode::Hybrid => Err(VelloRenderingModeUnavailable {
            mode,
            reason: "the sparse-strips renderer on the GPU is stage 7 of docs/porting/vello-backend.md",
        }),
        VelloRenderingMode::Gpu => Err(VelloRenderingModeUnavailable {
            mode,
            reason: "the compute renderer on the GPU is stage 8 of docs/porting/vello-backend.md",
        }),
    }
}

/// Creates the scene of a target that is rendered into memory, in the first
/// of the given modes that is available, and tells which one that is.
///
/// # Panics
/// Panics when none of the modes is available, with the reason of each.
pub fn create_scene_sink(modes: &[VelloRenderingMode], width: u16, height: u16) -> Box<dyn IVelloSceneSink> {
    let mut reasons = Vec::new();

    for mode in modes {
        match try_create_scene_sink(*mode, width, height) {
            Ok(sink) => return sink,
            Err(unavailable) => reasons.push(unavailable.to_string()),
        }
    }

    panic!("No rendering mode of the Vello backend is available. {}", reasons.join(" "));
}
