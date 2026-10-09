//! The scenes the backends are compared on.
//!
//! A scene draws through the contracts only; what it needs of a backend
//! (geometries, bitmaps) it creates with the render interface it is given.
//! Every scene is 200 by 200 pixels and starts on a white target.

use crate::Backend;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::immutable::{
    ImmutableConicGradientBrush, ImmutableDashStyle, ImmutableGradientStop, ImmutableImageBrush,
    ImmutableLinearGradientBrush, ImmutablePen, ImmutableRadialGradientBrush, ImmutableSolidColorBrush,
    ImmutableTransform,
};
use ferroui_base::media::{
    AlignmentX, AlignmentY, BoxShadows, Color, Colors, EdgeMode, FillRule, GeometryCombineMode, GradientSpreadMethod,
    PenLineCap, PenLineJoin, RenderOptions, Stretch, SweepDirection, TileMode,
};
use ferroui_base::platform::{AlphaFormat, IDrawingContextImpl, IGeometryImpl, IStreamGeometryImpl, PixelFormat};
use ferroui_base::{
    CornerRadius, Matrix, PixelSize, Point, Rect, RelativePoint, RelativeRect, RelativeScalar, RelativeUnit,
    RoundedRect, Size, Vector,
};
use std::rc::Rc;
use std::sync::Arc;

/// The size of every scene in pixels.
pub const SCENE_SIZE: PixelSize = PixelSize::new(200, 200);

/// A scene: its name in the tables and what it draws.
pub struct Scene {
    pub name: &'static str,
    pub draw: fn(&Backend, &mut dyn IDrawingContextImpl),
    /// The share of pixels (in percent) that may differ from the Skia
    /// backend by more than the tolerance: what was measured when the scene
    /// was added or last improved, with a margin. A scene that exceeds it
    /// has regressed.
    pub bound: f64,
}

const NAVY: Color = Color::from_argb(255, 20, 40, 120);
const ORANGE: Color = Color::from_argb(255, 240, 140, 20);
const TEAL: Color = Color::from_argb(255, 0, 150, 136);

fn solid(color: Color) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(color)
}

fn pen(color: Color, thickness: f64, cap: PenLineCap, join: PenLineJoin) -> ImmutablePen {
    ImmutablePen::new(Some(Rc::new(solid(color))), thickness, None, cap, join, 10.0)
}

fn no_shadows() -> BoxShadows {
    BoxShadows::default()
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> RoundedRect {
    RoundedRect::from_rect(Rect::new(x, y, width, height))
}

fn background(context: &mut dyn IDrawingContextImpl) {
    context.clear(Colors::WHITE);
}

fn stops(colors: &[(f64, Color)]) -> Vec<ImmutableGradientStop> {
    colors.iter().map(|(offset, color)| ImmutableGradientStop::new(*offset, *color)).collect()
}

fn three_stops() -> Vec<ImmutableGradientStop> {
    stops(&[(0.0, Colors::RED), (0.5, Colors::YELLOW), (1.0, Colors::BLUE)])
}

fn relative(x: f64, y: f64) -> Option<RelativePoint> {
    Some(RelativePoint::new(x, y, RelativeUnit::Relative))
}

/// A figure with a line, a quadratic and a cubic Bézier and an arc.
fn curved_geometry(backend: &Backend) -> Arc<dyn IStreamGeometryImpl> {
    let geometry = backend.interface.create_stream_geometry();
    let mut context = geometry.open();
    context.begin_figure(Point::new(30.0, 40.0), true);
    context.line_to(Point::new(120.0, 25.0), true);
    context.quadratic_bezier_to(Point::new(185.0, 60.0), Point::new(150.0, 110.0), true);
    context.cubic_bezier_to(Point::new(120.0, 150.0), Point::new(190.0, 160.0), Point::new(140.0, 185.0), true);
    context.arc_to(Point::new(40.0, 150.0), Size::new(70.0, 45.0), 20.0, false, SweepDirection::Clockwise, true);
    context.end_figure(true);
    context.dispose();
    geometry
}

/// A five-pointed star whose lines cross: its middle is filled by the
/// non-zero rule and left out by the even-odd rule.
fn star_geometry(backend: &Backend, fill_rule: FillRule) -> Arc<dyn IStreamGeometryImpl> {
    let geometry = backend.interface.create_stream_geometry();
    let mut context = geometry.open();
    context.set_fill_rule(fill_rule);
    let point = |index: usize| {
        let angle = -std::f64::consts::FRAC_PI_2 + index as f64 * 4.0 * std::f64::consts::PI / 5.0;
        Point::new(100.0 + 85.0 * angle.cos(), 105.0 + 85.0 * angle.sin())
    };
    context.begin_figure(point(0), true);
    for index in 1..5 {
        context.line_to(point(index), true);
    }
    context.end_figure(true);
    context.dispose();
    geometry
}

/// An open figure of three lines with two sharp corners.
fn zigzag_geometry(backend: &Backend) -> Arc<dyn IStreamGeometryImpl> {
    let geometry = backend.interface.create_stream_geometry();
    let mut context = geometry.open();
    context.begin_figure(Point::new(30.0, 150.0), false);
    context.line_to(Point::new(80.0, 45.0), true);
    context.line_to(Point::new(120.0, 150.0), true);
    context.line_to(Point::new(170.0, 45.0), true);
    context.end_figure(false);
    context.dispose();
    geometry
}

fn rectangle(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_rectangle(Some(&solid(NAVY)), None, rect(20.5, 30.25, 150.0, 120.5), &no_shadows());
    context.draw_rectangle(
        None,
        Some(&pen(ORANGE, 6.0, PenLineCap::Flat, PenLineJoin::Miter)),
        rect(40.0, 50.0, 140.0, 130.0),
        &no_shadows(),
    );
}

fn aliased_rectangle(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
    context.draw_rectangle(Some(&solid(NAVY)), None, rect(20.3, 30.3, 150.4, 120.4), &no_shadows());
    context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(60.0, 70.0, 120.0, 100.0));
    context.pop_render_options();
}

fn rounded_rectangle(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_rectangle(
        Some(&solid(TEAL)),
        Some(&pen(NAVY, 4.0, PenLineCap::Flat, PenLineJoin::Miter)),
        RoundedRect::from_radius(Rect::new(20.0, 20.0, 160.0, 70.0), 24.0),
        &no_shadows(),
    );
    // A radius for every corner, one of them none.
    context.draw_rectangle(
        Some(&solid(ORANGE)),
        None,
        RoundedRect::from_corner_radius(Rect::new(20.0, 110.0, 160.0, 70.0), CornerRadius::new(40.0, 8.0, 0.0, 20.0)),
        &no_shadows(),
    );
}

fn elliptical_corners(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_rectangle(
        Some(&solid(NAVY)),
        None,
        RoundedRect::new(
            Rect::new(20.0, 30.0, 160.0, 140.0),
            Vector::new(60.0, 20.0),
            Vector::new(20.0, 50.0),
            Vector::new(70.0, 70.0),
            Vector::new(10.0, 40.0),
        ),
        &no_shadows(),
    );
}

fn ellipse(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_ellipse(
        Some(&solid(ORANGE)),
        Some(&pen(NAVY, 8.0, PenLineCap::Flat, PenLineJoin::Miter)),
        Rect::new(20.0, 40.0, 160.0, 110.0),
    );
}

fn lines(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    for (index, thickness) in [1.0, 2.0, 3.5, 6.0, 10.0].into_iter().enumerate() {
        let y = 25.0 + 30.0 * index as f64;
        context.draw_line(
            Some(&pen(NAVY, thickness, PenLineCap::Flat, PenLineJoin::Miter)),
            Point::new(15.0, y),
            Point::new(185.0, y + 28.0),
        );
    }
}

fn curved_path(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let geometry = curved_geometry(backend);
    context.draw_geometry(
        Some(&solid(TEAL)),
        Some(&pen(NAVY, 5.0, PenLineCap::Round, PenLineJoin::Round)),
        &*geometry,
    );
}

fn star_non_zero(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_geometry(Some(&solid(NAVY)), None, &*star_geometry(backend, FillRule::NonZero));
}

fn star_even_odd(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_geometry(Some(&solid(NAVY)), None, &*star_geometry(backend, FillRule::EvenOdd));
}

fn stroke(backend: &Backend, context: &mut dyn IDrawingContextImpl, cap: PenLineCap, join: PenLineJoin) {
    background(context);
    context.draw_geometry(None, Some(&pen(NAVY, 22.0, cap, join)), &*zigzag_geometry(backend));
}

fn stroke_flat_miter(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    stroke(backend, context, PenLineCap::Flat, PenLineJoin::Miter);
}

fn stroke_round_round(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    stroke(backend, context, PenLineCap::Round, PenLineJoin::Round);
}

fn stroke_square_bevel(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    stroke(backend, context, PenLineCap::Square, PenLineJoin::Bevel);
}

fn stroke_miter_limit(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    // A limit below the miter of the corners: the join falls back to a bevel.
    let pen = ImmutablePen::new(Some(Rc::new(solid(NAVY))), 22.0, None, PenLineCap::Flat, PenLineJoin::Miter, 1.5);
    context.draw_geometry(None, Some(&pen), &*zigzag_geometry(backend));
}

fn dashes(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let dashed = |dashes: &[f64], offset: f64, cap| {
        ImmutablePen::new(
            Some(Rc::new(solid(NAVY))),
            6.0,
            Some(Rc::new(ImmutableDashStyle::new(Some(dashes), offset))),
            cap,
            PenLineJoin::Miter,
            10.0,
        )
    };

    context.draw_line(Some(&dashed(&[3.0, 2.0], 0.0, PenLineCap::Flat)), Point::new(15.0, 25.0), Point::new(185.0, 25.0));
    context.draw_line(Some(&dashed(&[3.0, 2.0], 1.5, PenLineCap::Flat)), Point::new(15.0, 50.0), Point::new(185.0, 50.0));
    // An odd number of dashes is repeated.
    context.draw_line(Some(&dashed(&[4.0, 1.0, 2.0], 0.0, PenLineCap::Flat)), Point::new(15.0, 75.0), Point::new(185.0, 75.0));
    context.draw_line(Some(&dashed(&[1.0, 3.0], 0.0, PenLineCap::Round)), Point::new(15.0, 100.0), Point::new(185.0, 100.0));
    context.draw_geometry(None, Some(&dashed(&[2.0, 2.0], 0.0, PenLineCap::Flat)), &*curved_geometry(backend));
}

fn linear_gradient(context: &mut dyn IDrawingContextImpl, spread: GradientSpreadMethod) {
    background(context);
    let brush = ImmutableLinearGradientBrush::new(
        &three_stops(),
        1.0,
        None,
        None,
        spread,
        relative(0.3, 0.3),
        relative(0.6, 0.5),
        None,
    );
    context.draw_rectangle(Some(&brush), None, rect(10.0, 10.0, 180.0, 180.0), &no_shadows());
}

fn linear_gradient_pad(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    linear_gradient(context, GradientSpreadMethod::Pad);
}

fn linear_gradient_repeat(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    linear_gradient(context, GradientSpreadMethod::Repeat);
}

fn linear_gradient_reflect(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    linear_gradient(context, GradientSpreadMethod::Reflect);
}

fn linear_gradient_translucent(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_rectangle(Some(&solid(NAVY)), None, rect(0.0, 80.0, 200.0, 40.0), &no_shadows());
    let brush = ImmutableLinearGradientBrush::new(
        &stops(&[(0.0, Colors::RED), (1.0, Colors::TRANSPARENT)]),
        0.8,
        None,
        None,
        GradientSpreadMethod::Pad,
        relative(0.0, 0.0),
        relative(1.0, 0.0),
        None,
    );
    context.draw_rectangle(Some(&brush), None, rect(10.0, 10.0, 180.0, 180.0), &no_shadows());
}

fn radial_gradient(context: &mut dyn IDrawingContextImpl, spread: GradientSpreadMethod, radius_y: f64) {
    background(context);
    let brush = ImmutableRadialGradientBrush::new(
        &three_stops(),
        1.0,
        None,
        None,
        spread,
        relative(0.5, 0.5),
        relative(0.5, 0.5),
        Some(RelativeScalar::new(0.35, RelativeUnit::Relative)),
        Some(RelativeScalar::new(radius_y, RelativeUnit::Relative)),
        None,
    );
    context.draw_rectangle(Some(&brush), None, rect(10.0, 10.0, 180.0, 180.0), &no_shadows());
}

fn radial_gradient_pad(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    radial_gradient(context, GradientSpreadMethod::Pad, 0.35);
}

fn radial_gradient_repeat(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    radial_gradient(context, GradientSpreadMethod::Repeat, 0.35);
}

fn radial_gradient_reflect(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    radial_gradient(context, GradientSpreadMethod::Reflect, 0.35);
}

fn radial_gradient_elliptical(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    radial_gradient(context, GradientSpreadMethod::Pad, 0.2);
}

fn radial_gradient_offset_origin(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let brush = ImmutableRadialGradientBrush::new(
        &three_stops(),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        relative(0.5, 0.5),
        relative(0.35, 0.4),
        Some(RelativeScalar::new(0.4, RelativeUnit::Relative)),
        Some(RelativeScalar::new(0.4, RelativeUnit::Relative)),
        None,
    );
    context.draw_rectangle(Some(&brush), None, rect(10.0, 10.0, 180.0, 180.0), &no_shadows());
}

fn conic_gradient(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let brush = ImmutableConicGradientBrush::new(
        &three_stops(),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        relative(0.4, 0.55),
        30.0,
        None,
    );
    context.draw_rectangle(Some(&brush), None, rect(10.0, 10.0, 180.0, 180.0), &no_shadows());
}

fn gradient_with_transform(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let brush = ImmutableLinearGradientBrush::new(
        &three_stops(),
        1.0,
        Some(Rc::new(ImmutableTransform::new(Matrix::create_rotation(0.6)))),
        relative(0.5, 0.5),
        GradientSpreadMethod::Reflect,
        relative(0.2, 0.0),
        relative(0.5, 0.0),
        None,
    );
    context.set_transform(Matrix::create_scale(0.8, 1.1) * Matrix::create_translation(15.0, -5.0));
    context.draw_ellipse(Some(&brush), None, Rect::new(20.0, 20.0, 170.0, 150.0));
    context.set_transform(Matrix::IDENTITY);
}

fn nested_clips(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.push_clip(Rect::new(15.0, 20.0, 170.0, 160.0));
    context.push_clip_rounded(RoundedRect::from_radius(Rect::new(30.0, 10.0, 150.0, 150.0), 40.0));
    let clip = backend.interface.create_ellipse_geometry(Rect::new(10.0, 40.0, 150.0, 150.0));
    context.push_geometry_clip(&*clip);
    context.draw_rectangle(Some(&solid(NAVY)), None, rect(0.0, 0.0, 200.0, 200.0), &no_shadows());
    context.pop_geometry_clip();
    context.draw_rectangle(Some(&solid(ORANGE)), None, rect(100.0, 0.0, 100.0, 60.0), &no_shadows());
    context.pop_clip();
    context.draw_rectangle(Some(&solid(TEAL)), None, rect(0.0, 150.0, 60.0, 50.0), &no_shadows());
    context.pop_clip();
}

fn transformed_clip(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.set_transform(Matrix::create_rotation(0.4) * Matrix::create_translation(70.0, -10.0));
    context.push_clip(Rect::new(20.0, 30.0, 110.0, 110.0));
    context.set_transform(Matrix::IDENTITY);
    context.draw_ellipse(Some(&solid(NAVY)), None, Rect::new(10.0, 10.0, 180.0, 180.0));
    context.pop_clip();
}

fn opacity_layers(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    // The opacity of each shape...
    context.push_opacity(0.6, None);
    context.draw_rectangle(Some(&solid(NAVY)), None, rect(15.0, 15.0, 90.0, 90.0), &no_shadows());
    context.push_opacity(0.5, None);
    context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(60.0, 40.0, 90.0, 90.0));
    context.pop_opacity();
    context.pop_opacity();

    // ...and of shapes as one.
    context.push_render_options(RenderOptions { requires_full_opacity_handling: Some(true), ..RenderOptions::default() });
    context.push_opacity(0.5, Some(Rect::new(0.0, 100.0, 200.0, 100.0)));
    context.draw_rectangle(Some(&solid(TEAL)), None, rect(30.0, 115.0, 90.0, 70.0), &no_shadows());
    context.draw_rectangle(Some(&solid(NAVY)), None, rect(80.0, 130.0, 100.0, 60.0), &no_shadows());
    context.pop_opacity();
    context.pop_render_options();
}

fn opacity_mask(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let mask = ImmutableLinearGradientBrush::new(
        &stops(&[(0.0, Colors::BLACK), (1.0, Colors::TRANSPARENT)]),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        relative(0.0, 0.0),
        relative(1.0, 1.0),
        None,
    );
    context.push_opacity_mask(&mask, Rect::new(20.0, 20.0, 160.0, 160.0));
    context.draw_rectangle(Some(&solid(NAVY)), None, rect(20.0, 20.0, 160.0, 160.0), &no_shadows());
    context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(60.0, 60.0, 120.0, 120.0));
    context.pop_opacity_mask();
}

fn layer(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_rectangle(Some(&solid(TEAL)), None, rect(20.0, 20.0, 100.0, 100.0), &no_shadows());
    context.push_layer(Rect::new(0.0, 0.0, 200.0, 200.0));
    context.draw_ellipse(Some(&solid(NAVY)), None, Rect::new(60.0, 60.0, 120.0, 120.0));
    context.pop_layer();
}

/// A 16x16 bitmap of four colors, one in each quarter.
fn quarters_bitmap(backend: &Backend) -> Rc<Bitmap> {
    let mut data = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            // BGRA.
            let pixel: [u8; 4] = match (x < 8, y < 8) {
                (true, true) => [0, 0, 255, 255],
                (false, true) => [255, 0, 0, 255],
                (true, false) => [0, 200, 0, 255],
                (false, false) => [0, 220, 255, 255],
            };
            data.extend_from_slice(&pixel);
        }
    }

    Rc::new(Bitmap::from_impl(backend.interface.load_bitmap_from_pixels(
        PixelFormat::BGRA8888,
        AlphaFormat::Premul,
        &data,
        PixelSize::new(16, 16),
        Vector::new(96.0, 96.0),
        64,
    )))
}

fn tile(backend: &Backend, context: &mut dyn IDrawingContextImpl, tile_mode: TileMode, transform: Option<Matrix>) {
    background(context);
    let brush = ImmutableImageBrush::new(
        Some(quarters_bitmap(backend)),
        AlignmentX::Center,
        AlignmentY::Center,
        Some(RelativeRect::new(0.0, 0.0, 48.0, 32.0, RelativeUnit::Absolute)),
        1.0,
        transform.map(|transform| Rc::new(ImmutableTransform::new(transform))),
        RelativePoint::default(),
        None,
        Stretch::Fill,
        tile_mode,
        None,
    );
    context.push_render_options(RenderOptions {
        bitmap_interpolation_mode: ferroui_base::media::imaging::BitmapInterpolationMode::None,
        ..RenderOptions::default()
    });
    context.draw_rectangle(Some(&brush), None, rect(10.0, 10.0, 180.0, 180.0), &no_shadows());
    context.pop_render_options();
}

fn tile_repeated(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    tile(backend, context, TileMode::Tile, None);
}

fn tile_flipped(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    tile(backend, context, TileMode::FlipXY, None);
}

fn tile_transformed(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    tile(
        backend,
        context,
        TileMode::Tile,
        Some(Matrix::create_rotation(0.5) * Matrix::create_scale(1.3, 0.9) * Matrix::create_translation(40.0, 10.0)),
    );
}

fn tile_single_transformed(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    tile(
        backend,
        context,
        TileMode::None,
        Some(Matrix::create_scale(2.5, 2.5) * Matrix::create_rotation(0.3) * Matrix::create_translation(60.0, 20.0)),
    );
}

fn bitmap(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let bitmap = backend.interface.create_render_target_bitmap(PixelSize::new(60, 40), Vector::new(96.0, 96.0));
    let mut bitmap_context = bitmap.create_drawing_context();
    bitmap_context.clear(TEAL);
    bitmap_context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(5.0, 5.0, 50.0, 30.0));
    bitmap_context.dispose();

    context.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 60.0, 40.0), Rect::new(10.0, 10.0, 60.0, 40.0));
    // A part of it, enlarged and half transparent.
    context.draw_bitmap(&*bitmap, 0.5, Rect::new(10.0, 5.0, 40.0, 30.0), Rect::new(30.0, 70.0, 160.0, 120.0));
    bitmap.dispose();
}

fn transforms(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let geometry = curved_geometry(backend);
    for (index, color) in [NAVY, TEAL, ORANGE].into_iter().enumerate() {
        let angle = 0.5 * index as f64;
        context.set_transform(
            Matrix::create_translation(-100.0, -100.0)
                * Matrix::create_scale(0.75 - 0.15 * index as f64, 0.75)
                * Matrix::create_rotation(angle)
                * Matrix::create_translation(100.0, 100.0),
        );
        context.draw_geometry(
            Some(&ImmutableSolidColorBrush::with_opacity(color, 0.8)),
            Some(&pen(Colors::BLACK, 2.0, PenLineCap::Flat, PenLineJoin::Miter)),
            &*geometry,
        );
    }
    context.set_transform(Matrix::IDENTITY);
}

fn combined(backend: &Backend, context: &mut dyn IDrawingContextImpl, mode: GeometryCombineMode) {
    background(context);
    let a = backend.interface.create_rectangle_geometry(Rect::new(25.0, 35.0, 110.0, 100.0));
    let b = backend.interface.create_ellipse_geometry(Rect::new(70.0, 60.0, 110.0, 110.0));
    let geometry = backend.interface.create_combined_geometry(mode, a, b);
    context.draw_geometry(
        Some(&solid(TEAL)),
        Some(&pen(NAVY, 3.0, PenLineCap::Flat, PenLineJoin::Miter)),
        &*geometry,
    );
}

fn combined_union(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    combined(backend, context, GeometryCombineMode::Union);
}

fn combined_intersect(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    combined(backend, context, GeometryCombineMode::Intersect);
}

fn combined_xor(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    combined(backend, context, GeometryCombineMode::Xor);
}

fn combined_exclude(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    combined(backend, context, GeometryCombineMode::Exclude);
}

fn geometry_group(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let children: Vec<Arc<dyn IGeometryImpl>> = vec![
        backend.interface.create_rectangle_geometry(Rect::new(20.0, 20.0, 160.0, 160.0)),
        backend.interface.create_ellipse_geometry(Rect::new(50.0, 50.0, 100.0, 100.0)),
        backend.interface.create_line_geometry(Point::new(10.0, 190.0), Point::new(190.0, 10.0)),
    ];
    let group = backend.interface.create_geometry_group(FillRule::EvenOdd, &children);
    context.draw_geometry(
        Some(&solid(ORANGE)),
        Some(&pen(NAVY, 4.0, PenLineCap::Round, PenLineJoin::Round)),
        &*group,
    );
}

/// What a compositor draws into the layer of a window in its first frame.
fn draw_layer_frame(layer_context: &mut dyn IDrawingContextImpl) {
    layer_context.draw_rectangle(Some(&solid(TEAL)), None, rect(20.0, 20.0, 100.0, 100.0), &no_shadows());
    layer_context.draw_ellipse(
        Some(&solid(Color::from_argb(160, 240, 140, 20))),
        Some(&pen(NAVY, 5.0, PenLineCap::Round, PenLineJoin::Round)),
        Rect::new(60.0, 60.0, 120.0, 110.0),
    );
}

/// A layer of the contract (`create_layer`), drawn into and blitted onto
/// the target: the layer a compositor keeps the frame of a window in. In
/// the GPU modes of the Vello backend the layer is a texture of the device.
fn surface_layer(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let layer = context.create_layer(SCENE_SIZE);
    {
        let mut layer_context = layer.create_drawing_context();
        layer_context.clear(Colors::TRANSPARENT);
        draw_layer_frame(&mut *layer_context);
        layer_context.dispose();
    }
    layer.blit(context);
    layer.dispose();
}

/// The layer of [`surface_layer`] drawn into again as a compositor redraws
/// what changed: a dirty rectangle is cleared and drawn again, then the two
/// rectangles of a region, of which nothing is drawn into the second; the
/// rest of the layer is what the frame before left.
fn surface_layer_redrawn(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let layer = context.create_layer(SCENE_SIZE);
    {
        let mut layer_context = layer.create_drawing_context();
        layer_context.push_clip(Rect::new(0.0, 0.0, 200.0, 200.0));
        layer_context.clear(Colors::TRANSPARENT);
        draw_layer_frame(&mut *layer_context);
        layer_context.pop_clip();
        layer_context.dispose();
    }
    {
        let mut layer_context = layer.create_drawing_context();
        layer_context.push_clip(Rect::new(40.0, 40.0, 90.0, 60.0));
        layer_context.clear(Colors::TRANSPARENT);
        layer_context.set_transform(Matrix::create_translation(5.0, 3.0));
        layer_context.draw_rectangle(
            Some(&solid(Color::from_argb(200, 20, 40, 120))),
            None,
            RoundedRect::from_radius(Rect::new(50.0, 50.0, 110.0, 60.0), 14.0),
            &no_shadows(),
        );
        layer_context.pop_clip();
        layer_context.dispose();
    }
    {
        let region = backend.interface.create_region();
        region.add_rect(ferroui_base::platform::LtrbPixelRect::new(130, 120, 190, 190));
        region.add_rect(ferroui_base::platform::LtrbPixelRect::new(10, 150, 60, 190));
        let mut layer_context = layer.create_drawing_context();
        layer_context.push_clip_region(&*region);
        layer_context.clear(Colors::TRANSPARENT);
        layer_context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(110.0, 100.0, 100.0, 100.0));
        layer_context.pop_clip();
        layer_context.dispose();
    }
    layer.blit(context);
    layer.dispose();
}

/// A layer drawn as a bitmap: enlarged, turned and half transparent, as a
/// compositor draws the cached layer of a visual.
fn surface_layer_as_bitmap(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let layer = context.create_layer(PixelSize::new(80, 60));
    {
        let mut layer_context = layer.create_drawing_context();
        layer_context.clear(Colors::TRANSPARENT);
        layer_context.draw_rectangle(Some(&solid(TEAL)), None, rect(0.0, 0.0, 40.0, 60.0), &no_shadows());
        layer_context.draw_rectangle(Some(&solid(ORANGE)), None, rect(40.0, 0.0, 40.0, 30.0), &no_shadows());
        layer_context.draw_ellipse(Some(&solid(NAVY)), None, Rect::new(20.0, 10.0, 40.0, 40.0));
        layer_context.dispose();
    }

    context.draw_bitmap(&*layer, 1.0, Rect::new(0.0, 0.0, 80.0, 60.0), Rect::new(10.0, 10.0, 80.0, 60.0));
    context.push_render_options(RenderOptions {
        bitmap_interpolation_mode: ferroui_base::media::imaging::BitmapInterpolationMode::LowQuality,
        ..RenderOptions::default()
    });
    context.set_transform(Matrix::create_rotation(0.2) * Matrix::create_translation(70.0, 70.0));
    context.draw_bitmap(&*layer, 0.5, Rect::new(0.0, 0.0, 80.0, 60.0), Rect::new(0.0, 0.0, 120.0, 90.0));
    context.pop_render_options();
    layer.dispose();
}

/// The scenes with the bound of each.
pub fn scenes() -> Vec<Scene> {
    macro_rules! scene {
        ($draw:ident, $bound:expr) => {
            Scene { name: stringify!($draw), draw: $draw, bound: $bound }
        };
    }

    vec![
        scene!(rectangle, 0.05),
        scene!(aliased_rectangle, 0.11),
        scene!(rounded_rectangle, 0.17),
        scene!(elliptical_corners, 0.10),
        scene!(ellipse, 0.62),
        scene!(lines, 0.05),
        scene!(curved_path, 0.37),
        scene!(star_non_zero, 0.06),
        scene!(star_even_odd, 0.06),
        scene!(stroke_flat_miter, 0.05),
        scene!(stroke_round_round, 0.13),
        scene!(stroke_square_bevel, 0.05),
        scene!(stroke_miter_limit, 0.05),
        scene!(dashes, 0.76),
        scene!(linear_gradient_pad, 0.05),
        scene!(linear_gradient_repeat, 0.05),
        scene!(linear_gradient_reflect, 0.05),
        scene!(linear_gradient_translucent, 0.05),
        scene!(radial_gradient_pad, 0.05),
        scene!(radial_gradient_repeat, 0.05),
        scene!(radial_gradient_reflect, 0.05),
        scene!(radial_gradient_elliptical, 0.05),
        scene!(radial_gradient_offset_origin, 0.05),
        scene!(conic_gradient, 0.05),
        scene!(gradient_with_transform, 0.14),
        scene!(nested_clips, 0.20),
        scene!(transformed_clip, 0.07),
        scene!(opacity_layers, 0.05),
        scene!(opacity_mask, 0.05),
        scene!(layer, 0.08),
        scene!(tile_repeated, 0.05),
        scene!(tile_flipped, 0.05),
        scene!(tile_transformed, 0.05),
        scene!(tile_single_transformed, 0.25),
        scene!(bitmap, 0.20),
        scene!(transforms, 0.43),
        scene!(combined_union, 0.22),
        scene!(combined_intersect, 0.15),
        scene!(combined_xor, 0.26),
        scene!(combined_exclude, 0.09),
        scene!(geometry_group, 0.35),
        scene!(surface_layer, 0.42),
        scene!(surface_layer_redrawn, 0.32),
        scene!(surface_layer_as_bitmap, 0.06),
    ]
}
