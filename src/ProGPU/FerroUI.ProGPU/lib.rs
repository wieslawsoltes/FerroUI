//! The ProGPU render backend (docs/porting/progpu-backend.md): the render contracts of the base
//! library on the native engine of ProGPU, its WGSL shaders and its text stack, on a WebGPU device.

mod app;
mod bitmap;
mod context;
mod device;
mod drawing_context;
mod geometry;
mod glyph_run;
mod gpu;
mod layer;
mod notes;
mod platform;
mod region;
mod scene;
mod targets;

pub use app::{initialize, ProGpuApplicationExtensions};
pub use bitmap::ProGpuBitmap;
pub use context::ProGpuContext;
pub use device::{ExternalImage, ProGpuDevice, ProGpuDeviceError, ProGpuEngine, TargetFormat};
pub use drawing_context::ProGpuDrawingContext;
pub use glyph_run::ProGpuGlyphRun;
pub use layer::ProGpuLayer;
pub use platform::ProGpuPlatformRenderInterface;
pub use region::ProGpuRegion;
pub use geometry::{path_data_of, PathBuilder, PathData, ProGpuGeometry, ProGpuStreamGeometryBuilder, ProGpuTransformedGeometry};
pub use scene::{SceneBuildError, SceneBuilder};

#[cfg(test)]
mod drawing_tests;
#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod platform_tests;
#[cfg(test)]
mod tests;
