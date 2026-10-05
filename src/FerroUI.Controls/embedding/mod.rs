//! Hosting a control tree inside a foreign window or surface.

mod embeddable_control_root;

pub use embeddable_control_root::{
    EmbeddableControlRoot, EmbeddableControlRootImpl, EmbeddableControlRootImplExt, EmbeddableControlRootVTable,
};

pub mod offscreen;
