//! The composition brushes and their server-side counterparts.

mod composition_brush;
mod composition_effect_brush;
mod composition_gradient_winui;
mod composition_nine_grid_brush;
mod server_composition_brush;
mod server_composition_effect_brush;
mod server_composition_nine_grid_brush;
mod server_composition_surface_brush;
mod server_simple_content_brush;
mod server_simple_image_brush;

pub use composition_brush::{
    CompositionBrush, CompositionConicGradientBrush, CompositionGradientBrush, CompositionLinearGradientBrush,
    CompositionRadialGradientBrush, CompositionSolidColorBrush, CompositionSurfaceBrush,
};
pub use server_composition_brush::{
    ServerCompositionConicGradientBrush, ServerCompositionGradientBrush, ServerCompositionLinearGradientBrush,
    ServerCompositionRadialGradientBrush, ServerCompositionSolidColorBrush,
};

pub use server_composition_surface_brush::{
    paint_composition_brush, surface_brush_content_rect, ServerCompositionSurfaceBrush,
};

pub use composition_gradient_winui::IServerBrushWinUiTransform;
pub use composition_nine_grid_brush::{CompositionMaskBrush, CompositionNineGridBrush};
pub use server_composition_nine_grid_brush::{
    nine_grid_rects, NineGridEdges, ServerCompositionMaskBrush, ServerCompositionNineGridBrush, NINE_GRID_CENTER,
};

pub use composition_effect_brush::{CompositionBackdropBrush, CompositionEffectBrush};
pub use server_composition_effect_brush::{ServerCompositionBackdropBrush, ServerCompositionEffectBrush};

pub use server_simple_content_brush::ServerCompositionSimpleContentBrush;
pub use server_simple_image_brush::ServerCompositionSimpleImageBrush;

#[cfg(test)]
mod composition_brush_tests;
