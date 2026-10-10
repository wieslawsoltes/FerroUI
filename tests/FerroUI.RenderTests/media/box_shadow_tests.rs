//! Port of upstream's `Media/BoxShadowTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{BoxShadow, BoxShadows, Brushes, Colors};
use ferroui_base::Thickness;
use ferroui_controls::Border;

fn base() -> TestBase {
    TestBase::new(r"Media\BoxShadow")
}

#[test]
fn box_shadow_should_be_rendered_even_with_null_brush_and_pen() {
    let t = base();
    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(None);
    let child = Border::new();
    child.set_background(None);
    child.set_margin(Thickness::uniform(40.0));
    child.set_box_shadow(BoxShadows::new(BoxShadow {
        blur: 0.0,
        color: Colors::BLUE,
        offset_x: 10.0,
        offset_y: 15.0,
        spread: 0.0,
        ..Default::default()
    }));
    let inner = Border::new();
    inner.set_background(Some(Brushes::red()));
    child.set_child(inner);
    target.set_child(child);

    t.render_to_file(&target, "BoxShadowShouldBeRenderedEvenWithNullBrushAndPen");
    t.compare_images("BoxShadowShouldBeRenderedEvenWithNullBrushAndPen");
}
