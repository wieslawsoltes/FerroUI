//! Port of upstream's `CrossUI/CrossUI.` file of the framework (the one
//! named after the framework): the cross UI over the framework. A cross
//! control is a control that draws its source through a cross drawing
//! context, which converts the cross brushes, pens, geometries, drawings and
//! images to the ones of the framework on every call.

use super::cross_ui::*;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{
    ArcSegment, BezierSegment, BoxShadows, Brush, BrushMappingMode, Drawing, DrawingBrush, DrawingCollection,
    DrawingContext, DrawingGroup, DrawingImage, EllipseGeometry, Geometry, GeometryDrawing, GradientBrush,
    GradientStops, IBrush, IImage, IPen, ITransform, ImageBrush, LineSegment, LinearGradientBrush, MatrixTransform,
    PathFigure, PathFigures, PathGeometry, PathSegment, PathSegments, Pen, Points, PolyBezierSegment, PolyLineSegment,
    PushedState, QuadraticBezierSegment, RadialGradientBrush, RectangleGeometry, RenderOptions, SolidColorBrush,
    StreamGeometry, StreamGeometryContext, SweepDirection, TileBrush,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point, Rect, Ref, RelativePoint,
    RelativeRect, RelativeScalar, RelativeUnit, Size, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

impl CrossGlobals {
    pub fn get_context_impl_provider() -> Box<dyn ICrossStreamGeometryContextImplProvider> {
        Box::new(FerroCrossStreamGeometryContextImplProvider)
    }
}

pub struct FerroCrossStreamGeometryContextImplProvider;

impl ICrossStreamGeometryContextImplProvider for FerroCrossStreamGeometryContextImplProvider {
    fn create(&self) -> Rc<dyn ICrossStreamGeometryContextImpl> {
        Rc::new(FerroCrossStreamGeometryContextImpl::new())
    }
}

pub struct FerroCrossStreamGeometryContextImpl {
    stream_geometry: Ref<StreamGeometry>,
    context: RefCell<StreamGeometryContext>,
    is_closed: Cell<bool>,
}

impl FerroCrossStreamGeometryContextImpl {
    pub fn new() -> FerroCrossStreamGeometryContextImpl {
        let stream_geometry = StreamGeometry::new();
        let context = RefCell::new(stream_geometry.open());
        FerroCrossStreamGeometryContextImpl { stream_geometry, context, is_closed: Cell::new(false) }
    }
}

impl Default for FerroCrossStreamGeometryContextImpl {
    fn default() -> Self {
        FerroCrossStreamGeometryContextImpl::new()
    }
}

impl ICrossStreamGeometryContextImpl for FerroCrossStreamGeometryContextImpl {
    fn arc_to(
        &self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        self.context.borrow_mut().arc_to(point, size, rotation_angle, is_large_arc, sweep_direction, is_stroked);
    }

    fn begin_figure(&self, point: Point, is_filled: bool, is_closed: bool) {
        self.is_closed.set(is_closed);
        self.context.borrow_mut().begin_figure(point, is_filled);
    }

    fn cubic_bezier_to(&self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        self.context.borrow_mut().cubic_bezier_to(control_point1, control_point2, end_point, is_stroked);
    }

    fn dispose(&self) {
        self.context.borrow_mut().dispose();
    }

    fn end_figure(&self) {
        self.context.borrow_mut().end_figure(self.is_closed.get());
        self.dispose();
    }

    fn get_geometry(&self) -> Box<dyn Any> {
        Box::new(self.stream_geometry.clone())
    }

    fn line_to(&self, point: Point, is_stroked: bool) {
        self.context.borrow_mut().line_to(point, is_stroked);
    }

    fn quadratic_bezier_to(&self, control_point: Point, end_point: Point, is_stroked: bool) {
        self.context.borrow_mut().quadratic_bezier_to(control_point, end_point, is_stroked);
    }
}

#[repr(C)]
pub struct FerroCrossControl {
    base: Control,
    src: Rc<CrossControl>,
    // A dictionary upstream, from the cross control to its control.
    children: Vec<(Rc<CrossControl>, Ref<FerroCrossControl>)>,
}

ferro_class!(FerroCrossControl: Control);
ferro_impl_classes!(FerroCrossControl: FerroObjectImpl, StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroCrossControl {
    pub fn new(src: Rc<CrossControl>) -> Ref<Self> {
        let children = src.children.iter().map(|x| (x.clone(), FerroCrossControl::new(x.clone()))).collect();
        let this = instantiate(Self { base: Control::construct(), src: src.clone(), children });
        this.set_width(src.bounds().width);
        this.set_height(src.bounds().height);
        this.set_render_transform(Some(MatrixTransform::with_matrix(src.render_transform).into()));
        this.set_render_transform_origin(RelativePoint::from_point(Point::default(), RelativeUnit::Relative));
        // `RenderOptions = RenderOptions with { BitmapInterpolationMode = src.BitmapInterpolationMode }`.
        RenderOptions::set_bitmap_interpolation_mode(&this, src.bitmap_interpolation_mode);
        for (_, c) in &this.children {
            this.visual_children().add(c.clone().upcast());
            this.logical_children().add(c.clone().upcast());
        }
        this
    }
}

impl LayoutableImpl for FerroCrossControl {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        for (key, value) in &this.children {
            value.measure(key.bounds().size());
        }
        this.src.bounds().size()
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        for (key, value) in &this.children {
            value.arrange(key.bounds());
        }
        final_size
    }
}

impl VisualImpl for FerroCrossControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        this.src.render(&mut FerroCrossDrawingContext::new(context));
    }
}

pub struct FerroCrossDrawingContext<'a, 'b> {
    ctx: &'a mut DrawingContext<'b>,
    stack: Vec<PushedState>,
}

impl<'a, 'b> FerroCrossDrawingContext<'a, 'b> {
    pub fn new(ctx: &'a mut DrawingContext<'b>) -> Self {
        FerroCrossDrawingContext { ctx, stack: Vec::new() }
    }

    fn convert_transform(m: Option<Matrix>) -> Option<Rc<dyn ITransform>> {
        m.map(|m| MatrixTransform::with_matrix(m).into())
    }

    fn convert_unit(mode: BrushMappingMode) -> RelativeUnit {
        if mode == BrushMappingMode::RelativeToBoundingBox {
            RelativeUnit::Relative
        } else {
            RelativeUnit::Absolute
        }
    }

    fn convert_rect(rc: Rect, mode: BrushMappingMode) -> RelativeRect {
        RelativeRect::from_rect(rc, Self::convert_unit(mode))
    }

    fn convert_point(pt: Point, mode: BrushMappingMode) -> RelativePoint {
        RelativePoint::from_point(pt, Self::convert_unit(mode))
    }

    fn convert_scalar(scalar: f64, mode: BrushMappingMode) -> RelativeScalar {
        RelativeScalar::new(scalar, Self::convert_unit(mode))
    }

    fn convert_geometry(g: &CrossGeometry) -> Ref<Geometry> {
        match g {
            CrossGeometry::Rectangle(rg) => RectangleGeometry::with_rect(rg.rect).upcast(),
            CrossGeometry::Svg(svg) => PathGeometry::parse(&svg.path).expect("the path is parsed").upcast(),
            CrossGeometry::Ellipse(ellipse) => EllipseGeometry::with_rect(ellipse.rect).upcast(),
            CrossGeometry::Stream(stream_geometry) => stream_geometry
                .get_context()
                .get_geometry()
                .downcast::<Ref<StreamGeometry>>()
                .expect("the geometry is a stream geometry")
                .upcast(),
            CrossGeometry::Path(path) => {
                let geometry = PathGeometry::new();
                geometry.set_figures(Some(Self::ret_add_range_figures(
                    PathFigures::new(),
                    path.figures.iter().map(|f| {
                        let figure = PathFigure::new();
                        figure.set_start_point(f.start);
                        figure.set_is_closed(f.closed);
                        figure.set_segments(Some(Self::ret_add_range_segments(
                            PathSegments::new(),
                            f.segments.iter().map(|s| -> Ref<PathSegment> {
                                match s {
                                    CrossPathSegment::Line { to, is_stroked } => {
                                        let segment = LineSegment::new();
                                        segment.set_point(*to);
                                        segment.set_is_stroked(*is_stroked);
                                        segment.upcast()
                                    }
                                    CrossPathSegment::Arc {
                                        point,
                                        size,
                                        rotation_angle,
                                        is_large_arc,
                                        sweep_direction,
                                        is_stroked,
                                    } => {
                                        let segment = ArcSegment::new();
                                        segment.set_point(*point);
                                        segment.set_rotation_angle(*rotation_angle);
                                        segment.set_size(*size);
                                        segment.set_is_large_arc(*is_large_arc);
                                        segment.set_sweep_direction(*sweep_direction);
                                        segment.set_is_stroked(*is_stroked);
                                        segment.upcast()
                                    }
                                    CrossPathSegment::CubicBezier { point1, point2, point3, is_stroked } => {
                                        let segment = BezierSegment::new();
                                        segment.set_point1(*point1);
                                        segment.set_point2(*point2);
                                        segment.set_point3(*point3);
                                        segment.set_is_stroked(*is_stroked);
                                        segment.upcast()
                                    }
                                    CrossPathSegment::QuadraticBezier { point1, point2, is_stroked } => {
                                        let segment = QuadraticBezierSegment::new();
                                        segment.set_point1(*point1);
                                        segment.set_point2(*point2);
                                        segment.set_is_stroked(*is_stroked);
                                        segment.upcast()
                                    }
                                    CrossPathSegment::PolyLine { points, is_stroked } => {
                                        let segment = PolyLineSegment::new();
                                        segment.set_points(Points::from_items(points.iter().copied()));
                                        segment.set_is_stroked(*is_stroked);
                                        segment.upcast()
                                    }
                                    CrossPathSegment::PolyBezierSegment { points, is_stroked } => {
                                        PolyBezierSegment::with_points(points.iter().copied(), *is_stroked).upcast()
                                    }
                                }
                            }),
                        )));
                        figure
                    }),
                )));
                geometry.upcast()
            }
        }
    }

    // `RetAddRange` is generic over the list upstream.
    fn ret_add_range_figures(l: PathFigures, en: impl Iterator<Item = Ref<PathFigure>>) -> PathFigures {
        for e in en {
            l.add(e);
        }
        l
    }

    fn ret_add_range_segments(l: PathSegments, en: impl Iterator<Item = Ref<PathSegment>>) -> PathSegments {
        for e in en {
            l.add(e);
        }
        l
    }

    fn convert_drawing(src: &CrossDrawing) -> Ref<Drawing> {
        match src {
            CrossDrawing::Group(g) => {
                let group = DrawingGroup::new();
                group.set_children(DrawingCollection::from_items(g.children.iter().map(Self::convert_drawing)));
                group.upcast()
            }
            CrossDrawing::Geometry(geo) => {
                let drawing = GeometryDrawing::new();
                drawing.set_geometry(Self::convert_geometry(&geo.geometry));
                drawing.set_brush(Self::convert_brush(geo.brush.as_ref()));
                drawing.set_pen(Self::convert_pen(geo.pen.as_ref()));
                drawing.upcast()
            }
        }
    }

    fn convert_brush(brush: Option<&CrossBrush>) -> Option<Rc<dyn IBrush>> {
        let brush = brush?;

        fn sync(dst: &Brush, src: &CrossBrushBase) {
            dst.set_opacity(src.opacity);
            dst.set_transform(FerroCrossDrawingContext::convert_transform(src.transform));
            dst.set_transform_origin(RelativePoint::from_point(Point::default(), RelativeUnit::Absolute));
            dst.set_relative_transform(FerroCrossDrawingContext::convert_transform(src.relative_transform));
        }

        fn sync_tile(dst: &TileBrush, src: &CrossTileBrush) {
            dst.set_stretch(src.stretch);
            dst.set_alignment_x(src.alignment_x);
            dst.set_alignment_y(src.alignment_y);
            dst.set_tile_mode(src.tile_mode);
            dst.set_source_rect(FerroCrossDrawingContext::convert_rect(src.viewbox, src.viewbox_units));
            dst.set_destination_rect(FerroCrossDrawingContext::convert_rect(src.viewport, src.viewport_units));
            sync(dst, src);
        }

        fn sync_gradient(dst: &GradientBrush, src: &CrossGradientBrush) {
            dst.set_gradient_stops(GradientStops::new());
            dst.gradient_stops().add_range(src.gradient_stops.iter().cloned());
            dst.set_spread_method(src.spread_method);
            sync(dst, src);
        }

        Some(match brush {
            CrossBrush::SolidColor(br) => {
                let dst = SolidColorBrush::with_color(br.color);
                sync(&dst, br);
                dst.into()
            }
            CrossBrush::Drawing(db) => {
                let dst = DrawingBrush::with_drawing(Self::convert_drawing(&db.drawing));
                sync_tile(&dst, db);
                dst.into()
            }
            CrossBrush::Image(ib) => {
                let dst = ImageBrush::with_source(Some(Rc::new(
                    Bitmap::from_file(&ib.path).expect("the bitmap is loaded"),
                )));
                sync_tile(&dst, ib);
                dst.into()
            }
            CrossBrush::LinearGradient(linear) => {
                let dst = LinearGradientBrush::new();
                dst.set_start_point(Self::convert_point(linear.start_point, linear.mapping_mode));
                dst.set_end_point(Self::convert_point(linear.end_point, linear.mapping_mode));
                sync_gradient(&dst, linear);
                dst.into()
            }
            CrossBrush::RadialGradient(radial) => {
                let dst = RadialGradientBrush::new();
                dst.set_center(Self::convert_point(radial.center, radial.mapping_mode));
                dst.set_gradient_origin(Self::convert_point(radial.gradient_origin, radial.mapping_mode));
                dst.set_radius_x(Self::convert_scalar(radial.radius_x, radial.mapping_mode));
                dst.set_radius_y(Self::convert_scalar(radial.radius_y, radial.mapping_mode));
                sync_gradient(&dst, radial);
                dst.into()
            }
        })
    }

    fn convert_pen(pen: Option<&CrossPen>) -> Option<Rc<dyn IPen>> {
        let pen = pen?;
        let result = Pen::with_brush(Self::convert_brush(Some(&pen.brush)), pen.thickness);
        result.set_line_cap(pen.line_cap);
        result.set_line_join(pen.line_join);
        Some(result.into())
    }

    fn convert_image(image: &CrossImage) -> Rc<dyn IImage> {
        match image {
            CrossImage::Bitmap(bi) => Rc::new(Bitmap::from_file(&bi.path).expect("the bitmap is loaded")),
            CrossImage::Drawing(di) => DrawingImage::with_drawing(Self::convert_drawing(&di.drawing)).into(),
        }
    }
}

impl ICrossDrawingContext for FerroCrossDrawingContext<'_, '_> {
    fn push_transform(&mut self, matrix: Matrix) {
        let state = self.ctx.push_transform(matrix);
        self.stack.push(state);
    }

    fn pop(&mut self) {
        let state = self.stack.pop().expect("a state was pushed");

        self.ctx.pop(state);
    }

    fn draw_line(&mut self, pen: &CrossPen, p1: Point, p2: Point) {
        let ferro_pen = Self::convert_pen(Some(pen));

        let Some(ferro_pen) = ferro_pen else {
            return;
        };

        self.ctx.draw_line(&ferro_pen, p1, p2);
    }

    fn draw_rectangle(&mut self, brush: Option<&CrossBrush>, pen: Option<&CrossPen>, rc: Rect) {
        self.ctx.draw_rectangle(
            Self::convert_brush(brush).as_ref(),
            Self::convert_pen(pen).as_ref(),
            rc,
            0.0,
            0.0,
            &BoxShadows::default(),
        );
    }

    fn draw_geometry(&mut self, brush: Option<&CrossBrush>, pen: Option<&CrossPen>, geometry: &CrossGeometry) {
        self.ctx.draw_geometry(
            Self::convert_brush(brush).as_ref(),
            Self::convert_pen(pen).as_ref(),
            &Self::convert_geometry(geometry),
        );
    }

    fn draw_image(&mut self, image: &CrossImage, rc: Rect) {
        self.ctx.draw_image(&*Self::convert_image(image), rc);
    }
}
