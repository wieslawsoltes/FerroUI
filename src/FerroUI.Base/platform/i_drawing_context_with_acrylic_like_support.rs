use crate::media::IExperimentalAcrylicMaterial;
use crate::RoundedRect;

/// A drawing context that can draw acrylic-like materials.
pub trait IDrawingContextWithAcrylicLikeSupport {
    /// Draws a rectangle filled with an acrylic-like material.
    fn draw_rectangle_with_material(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect);
}
