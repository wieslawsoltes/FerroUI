//! Port of upstream's `CrossUI/CrossUI.cs`.
//!
//! Upstream's classes form small hierarchies that the implementation tells
//! apart by type. Here the base of a hierarchy that is only ever one of a
//! fixed set of classes (`CrossBrush`, `CrossDrawing`, `CrossGeometry`,
//! `CrossImage`, `CrossPathSegment`) is an enum of those classes, and a class
//! with a base class that has fields dereferences to it (`CrossBrushBase` are
//! the fields of upstream's `CrossBrush`). The values are copied where
//! upstream shares an instance: nothing reads them back, the implementation
//! converts them anew on every use, as it does upstream. A stream geometry,
//! which holds the geometry it builds, is shared.
//!
//! `CrossControl` has two derived classes upstream, which override `Render`;
//! here the control carries what its class draws after the base
//! (`CrossFuncControl::new`, `CrossImageControl::new`).

use ferroui_base::media::imaging::BitmapInterpolationMode;
use ferroui_base::media::{
    AlignmentX, AlignmentY, BrushMappingMode, Color, Colors, GradientSpreadMethod, GradientStop, PenLineCap,
    PenLineJoin, Stretch, SweepDirection, TileMode,
};
use ferroui_base::{Matrix, Point, Rect, Ref, Size};
use std::any::Any;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

pub struct CrossGlobals;

/// The fields of upstream's `CrossBrush`.
#[derive(Clone)]
pub struct CrossBrushBase {
    pub opacity: f64,
    pub transform: Option<Matrix>,
    pub relative_transform: Option<Matrix>,
}

impl Default for CrossBrushBase {
    fn default() -> Self {
        CrossBrushBase { opacity: 1.0, transform: None, relative_transform: None }
    }
}

/// A brush of any class.
#[derive(Clone)]
pub enum CrossBrush {
    SolidColor(CrossSolidColorBrush),
    LinearGradient(CrossLinearGradientBrush),
    RadialGradient(CrossRadialGradientBrush),
    Drawing(CrossDrawingBrush),
    Image(CrossImageBrush),
}

macro_rules! derives {
    ($class:ident: $base:ident) => {
        impl Deref for $class {
            type Target = $base;

            fn deref(&self) -> &$base {
                &self.base
            }
        }

        impl DerefMut for $class {
            fn deref_mut(&mut self) -> &mut $base {
                &mut self.base
            }
        }
    };
}

macro_rules! brush_class {
    ($class:ident, $variant:ident) => {
        impl From<$class> for CrossBrush {
            fn from(value: $class) -> CrossBrush {
                CrossBrush::$variant(value)
            }
        }

        impl From<$class> for Option<CrossBrush> {
            fn from(value: $class) -> Option<CrossBrush> {
                Some(CrossBrush::$variant(value))
            }
        }
    };
}

#[derive(Clone)]
pub struct CrossSolidColorBrush {
    base: CrossBrushBase,
    pub color: Color,
}

derives!(CrossSolidColorBrush: CrossBrushBase);
brush_class!(CrossSolidColorBrush, SolidColor);

impl Default for CrossSolidColorBrush {
    fn default() -> Self {
        CrossSolidColorBrush { base: CrossBrushBase::default(), color: Colors::BLACK }
    }
}

impl CrossSolidColorBrush {
    pub fn new(color: Color) -> CrossSolidColorBrush {
        CrossSolidColorBrush { base: CrossBrushBase::default(), color }
    }
}

#[derive(Clone)]
pub struct CrossGradientBrush {
    base: CrossBrushBase,
    pub gradient_stops: Vec<Ref<GradientStop>>,
    pub spread_method: GradientSpreadMethod,
    pub mapping_mode: BrushMappingMode,
}

derives!(CrossGradientBrush: CrossBrushBase);

impl Default for CrossGradientBrush {
    fn default() -> Self {
        CrossGradientBrush {
            base: CrossBrushBase::default(),
            gradient_stops: Vec::new(),
            // The fields have no initializer upstream: the first member of each enumeration.
            spread_method: GradientSpreadMethod::Pad,
            mapping_mode: BrushMappingMode::Absolute,
        }
    }
}

#[derive(Clone)]
pub struct CrossLinearGradientBrush {
    base: CrossGradientBrush,
    pub start_point: Point,
    pub end_point: Point,
}

derives!(CrossLinearGradientBrush: CrossGradientBrush);
brush_class!(CrossLinearGradientBrush, LinearGradient);

impl Default for CrossLinearGradientBrush {
    fn default() -> Self {
        CrossLinearGradientBrush {
            base: CrossGradientBrush::default(),
            start_point: Point::new(0.0, 0.0),
            end_point: Point::new(1.0, 1.0),
        }
    }
}

#[derive(Clone, Default)]
pub struct CrossRadialGradientBrush {
    base: CrossGradientBrush,
    pub center: Point,
    pub gradient_origin: Point,
    pub radius_x: f64,
    pub radius_y: f64,
}

derives!(CrossRadialGradientBrush: CrossGradientBrush);
brush_class!(CrossRadialGradientBrush, RadialGradient);

#[derive(Clone)]
pub struct CrossTileBrush {
    base: CrossBrushBase,
    pub alignment_x: AlignmentX,
    pub alignment_y: AlignmentY,
    pub stretch: Stretch,
    pub tile_mode: TileMode,
    pub viewbox: Rect,
    pub viewbox_units: BrushMappingMode,
    pub viewport: Rect,
    pub viewport_units: BrushMappingMode,
}

derives!(CrossTileBrush: CrossBrushBase);

impl Default for CrossTileBrush {
    fn default() -> Self {
        CrossTileBrush {
            base: CrossBrushBase::default(),
            alignment_x: AlignmentX::Center,
            alignment_y: AlignmentY::Center,
            stretch: Stretch::Fill,
            tile_mode: TileMode::None,
            viewbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            viewbox_units: BrushMappingMode::RelativeToBoundingBox,
            viewport: Rect::new(0.0, 0.0, 1.0, 1.0),
            viewport_units: BrushMappingMode::RelativeToBoundingBox,
        }
    }
}

/// A drawing of any class.
#[derive(Clone)]
pub enum CrossDrawing {
    Geometry(CrossGeometryDrawing),
    Group(CrossDrawingGroup),
}

#[derive(Clone)]
pub struct CrossGeometryDrawing {
    pub geometry: CrossGeometry,
    pub brush: Option<CrossBrush>,
    pub pen: Option<CrossPen>,
}

impl CrossGeometryDrawing {
    pub fn new(geometry: impl Into<CrossGeometry>) -> CrossGeometryDrawing {
        CrossGeometryDrawing { geometry: geometry.into(), brush: None, pen: None }
    }
}

impl From<CrossGeometryDrawing> for CrossDrawing {
    fn from(value: CrossGeometryDrawing) -> CrossDrawing {
        CrossDrawing::Geometry(value)
    }
}

#[derive(Clone, Default)]
pub struct CrossDrawingGroup {
    pub children: Vec<CrossDrawing>,
}

impl From<CrossDrawingGroup> for CrossDrawing {
    fn from(value: CrossDrawingGroup) -> CrossDrawing {
        CrossDrawing::Group(value)
    }
}

/// A geometry of any class.
#[derive(Clone)]
pub enum CrossGeometry {
    Svg(CrossSvgGeometry),
    Ellipse(CrossEllipseGeometry),
    Stream(CrossStreamGeometry),
    Rectangle(CrossRectangleGeometry),
    Path(CrossPathGeometry),
}

macro_rules! geometry_class {
    ($class:ident, $variant:ident) => {
        impl From<$class> for CrossGeometry {
            fn from(value: $class) -> CrossGeometry {
                CrossGeometry::$variant(value)
            }
        }
    };
}

#[derive(Clone)]
pub struct CrossSvgGeometry {
    pub path: String,
}

geometry_class!(CrossSvgGeometry, Svg);

impl CrossSvgGeometry {
    pub fn new(path: &str) -> CrossSvgGeometry {
        CrossSvgGeometry { path: path.to_string() }
    }
}

#[derive(Clone, Default)]
pub struct CrossEllipseGeometry {
    pub rect: Rect,
}

geometry_class!(CrossEllipseGeometry, Ellipse);

impl CrossEllipseGeometry {
    pub fn new(rect: Rect) -> CrossEllipseGeometry {
        CrossEllipseGeometry { rect }
    }
}

#[derive(Clone, Default)]
pub struct CrossStreamGeometry {
    context_impl: Rc<RefCell<Option<Rc<dyn ICrossStreamGeometryContextImpl>>>>,
}

geometry_class!(CrossStreamGeometry, Stream);

impl CrossStreamGeometry {
    pub fn new() -> CrossStreamGeometry {
        CrossStreamGeometry::default()
    }

    pub fn get_context(&self) -> Rc<dyn ICrossStreamGeometryContextImpl> {
        self.context_impl.borrow_mut().get_or_insert_with(|| CrossGlobals::get_context_impl_provider().create()).clone()
    }
}

#[derive(Clone)]
pub struct CrossRectangleGeometry {
    pub rect: Rect,
}

geometry_class!(CrossRectangleGeometry, Rectangle);

impl CrossRectangleGeometry {
    pub fn new(rect: Rect) -> CrossRectangleGeometry {
        CrossRectangleGeometry { rect }
    }
}

#[derive(Clone, Default)]
pub struct CrossPathGeometry {
    pub figures: Vec<CrossPathFigure>,
}

geometry_class!(CrossPathGeometry, Path);

#[derive(Clone, Default)]
pub struct CrossPathFigure {
    pub start: Point,
    pub segments: Vec<CrossPathSegment>,
    pub closed: bool,
}

#[derive(Clone)]
pub enum CrossPathSegment {
    Line { to: Point, is_stroked: bool },
    Arc {
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    },
    CubicBezier { point1: Point, point2: Point, point3: Point, is_stroked: bool },
    QuadraticBezier { point1: Point, point2: Point, is_stroked: bool },
    PolyLine { points: Vec<Point>, is_stroked: bool },
    PolyBezierSegment { points: Vec<Point>, is_stroked: bool },
}

impl CrossPathSegment {
    pub fn is_stroked(&self) -> bool {
        match self {
            CrossPathSegment::Line { is_stroked, .. }
            | CrossPathSegment::Arc { is_stroked, .. }
            | CrossPathSegment::CubicBezier { is_stroked, .. }
            | CrossPathSegment::QuadraticBezier { is_stroked, .. }
            | CrossPathSegment::PolyLine { is_stroked, .. }
            | CrossPathSegment::PolyBezierSegment { is_stroked, .. } => *is_stroked,
        }
    }
}

#[derive(Clone)]
pub struct CrossDrawingBrush {
    base: CrossTileBrush,
    pub drawing: Box<CrossDrawing>,
}

derives!(CrossDrawingBrush: CrossTileBrush);
brush_class!(CrossDrawingBrush, Drawing);

impl CrossDrawingBrush {
    /// `Drawing` is a required member upstream.
    pub fn new(drawing: impl Into<CrossDrawing>) -> CrossDrawingBrush {
        CrossDrawingBrush { base: CrossTileBrush::default(), drawing: Box::new(drawing.into()) }
    }
}

#[derive(Clone)]
pub struct CrossImageBrush {
    base: CrossTileBrush,
    pub path: String,
}

derives!(CrossImageBrush: CrossTileBrush);
brush_class!(CrossImageBrush, Image);

impl CrossImageBrush {
    /// `Path` is a required member upstream.
    pub fn new(path: &str) -> CrossImageBrush {
        CrossImageBrush { base: CrossTileBrush::default(), path: path.to_string() }
    }
}

#[derive(Clone)]
pub struct CrossPen {
    pub brush: CrossBrush,
    pub thickness: f64,
    pub line_join: PenLineJoin,
    pub line_cap: PenLineCap,
}

impl CrossPen {
    /// `Brush` is a required member upstream.
    pub fn new(brush: impl Into<CrossBrush>) -> CrossPen {
        CrossPen { brush: brush.into(), thickness: 1.0, line_join: PenLineJoin::Miter, line_cap: PenLineCap::Flat }
    }
}

pub trait ICrossStreamGeometryContextImpl {
    fn get_geometry(&self) -> Box<dyn Any>;
    fn begin_figure(&self, point: Point, is_filled: bool, is_closed: bool);
    fn end_figure(&self);
    fn line_to(&self, point: Point, is_stroked: bool);
    fn arc_to(
        &self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    );
    fn cubic_bezier_to(&self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool);
    fn quadratic_bezier_to(&self, control_point: Point, end_point: Point, is_stroked: bool);
    fn dispose(&self);
}

pub trait ICrossStreamGeometryContextImplProvider {
    fn create(&self) -> Rc<dyn ICrossStreamGeometryContextImpl>;
}

pub trait ICrossDrawingContext {
    fn push_transform(&mut self, matrix: Matrix);
    fn pop(&mut self);
    fn draw_line(&mut self, pen: &CrossPen, p1: Point, p2: Point);
    fn draw_rectangle(&mut self, brush: Option<&CrossBrush>, pen: Option<&CrossPen>, rc: Rect);
    fn draw_geometry(&mut self, brush: Option<&CrossBrush>, pen: Option<&CrossPen>, geometry: &CrossGeometry);
    fn draw_image(&mut self, image: &CrossImage, rc: Rect);
}

/// An image of any class.
#[derive(Clone)]
pub enum CrossImage {
    Bitmap(CrossBitmapImage),
    Drawing(CrossDrawingImage),
}

#[derive(Clone)]
pub struct CrossBitmapImage {
    pub path: String,
}

impl CrossBitmapImage {
    pub fn new(path: &str) -> CrossBitmapImage {
        CrossBitmapImage { path: path.to_string() }
    }
}

impl From<CrossBitmapImage> for CrossImage {
    fn from(value: CrossBitmapImage) -> CrossImage {
        CrossImage::Bitmap(value)
    }
}

#[derive(Clone)]
pub struct CrossDrawingImage {
    pub drawing: CrossDrawing,
}

impl CrossDrawingImage {
    /// `Drawing` is a required member upstream. No test of this configuration draws a drawing image.
    #[allow(dead_code)]
    pub fn new(drawing: impl Into<CrossDrawing>) -> CrossDrawingImage {
        CrossDrawingImage { drawing: drawing.into() }
    }
}

impl From<CrossDrawingImage> for CrossImage {
    fn from(value: CrossDrawingImage) -> CrossImage {
        CrossImage::Drawing(value)
    }
}

/// What a class derived from `CrossControl` draws after the base.
enum CrossControlRender {
    Control,
    Func(Box<dyn Fn(&mut dyn ICrossDrawingContext)>),
    Image(CrossImage),
}

pub struct CrossControl {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub background: Option<CrossBrush>,
    pub outline: Option<CrossPen>,
    pub children: Vec<Rc<CrossControl>>,
    pub render_transform: Matrix,
    pub bitmap_interpolation_mode: BitmapInterpolationMode,
    render: CrossControlRender,
}

impl Default for CrossControl {
    fn default() -> Self {
        CrossControl::new()
    }
}

impl CrossControl {
    pub fn new() -> CrossControl {
        CrossControl::with_render(CrossControlRender::Control)
    }

    fn with_render(render: CrossControlRender) -> CrossControl {
        CrossControl {
            left: 0.0,
            top: 0.0,
            width: 0.0,
            height: 0.0,
            background: None,
            outline: None,
            children: Vec::new(),
            render_transform: Matrix::IDENTITY,
            // No initializer upstream: the first member of the enumeration.
            bitmap_interpolation_mode: BitmapInterpolationMode::Unspecified,
            render,
        }
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.left, self.top, self.width, self.height)
    }

    pub fn render(&self, ctx: &mut dyn ICrossDrawingContext) {
        let rc = Rect::from_size(self.bounds().size());
        if self.background.is_some() || self.outline.is_some() {
            ctx.draw_rectangle(self.background.as_ref(), self.outline.as_ref(), rc);
        }

        match &self.render {
            CrossControlRender::Control => {}
            // `CrossFuncControl.Render`.
            CrossControlRender::Func(render) => render(ctx),
            // `CrossImageControl.Render`.
            CrossControlRender::Image(image) => {
                let rc = Rect::from_size(self.bounds().size());
                ctx.draw_image(image, rc);
            }
        }
    }
}

pub struct CrossFuncControl;

impl CrossFuncControl {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(render: impl Fn(&mut dyn ICrossDrawingContext) + 'static) -> CrossControl {
        CrossControl::with_render(CrossControlRender::Func(Box::new(render)))
    }
}

pub struct CrossImageControl;

impl CrossImageControl {
    /// `Image` is a required member upstream.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(image: impl Into<CrossImage>) -> CrossControl {
        CrossControl::with_render(CrossControlRender::Image(image.into()))
    }
}
