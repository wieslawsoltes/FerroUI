//! Rendering with OpenGL, OpenGL ES and WebGL through Skia's Ganesh backend.
//!
//! The GPU itself and its external objects feature need Ganesh, which the
//! Skia build of a platform either has or has not (see the features of the
//! Skia dependency in the manifest of this crate); the option contract and
//! the texture a context shares with the compositor, which is plain OpenGL,
//! are always present.

#[cfg(ferro_skia_ganesh_gl)]
mod fbo_skia_surface;
#[cfg(ferro_skia_ganesh_gl)]
mod gl_render_target;
#[cfg(ferro_skia_ganesh_gl)]
mod gl_skia_external_objects_feature;
#[cfg(ferro_skia_ganesh_gl)]
mod gl_skia_gpu;
mod gl_skia_shared_texture_for_composition;
mod i_gl_skia_specific_options_feature;

#[cfg(ferro_skia_ganesh_gl)]
pub use fbo_skia_surface::FboSkiaSurface;
#[cfg(ferro_skia_ganesh_gl)]
pub use gl_render_target::GlRenderTarget;
#[cfg(ferro_skia_ganesh_gl)]
pub use gl_skia_external_objects_feature::{GlSkiaExternalObjectsFeature, GlSkiaImportedImage, GlSkiaImportedSemaphore};
#[cfg(ferro_skia_ganesh_gl)]
pub use gl_skia_gpu::GlSkiaGpu;
pub use gl_skia_shared_texture_for_composition::GlSkiaSharedTextureForComposition;
pub use i_gl_skia_specific_options_feature::IGlSkiaSpecificOptionsFeature;
