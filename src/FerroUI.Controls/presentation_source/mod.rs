//! The presentation source: what connects the root visual of a top-level to
//! its platform implementation, renderer, layout manager and input.

mod presentation_source;
mod presentation_source_cursor;
mod presentation_source_input;
mod presentation_source_layout;
mod presentation_source_render_root;
mod renderer_factory;

pub use presentation_source::PresentationSource;
pub(crate) use presentation_source::{add_scaling_changed, try_get_service};
pub use renderer_factory::{create_renderer, IRendererFactory, ITopLevelRenderer, RenderSurfaces};

#[cfg(test)]
mod presentation_source_tests;
