//! The Metal platform contracts: what a platform exposes so that a render
//! backend can draw with Metal.
//!
//! These traits are independent of Skia. Handles to Metal objects are passed
//! as raw pointers, exactly as the platform hands them out.

mod i_metal_device;
mod i_metal_external_objects_feature;

pub use i_metal_device::{
    try_get_metal_surface, IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget,
    IMetalPlatformSurfaceRenderingSession,
};
pub use i_metal_external_objects_feature::{IMetalExternalObjectsFeature, IMetalExternalTexture, IMetalSharedEvent};
