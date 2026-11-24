//! Skia's Graphite backend behind the backend-neutral GPU context: on Metal for Apple targets, and
//! on Vulkan with the feature `graphite-vulkan` (#1365; the `ferro_skia_graphite` configuration of
//! `build.rs`).

mod band_queue;
mod graphite_gr_context;
mod graphite_prewarm;
// The pipelines made ahead from records (Metal only: its native half is compiled for Apple targets).
#[cfg(target_vendor = "apple")]
pub(crate) mod pipeline_record;
mod pipelined_layer;

pub use graphite_gr_context::GraphiteGrContext;
#[cfg(feature = "graphite-vulkan")]
pub(crate) use graphite_gr_context::FrameTarget;
// What the Metal tests use (`gpu/metal/tests.rs`).
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use graphite_gr_context::UPLOADED_IMAGE_SWEEP_INTERVAL;
pub use graphite_prewarm::GraphitePrewarmContext;
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use pipelined_layer::{band_clips, band_count};
// The Metal tests and the Graphite Vulkan tests choose the insertion with it.
#[cfg(test)]
pub(crate) use pipelined_layer::BandInsert;
#[cfg(feature = "graphite-vulkan")]
pub(crate) use pipelined_layer::LayerTexture;
pub(crate) use pipelined_layer::PipelinedLayerSurface;
#[cfg(target_vendor = "apple")]
pub(crate) use crate::gpu::metal::AutoReleasePool;

/// Where Metal pools the objects Skia autoreleases on a thread, other backends have nothing to
/// pool: the scope of a pool is empty.
#[cfg(not(target_vendor = "apple"))]
#[derive(Default)]
pub(crate) struct AutoReleasePool;

#[cfg(not(target_vendor = "apple"))]
impl AutoReleasePool {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn dispose(&self) {}
}
