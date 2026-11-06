//! Rendering with Metal, on Skia's Graphite backend; in the configuration
//! `ferro_skia_apple_ganesh` (Ganesh on OpenGL next to Metal, which no
//! published binary has with Graphite) on Ganesh, as the original does.

mod auto_release_pool;
mod skia_metal_external_objects_feature;
mod skia_metal_gpu;

pub use auto_release_pool::AutoReleasePool;
pub use skia_metal_external_objects_feature::SkiaMetalExternalObjectsFeature;
pub use skia_metal_gpu::{SkiaMetalGpu, SkiaMetalRenderSession, SkiaMetalRenderTarget};

#[cfg(all(test, target_os = "macos", not(ferro_skia_apple_ganesh)))]
mod tests;

#[cfg(not(ferro_skia_apple_ganesh))]
mod frame_completion;
