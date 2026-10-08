//! Port of upstream's `RenderBoundsTests.cs` of the Skia unit tests.

use super::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::{Brushes, IPen, PathGeometry, Pen, PenLineCap, PenLineJoin};
use ferroui_controls::testing::UnitTestApplication;
use std::rc::Rc;

#[test]
fn render_bounds_are_correctly_calculated() {
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
        let _app = UnitTestApplication::start(
            mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
        );

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
