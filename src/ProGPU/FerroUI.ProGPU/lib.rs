//! The ProGPU render backend (docs/porting/progpu-backend.md): the render contracts of the base
//! library on the native engine of ProGPU, its WGSL shaders and its text stack, on a WebGPU device.

mod app;
mod bitmap;
#[cfg(target_os = "emscripten")]
mod canvas_target;
mod context;
mod device;
mod drawing_context;
mod effect_passes;
mod effect_records;
mod font;
mod font_manager;
mod fragment;
mod geometry;
mod glyph_run;
mod gpu;
mod image_textures;
mod layer;
mod notes;
mod outline_log;
mod perf;
mod platform;
mod region;
mod scene;
mod scene3d;
mod targets;
mod text_shaper;
mod typeface;
#[cfg(any(target_os = "linux", windows))]
mod window_surface;

pub use app::{initialize, initialize_text_shaping, ProGpuApplicationExtensions};
pub use bitmap::ProGpuBitmap;
pub use context::ProGpuContext;
pub use device::{ExternalImage, ProGpuDevice, ProGpuDeviceError, ProGpuEngine, TargetFormat};
pub use drawing_context::ProGpuDrawingContext;
pub use font_manager::ProGpuFontManager;
pub use fragment::ProGpuFragment;
pub use glyph_run::{ProGpuGlyphRun, RunGlyph};
pub use layer::ProGpuLayer;
pub use platform::ProGpuPlatformRenderInterface;
pub use region::ProGpuRegion;
pub use geometry::{path_data_of, PathBuilder, PathData, ProGpuGeometry, ProGpuStreamGeometryBuilder, ProGpuTransformedGeometry};
pub use scene::{SceneBuildError, SceneBuilder};
pub use text_shaper::{ProGpuShaperTypeface, ProGpuTextShaper};
pub use typeface::ProGpuTypeface;

#[cfg(test)]
mod drawing_tests;
#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod platform_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod text_draw_tests;
#[cfg(test)]
mod text_tests;
