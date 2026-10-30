//! The suites of the Skia backend's `unit_tests/` that test the contract of
//! a backend and not Skia, under their names there: `RenderBoundsTests`,
//! `CombinedGeometryImplTests` and `DrawingContextImplTests`.
//!
//! The suites of text (fonts, glyph runs, text formatting) are the modules
//! of `unit_tests/media`, file by file as in the Skia backend, with the
//! test doubles they share (`test_font_manager.rs`, `text_support.rs`).
//! `HitTesting` follows with the stage it needs (design document, section
//! 7); the suites of Skia's own types (its caches, its options) have no
//! counterpart.

mod media;
mod test_font_manager;
mod text_support;

pub(crate) use test_font_manager::TestFontManager;
pub(crate) use text_support::{mock_platform_render_interface, register_test_assets, ASSEMBLY};

use crate::geometry_impl::{FillPath, VelloPath};
use crate::{CombinedGeometryImpl, PlatformRenderInterface};
use ferroui_base::media::{
    BoxShadows, Brushes, FillRule, IBrush, IPen, PathGeometry, Pen, PenLineCap, PenLineJoin,
};
use ferroui_base::platform::{IDrawingContextImpl, IGeometryImpl, IPlatformRenderInterface};
use ferroui_base::{CornerRadius, PixelSize, Point, Rect, RoundedRect, Vector};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use kurbo::BezPath;
use std::rc::Rc;

#[test]
fn render_bounds_are_correctly_calculated() {
    // The values are those of the suite of the Skia backend, which are
    // Skia's in single precision: the bounds agree to a thousandth.
    #[allow(clippy::type_complexity)]
    let rows: [(&str, PenLineCap, PenLineJoin, f64, f64, f64, f64, f64, f64); 4] = [
        ("M10 20 L 20 10 L 30 20", PenLineCap::Round, PenLineJoin::Miter, 2.0, 10.0,
            9.0, 8.585786819458008, 22.000001907348633, 12.414215087890625),
        ("M10 10 L 20 10", PenLineCap::Round, PenLineJoin::Miter, 2.0, 10.0,
            9.0, 9.0, 12.0, 2.0),
        ("M10 10 L 20 15 L 10 20", PenLineCap::Flat, PenLineJoin::Miter, 2.0, 20.0,
            9.552786827087402, 9.105572700500488, 12.683281898498535, 11.788853645324707),
        ("M0,0 A128,128 0 0 0 128,0", PenLineCap::Flat, PenLineJoin::Bevel, 0.0, 0.0,
            0.0, 0.0, 128.0, 17.14875030517578),
    ];

    for (path, cap, join, thickness, miter_limit, x, y, width, height) in rows {
        let _app = UnitTestApplication::start(TestServices {
            render_interface: Some(Rc::new(PlatformRenderInterface::default()) as Rc<dyn IPlatformRenderInterface>),
            ..TestServices::default()
        });

        let geo = PathGeometry::parse(path).unwrap();
        let pen: Rc<dyn IPen> = Pen::with_all(Some(Brushes::black()), thickness, None, cap, join, miter_limit).into();
        let bounds = geo.get_render_bounds(&*pen);
        let tolerance = 0.001;

        assert!((bounds.x - x).abs() <= tolerance, "{path}: x {} != {x}", bounds.x);
        assert!((bounds.y - y).abs() <= tolerance, "{path}: y {} != {y}", bounds.y);
        assert!((bounds.width - width).abs() <= tolerance, "{path}: width {} != {width}", bounds.width);
        assert!((bounds.height - height).abs() <= tolerance, "{path}: height {} != {height}", bounds.height);
    }
}

#[test]
fn combining_fill_with_empty_stroke_returns_fill_bounds() {
    let mut fill = BezPath::new();
    fill.move_to((0.0, 0.0));
    fill.line_to((100.0, 0.0));
    fill.line_to((100.0, 100.0));
    fill.line_to((0.0, 100.0));
    fill.close_path();

    let stroke = VelloPath::new(BezPath::new(), FillRule::NonZero);

    let result = CombinedGeometryImpl::new(Some(stroke), FillPath::Separate(VelloPath::new(fill, FillRule::NonZero)));

    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result.bounds());
}

#[test]
fn draw_line_with_zero_thickness_pen_does_not_throw() {
    let mut target = create_target();
    let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black()), 0.0).into();
    target.draw_line(Some(&*pen), Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    target.dispose();
}

#[test]
fn draw_rectangle_with_zero_thickness_pen_does_not_throw() {
    let mut target = create_target();
    let brush: Rc<dyn IBrush> = Brushes::black();
    let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black()), 0.0).into();
    target.draw_rectangle(
        Some(&*brush),
        Some(&*pen),
        RoundedRect::from_corner_radius(Rect::new(0.0, 0.0, 100.0, 100.0), CornerRadius::uniform(4.0)),
        &BoxShadows::default(),
    );
    target.dispose();
}

/// The drawing context of a 100x100 render target bitmap: the suite of the
/// Skia backend wraps the canvas of a bitmap of that size.
fn create_target() -> Box<dyn IDrawingContextImpl> {
    PlatformRenderInterface::default()
        .create_render_target_bitmap(PixelSize::new(100, 100), Vector::new(96.0, 96.0))
        .create_drawing_context()
}

#[test]
fn geometry_path_data_retains_curves_fill_rules_and_affine_transform() {
    use ferroui_base::media::GeometryPathSegment;
    let mut path = BezPath::new();
    path.move_to((1.0, 2.0));
    path.quad_to((3.0, 4.0), (5.0, 6.0));
    path.curve_to((7.0, 8.0), (9.0, 10.0), (11.0, 12.0));
    path.close_path();
    let geometry = crate::SimpleGeometryImpl::new(VelloPath::new(path, FillRule::EvenOdd), FillPath::SameAsStroke, None);
    let original = geometry.try_get_path_data().unwrap();
    assert_eq!(original.fill, original.stroke);
    assert_eq!(original.fill.as_ref().unwrap().fill_rule, FillRule::EvenOdd);
    let matrix = ferroui_base::Matrix::new(2.0, 0.0, 0.5, 3.0, 10.0, 20.0);
    let transformed = geometry.with_transform(matrix).try_get_path_data().unwrap();
    let result = transformed.fill.unwrap();
    assert_eq!(result.fill_rule, FillRule::EvenOdd);
    assert_eq!(result.segments, vec![
        GeometryPathSegment::MoveTo(Point::new(13.0, 26.0)),
        GeometryPathSegment::QuadraticTo { control: Point::new(18.0, 32.0), point: Point::new(23.0, 38.0) },
        GeometryPathSegment::CubicTo { control1: Point::new(28.0, 44.0), control2: Point::new(33.0, 50.0), point: Point::new(38.0, 56.0) },
        GeometryPathSegment::Close,
    ]);
}

#[test]
fn geometry_path_data_distinguishes_independent_fill_and_stroke_and_empty_geometry() {
    let mut stroke = BezPath::new(); stroke.move_to((0.0, 0.0)); stroke.line_to((20.0, 0.0));
    let mut fill = stroke.clone(); fill.line_to((20.0, 10.0)); fill.close_path();
    let geometry = crate::SimpleGeometryImpl::new(VelloPath::new(stroke, FillRule::NonZero), FillPath::Separate(VelloPath::new(fill, FillRule::EvenOdd)), None);
    let data = geometry.try_get_path_data().unwrap();
    assert_ne!(data.fill, data.stroke);
    assert_eq!(data.fill.unwrap().fill_rule, FillRule::EvenOdd);
    let empty = CombinedGeometryImpl::new(None, FillPath::None).try_get_path_data().unwrap();
    assert!(empty.fill.is_none()); assert!(empty.stroke.is_none());
}
