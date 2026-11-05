//! The ProGPU render backend (docs/porting/progpu-backend.md): the render contracts of the base
//! library on the native engine of ProGPU, its WGSL shaders and its text stack, on a WebGPU device.

mod device;

pub use device::{ProGpuDevice, ProGpuDeviceError};

#[cfg(test)]
mod tests;
