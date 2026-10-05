//! The composition brushes and their server-side counterparts.

mod composition_brush;
mod server_composition_brush;
mod server_simple_content_brush;
mod server_simple_image_brush;

pub use composition_brush::{
    CompositionBrush, CompositionConicGradientBrush, CompositionGradientBrush, CompositionLinearGradientBrush,
    CompositionRadialGradientBrush, CompositionSolidColorBrush,
};
pub use server_composition_brush::{
    ServerCompositionConicGradientBrush, ServerCompositionGradientBrush, ServerCompositionLinearGradientBrush,
    ServerCompositionRadialGradientBrush, ServerCompositionSolidColorBrush,
};

pub use server_simple_content_brush::ServerCompositionSimpleContentBrush;
pub use server_simple_image_brush::ServerCompositionSimpleImageBrush;

#[cfg(test)]
mod composition_brush_tests;
