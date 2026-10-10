//! Port of upstream's `CrossTests/Media/DrawingContextTests.cs`.

use crate::cross_test_base::CrossTestBase;
use crate::cross_ui::*;
use ferroui_base::media::Colors;
use ferroui_base::{Matrix, Point};

fn base() -> CrossTestBase {
    CrossTestBase::new("Media/DrawingContext")
}

fn pen(brush: CrossSolidColorBrush, thickness: f64) -> CrossPen {
    let mut pen = CrossPen::new(brush);
    pen.thickness = thickness;
    pen
}

#[test]
fn transform_should_work_as_expected() {
    let t = base();
    let mut root = CrossFuncControl::new(|ctx| {
        ctx.push_transform(Matrix::create_translation(100.0, 100.0));
        ctx.draw_line(&pen(CrossSolidColorBrush::new(Colors::RED), 1.0), Point::new(0.0, 0.0), Point::new(100.0, 0.0));
        ctx.pop();

        ctx.push_transform(Matrix::create_translation(200.0, 100.0));
        ctx.draw_line(
            &pen(CrossSolidColorBrush::new(Colors::ORANGE), 1.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 100.0),
        );
        ctx.pop();

        ctx.push_transform(Matrix::create_translation(200.0, 200.0));
        ctx.draw_line(
            &pen(CrossSolidColorBrush::new(Colors::YELLOW), 1.0),
            Point::new(0.0, 0.0),
            Point::new(-100.0, 0.0),
        );
        ctx.pop();

        ctx.push_transform(Matrix::create_translation(100.0, 200.0));
        ctx.draw_line(
            &pen(CrossSolidColorBrush::new(Colors::GREEN), 1.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, -100.0),
        );
        ctx.pop();
    });
    root.width = 300.0;
    root.height = 300.0;

    t.render_and_compare(root, "Transform_Should_Work_As_Expected");
}
