//! Port of upstream's `Shapes/EllipseTests.cs`.
//!
//! `Should_Render_Circle_Aliased` passes `gpuAllowedError: 0.05` to
//! `CompareImages` upstream. That argument concerns the Mesa GL outputs of
//! upstream, which the port does not render, so it is dropped.

use crate::test_base::TestBase;
use ferroui_base::media::{Brushes, EdgeMode, RenderOptions};
use ferroui_base::Thickness;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::{Border, Decorator};

fn base() -> TestBase {
    TestBase::new(r"Shapes\Ellipse")
}

#[test]
fn circle_1px_stroke() {
    let t = base();
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Ellipse::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(1.0);
    target.set_child(child);

    t.render_to_file(&target, "Circle_1px_Stroke");
    t.compare_images("Circle_1px_Stroke");
}

#[test]
fn should_render_circle_aliased() {
    let t = base();
    let target = Border::new();
    target.set_background(Some(Brushes::white()));
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Ellipse::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(3.5);
    target.set_child(child);

    RenderOptions::set_edge_mode(&target, EdgeMode::Aliased);

    t.render_to_file(&target, "Should_Render_Circle_Aliased");
    t.compare_images("Should_Render_Circle_Aliased");
}

#[test]
fn should_render_circle_antialiased() {
    let t = base();
    let target = Border::new();
    target.set_background(Some(Brushes::white()));
    target.set_padding(Thickness::uniform(8.0));
    target.set_width(200.0);
    target.set_height(200.0);
    let child = Ellipse::new();
    child.set_stroke(Some(Brushes::black()));
    child.set_stroke_thickness(3.5);
    target.set_child(child);

    RenderOptions::set_edge_mode(&target, EdgeMode::Antialias);

    t.render_to_file(&target, "Should_Render_Circle_Antialiased");
    t.compare_images("Should_Render_Circle_Antialiased");
}
