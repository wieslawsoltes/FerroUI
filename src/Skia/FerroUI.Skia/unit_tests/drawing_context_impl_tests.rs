//! Port of upstream's `DrawingContextImplTests.cs` of the Skia unit tests.
//!
//! Upstream wraps the canvas of a 100x100 `SKBitmap` with
//! `DrawingContextHelper.WrapSkiaCanvas`; the port's helper wraps the canvas
//! of a raster surface of the same size.

use crate::helpers::drawing_context_helper::wrap_skia_surface;
use crate::DrawingContextImpl;
use ferroui_base::media::{BoxShadows, Brushes, IBrush, IPen, Pen};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{CornerRadius, Point, Rect, RoundedRect, Vector};
use std::rc::Rc;

#[test]
fn draw_line_with_zero_thickness_pen_does_not_throw() {
    let (mut target, _surface) = create_target();
    let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black()), 0.0).into();
    target.draw_line(Some(&*pen), Point::new(0.0, 0.0), Point::new(10.0, 10.0));
}

#[test]
fn draw_rectangle_with_zero_thickness_pen_does_not_throw() {
    let (mut target, _surface) = create_target();
    let brush: Rc<dyn IBrush> = Brushes::black();
    let pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black()), 0.0).into();
    target.draw_rectangle(
        Some(&*brush),
        Some(&*pen),
        RoundedRect::from_corner_radius(Rect::new(0.0, 0.0, 100.0, 100.0), CornerRadius::uniform(4.0)),
        &BoxShadows::default(),
    );
}

fn create_target() -> (DrawingContextImpl, skia_safe::Surface) {
    let surface = skia_safe::surfaces::raster_n32_premul((100, 100)).unwrap();
    let target = wrap_skia_surface(&surface, Vector::new(96.0, 96.0));
    (target, surface)
}

#[test]
fn solid_color_composition_preserves_destination_alpha_clip_and_transform() {
    use ferroui_base::{
        Matrix,
        media::{Color, imaging::BitmapBlendingMode},
    };
    let (mut context, mut surface) = create_target();
    context.clear(Color::from_uint32(0));
    context.push_clip(Rect::new(1., 1., 3., 2.));
    context.draw_color(Color::from_uint32(0xffff0000), BitmapBlendingMode::Source);
    context.pop_clip();
    let transform = Matrix::create_translation(1000., 2000.);
    context.set_transform(transform);
    context.draw_color(Color::from_uint32(0x800000ff), BitmapBlendingMode::SourceIn);
    assert_eq!(context.transform(), transform);
    let pixels = surface.peek_pixels().unwrap();
    assert_eq!(
        pixels.get_color((2, 1)),
        skia_safe::Color::from_argb(128, 0, 0, 255)
    );
    assert_eq!(pixels.get_color((0, 0)), skia_safe::Color::TRANSPARENT);
    context.draw_color(Color::from_uint32(0x00ff0000), BitmapBlendingMode::SourceIn);
    assert_eq!(
        surface.peek_pixels().unwrap().get_color((2, 1)),
        skia_safe::Color::TRANSPARENT
    );
}

#[test]
fn recorded_source_in_layer_preserves_unrelated_background() {
    let _scope = ferroui_base::FerroLocator::enter_scope();
    ferroui_base::FerroLocator::current_mutable()
        .bind::<dyn ferroui_base::platform::IPlatformRenderInterface>()
        .to_constant(Rc::new(crate::PlatformRenderInterface::default()));
    use ferroui_base::media::{Color, DrawingContext, ISceneBrushContent, SolidColorBrush, imaging::BitmapBlendingMode, immutable::ImmutableImageBrush};
    use ferroui_base::rendering::composition::drawing::RenderDataDrawingContext;
    let mut recorder = RenderDataDrawingContext::new(None);
    {
        let mut context = DrawingContext::new(&mut recorder);
        let bounds = Rect::new(0.,0.,6.,4.);
        let layer = context.try_push_compositing_layer(BitmapBlendingMode::SourceOver,1.0,bounds).unwrap();
        let brush: Rc<dyn IBrush> = SolidColorBrush::from_uint32(0x80ff0000).into();
        context.draw_rectangle(Some(&brush),None,Rect::new(1.,1.,3.,2.),0.,0.,&BoxShadows::default());
        context.draw_color(Color::from_uint32(0xff0000ff),BitmapBlendingMode::SourceIn,Rect::new(1.,1.,3.,2.));
        context.pop(layer);
    }
    let content=recorder.get_immediate_scene_brush_content(Rc::new(ImmutableImageBrush::from_bitmap(None)),None,true).unwrap();
    let (mut target,mut surface)=create_target();
    target.clear(Color::from_uint32(0xff00ff00));
    content.render(&mut target,None);
    let pixels=surface.peek_pixels().unwrap();
    assert_eq!(pixels.get_color((0,0)),skia_safe::Color::GREEN);
    assert_eq!(pixels.get_color((2,1)),skia_safe::Color::from_argb(255,0,127,128));
}
