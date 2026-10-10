//! Port of upstream's `Media/RenderTargetBitmapTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::effects::DropShadowEffect;
use ferroui_base::media::{Brushes, Colors};
use ferroui_base::Thickness;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Canvas, Grid};

fn base() -> TestBase {
    TestBase::new(r"Media\RenderTargetBitmap")
}

#[test]
fn render_target_bitmap_drop_shadow_effect() {
    let t = base();
    let root = Grid::new();
    root.set_width(300.0);
    root.set_height(300.0);
    let canvas = Canvas::new();
    canvas.set_width(300.0);
    canvas.set_height(300.0);
    canvas.set_background(Some(Brushes::white()));
    let rectangle = Rectangle::new();
    rectangle.set_fill(Some(Brushes::red()));
    rectangle.set_width(200.0);
    rectangle.set_height(200.0);
    rectangle.set_margin(Thickness::uniform(50.0));
    let effect = DropShadowEffect::new();
    effect.set_color(Colors::BLACK);
    effect.set_blur_radius(30.0);
    rectangle.set_effect(Some(effect.into()));
    canvas.children().add(rectangle);
    root.children().add(canvas);

    t.render_to_file(&root, "RenderTargetBitmap_DropShadowEffect");
    t.compare_images("RenderTargetBitmap_DropShadowEffect");
}
