//! The decoded document and what it is drawn with.

use crate::decoder;
use ferroui_base::media::immutable::{
    ImmutableDashStyle, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableRadialGradientBrush,
    ImmutableSolidColorBrush, ImmutableTransform,
};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::{
    Color, DrawingGroup, DrawingImage, GeometryDrawing, GradientSpreadMethod, IImmutableBrush, IPen, MatrixTransform, PathCommand,
    PenLineCap, PenLineJoin, StreamGeometry,
};
use ferroui_base::{Matrix, Point, Rect, Ref, RelativePoint, RelativeScalar, RelativeUnit, Size};
use std::fmt;
use std::rc::Rc;

/// What a document may cost to decode. The defaults hold every image an application is likely to
/// ship and stop a document made to exhaust it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SvgLimits {
    /// The most bytes of a document.
    pub max_bytes: usize,
    /// The most elements visited, each `use` counting what it expands to.
    pub max_elements: usize,
    /// The deepest nesting of elements.
    pub max_depth: usize,
    /// The most path segments of the document.
    pub max_path_segments: usize,
    /// The largest width or height of the image, in device independent pixels.
    pub max_dimension: f64,
}

impl Default for SvgLimits {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024,
            max_elements: 100_000,
            max_depth: 64,
            max_path_segments: 2_000_000,
            max_dimension: 16_384.0,
        }
    }
}

/// Why a document was not decoded.
#[derive(Clone, Debug, PartialEq)]
pub enum SvgError {
    /// The document has more bytes than [`SvgLimits::max_bytes`].
    TooLarge {
        /// The bytes of the document.
        bytes: usize,
        /// The limit.
        limit: usize,
    },
    /// The document is not UTF-8.
    InvalidEncoding,
    /// The document is not well-formed XML, or has a document type definition.
    InvalidXml(String),
    /// The root element is not `svg`.
    NotSvg,
    /// The image has no area, or is larger than [`SvgLimits::max_dimension`].
    InvalidSize,
    /// A limit of [`SvgLimits`] was reached; the name of the limit.
    LimitExceeded(&'static str),
}

impl fmt::Display for SvgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SvgError::TooLarge { bytes, limit } => write!(f, "the SVG document has {bytes} bytes, more than {limit}"),
            SvgError::InvalidEncoding => f.write_str("the SVG document is not UTF-8"),
            SvgError::InvalidXml(error) => write!(f, "the SVG document is not valid XML: {error}"),
            SvgError::NotSvg => f.write_str("the root element of the document is not svg"),
            SvgError::InvalidSize => f.write_str("the SVG image has no area or is too large"),
            SvgError::LimitExceeded(name) => write!(f, "the SVG document exceeds the limit of {name}"),
        }
    }
}

impl std::error::Error for SvgError {}

/// A decoded SVG document.
pub struct SvgDocument {
    pub(crate) size: Size,
    pub(crate) root: Group,
}

impl SvgDocument {
    /// Decodes a document with the default limits.
    pub fn parse(bytes: &[u8]) -> Result<Self, SvgError> {
        Self::parse_with_limits(bytes, &SvgLimits::default())
    }

    /// Decodes a document under `limits`.
    pub fn parse_with_limits(bytes: &[u8], limits: &SvgLimits) -> Result<Self, SvgError> {
        decoder::decode(bytes, limits)
    }

    /// The size of the image, in device independent pixels.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The image, as a drawing of geometry that each backend draws as vectors. Needs the render
    /// interface of a running application.
    pub fn to_image(&self) -> Ref<DrawingImage> {
        let image = DrawingImage::with_drawing(group_drawing(&self.root));
        image.set_viewbox(Some(Rect::new(0.0, 0.0, self.size.width, self.size.height)));
        image
    }
}

/// A group of items with what applies to all of them.
pub(crate) struct Group {
    pub(crate) transform: Matrix,
    pub(crate) opacity: f64,
    pub(crate) clip: Option<Vec<PathCommand>>,
    pub(crate) children: Vec<Item>,
}

impl Group {
    pub(crate) fn new(children: Vec<Item>) -> Self {
        Self { transform: Matrix::IDENTITY, opacity: 1.0, clip: None, children }
    }

    /// Whether the group changes nothing about its children.
    pub(crate) fn is_plain(&self) -> bool {
        self.transform == Matrix::IDENTITY && self.opacity >= 1.0 && self.clip.is_none()
    }
}

pub(crate) enum Item {
    Group(Group),
    Shape(Shape),
}

impl Item {
    /// The bounding box in the coordinates the item is placed in, without strokes.
    pub(crate) fn bounds(&self) -> Option<Rect> {
        match self {
            Item::Shape(shape) => Some(shape.bounds),
            Item::Group(group) => {
                let inner = bounds_of(&group.children)?;
                Some(inner.transform_to_aabb(group.transform))
            }
        }
    }
}

/// The union of the bounding boxes of `items`.
pub(crate) fn bounds_of(items: &[Item]) -> Option<Rect> {
    items.iter().filter_map(Item::bounds).reduce(|left, right| left.union(right))
}

pub(crate) struct Shape {
    pub(crate) commands: Vec<PathCommand>,
    pub(crate) bounds: Rect,
    pub(crate) fill: Option<Paint>,
    pub(crate) stroke: Option<Stroke>,
}

#[derive(Clone)]
pub(crate) enum Paint {
    Solid(Color, f64),
    Gradient(Rc<Gradient>, f64),
}

impl Paint {
    /// The paint with its opacity multiplied by `opacity`.
    pub(crate) fn with_opacity(self, opacity: f64) -> Self {
        match self {
            Paint::Solid(color, own) => Paint::Solid(color, own * opacity),
            Paint::Gradient(gradient, own) => Paint::Gradient(gradient, own * opacity),
        }
    }
}

pub(crate) struct Gradient {
    pub(crate) kind: GradientKind,
    pub(crate) stops: Vec<ImmutableGradientStop>,
    pub(crate) transform: Matrix,
    pub(crate) spread: GradientSpreadMethod,
    /// The coordinates are fractions of the bounding box of what is painted.
    pub(crate) in_bounding_box: bool,
}

pub(crate) enum GradientKind {
    Linear { start: Point, end: Point },
    Radial { center: Point, radius: f64, focus: Point },
}

pub(crate) struct Stroke {
    pub(crate) paint: Paint,
    pub(crate) width: f64,
    pub(crate) cap: PenLineCap,
    pub(crate) join: PenLineJoin,
    pub(crate) miter_limit: f64,
    pub(crate) dashes: Option<Rc<[f64]>>,
    pub(crate) dash_offset: f64,
}

fn absolute(point: Point) -> RelativePoint {
    RelativePoint::new(point.x, point.y, RelativeUnit::Absolute)
}

/// The brush of a paint over `bounds`. `None` for a gradient in a bounding box without area,
/// which SVG does not paint.
fn brush(paint: &Paint, bounds: Rect) -> Option<Rc<dyn IImmutableBrush>> {
    let (gradient, opacity) = match paint {
        Paint::Solid(color, opacity) => return Some(Rc::new(ImmutableSolidColorBrush::with_opacity(*color, *opacity))),
        Paint::Gradient(gradient, opacity) => (gradient, *opacity),
    };
    let mut transform = gradient.transform;
    if gradient.in_bounding_box {
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return None;
        }
        transform = transform * Matrix::new(bounds.width, 0.0, 0.0, bounds.height, bounds.x, bounds.y);
    }
    // The coordinates are absolute, so the transform turns about the origin of the user space
    // rather than a corner of the shape.
    let transform = Some(Rc::new(ImmutableTransform::new(transform)));
    let origin = Some(absolute(Point::new(0.0, 0.0)));
    Some(match gradient.kind {
        GradientKind::Linear { start, end } => Rc::new(ImmutableLinearGradientBrush::new(
            &gradient.stops,
            opacity,
            transform,
            origin,
            gradient.spread,
            Some(absolute(start)),
            Some(absolute(end)),
            None,
        )),
        GradientKind::Radial { center, radius, focus } => {
            let radius = RelativeScalar::new(radius, RelativeUnit::Absolute);
            Rc::new(ImmutableRadialGradientBrush::new(
                &gradient.stops,
                opacity,
                transform,
                origin,
                gradient.spread,
                Some(absolute(center)),
                Some(absolute(focus)),
                Some(radius),
                Some(radius),
                None,
            ))
        }
    })
}

fn pen(stroke: &Stroke, bounds: Rect) -> Option<Rc<dyn IPen>> {
    let brush = brush(&stroke.paint, bounds)?;
    // The dashes of a pen are multiples of its thickness.
    let dash_style = stroke.dashes.as_ref().map(|dashes| {
        let scaled: Vec<f64> = dashes.iter().map(|dash| dash / stroke.width).collect();
        Rc::new(ImmutableDashStyle::new(Some(&scaled), stroke.dash_offset / stroke.width))
    });
    Some(Rc::new(ImmutablePen::new(Some(brush), stroke.width, dash_style, stroke.cap, stroke.join, stroke.miter_limit)))
}

fn shape_drawing(shape: &Shape) -> Option<Ref<GeometryDrawing>> {
    let fill = shape.fill.as_ref().and_then(|paint| brush(paint, shape.bounds));
    let stroke = shape.stroke.as_ref().and_then(|stroke| pen(stroke, shape.bounds));
    if fill.is_none() && stroke.is_none() {
        return None;
    }
    let drawing = GeometryDrawing::new();
    drawing.set_geometry(StreamGeometry::from_commands(&shape.commands));
    drawing.set_brush(fill.map(|brush| brush as _));
    drawing.set_pen(stroke);
    Some(drawing)
}

fn group_drawing(group: &Group) -> Ref<DrawingGroup> {
    let drawing = DrawingGroup::new();
    if group.transform != Matrix::IDENTITY {
        drawing.set_transform(MatrixTransform::with_matrix(group.transform));
    }
    if let Some(clip) = &group.clip {
        drawing.set_clip_geometry(StreamGeometry::from_commands(clip));
    }
    let children = drawing.children();
    for child in &group.children {
        match child {
            Item::Group(group) => children.add(group_drawing(group).upcast()),
            Item::Shape(shape) => {
                if let Some(shape) = shape_drawing(shape) {
                    children.add(shape.upcast());
                }
            }
        }
    }
    if group.opacity < 1.0 {
        return in_layer(drawing, group.opacity);
    }
    drawing
}

/// `content` composed as one layer at `opacity`, as SVG asks of a group: what overlaps inside it
/// does not show through. A backend without compositing layers applies the opacity per drawing.
fn in_layer(content: Ref<DrawingGroup>, opacity: f64) -> Ref<DrawingGroup> {
    let bounds = content.get_bounds();
    let drawing = DrawingGroup::new();
    {
        let mut context = drawing.open();
        match context.try_push_compositing_layer(BitmapBlendingMode::SourceOver, opacity, bounds) {
            Some(layer) => {
                content.draw(&mut context);
                context.pop(layer);
            }
            None => {
                content.set_opacity(opacity);
                content.draw(&mut context);
            }
        }
    }
    drawing
}
