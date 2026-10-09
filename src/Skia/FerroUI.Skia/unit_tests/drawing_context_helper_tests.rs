//! Tests of the functions of `DrawingContextHelper` that render a visual
//! onto a surface of the application. Not from upstream, which has no tests
//! of them.

use crate::helpers::drawing_context_helper::{render_async, render_async_clipped};
use crate::SkiaPlatform;
use ferroui_base::media::Brushes;
use ferroui_base::{FerroLocator, Rect, Ref, Size, Vector, Visual};
use ferroui_controls::Border;
use skia_safe::{surfaces, Color, Surface};

/// A red border of 20 by 10.
fn red_border() -> Ref<Visual> {
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    border.measure(Size::new(20.0, 10.0));
    border.arrange(Rect::new(0.0, 0.0, 20.0, 10.0));
    border.clone().upcast()
}

fn white_surface() -> Surface {
    let mut surface = surfaces::raster_n32_premul((30, 20)).expect("a raster surface");
    surface.canvas().clear(Color::WHITE);
    surface
}

fn color(surface: &mut Surface, x: i32, y: i32) -> Color {
    surface.peek_pixels().expect("the pixels of a raster surface").get_color((x, y))
}

#[test]
fn render_async_draws_the_whole_visual_onto_the_surface() {
    let scope = FerroLocator::enter_scope();
    SkiaPlatform::initialize();

    let visual = red_border();
    let mut surface = white_surface();

    // The visual is rendered when the call returns.
    drop(render_async(&surface, &visual));

    assert_eq!(Color::RED, color(&mut surface, 5, 5));
    assert_eq!(Color::RED, color(&mut surface, 15, 5));
    assert_eq!(Color::WHITE, color(&mut surface, 25, 5));
    assert_eq!(Color::WHITE, color(&mut surface, 5, 15));

    scope.dispose();
}

#[test]
fn render_async_clipped_draws_the_part_of_the_visual_inside_the_clip_at_the_origin() {
    let scope = FerroLocator::enter_scope();
    SkiaPlatform::initialize();

    let visual = red_border();
    let mut surface = white_surface();

    drop(render_async_clipped(&surface, &visual, Rect::new(10.0, 0.0, 10.0, 10.0), Vector::new(96.0, 96.0)));

    // The right half of the visual, moved to the origin of the surface.
    assert_eq!(Color::RED, color(&mut surface, 5, 5));
    assert_eq!(Color::WHITE, color(&mut surface, 15, 5));
    assert_eq!(Color::WHITE, color(&mut surface, 5, 15));

    scope.dispose();
}
