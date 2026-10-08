//! The OpenGL platform contracts.
//!
//! What a platform exposes so that a render backend can draw with OpenGL,
//! OpenGL ES or WebGL: the context ([`IGlContext`]), the surfaces it renders
//! to ([`surfaces`]) and the entry points the framework itself calls
//! ([`GlInterface`]).
//!
//! The entry point tables call into the platform's OpenGL library through
//! function pointers resolved by name. That is the one place of this crate
//! with `unsafe` code; see [`GlInterface`] for the contract.

pub mod composition;
pub mod controls;
pub mod egl;
pub mod features;
pub mod gl_consts;
pub mod surfaces;

mod entry_points;
mod gl_basic_info_interface;
mod gl_entry_point_attribute;
mod gl_errors;
mod gl_interface;
mod gl_version;
mod i_gl_context;
mod i_gl_context_external_objects_feature;
mod i_open_gl_texture_sharing_render_interface_context_feature;
mod i_platform_graphics_open_gl_context_factory;
mod open_gl_exception;
mod register_types;
mod rust_paths;

pub use entry_points::GetProcAddress;
pub use gl_basic_info_interface::GlBasicInfoInterface;
pub use gl_entry_point_attribute::{GlExtensionEntryPoint, GlMinVersionEntryPoint};
pub use gl_errors::GlErrors;
pub use gl_interface::{GlContextInfo, GlInterface};
pub use gl_version::{GlProfileType, GlVersion};
pub use i_gl_context::{IGlContext, IGlPlatformSurfaceRenderTargetFactory};
pub use i_gl_context_external_objects_feature::{
    IGlContextExternalObjectsFeature, IGlExportableExternalImageTexture, IGlExportableExternalSemaphore,
    IGlExternalImageTexture, IGlExternalSemaphore,
};
pub use i_open_gl_texture_sharing_render_interface_context_feature::{
    ICompositionImportableOpenGlSharedTexture, IOpenGlTextureSharingRenderInterfaceContextFeature,
};
pub use i_platform_graphics_open_gl_context_factory::IPlatformGraphicsOpenGlContextFactory;
pub use open_gl_exception::OpenGlException;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod testing;
