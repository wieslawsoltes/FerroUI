//! The blend layers and colour matrix layers of the drawing context
//! (`IDrawingContextImplWithLayerEffects`, docs/porting/winui-materials.md,
//! section 6.2): drawn onto a raster target and checked pixel by pixel. The
//! other backend has the same tests with the same expectations.

use crate::PlatformRenderInterface;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::{BoxShadows, Color, ColorMatrix, ColorMatrixAlphaMode};
use ferroui_base::platform::{
    IDrawingContextImpl, IDrawingContextImplWithLayerEffects, IPlatformRenderInterface, IRenderTargetBitmapImpl,
    PixelFormat,
};
use ferroui_base::{PixelSize, Rect, RoundedRect, Vector};
use std::sync::Arc;

const SIZE: i32 = 40;

fn target() -> Arc<dyn IRenderTargetBitmapImpl> {
    PlatformRenderInterface::default().create_render_target_bitmap(PixelSize::new(SIZE, SIZE), Vector::new(96.0, 96.0))
}

fn draw(target: &Arc<dyn IRenderTargetBitmapImpl>, f: impl FnOnce(&mut dyn IDrawingContextImplWithLayerEffects)) {
    let mut context = target.create_drawing_context();
    let effects = context.as_drawing_context_impl_with_layer_effects().expect("the context has layer effects");
    f(effects);
    context.dispose();
}

fn fill(context: &mut dyn IDrawingContextImplWithLayerEffects, color: Color) {
    let brush = ImmutableSolidColorBrush::new(color);
    let all = RoundedRect::from_rect(Rect::new(0.0, 0.0, f64::from(SIZE), f64::from(SIZE)));
    context.draw_rectangle(Some(&brush), None, all, &BoxShadows::default());
}

fn bounds() -> Rect {
    Rect::new(0.0, 0.0, f64::from(SIZE), f64::from(SIZE))
}

/// The pixel in the middle of the target, unpremultiplied (r, g, b, a).
fn middle(target: &Arc<dyn IRenderTargetBitmapImpl>) -> (u8, u8, u8, u8) {
    let framebuffer = target.lock();
    let offset = ((SIZE / 2) * framebuffer.row_bytes() + (SIZE / 2) * 4) as usize;
    let mut bytes = [0u8; 4];
    framebuffer.with_data(&mut |data| bytes.copy_from_slice(&data[offset..offset + 4]));
    let format = framebuffer.format();
    framebuffer.dispose();
    let (r, g, b, a) =
        if format == PixelFormat::BGRA8888 { (bytes[2], bytes[1], bytes[0], bytes[3]) } else { (bytes[0], bytes[1], bytes[2], bytes[3]) };
    if a == 0 || a == 255 {
        return (r, g, b, a);
    }
    let un = |c: u8| ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8;
    (un(r), un(g), un(b), a)
}

#[track_caller]
fn assert_near(expected: (u8, u8, u8, u8), actual: (u8, u8, u8, u8)) {
    let close = |e: u8, a: u8| (i32::from(e) - i32::from(a)).abs() <= 3;
    assert!(
        close(expected.0, actual.0) && close(expected.1, actual.1) && close(expected.2, actual.2) && close(expected.3, actual.3),
        "expected {expected:?}, got {actual:?}"
    );
}

fn blend(background: Color, foreground: Color, mode: BitmapBlendingMode) -> (u8, u8, u8, u8) {
    let target = target();
    draw(&target, |context| {
        fill(context, background);
        context.push_blend_layer(bounds(), mode);
        fill(context, foreground);
        context.pop_blend_layer();
    });
    middle(&target)
}

#[test]
fn a_multiply_layer_multiplies_the_colours() {
    let actual = blend(Color::from_rgb(255, 128, 0), Color::from_rgb(128, 128, 255), BitmapBlendingMode::Multiply);
    assert_near((128, 64, 0, 255), actual);
}

#[test]
fn a_screen_layer_screens_the_colours() {
    let actual = blend(Color::from_rgb(255, 128, 0), Color::from_rgb(0, 128, 128), BitmapBlendingMode::Screen);
    assert_near((255, 192, 128, 255), actual);
}

#[test]
fn a_luminosity_layer_keeps_the_hue_of_what_is_under_it() {
    // Black foreground: the luminosity of black with the hue of red is black.
    let actual = blend(Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 0), BitmapBlendingMode::Luminosity);
    assert_near((0, 0, 0, 255), actual);
    // A grey foreground over grey stays grey.
    let actual = blend(Color::from_rgb(100, 100, 100), Color::from_rgb(200, 200, 200), BitmapBlendingMode::Luminosity);
    assert_near((200, 200, 200, 255), actual);
}

fn through_matrix(color: Color, matrix: &ColorMatrix, alpha_mode: ColorMatrixAlphaMode) -> (u8, u8, u8, u8) {
    let target = target();
    draw(&target, |context| {
        context.push_color_matrix_layer(bounds(), matrix, alpha_mode);
        fill(context, color);
        context.pop_color_matrix_layer();
    });
    middle(&target)
}

#[test]
fn an_invert_matrix_layer_inverts_the_colour() {
    let actual = through_matrix(Color::from_rgb(255, 0, 0), &ColorMatrix::invert(), ColorMatrixAlphaMode::Premultiplied);
    assert_near((0, 255, 255, 255), actual);
}

#[test]
fn a_saturation_matrix_layer_of_zero_is_grey() {
    let (r, g, b, a) = through_matrix(Color::from_rgb(255, 0, 0), &ColorMatrix::saturation(0.0), ColorMatrixAlphaMode::Premultiplied);
    assert_eq!(255, a);
    assert!(r == g && g == b || (i32::from(r) - i32::from(b)).abs() <= 2, "grey, got {r} {g} {b}");
    assert!(r > 40 && r < 80, "the luminance of red, got {r}");
}

#[test]
fn an_invert_matrix_layer_of_a_translucent_colour_keeps_its_alpha() {
    let half_red = Color::from_argb(128, 255, 0, 0);
    for alpha_mode in [ColorMatrixAlphaMode::Premultiplied, ColorMatrixAlphaMode::Straight] {
        let actual = through_matrix(half_red, &ColorMatrix::invert(), alpha_mode);
        assert_near((0, 255, 255, 128), actual);
    }
}
