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

/// Outer and inset shadows, rounded and square, with offsets, spreads and blurs from small to
/// large. The expected image was rendered with the blur as an image filter, upstream's way; the
/// Skia backend draws it as a mask filter on the shadow's shape (docs/porting/desktop-performance.md
/// section 9.11), and the two must agree within the allowed error.
#[test]
fn box_shadows_of_every_kind() {
    use ferroui_base::media::Color;
    use ferroui_base::CornerRadius;
    use ferroui_controls::Canvas;

    let t = base();
    let target = Canvas::new();
    target.set_width(400.0);
    target.set_height(300.0);
    target.set_background(Some(Brushes::white()));
    let shadow = |blur: f64, spread: f64, offset_x: f64, offset_y: f64, color: Color, is_inset: bool| BoxShadow {
        blur,
        spread,
        offset_x,
        offset_y,
        color,
        is_inset,
    };
    let cases: [(f64, f64, f64, BoxShadows); 6] = [
        (30.0, 30.0, 12.0, BoxShadows::new(shadow(10.0, 0.0, 4.0, 6.0, Color::from_argb(160, 0, 0, 0), false))),
        (160.0, 30.0, 0.0, BoxShadows::new(shadow(20.0, 5.0, 0.0, 0.0, Color::from_argb(200, 0, 0, 255), false))),
        (290.0, 30.0, 24.0, BoxShadows::new(shadow(40.0, 0.0, 8.0, 8.0, Color::from_argb(255, 255, 0, 0), false))),
        (30.0, 170.0, 16.0, BoxShadows::new(shadow(8.0, 2.0, 3.0, 3.0, Color::from_argb(200, 0, 0, 0), true))),
        (160.0, 170.0, 0.0, BoxShadows::new(shadow(6.0, 0.0, -4.0, -4.0, Color::from_argb(255, 0, 128, 0), true))),
        (
            290.0,
            170.0,
            8.0,
            BoxShadows::with_rest(
                shadow(2.0, 0.0, 0.0, 1.0, Color::from_argb(80, 0, 0, 0), false),
                &[shadow(16.0, -2.0, 0.0, 8.0, Color::from_argb(120, 0, 0, 0), false)],
            ),
        ),
    ];
    for (left, top, radius, shadows) in cases {
        let card = Border::new();
        card.set_width(80.0);
        card.set_height(80.0);
        card.set_corner_radius(CornerRadius::uniform(radius));
        card.set_background(Some(Brushes::light_gray()));
        card.set_box_shadow(shadows);
        Canvas::set_left(&card, left);
        Canvas::set_top(&card, top);
        target.children().add(card);
    }

    t.render_to_file(&target, "BoxShadowsOfEveryKind");
    t.compare_images("BoxShadowsOfEveryKind");
}
