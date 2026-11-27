//! SVG icons drawn as geometry through the drawing context, on the GPU of every backend.
//!
//! The document is parsed once (`usvg`, which resolves styles, `use` elements, units and text
//! into paths) and turned into a cached drawing: a tree of groups with their transforms, opacity
//! and clip geometry, and shapes with a `StreamGeometry`, a brush (solid, linear or radial
//! gradient) and a pen. Drawing it pushes the transform that maps the requested part of the icon
//! onto its destination and draws the geometry: nothing is rasterised on the CPU, and the icon
//! is as sharp at every scale as its paths. Raster images embedded in the SVG are decoded once
//! into bitmaps, which the backends draw as textures. Masks and patterns, which no ribbon icon
//! uses, are not drawn (a pattern paint draws nothing).
use ferroui_base::{
    Matrix, Point, Ref, Rect, RelativePoint, RelativeScalar, RelativeUnit, Size, WeakRef,
    media::{
        Color, DrawingContext, FillRule, Geometry, GradientSpreadMethod, IBrush, IImage,
        IImmutableBrush, IPen, PenLineCap, PenLineJoin, StreamGeometry,
        immutable::{
            ImmutableDashStyle, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen,
            ImmutableRadialGradientBrush, ImmutableSolidColorBrush, ImmutableTransform,
        },
        imaging::Bitmap,
    },
    platform::IGeometryContext,
};
use ferroui_controls::Image;
use std::{any::Any, io, rc::Rc, sync::Arc};
use usvg::tiny_skia_path::PathSegment;

/// A part of the cached drawing of an icon.
enum SvgItem {
    Group {
        transform: Matrix,
        opacity: f64,
        clip: Option<Ref<Geometry>>,
        children: Vec<SvgItem>,
    },
    Shape {
        geometry: Ref<Geometry>,
        fill: Option<Rc<dyn IBrush>>,
        stroke: Option<Rc<dyn IPen>>,
        stroke_first: bool,
    },
    Bitmap {
        bitmap: Rc<Bitmap>,
        bounds: Rect,
    },
}

struct SvgImage {
    size: Size,
    root: SvgItem,
}

/// Parses `bytes` into the cached drawing of an icon. `_target`, the `Image` that shows it, is
/// not needed: the geometry is drawn at the scale of every frame.
pub(super) fn decode_svg(bytes: &[u8], _target: WeakRef<Image>) -> io::Result<Rc<dyn IImage>> {
    let mut options = usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(|_, _| None),
            ..Default::default()
        },
        ..Default::default()
    };
    // The fonts of the system are read only for an icon with text, which usvg turns into paths:
    // an icon of paths alone loads none.
    if contains(bytes, b"<text") {
        options.fontdb = system_fonts();
    }
    let tree = usvg::Tree::from_data(bytes, &options)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let size = Size::new(f64::from(tree.size().width()), f64::from(tree.size().height()));
    Ok(Rc::new(SvgImage { size, root: group(tree.root()) }))
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

fn system_fonts() -> Arc<usvg::fontdb::Database> {
    use std::sync::OnceLock;
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut fonts = usvg::fontdb::Database::new();
            fonts.load_system_fonts();
            Arc::new(fonts)
        })
        .clone()
}

fn matrix(transform: usvg::Transform) -> Matrix {
    Matrix::new(
        f64::from(transform.sx),
        f64::from(transform.ky),
        f64::from(transform.kx),
        f64::from(transform.sy),
        f64::from(transform.tx),
        f64::from(transform.ty),
    )
}

fn group(source: &usvg::Group) -> SvgItem {
    let children = source.children().iter().filter_map(node).collect();
    SvgItem::Group {
        transform: matrix(source.transform()),
        opacity: f64::from(source.opacity().get()),
        clip: source.clip_path().map(clip_geometry),
        children,
    }
}

fn node(source: &usvg::Node) -> Option<SvgItem> {
    match source {
        usvg::Node::Group(source) => Some(group(source)),
        usvg::Node::Path(source) => shape(source),
        usvg::Node::Text(source) => Some(group(source.flattened())),
        usvg::Node::Image(source) => image(source),
    }
}

/// The geometry of a path, with every point mapped by `transform`.
fn geometry(path: &usvg::tiny_skia_path::Path, rule: FillRule, transform: usvg::Transform) -> Ref<Geometry> {
    let geometry = StreamGeometry::new();
    {
        let mut context = geometry.open();
        context.set_fill_rule(rule);
        append_path(&mut context, path, transform);
    }
    geometry.upcast()
}

fn append_path(context: &mut impl IGeometryContext, path: &usvg::tiny_skia_path::Path, transform: usvg::Transform) {
    let point = |point: usvg::tiny_skia_path::Point| {
        let mut mapped = point;
        transform.map_point(&mut mapped);
        Point::new(f64::from(mapped.x), f64::from(mapped.y))
    };
    let mut open = false;
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(to) => {
                if open {
                    context.end_figure(false);
                }
                context.begin_figure(point(to), true);
                open = true;
            }
            PathSegment::LineTo(to) => context.line_to(point(to), true),
            PathSegment::QuadTo(control, to) => context.quadratic_bezier_to(point(control), point(to), true),
            PathSegment::CubicTo(first, second, to) => {
                context.cubic_bezier_to(point(first), point(second), point(to), true)
            }
            PathSegment::Close => {
                if open {
                    context.end_figure(true);
                    open = false;
                }
            }
        }
    }
    if open {
        context.end_figure(false);
    }
}

/// The clip of a group: the union of the paths of its clip path, in the space of the group.
fn clip_geometry(clip: &usvg::ClipPath) -> Ref<Geometry> {
    fn collect(group: &usvg::Group, transform: usvg::Transform, context: &mut impl IGeometryContext) {
        let transform = transform.pre_concat(group.transform());
        for child in group.children() {
            match child {
                usvg::Node::Group(child) => collect(child, transform, context),
                usvg::Node::Path(path) if path.is_visible() => append_path(context, path.data(), transform),
                usvg::Node::Text(text) => collect(text.flattened(), transform, context),
                _ => {}
            }
        }
    }
    let geometry = StreamGeometry::new();
    {
        let mut context = geometry.open();
        context.set_fill_rule(FillRule::NonZero);
        collect(clip.root(), clip.transform(), &mut context);
    }
    geometry.upcast()
}

fn shape(path: &usvg::Path) -> Option<SvgItem> {
    if !path.is_visible() {
        return None;
    }
    let rule = match path.fill().map(usvg::Fill::rule) {
        Some(usvg::FillRule::EvenOdd) => FillRule::EvenOdd,
        _ => FillRule::NonZero,
    };
    let fill = path
        .fill()
        .and_then(|fill| brush(fill.paint(), f64::from(fill.opacity().get())))
        .map(|brush| brush as Rc<dyn IBrush>);
    let stroke = path.stroke().and_then(pen);
    if fill.is_none() && stroke.is_none() {
        return None;
    }
    Some(SvgItem::Shape {
        geometry: geometry(path.data(), rule, usvg::Transform::identity()),
        fill,
        stroke,
        stroke_first: path.paint_order() == usvg::PaintOrder::StrokeAndFill,
    })
}

fn color(color: usvg::Color, opacity: f32) -> Color {
    Color::from_argb((opacity.clamp(0.0, 1.0) * 255.0).round() as u8, color.red, color.green, color.blue)
}

fn stops(stops: &[usvg::Stop]) -> Vec<ImmutableGradientStop> {
    stops
        .iter()
        .map(|stop| ImmutableGradientStop::new(f64::from(stop.offset().get()), color(stop.color(), stop.opacity().get())))
        .collect()
}

fn spread(method: usvg::SpreadMethod) -> GradientSpreadMethod {
    match method {
        usvg::SpreadMethod::Pad => GradientSpreadMethod::Pad,
        usvg::SpreadMethod::Reflect => GradientSpreadMethod::Reflect,
        usvg::SpreadMethod::Repeat => GradientSpreadMethod::Repeat,
    }
}

fn absolute(x: f32, y: f32) -> RelativePoint {
    RelativePoint::new(f64::from(x), f64::from(y), RelativeUnit::Absolute)
}

/// The gradient transform of usvg applies in the space of the path, from its origin.
fn gradient_transform(transform: usvg::Transform) -> Option<Rc<ImmutableTransform>> {
    (!transform.is_identity()).then(|| Rc::new(ImmutableTransform::new(matrix(transform))))
}

fn brush(paint: &usvg::Paint, opacity: f64) -> Option<Rc<dyn IImmutableBrush>> {
    let origin = Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute));
    match paint {
        usvg::Paint::Color(value) => {
            Some(Rc::new(ImmutableSolidColorBrush::with_opacity(color(*value, 1.0), opacity)) as Rc<dyn IImmutableBrush>)
        }
        usvg::Paint::LinearGradient(gradient) => Some(Rc::new(ImmutableLinearGradientBrush::new(
            &stops(gradient.stops()),
            opacity,
            gradient_transform(gradient.transform()),
            origin,
            spread(gradient.spread_method()),
            Some(absolute(gradient.x1(), gradient.y1())),
            Some(absolute(gradient.x2(), gradient.y2())),
            None,
        ))),
        usvg::Paint::RadialGradient(gradient) => {
            let radius = Some(RelativeScalar::new(f64::from(gradient.r().get()), RelativeUnit::Absolute));
            Some(Rc::new(ImmutableRadialGradientBrush::new(
                &stops(gradient.stops()),
                opacity,
                gradient_transform(gradient.transform()),
                origin,
                spread(gradient.spread_method()),
                Some(absolute(gradient.cx(), gradient.cy())),
                Some(absolute(gradient.fx(), gradient.fy())),
                radius,
                radius,
                None,
            )))
        }
        usvg::Paint::Pattern(_) => None,
    }
}

fn pen(stroke: &usvg::Stroke) -> Option<Rc<dyn IPen>> {
    let brush = brush(stroke.paint(), f64::from(stroke.opacity().get()))?;
    let width = f64::from(stroke.width().get());
    // The dashes of a pen are in units of its thickness; those of SVG in user units.
    let dashes = stroke.dasharray().filter(|_| width > 0.0).map(|dashes| {
        let dashes: Vec<f64> = dashes.iter().map(|dash| f64::from(*dash) / width).collect();
        Rc::new(ImmutableDashStyle::new(Some(&dashes), f64::from(stroke.dashoffset()) / width))
    });
    let cap = match stroke.linecap() {
        usvg::LineCap::Butt => PenLineCap::Flat,
        usvg::LineCap::Round => PenLineCap::Round,
        usvg::LineCap::Square => PenLineCap::Square,
    };
    let join = match stroke.linejoin() {
        usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => PenLineJoin::Miter,
        usvg::LineJoin::Round => PenLineJoin::Round,
        usvg::LineJoin::Bevel => PenLineJoin::Bevel,
    };
    Some(Rc::new(ImmutablePen::new(Some(brush), width, dashes, cap, join, f64::from(stroke.miterlimit().get()))))
}

fn image(source: &usvg::Image) -> Option<SvgItem> {
    if !source.is_visible() {
        return None;
    }
    let bytes = match source.kind() {
        usvg::ImageKind::PNG(bytes)
        | usvg::ImageKind::JPEG(bytes)
        | usvg::ImageKind::GIF(bytes)
        | usvg::ImageKind::WEBP(bytes) => bytes.clone(),
        usvg::ImageKind::SVG(tree) => return Some(group(tree.root())),
    };
    let bitmap = Bitmap::from_stream(&mut io::Cursor::new(bytes.as_slice())).ok()?;
    let size = source.size();
    Some(SvgItem::Bitmap {
        bitmap: Rc::new(bitmap),
        bounds: Rect::new(0.0, 0.0, f64::from(size.width()), f64::from(size.height())),
    })
}

fn draw_item(item: &SvgItem, context: &mut DrawingContext<'_>) {
    match item {
        SvgItem::Group { transform, opacity, clip, children } => {
            if *opacity <= 0.0 {
                return;
            }
            let _transform = (!transform.is_identity()).then(|| context.push_transform(*transform));
            let _opacity = (*opacity < 1.0).then(|| context.push_opacity(*opacity));
            let _clip = clip.as_ref().map(|clip| context.push_geometry_clip(clip));
            for child in children {
                draw_item(child, context);
            }
        }
        SvgItem::Shape { geometry, fill, stroke, stroke_first } => {
            if *stroke_first {
                context.draw_geometry(None, stroke.as_ref(), geometry);
                context.draw_geometry(fill.as_ref(), None, geometry);
            } else {
                context.draw_geometry(fill.as_ref(), stroke.as_ref(), geometry);
            }
        }
        SvgItem::Bitmap { bitmap, bounds } => {
            let size = bitmap.size();
            bitmap.draw(context, Rect::new(0.0, 0.0, size.width, size.height), *bounds);
        }
    }
}

impl IImage for SvgImage {
    fn size(&self) -> Size {
        self.size
    }

    /// Maps `source` (in the units of the icon) onto `destination` and draws the geometry there,
    /// clipped to `destination`.
    fn draw(&self, context: &mut DrawingContext<'_>, source: Rect, destination: Rect) {
        if source.width <= 0.0 || source.height <= 0.0 || destination.width <= 0.0 || destination.height <= 0.0 {
            return;
        }
        let scale_x = destination.width / source.width;
        let scale_y = destination.height / source.height;
        let placement = Matrix::new(
            scale_x,
            0.0,
            0.0,
            scale_y,
            destination.x - source.x * scale_x,
            destination.y - source.y * scale_y,
        );
        let _clip = context.push_clip(destination);
        let _placement = context.push_transform(placement);
        draw_item(&self.root, context);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::PlatformDrawingContext;
    use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};

    fn record(image: &dyn IImage, source: Rect, destination: Rect) -> DrawingLog {
        let log = DrawingLog::new();
        let mut platform_impl = MockDrawingContextImpl::new(log.clone());
        let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
        let mut context = DrawingContext::new(&mut core);
        image.draw(&mut context, source, destination);
        context.dispose();
        log
    }

    const ICON: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">
        <defs><linearGradient id="g" x1="0" y1="0" x2="16" y2="0" gradientUnits="userSpaceOnUse">
            <stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient></defs>
        <rect x="1" y="1" width="14" height="14" fill="url(#g)" stroke="#000000" stroke-width="2"/>
        <g opacity="0.5"><circle cx="8" cy="8" r="3" fill="#00ff00"/></g>
    </svg>"##;

    /// The icon is drawn as geometry with its brushes and pen, under the transform that maps it
    /// onto the destination: no bitmap is drawn for a vector icon.
    #[test]
    fn a_vector_icon_is_drawn_as_geometry_and_never_as_a_bitmap() {
        crate::group_tests::run(|| {
            let image = decode_svg(ICON, Image::new().downgrade()).expect("the icon parses");
            assert_eq!(image.size(), Size::new(16.0, 16.0));
            let log = record(&*image, Rect::new(0.0, 0.0, 16.0, 16.0), Rect::new(10.0, 20.0, 32.0, 32.0));
            let entries = log.entries();
            assert_eq!(log.count("DrawBitmap"), 0, "{entries:?}");
            assert_eq!(log.count("DrawGeometry"), 2, "the rectangle and the circle: {entries:?}");
            assert_eq!(log.count("PushOpacity"), 1, "the group opacity: {entries:?}");
            assert_eq!(log.count("PushClip"), 1, "clipped to the destination: {entries:?}");
        });
    }

    /// An icon of paths alone loads no font of the system.
    #[test]
    fn an_icon_without_text_needs_no_fonts() {
        assert!(!contains(ICON, b"<text"));
        assert!(contains(br#"<svg><text>a</text></svg>"#, b"<text"));
    }
}
