//! The GPU abstraction of the backend.
//!
//! [`ISkiaGpu`] turns a platform graphics context into Skia render targets
//! and surfaces. Its Skia GPU context is exposed through [`ISkiaGrContext`],
//! which hides whether Skia runs on Graphite or Ganesh.

#[cfg(ferro_skia_ganesh_gl)]
pub mod ganesh;
#[cfg(target_vendor = "apple")]
pub mod graphite;
#[cfg(target_vendor = "apple")]
pub mod metal;
pub mod open_gl;

mod i_skia_gpu;
mod i_skia_gpu_render_session;
mod i_skia_gpu_render_target;
mod i_skia_gr_context;
mod skia_gpu_render_target;

pub use i_skia_gpu::{ISkiaGpu, ISkiaSurface, ScopedGrContext};
pub use i_skia_gpu_render_session::{ISkiaGpuRenderSession, SkiaSurfaceOrigin};
pub use i_skia_gpu_render_target::ISkiaGpuRenderTarget;
pub use i_skia_gr_context::{drawable_image, needs_mipmaps, ISkiaGrContext, SkiaGpuBackend};
pub use skia_gpu_render_target::SkiaGpuRenderTarget;
