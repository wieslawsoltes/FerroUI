//! The scenes of stage 6 of the Vello backend: box shadows, effects, scene
//! brushes, the interpolation of bitmaps and blending modes.
//!
//! As the scenes of `scenes.rs`: drawn through the contracts only, 200 by
//! 200 pixels, on a white target. A blur is a smooth ramp, and two
//! renderers that blur a little differently differ a little in every pixel
//! of the ramp without any pixel being far off: such a scene has, beside the
//! bound on the share of pixels beyond the tolerance, a bound on the mean
//! difference of all pixels, which a blur that is too wide, too narrow or
//! in the wrong place exceeds.

use crate::Backend;
use ferroui_base::media::effects::{ImmutableBlurEffect, ImmutableDropShadowEffect};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadow, BoxShadows, Color, Colors};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Matrix, PixelSize, Rect, RoundedRect, Vector};

/// The size of every scene in pixels.
pub const SCENE_SIZE: PixelSize = PixelSize::new(200, 200);

/// A scene with the two bounds of its difference from the Skia backend.
pub struct EffectScene {
    pub name: &'static str,
    pub draw: fn(&Backend, &mut dyn IDrawingContextImpl),
    /// The share of pixels (in percent) that may differ by more than the
    /// tolerance.
    pub share_bound: f64,
    /// The mean difference of the channels of all pixels (of 255) that the
    /// scene may have.
    pub mean_bound: f64,
}

const NAVY: Color = Color::from_argb(255, 20, 40, 120);
const ORANGE: Color = Color::from_argb(255, 240, 140, 20);
const TEAL: Color = Color::from_argb(255, 0, 150, 136);
const SHADE: Color = Color::from_argb(200, 0, 0, 0);

fn solid(color: Color) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(color)
}

fn no_shadows() -> BoxShadows {
    BoxShadows::default()
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> RoundedRect {
    RoundedRect::from_rect(Rect::new(x, y, width, height))
}

fn rounded(x: f64, y: f64, width: f64, height: f64, radius: f64) -> RoundedRect {
    RoundedRect::from_radius(Rect::new(x, y, width, height), radius)
}

fn background(context: &mut dyn IDrawingContextImpl) {
    context.clear(Colors::WHITE);
}

fn shadow(offset_x: f64, offset_y: f64, blur: f64, spread: f64, color: Color, is_inset: bool) -> BoxShadows {
    BoxShadows::new(BoxShadow { offset_x, offset_y, blur, spread, color, is_inset })
}

/// Four boxes with a shadow each: sharp and moved, blurred, blurred with a
/// spread, wide and moved with a negative spread.
fn four_shadows(context: &mut dyn IDrawingContextImpl, is_inset: bool, shape: fn(f64, f64, f64, f64) -> RoundedRect) {
    background(context);
    let fill = solid(Color::from_argb(255, 235, 235, 235));
    let shadows = [
        shadow(6.0, 4.0, 0.0, 0.0, NAVY, is_inset),
        shadow(0.0, 0.0, 8.0, 0.0, SHADE, is_inset),
        shadow(3.0, 5.0, 14.0, 6.0, SHADE, is_inset),
        shadow(-8.0, 6.0, 30.0, -4.0, TEAL, is_inset),
    ];
    for (index, shadows) in shadows.iter().enumerate() {
        let (x, y) = (25.0 + 100.0 * (index % 2) as f64, 25.0 + 100.0 * (index / 2) as f64);
        context.draw_rectangle(Some(&fill), None, shape(x, y, 50.0, 50.0), shadows);
    }
}

fn shadows_outset_rectangle(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, false, rect);
}

fn shadows_outset_rounded(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, false, |x, y, width, height| rounded(x, y, width, height, 10.0));
}

/// Corners that are elliptical and differ: the top left one square.
fn uneven(x: f64, y: f64, width: f64, height: f64) -> RoundedRect {
    RoundedRect::new(
        Rect::new(x, y, width, height),
        Vector::new(0.0, 0.0),
        Vector::new(22.0, 12.0),
        Vector::new(8.0, 8.0),
        Vector::new(14.0, 25.0),
    )
}

fn shadows_outset_elliptical(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, false, uneven);
}

fn shadows_outset_capsule(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, false, |x, y, width, height| rounded(x, y + 10.0, width, height - 20.0, 15.0));
}

fn shadows_inset_rectangle(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, true, rect);
}

fn shadows_inset_rounded(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, true, |x, y, width, height| rounded(x, y, width, height, 10.0));
}

fn shadows_inset_elliptical(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, true, uneven);
}

/// Several shadows of one box, outside and inside, under a transform, a
/// clip and an opacity.
fn shadows_combined(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let shadows = BoxShadows::with_rest(
        BoxShadow { offset_x: 4.0, offset_y: 6.0, blur: 12.0, spread: 2.0, color: SHADE, is_inset: false },
        &[
            BoxShadow { offset_x: -5.0, offset_y: -5.0, blur: 6.0, spread: 0.0, color: ORANGE, is_inset: false },
            BoxShadow { offset_x: 2.0, offset_y: 3.0, blur: 10.0, spread: 3.0, color: NAVY, is_inset: true },
        ],
    );

    context.push_clip_rounded(rounded(8.0, 8.0, 150.0, 184.0, 30.0));
    context.push_opacity(0.8, None);
    context.set_transform(Matrix::create_rotation(0.2) * Matrix::create_scale(1.4, 1.2) * Matrix::create_translation(50.0, 10.0));
    context.draw_rectangle(Some(&solid(Colors::WHITE)), None, rounded(10.0, 20.0, 70.0, 90.0, 12.0), &shadows);
    context.set_transform(Matrix::IDENTITY);
    context.pop_opacity();
    context.pop_clip();
}

fn with_effect(
    context: &mut dyn IDrawingContextImpl,
    clip_rect: Option<Rect>,
    effect: &dyn ferroui_base::media::effects::IEffect,
    draw: impl FnOnce(&mut dyn IDrawingContextImpl),
) {
    let effects = context.as_drawing_context_impl_with_effects().expect("a drawing context with effects");
    effects.push_effect(clip_rect, effect);
    draw(effects);
    effects.pop_effect();
}

/// Shapes blurred by a radius of 3, 10 and 24.
fn effect_blur(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_effect(context, None, &ImmutableBlurEffect::new(3.0), |context| {
        context.draw_rectangle(Some(&solid(NAVY)), None, rect(15.0, 15.0, 70.0, 50.0), &no_shadows());
    });
    with_effect(context, None, &ImmutableBlurEffect::new(10.0), |context| {
        context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(110.0, 15.0, 70.0, 60.0));
    });
    with_effect(context, None, &ImmutableBlurEffect::new(24.0), |context| {
        context.draw_rectangle(Some(&solid(TEAL)), None, rounded(40.0, 105.0, 120.0, 60.0, 12.0), &no_shadows());
        context.draw_rectangle(Some(&solid(NAVY)), None, rect(80.0, 120.0, 40.0, 30.0), &no_shadows());
    });
}

/// A blur under a transform and a rounded clip, with the clip rectangle
/// the compositor gives an effect: the bounds of the content and the
/// padding of the effect.
fn effect_blur_transformed(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.push_clip_rounded(rounded(20.0, 20.0, 160.0, 150.0, 40.0));
    context.set_transform(Matrix::create_rotation(0.35) * Matrix::create_scale(1.5, 1.5) * Matrix::create_translation(60.0, 0.0));
    with_effect(context, Some(Rect::new(9.0, 9.0, 82.0, 72.0)), &ImmutableBlurEffect::new(8.0), |context| {
        context.draw_rectangle(Some(&solid(NAVY)), None, rect(20.0, 20.0, 60.0, 50.0), &no_shadows());
        context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(35.0, 30.0, 30.0, 30.0));
    });
    context.set_transform(Matrix::IDENTITY);
    context.pop_clip();
}

/// A blur whose clip rectangle is smaller than its content: the content is
/// cut before it is blurred.
fn effect_blur_bounded(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_effect(context, Some(Rect::new(40.0, 40.0, 120.0, 120.0)), &ImmutableBlurEffect::new(12.0), |context| {
        context.draw_rectangle(Some(&solid(NAVY)), None, rect(10.0, 70.0, 180.0, 60.0), &no_shadows());
    });
}

/// Drop shadows: sharp, blurred, translucent and of a translucent shape.
fn effect_drop_shadow(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_effect(context, None, &ImmutableDropShadowEffect::new(8.0, 6.0, 0.0, NAVY, 1.0), |context| {
        context.draw_rectangle(Some(&solid(ORANGE)), None, rect(15.0, 15.0, 60.0, 50.0), &no_shadows());
    });
    with_effect(context, None, &ImmutableDropShadowEffect::new(6.0, 8.0, 10.0, Colors::BLACK, 0.7), |context| {
        context.draw_ellipse(Some(&solid(TEAL)), None, Rect::new(110.0, 15.0, 70.0, 55.0));
    });
    with_effect(context, None, &ImmutableDropShadowEffect::new(-10.0, 12.0, 20.0, NAVY, 1.0), |context| {
        context.draw_rectangle(
            Some(&ImmutableSolidColorBrush::with_opacity(ORANGE, 0.6)),
            None,
            rounded(50.0, 105.0, 110.0, 55.0, 14.0),
            &no_shadows(),
        );
    });
}

/// A drop shadow under a transform, a clip and an opacity.
fn effect_drop_shadow_transformed(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.push_clip(Rect::new(10.0, 10.0, 165.0, 180.0));
    context.push_opacity(0.75, None);
    context.set_transform(Matrix::create_scale(1.6, 1.3) * Matrix::create_rotation(-0.25) * Matrix::create_translation(10.0, 70.0));
    with_effect(context, None, &ImmutableDropShadowEffect::new(6.0, 6.0, 8.0, Colors::BLACK, 0.8), |context| {
        context.draw_rectangle(Some(&solid(TEAL)), None, rounded(20.0, 10.0, 70.0, 50.0, 8.0), &no_shadows());
        context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(60.0, 40.0, 40.0, 40.0));
    });
    context.set_transform(Matrix::IDENTITY);
    context.pop_opacity();
    context.pop_clip();
}

/// The scenes with the bounds of each.
pub fn scenes() -> Vec<EffectScene> {
    macro_rules! scene {
        ($draw:ident, $share_bound:expr, $mean_bound:expr) => {
            EffectScene { name: stringify!($draw), draw: $draw, share_bound: $share_bound, mean_bound: $mean_bound }
        };
    }

    vec![
        scene!(shadows_outset_rectangle, 100.0, 255.0),
        scene!(shadows_outset_rounded, 100.0, 255.0),
        scene!(shadows_outset_elliptical, 100.0, 255.0),
        scene!(shadows_outset_capsule, 100.0, 255.0),
        scene!(shadows_inset_rectangle, 100.0, 255.0),
        scene!(shadows_inset_rounded, 100.0, 255.0),
        scene!(shadows_inset_elliptical, 100.0, 255.0),
        scene!(shadows_combined, 100.0, 255.0),
        scene!(effect_blur, 100.0, 255.0),
        scene!(effect_blur_transformed, 100.0, 255.0),
        scene!(effect_blur_bounded, 100.0, 255.0),
        scene!(effect_drop_shadow, 100.0, 255.0),
        scene!(effect_drop_shadow_transformed, 100.0, 255.0),
    ]
}
