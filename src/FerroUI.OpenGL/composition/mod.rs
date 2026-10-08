//! Drawing with OpenGL into the surfaces of the compositor.

mod composition_gl_context;
mod composition_gl_context_options;
mod composition_gl_texture;
mod i_composition_gl_context;
mod i_composition_gl_texture;
mod open_gl_composition_interop;
pub(crate) mod task_support;

pub use composition_gl_context_options::CompositionGlContextOptions;
pub use i_composition_gl_context::ICompositionGlContext;
pub use i_composition_gl_texture::{CompositionGlTextureInfo, ICompositionGlTexture, ICompositionGlTextureLease};
pub use open_gl_composition_interop::OpenGlCompositionInterop;
