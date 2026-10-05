use super::{
    IBitmapImpl, IDrawingContextImplWithEffects, IDrawingContextWithAcrylicLikeSupport, IGeometryImpl, IGlyphRunImpl,
    IPlatformRenderInterfaceRegion,
};
use crate::media::{BoxShadows, Color, IBrush, IPen, RenderOptions};
use crate::{Matrix, PixelSize, Point, Rect, RoundedRect};
use std::any::{Any, TypeId};
use std::rc::Rc;

/// Defines the interface through which drawing occurs: the backend end of a
/// drawing context.
pub trait IDrawingContextImpl {
    /// The current transform of the drawing context.
    fn transform(&self) -> Matrix;

    /// Sets the current transform of the drawing context.
    fn set_transform(&mut self, value: Matrix);

    /// Clears the render target to the specified color.
    fn clear(&mut self, color: Color);

    /// Draws a bitmap image.
    fn draw_bitmap(&mut self, source: &dyn IBitmapImpl, opacity: f64, source_rect: Rect, dest_rect: Rect);

    /// Draws a bitmap image through an opacity mask.
    fn draw_bitmap_with_mask(
        &mut self,
        source: &dyn IBitmapImpl,
        opacity_mask: &dyn IBrush,
        opacity_mask_rect: Rect,
        dest_rect: Rect,
    );

    /// Draws a line.
    fn draw_line(&mut self, pen: Option<&dyn IPen>, p1: Point, p2: Point);

    /// Draws a geometry.
    fn draw_geometry(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, geometry: &dyn IGeometryImpl);

    /// Draws a rectangle with the specified brush and pen.
    ///
    /// The brush and the pen can both be `None`. If the brush is `None`, no
    /// fill is performed. If the pen is `None`, no stroke is performed. If
    /// both are `None`, the call is a no-op.
    fn draw_rectangle(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    );

    /// Draws a region with the specified brush and pen.
    fn draw_region(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        region: &dyn IPlatformRenderInterfaceRegion,
    );

    /// Draws an ellipse with the specified brush and pen.
    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: Rect);

    /// Draws a glyph run.
    fn draw_glyph_run(&mut self, foreground: Option<&dyn IBrush>, glyph_run: &dyn IGlyphRunImpl);

    /// Creates a new layer: an offscreen render target compatible with this
    /// drawing context that can be blitted back onto it.
    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl>;

    /// Pushes a clip rectangle.
    fn push_clip(&mut self, clip: Rect);

    /// Pushes a rounded clip rectangle.
    fn push_clip_rounded(&mut self, clip: RoundedRect);

    /// Pushes a clip region.
    fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion);

    /// Pops the latest pushed clip rectangle or region.
    fn pop_clip(&mut self);

    /// Enforces rendering to happen on an intermediate surface.
    fn push_layer(&mut self, bounds: Rect);

    /// Pops the latest pushed intermediate surface layer.
    fn pop_layer(&mut self);

    /// Pushes an opacity value.
    fn push_opacity(&mut self, opacity: f64, bounds: Option<Rect>);

    /// Pops the latest pushed opacity value.
    fn pop_opacity(&mut self);

    /// Pushes an opacity mask.
    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect);

    /// Pops the latest pushed opacity mask.
    fn pop_opacity_mask(&mut self);

    /// Pushes a clip geometry.
    fn push_geometry_clip(&mut self, clip: &dyn IGeometryImpl);

    /// Pops the latest pushed geometry clip.
    fn pop_geometry_clip(&mut self);

    /// Pushes render options.
    fn push_render_options(&mut self, render_options: RenderOptions);

    /// Pops the latest pushed render options.
    fn pop_render_options(&mut self);

    /// Pushes text options for the drawing context.
    fn push_text_options(&mut self, text_options: crate::media::TextOptions);

    /// Pops the latest text options.
    fn pop_text_options(&mut self);

    /// Attempts to get an optional feature from the drawing context
    /// implementation; see
    /// [`IOptionalFeatureProvider`](super::IOptionalFeatureProvider).
    fn get_feature(&mut self, feature_type: TypeId) -> Option<Rc<dyn Any>>;

    /// The context viewed as [`IDrawingContextImplWithEffects`], when the
    /// backend can apply bitmap effects. This replaces the interface cast
    /// (`impl as IDrawingContextImplWithEffects`): a backend that implements
    /// the trait overrides this to return itself.
    fn as_drawing_context_impl_with_effects(&mut self) -> Option<&mut dyn IDrawingContextImplWithEffects> {
        None
    }

    /// The context viewed as [`IDrawingContextWithAcrylicLikeSupport`], when
    /// the backend can draw acrylic-like materials. This replaces the
    /// interface cast: a backend that implements the trait overrides this to
    /// return itself.
    fn as_drawing_context_with_acrylic_like_support(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextWithAcrylicLikeSupport> {
        None
    }

    /// Lets the backend recover its concrete type.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Finishes drawing and releases the context (presenting the frame where
    /// the context was created by a render target).
    fn dispose(&mut self);
}

/// An offscreen surface that can be drawn into and blitted onto a compatible
/// drawing context.
pub trait IDrawingContextLayerImpl: IBitmapImpl {
    /// Does optimized blit with source OVER overlay mode.
    fn blit(&self, context: &mut dyn IDrawingContextImpl);

    /// Whether the layer can be blitted with [`blit`](Self::blit).
    fn can_blit(&self) -> bool;

    /// Whether the layer's contents have been lost.
    fn is_corrupted(&self) -> bool;

    /// Creates a drawing context that draws into the layer.
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl>;

    /// The layer as one that may be bound to the render context it was
    /// created with, if it is one.
    fn as_layer_with_render_context_affinity(&self) -> Option<&dyn IDrawingContextLayerWithRenderContextAffinityImpl> {
        None
    }
}

/// A layer that may only be usable with the render context it was created
/// with.
pub trait IDrawingContextLayerWithRenderContextAffinityImpl: IDrawingContextLayerImpl {
    /// Whether the layer is bound to its render context.
    fn has_render_context_affinity(&self) -> bool;

    /// Creates a snapshot of the layer that is usable with any context.
    fn create_non_affined_snapshot(&self) -> Rc<dyn IBitmapImpl>;
}
