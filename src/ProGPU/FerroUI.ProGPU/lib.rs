//! The ProGPU render backend (docs/porting/progpu-backend.md): the render contracts of the base
//! library on the native engine of ProGPU, its WGSL shaders and its text stack, on a WebGPU device.

mod device;
mod geometry;
mod runtime;
mod scene;

pub use device::{ProGpuDevice, ProGpuDeviceError};
pub use geometry::{path_data_of, PathBuilder, PathData, ProGpuGeometry, ProGpuStreamGeometryBuilder, ProGpuTransformedGeometry};
pub use runtime::runtime;
pub use scene::{SceneBuildError, SceneBuilder};

#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod tests;
