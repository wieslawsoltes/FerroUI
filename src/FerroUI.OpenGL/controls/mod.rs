//! Drawing with OpenGL from a control.

mod composition_open_gl_swapchain;
mod open_gl_control_base;
mod open_gl_control_resources;

pub use open_gl_control_base::{
    OpenGlControlBase, OpenGlControlBaseImpl, OpenGlControlBaseImplExt, OpenGlControlBaseVTable,
};
