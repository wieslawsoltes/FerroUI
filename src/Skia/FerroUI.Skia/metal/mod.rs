//! The Metal platform contracts: what a platform exposes so that a render
//! backend can draw with Metal.
//!
//! These traits are independent of Skia. Handles to Metal objects are passed
//! as raw pointers, exactly as the platform hands them out.

mod i_metal_device;

pub use i_metal_device::{
    try_get_metal_surface, IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget,
    IMetalPlatformSurfaceRenderingSession,
};
