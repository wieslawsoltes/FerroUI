//! Server-side counterpart of `CompositionContainerVisual`.
//!
//! Every server visual has a children collection, so the container class
//! adds nothing to [`ServerCompositionVisual`]; its content type is declared
//! next to it.

pub use super::server_composition_visual::ServerCompositionContainerVisual;
