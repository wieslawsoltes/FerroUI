//! Rendering with Metal, on Skia's Graphite backend.

mod auto_release_pool;
mod skia_metal_external_objects_feature;
mod skia_metal_gpu;

pub use auto_release_pool::AutoReleasePool;
pub use skia_metal_external_objects_feature::SkiaMetalExternalObjectsFeature;
pub use skia_metal_gpu::{SkiaMetalGpu, SkiaMetalRenderSession, SkiaMetalRenderTarget};

#[cfg(all(test, target_os = "macos"))]
mod tests;
