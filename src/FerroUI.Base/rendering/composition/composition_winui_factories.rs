//! The factories of the WinUI visuals, brushes, surfaces, shadows and clips
//! (`Compositor.Create*` of `Microsoft.UI.Composition`).

use super::{
    CompositionSolidColorBrush, CompositionSurfaceBrush, CompositionVisualSurface, Compositor, DropShadow,
    ICompositionSurface, InsetClip, RectangleClip, SpriteVisual,
};
use crate::media::Color;
use crate::Vector;
use std::rc::Rc;

impl Compositor {
    /// `CreateSpriteVisual`: a visual that paints its size with a brush.
    pub fn create_sprite_visual(&self) -> SpriteVisual {
        SpriteVisual::new(&self.this_handle())
    }

    /// `CreateColorBrush`: the WinUI name of
    /// [`create_solid_color_brush`](Self::create_solid_color_brush).
    pub fn create_color_brush(&self) -> CompositionSolidColorBrush {
        self.create_solid_color_brush()
    }

    /// `CreateColorBrush(color)`: the WinUI name of
    /// [`create_solid_color_brush_with`](Self::create_solid_color_brush_with).
    pub fn create_color_brush_with_color(&self, color: Color) -> CompositionSolidColorBrush {
        self.create_solid_color_brush_with(color)
    }

    /// `CreateSurfaceBrush`: a brush that paints a surface, without a
    /// surface.
    pub fn create_surface_brush(&self) -> CompositionSurfaceBrush {
        CompositionSurfaceBrush::new(&self.this_handle())
    }

    /// `CreateSurfaceBrush(surface)`: a brush that paints `surface`.
    pub fn create_surface_brush_with_surface(&self, surface: Rc<dyn ICompositionSurface>) -> CompositionSurfaceBrush {
        CompositionSurfaceBrush::with_surface(&self.this_handle(), surface)
    }

    /// `CreateVisualSurface`: a live image of the subtree of a visual.
    pub fn create_visual_surface(&self) -> Rc<CompositionVisualSurface> {
        CompositionVisualSurface::new(&self.this_handle())
    }

    /// `CreateDropShadow`: a shadow for a sprite visual.
    pub fn create_drop_shadow(&self) -> Rc<DropShadow> {
        DropShadow::new(&self.this_handle())
    }

    /// `CreateInsetClip`: a clip with zero insets.
    pub fn create_inset_clip(&self) -> InsetClip {
        InsetClip::new(&self.this_handle())
    }

    /// `CreateInsetClip(leftInset, topInset, rightInset, bottomInset)`.
    pub fn create_inset_clip_with_insets(&self, left: f32, top: f32, right: f32, bottom: f32) -> InsetClip {
        InsetClip::with_insets(&self.this_handle(), left, top, right, bottom)
    }

    /// `CreateRectangleClip`: a clip whose edges are all zero (it leaves
    /// nothing visible until its edges are set).
    pub fn create_rectangle_clip(&self) -> RectangleClip {
        RectangleClip::new(&self.this_handle())
    }

    /// `CreateRectangleClip(left, top, right, bottom)`.
    pub fn create_rectangle_clip_with(&self, left: f32, top: f32, right: f32, bottom: f32) -> RectangleClip {
        RectangleClip::with_edges(&self.this_handle(), left, top, right, bottom)
    }

    /// `CreateRectangleClip(left, top, right, bottom, topLeftRadius,
    /// topRightRadius, bottomRightRadius, bottomLeftRadius)`.
    #[allow(clippy::too_many_arguments)]
    pub fn create_rectangle_clip_with_radii(
        &self,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        top_left_radius: Vector,
        top_right_radius: Vector,
        bottom_right_radius: Vector,
        bottom_left_radius: Vector,
    ) -> RectangleClip {
        RectangleClip::with_edges_and_radii(
            &self.this_handle(),
            left,
            top,
            right,
            bottom,
            top_left_radius,
            top_right_radius,
            bottom_right_radius,
            bottom_left_radius,
        )
    }
}
