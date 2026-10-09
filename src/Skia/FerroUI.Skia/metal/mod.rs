//! The Metal platform contracts: what a platform exposes so that a render
//! backend can draw with Metal.
//!
//! The contracts are independent of Skia and live in a crate of their own
//! (`ferroui-metal`), which the platform and every backend that draws with
//! Metal depend on; they are re-exported here under the names this crate
//! has always had for them.

pub use ferroui_metal::{
    try_get_metal_surface, IMetalDevice, IMetalExternalObjectsFeature, IMetalExternalTexture, IMetalPlatformSurface,
    IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession, IMetalSharedEvent,
};
