//! The Metal platform contracts: what a platform exposes so that a render
//! backend can draw with Metal.
//!
//! The device and its command queue ([`IMetalDevice`]), the surface of a
//! top-level with its render target and the session of a frame
//! ([`IMetalPlatformSurface`], [`IMetalPlatformSurfaceRenderTarget`],
//! [`IMetalPlatformSurfaceRenderingSession`]), and the import of images and
//! shared events that were created outside of a device
//! ([`IMetalExternalObjectsFeature`]).
//!
//! These traits belong to no render backend: the platform that has a Metal
//! device implements them, and each backend that draws with Metal (Skia
//! through Graphite, Vello through `wgpu`) reads them. Handles to Metal
//! objects are passed as raw pointers, exactly as the platform hands them
//! out.

mod i_metal_device;
mod i_metal_external_objects_feature;

pub use i_metal_device::{
    try_get_metal_surface, IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget,
    IMetalPlatformSurfaceRenderingSession,
};
pub use i_metal_external_objects_feature::{IMetalExternalObjectsFeature, IMetalExternalTexture, IMetalSharedEvent};
