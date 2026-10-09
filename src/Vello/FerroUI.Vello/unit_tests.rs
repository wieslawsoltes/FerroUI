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
