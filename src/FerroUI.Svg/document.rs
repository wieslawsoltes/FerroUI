//! The decoded document and what it is drawn with.

use crate::decoder;
use ferroui_base::media::immutable::{
    ImmutableDashStyle, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableRadialGradientBrush,
    ImmutableSolidColorBrush, ImmutableTransform,
};
use ferroui_base::media::effects::ImmutableLuminanceToAlphaEffect;
use ferroui_base::media::imaging::{Bitmap, BitmapBlendingMode};
use ferroui_base::media::{
    AlignmentX, AlignmentY, Color, DrawingBrush, DrawingGroup, DrawingImage, FlowDirection, FontFamily, FontManager,
    FontStretch, FontStyle, FontWeight, FormattedText, GeometryDrawing, GradientSpreadMethod, IBrush, IDashStyle, IImage,
    IImmutableBrush, IPen, ImageDrawing, MatrixTransform, PathCommand, Pen, PenLineCap, PenLineJoin, RectangleGeometry,
    Stretch, StreamGeometry, TileMode, Typeface,
};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::current_locale;
use ferroui_base::{FerroLocator, LocatorExtensions};
use crate::text_layout::{place_runs, RunPosition};
use ferroui_base::{Matrix, Point, Rect, Ref, RelativePoint, RelativeRect, RelativeScalar, RelativeUnit, Size};
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
    /// The most pixels of all the embedded images together, read from their headers before
    /// they are decoded.
    pub max_image_pixels: u64,
}

impl Default for SvgLimits {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024,
            max_elements: 100_000,
            max_depth: 64,
            max_path_segments: 2_000_000,
            max_dimension: 16_384.0,
            max_image_pixels: 64 * 1024 * 1024,
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
    pub(crate) unsupported: Vec<String>,
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

    /// What the document asks for that is not drawn, each named once, so that a picture that
    /// differs from its document is never silent. Each is also logged as a warning of the area
    /// `Visual`, once in the life of the process.
    pub fn unsupported_features(&self) -> &[String] {
        &self.unsupported
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
    pub(crate) mask: Option<Box<Mask>>,
    pub(crate) children: Vec<Item>,
}

/// A `mask` for one element.
pub(crate) struct Mask {
    pub(crate) content: Group,
    /// The mask region, in the user space of the element.
    pub(crate) region: Rect,
    /// The value of the mask is the luminance of its content (`mask-type: luminance`, the
    /// default), else its alpha.
    pub(crate) is_luminance: bool,
}

impl Group {
    pub(crate) fn new(children: Vec<Item>) -> Self {
        Self { transform: Matrix::IDENTITY, opacity: 1.0, clip: None, mask: None, children }
    }

    /// Whether the group changes nothing about its children.
    pub(crate) fn is_plain(&self) -> bool {
        self.transform == Matrix::IDENTITY && self.opacity >= 1.0 && self.clip.is_none() && self.mask.is_none()
    }
}

pub(crate) enum Item {
    Group(Group),
    Shape(Shape),
    Image(Image),
    Text(TextBlock),
}

/// An embedded PNG or JPEG, decoded when the document is drawn.
pub(crate) struct Image {
    pub(crate) bytes: Rc<[u8]>,
    /// The size in pixels, from the header.
    pub(crate) size: Size,
    pub(crate) target: Rect,
    /// The `preserveAspectRatio` of the image.
    pub(crate) aspect: String,
}

/// The runs of a `text` element, shaped when the document is drawn.
pub(crate) struct TextBlock {
    pub(crate) runs: Vec<TextRun>,
}

pub(crate) struct TextRun {
    pub(crate) text: String,
    pub(crate) position: RunPosition,
    /// The families to look up; `None` for the default font.
    pub(crate) family: Option<String>,
    pub(crate) size: f64,
    pub(crate) weight: i32,
    pub(crate) is_italic: bool,
    pub(crate) fill: Option<Paint>,
    pub(crate) stroke: Option<Stroke>,
}

impl TextBlock {
    /// A box for what needs the bounding box of the text before it is shaped (a clip, a mask or a
    /// paint in its bounding box units): about half an em per character, from the baseline up.
    pub(crate) fn estimated_bounds(&self) -> Option<Rect> {
        let width = |run: &TextRun| 0.55 * run.size * run.text.chars().count() as f64;
        let positions: Vec<RunPosition> = self.runs.iter().map(|run| run.position).collect();
        let places = place_runs(&positions, |index| width(&self.runs[index]));
        self.runs
            .iter()
            .zip(places)
            .filter(|(run, _)| !run.text.is_empty())
            .map(|(run, (x, y))| Rect::new(x, y - 0.8 * run.size, width(run), run.size))
            .reduce(|left, right| left.union(right))
    }
}

impl Item {
    /// The bounding box in the coordinates the item is placed in, without strokes.
    pub(crate) fn bounds(&self) -> Option<Rect> {
        match self {
            Item::Shape(shape) => Some(shape.bounds),
            Item::Image(image) => Some(image.target),
            Item::Text(text) => text.estimated_bounds(),
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
    Pattern(Rc<Pattern>, f64),
}

impl Paint {
    /// The paint with its opacity multiplied by `opacity`.
    pub(crate) fn with_opacity(self, opacity: f64) -> Self {
        match self {
            Paint::Solid(color, own) => Paint::Solid(color, own * opacity),
            Paint::Gradient(gradient, own) => Paint::Gradient(gradient, own * opacity),
            Paint::Pattern(pattern, own) => Paint::Pattern(pattern, own * opacity),
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

/// A `pattern`: its content, repeated over what it paints.
pub(crate) struct Pattern {
    pub(crate) content: Group,
    /// The first tile, in the user space or as fractions of the bounding box.
    pub(crate) tile: Rect,
    pub(crate) tile_in_bounding_box: bool,
    pub(crate) content_in_bounding_box: bool,
    /// The `viewBox` of the content and its `preserveAspectRatio`.
    pub(crate) view: Option<(Rect, String)>,
    pub(crate) transform: Matrix,
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

/// The brush of a paint over `bounds`. `None` for a paint in a bounding box without area, which
/// SVG does not paint.
fn brush(paint: &Paint, bounds: Rect) -> Option<Rc<dyn IBrush>> {
    match paint {
        Paint::Pattern(pattern, opacity) => pattern_brush(pattern, *opacity, bounds),
        _ => immutable_brush(paint, bounds).map(|brush| brush as Rc<dyn IBrush>),
    }
}

/// A tile of the content, repeated by the backend: its content stays vectors at every scale.
fn pattern_brush(pattern: &Pattern, opacity: f64, bounds: Rect) -> Option<Rc<dyn IBrush>> {
    let tile = if pattern.tile_in_bounding_box {
        Rect::new(
            bounds.x + pattern.tile.x * bounds.width,
            bounds.y + pattern.tile.y * bounds.height,
            pattern.tile.width * bounds.width,
            pattern.tile.height * bounds.height,
        )
    } else {
        pattern.tile
    };
    if !(tile.width > 0.0 && tile.height > 0.0) {
        return None;
    }
    let local = Rect::new(0.0, 0.0, tile.width, tile.height);
    let content = group_drawing(&pattern.content);
    let to_tile = match &pattern.view {
        Some((view, aspect)) => Some(crate::decoder::view_transform(*view, aspect, local)),
        None if pattern.content_in_bounding_box => Some(Matrix::create_scale(bounds.width, bounds.height)),
        None => None,
    };
    if let Some(to_tile) = to_tile {
        content.set_transform(MatrixTransform::with_matrix(to_tile));
    }
    // A transparent drawing of the whole tile makes the bounds of the content those of the tile.
    let area = GeometryDrawing::new();
    area.set_geometry(RectangleGeometry::with_rect(local));
    area.set_brush(Some(Rc::new(ImmutableSolidColorBrush::with_opacity(Color::from_argb(0, 0, 0, 0), 1.0)) as Rc<dyn IBrush>));
    let drawing = DrawingGroup::new();
    drawing.set_clip_geometry(RectangleGeometry::with_rect(local));
    drawing.children().add(area.upcast());
    drawing.children().add(content.upcast());

    let brush = DrawingBrush::new();
    brush.set_drawing(Some(drawing.upcast()));
    brush.set_tile_mode(TileMode::Tile);
    brush.set_stretch(Stretch::None);
    brush.set_alignment_x(AlignmentX::Left);
    brush.set_alignment_y(AlignmentY::Top);
    brush.set_source_rect(RelativeRect::new(0.0, 0.0, tile.width, tile.height, RelativeUnit::Absolute));
    brush.set_destination_rect(RelativeRect::new(tile.x, tile.y, tile.width, tile.height, RelativeUnit::Absolute));
    brush.set_opacity(opacity);
    if pattern.transform != Matrix::IDENTITY {
        brush.set_transform(Some(Rc::new(ImmutableTransform::new(pattern.transform))));
        brush.set_transform_origin(absolute(Point::new(0.0, 0.0)));
    }
    Some(brush.into())
}

/// The brush of a solid or gradient paint.
fn immutable_brush(paint: &Paint, bounds: Rect) -> Option<Rc<dyn IImmutableBrush>> {
    let (gradient, opacity) = match paint {
        Paint::Solid(color, opacity) => return Some(Rc::new(ImmutableSolidColorBrush::with_opacity(*color, *opacity))),
        Paint::Gradient(gradient, opacity) => (gradient, *opacity),
        Paint::Pattern(..) => return None,
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
    // The dashes of a pen are multiples of its thickness.
    let dash_style = stroke.dashes.as_ref().map(|dashes| {
        let scaled: Vec<f64> = dashes.iter().map(|dash| dash / stroke.width).collect();
        Rc::new(ImmutableDashStyle::new(Some(&scaled), stroke.dash_offset / stroke.width))
    });
    if matches!(stroke.paint, Paint::Pattern(..)) {
        // The brush of a pattern is not immutable, so neither is its pen.
        let pen = Pen::with_brush(brush(&stroke.paint, bounds), stroke.width);
        pen.brush()?;
        pen.set_dash_style(dash_style.map(|style| style as Rc<dyn IDashStyle>));
        pen.set_line_cap(stroke.cap);
        pen.set_line_join(stroke.join);
        pen.set_miter_limit(stroke.miter_limit);
        return Some(pen.into());
    }
    let brush = immutable_brush(&stroke.paint, bounds)?;
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
    drawing.set_brush(fill);
    drawing.set_pen(stroke);
    Some(drawing)
}

/// The brush of a mask over `target`, in the same coordinates: the content is drawn at its place,
/// and its luminance becomes alpha through the effect every backend runs on the GPU.
fn mask_brush(mask: &Mask, target: Rect) -> Rc<dyn IBrush> {
    let content = group_drawing(&mask.content);
    let region = DrawingGroup::new();
    region.set_clip_geometry(RectangleGeometry::with_rect(mask.region));
    region.children().add(content.upcast());
    if mask.is_luminance {
        region.set_effect(Some(Rc::new(ImmutableLuminanceToAlphaEffect)));
    }
    let area = GeometryDrawing::new();
    area.set_geometry(RectangleGeometry::with_rect(target));
    area.set_brush(Some(Rc::new(ImmutableSolidColorBrush::with_opacity(Color::from_argb(0, 0, 0, 0), 1.0)) as Rc<dyn IBrush>));
    let drawing = DrawingGroup::new();
    drawing.children().add(area.upcast());
    drawing.children().add(region.upcast());

    let brush = DrawingBrush::new();
    brush.set_drawing(Some(drawing.upcast()));
    brush.set_stretch(Stretch::None);
    brush.set_alignment_x(AlignmentX::Left);
    brush.set_alignment_y(AlignmentY::Top);
    let rect = RelativeRect::new(target.x, target.y, target.width, target.height, RelativeUnit::Absolute);
    brush.set_source_rect(rect);
    brush.set_destination_rect(rect);
    brush.into()
}

fn group_drawing(group: &Group) -> Ref<DrawingGroup> {
    let drawing = DrawingGroup::new();
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
            Item::Image(image) => {
                if let Some(image) = image_drawing(image) {
                    children.add(image.upcast());
                }
            }
            Item::Text(text) => {
                if let Some(text) = text_drawing(text) {
                    children.add(text.upcast());
                }
            }
        }
    }
    // The mask is in the coordinates of the content, so it is read before the transform.
    if let Some(mask) = &group.mask {
        let bounds = drawing.get_bounds();
        drawing.set_opacity_mask(Some(mask_brush(mask, bounds)));
    }
    if group.transform != Matrix::IDENTITY {
        drawing.set_transform(MatrixTransform::with_matrix(group.transform));
    }
    if group.opacity < 1.0 {
        return in_layer(drawing, group.opacity);
    }
    drawing
}

/// Logs, once in the life of the process, what could not be drawn although the document was
/// decoded: an image the platform does not decode, text without a font backend.
fn report_draw_problem(problem: String) {
    for problem in crate::decoder::newly_reported(&[problem]) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
            logger.log(None, &format!("SVG: the {problem} is not drawn; the picture differs from the document."));
        }
    }
}

/// The image decoded by the platform and drawn into its place by the backend, as a texture of the
/// GPU where the backend draws on one.
fn image_drawing(image: &Image) -> Option<Ref<DrawingGroup>> {
    let bitmap = match Bitmap::from_stream(&mut std::io::Cursor::new(&image.bytes[..])) {
        Ok(bitmap) => bitmap,
        Err(error) => {
            report_draw_problem(format!("embedded image that the platform does not decode ({error})"));
            return None;
        }
    };
    let source = Rect::new(0.0, 0.0, image.size.width, image.size.height);
    let picture = ImageDrawing::new();
    picture.set_image_source(Some(Rc::new(bitmap) as Rc<dyn IImage>));
    picture.set_rect(source);
    let placed = DrawingGroup::new();
    placed.set_transform(MatrixTransform::with_matrix(crate::decoder::view_transform(source, &image.aspect, image.target)));
    placed.children().add(picture.upcast());
    // With `slice` the image overflows its place, which cuts it.
    let drawing = DrawingGroup::new();
    drawing.set_clip_geometry(RectangleGeometry::with_rect(image.target));
    drawing.children().add(placed.upcast());
    Some(drawing)
}

/// The runs shaped by the font manager and drawn as the geometry of their glyphs, so that a
/// gradient, a pattern or a stroke paints text as it paints a path.
fn text_drawing(block: &TextBlock) -> Option<Ref<DrawingGroup>> {
    let locator = FerroLocator::current();
    if locator.get_service::<FontManager>().is_none() && locator.get_service::<dyn IFontManagerImpl>().is_none() {
        report_draw_problem("text, for want of a font backend,".to_owned());
        return None;
    }
    let formatted: Vec<Option<FormattedText>> = block
        .runs
        .iter()
        .map(|run| {
            if run.text.is_empty() {
                return None;
            }
            let family = run.family.as_deref().and_then(|names| FontFamily::parse(names).ok()).unwrap_or_default();
            let style = if run.is_italic { FontStyle::Italic } else { FontStyle::Normal };
            let typeface = Typeface::with_style(family, style, FontWeight(run.weight), FontStretch::Normal);
            // A line draws its glyphs only with a foreground; the outlines are what is kept of them.
            let foreground: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Color::from_argb(255, 0, 0, 0)));
            Some(FormattedText::new(&run.text, current_locale(), FlowDirection::LeftToRight, typeface, run.size, Some(foreground)))
        })
        .collect();
    let positions: Vec<RunPosition> = block.runs.iter().map(|run| run.position).collect();
    let places = place_runs(&positions, |index| {
        formatted[index].as_ref().map_or(0.0, |text| text.width_including_trailing_whitespace())
    });
    let drawing = DrawingGroup::new();
    for ((run, text), (x, y)) in block.runs.iter().zip(&formatted).zip(places) {
        let Some(text) = text else { continue };
        let Some(geometry) = text.build_geometry(Point::new(x, y - text.baseline())) else { continue };
        let bounds = geometry.bounds();
        let fill = run.fill.as_ref().and_then(|paint| brush(paint, bounds));
        let stroke = run.stroke.as_ref().and_then(|stroke| pen(stroke, bounds));
        if fill.is_none() && stroke.is_none() {
            continue;
        }
        let glyphs = GeometryDrawing::new();
        glyphs.set_geometry(geometry);
        glyphs.set_brush(fill);
        glyphs.set_pen(stroke);
        drawing.children().add(glyphs.upcast());
    }
    Some(drawing)
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
