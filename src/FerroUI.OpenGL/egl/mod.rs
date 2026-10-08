//! EGL: the constants, the error codes and the entry points, and the display, the contexts,
//! the surfaces and the images that are built on them.
//!
//! The surfaces a context renders to are `EglGlPlatformSurfaceBase` (with the base of their
//! render targets), `EglGlPlatformSurface` for a window and `EglGlPlatformImageSurfaceBase`
//! for an `EGLImage`.
//!
//! A context offers the import of dma-buf images, and what the extensions of OpenGL for
//! external objects offer, through its external objects feature
//! (`EglExternalObjectsFeature`, in two files as the partial class of the original).

pub mod egl_consts;

mod egl_context;
mod egl_display;
mod egl_display_options;
mod egl_display_utils;
mod egl_errors;
mod egl_external_objects_feature;
mod egl_external_objects_feature_drm;
mod egl_gl_platform_image_surface_base;
mod egl_gl_platform_surface;
mod egl_gl_platform_surface_base;
mod egl_image;
mod egl_interface;
mod egl_platform_graphics;
mod egl_surface;

pub use egl_context::{EglContext, MakeCurrentError};
pub use egl_display::EglDisplay;
pub use egl_display_options::{
    EglConfigProbeCallback, EglContextFeature, EglContextFeatureFactory, EglContextOptions, EglDisplayCreationOptions,
    EglDisplayOptions,
};
pub use egl_errors::EglErrors;
pub use egl_gl_platform_image_surface_base::{
    BeginImageDrawError, EglGlPlatformImageSurfaceBase, EglPlatformImageSurfaceRenderTarget,
    EglPlatformImageSurfaceRenderTargetBase,
};
pub use egl_gl_platform_surface::{
    EglGlPlatformSurface, IEglWindowGlPlatformSurfaceInfo, IEglWindowGlPlatformSurfaceInfoWithWaitPolicy,
};
pub use egl_gl_platform_surface_base::{
    EglGlPlatformSurfaceBase, EglPlatformSurfaceRenderTarget, EglPlatformSurfaceRenderTargetBase,
};
pub use egl_image::EglImage;
pub use egl_interface::EglInterface;
pub use egl_platform_graphics::EglPlatformGraphics;
pub use egl_surface::EglSurface;
