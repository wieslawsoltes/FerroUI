//! Composition brushes that the compositor paints itself (effect, backdrop,
//! surface, nine-grid and mask brushes, and the XAML brushes over them) as
//! the fill or the stroke of what render data draws: an element whose
//! background is such a brush (winui-materials.md).

use crate::media::immutable::{ImmutableDashStyle, ImmutablePen, ImmutableSolidColorBrush};
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{BoxShadows, Colors, IBrush, IImmutableBrush, IPen, ITransform};
use crate::platform::{IDrawingContextImpl, IGeometryImpl, IGlyphRunImpl};
use crate::rendering::composition::brushes::paint_composition_brush;
use crate::rendering::composition::server::IServerObject;
use crate::{Point, Rect, RelativePoint, RoundedRect};
use std::any::Any;
use std::rc::Rc;

/// The brush of the render thread that stands for a composition brush the
/// drawing contexts cannot draw: what `as_brush` of such a server brush
/// returns. The replay of render data fills and strokes with it by painting
/// the brush ([`paint_composition_brush`]) inside the shape; a drawing
/// context that is handed one directly draws nothing with it.
pub struct CompositionPaintedBrush {
    object: Rc<dyn IServerObject>,
}

/// What a painted brush fills or strokes.
#[derive(Clone, Copy)]
pub(crate) enum PaintedShape<'a> {
    Rectangle(RoundedRect),
    Ellipse(Rect),
    Geometry(&'a dyn IGeometryImpl),
    Line(Point, Point),
    GlyphRun(&'a dyn IGlyphRunImpl),
}

impl PaintedShape<'_> {
    /// The area the shape covers when it is filled.
    fn bounds(&self) -> Rect {
        match *self {
            PaintedShape::Rectangle(rect) => rect.rect,
            PaintedShape::Ellipse(rect) => rect,
            PaintedShape::Geometry(geometry) => geometry.bounds(),
            PaintedShape::Line(p1, p2) => Rect::from_points(Point::new(p1.x.min(p2.x), p1.y.min(p2.y)), Point::new(p1.x.max(p2.x), p1.y.max(p2.y))),
            PaintedShape::GlyphRun(glyph_run) => glyph_run.bounds(),
        }
    }

    /// Draws the shape with `brush` and `pen`.
    fn draw(&self, canvas: &mut dyn IDrawingContextImpl, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>) {
        match *self {
            PaintedShape::Rectangle(rect) => canvas.draw_rectangle(brush, pen, rect, &BoxShadows::default()),
            PaintedShape::Ellipse(rect) => canvas.draw_ellipse(brush, pen, rect),
            PaintedShape::Geometry(geometry) => canvas.draw_geometry(brush, pen, geometry),
            PaintedShape::Line(p1, p2) => canvas.draw_line(pen, p1, p2),
            PaintedShape::GlyphRun(glyph_run) => {
                if let Some(brush) = brush {
                    canvas.draw_glyph_run(Some(brush), glyph_run);
                }
            }
        }
    }
}

/// An opaque pen of the form of `pen`: the coverage of its stroke.
fn coverage_pen(pen: &dyn IPen) -> ImmutablePen {
    let white: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::WHITE));
    let dash_style = pen
        .dash_style()
        .map(|dash_style| Rc::new(ImmutableDashStyle::new(dash_style.dashes().as_deref(), dash_style.offset())));
    ImmutablePen::new(Some(white), pen.thickness(), dash_style, pen.line_cap(), pen.line_join(), pen.miter_limit())
}

impl CompositionPaintedBrush {
    /// The brush for the server-side composition brush `object`.
    pub fn new(object: Rc<dyn IServerObject>) -> CompositionPaintedBrush {
        CompositionPaintedBrush { object }
    }

    /// The server-side brush.
    pub fn object(&self) -> &Rc<dyn IServerObject> {
        &self.object
    }

    /// The painted brush `brush` is, if it is one.
    pub fn of(brush: Option<&dyn IBrush>) -> Option<&CompositionPaintedBrush> {
        brush?.as_any().downcast_ref::<CompositionPaintedBrush>()
    }

    /// Paints the brush over `rect`.
    pub fn paint(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        paint_composition_brush(canvas, &self.object, rect);
    }

    /// Fills `shape` with the brush: inside a clip of the shape for a
    /// rectangle, an ellipse and a geometry; inside the coverage of the
    /// shape (a destination-in layer) for a glyph run.
    pub(crate) fn fill(&self, canvas: &mut dyn IDrawingContextImpl, shape: PaintedShape<'_>) {
        match shape {
            PaintedShape::Rectangle(rect) => {
                canvas.push_clip_rounded(rect);
                self.paint(canvas, rect.rect);
                canvas.pop_clip();
            }
            PaintedShape::Ellipse(rect) => {
                canvas.push_clip_rounded(RoundedRect::from_radius_xy(rect, rect.width / 2.0, rect.height / 2.0));
                self.paint(canvas, rect);
                canvas.pop_clip();
            }
            PaintedShape::Geometry(geometry) => {
                canvas.push_geometry_clip(geometry);
                self.paint(canvas, geometry.bounds());
                canvas.pop_geometry_clip();
            }
            PaintedShape::Line(..) => {}
            PaintedShape::GlyphRun(_) => self.paint_through_coverage(canvas, shape, shape.bounds(), None),
        }
    }

    /// Strokes `shape` with `pen`, whose brush is this one: the brush is
    /// painted over the bounds of the stroke inside the coverage of the
    /// stroke.
    pub(crate) fn stroke(&self, canvas: &mut dyn IDrawingContextImpl, pen: &dyn IPen, shape: PaintedShape<'_>) {
        let reach = pen.thickness().max(0.0) + pen.miter_limit().max(1.0) * pen.thickness().max(0.0) / 2.0;
        let bounds = shape.bounds().inflate(reach);
        let coverage = coverage_pen(pen);
        self.paint_through_coverage(canvas, shape, bounds, Some(&coverage));
    }

    /// Paints the brush over `bounds` in a layer, then keeps what the shape
    /// covers (drawn with an opaque brush, or with `pen`) with
    /// destination-in. Without the layers of the drawing context nothing is
    /// drawn.
    fn paint_through_coverage(
        &self,
        canvas: &mut dyn IDrawingContextImpl,
        shape: PaintedShape<'_>,
        bounds: Rect,
        pen: Option<&ImmutablePen>,
    ) {
        if canvas.as_drawing_context_impl_with_layer_effects().is_none() || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }
        canvas.push_layer(bounds);
        self.paint(canvas, bounds);
        if let Some(layers) = canvas.as_drawing_context_impl_with_layer_effects() {
            layers.push_blend_layer(bounds, BitmapBlendingMode::DestinationIn);
        }
        let white = ImmutableSolidColorBrush::new(Colors::WHITE);
        match pen {
            Some(pen) => shape.draw(canvas, None, Some(pen)),
            None => shape.draw(canvas, Some(&white), None),
        }
        if let Some(layers) = canvas.as_drawing_context_impl_with_layer_effects() {
            layers.pop_blend_layer();
        }
        canvas.pop_layer();
    }
}

impl IBrush for CompositionPaintedBrush {
    fn opacity(&self) -> f64 {
        1.0
    }

    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }

    fn transform_origin(&self) -> RelativePoint {
        RelativePoint::default()
    }

    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Draws what render data draws with a brush and a pen, painting the brush
/// or the brush of the pen when it is a [`CompositionPaintedBrush`].
pub(crate) fn draw_with_painted_brushes(
    canvas: &mut dyn IDrawingContextImpl,
    brush: Option<&dyn IBrush>,
    pen: Option<&dyn IPen>,
    shape: PaintedShape<'_>,
    box_shadows: Option<&BoxShadows>,
) -> bool {
    let painted_fill = CompositionPaintedBrush::of(brush);
    let pen_brush = pen.and_then(|pen| pen.brush());
    let painted_stroke = CompositionPaintedBrush::of(pen_brush.as_deref());
    if painted_fill.is_none() && painted_stroke.is_none() {
        return false;
    }
    // The shadows first, under the fill.
    if let (Some(box_shadows), PaintedShape::Rectangle(rect)) = (box_shadows, shape) {
        if box_shadows.count() > 0 {
            canvas.draw_rectangle(None, None, rect, box_shadows);
        }
    }
    match painted_fill {
        Some(painted) => painted.fill(canvas, shape),
        None => {
            if brush.is_some() {
                shape.draw(canvas, brush, None);
            }
        }
    }
    if let Some(pen) = pen {
        match painted_stroke {
            Some(painted) => painted.stroke(canvas, pen, shape),
            None => shape.draw(canvas, None, Some(pen)),
        }
    }
    true
}

/// The painted brush of a server-side composition brush that the drawing
/// contexts cannot draw (effect, backdrop, host backdrop, surface,
/// nine-grid, mask and XAML brushes); `None` for any other object.
pub(crate) fn painted_brush_of(object: &Rc<dyn IServerObject>) -> Option<Rc<dyn IBrush>> {
    is_painted_composition_brush(&**object).then(|| Rc::new(CompositionPaintedBrush::new(object.clone())) as Rc<dyn IBrush>)
}

/// Whether `object` is a composition brush that [`paint_composition_brush`]
/// paints itself rather than handing it to the drawing context.
pub(crate) fn is_painted_composition_brush(object: &dyn IServerObject) -> bool {
    use crate::rendering::composition::brushes::{
        ServerCompositionBackdropBrush, ServerCompositionEffectBrush, ServerCompositionMaskBrush,
        ServerCompositionNineGridBrush, ServerCompositionSurfaceBrush,
    };
    let any = object.as_any();
    any.is::<ServerCompositionSurfaceBrush>()
        || any.is::<ServerCompositionNineGridBrush>()
        || any.is::<ServerCompositionMaskBrush>()
        || any.is::<ServerCompositionEffectBrush>()
        || any.is::<ServerCompositionBackdropBrush>()
        || any.is::<crate::rendering::composition::brushes::ServerCompositionHostBackdropBrush>()
        || any.is::<crate::rendering::composition::brushes::ServerXamlCompositionBrush>()
}
