//! The layers with an effect, checked in the pixels that come back.

use super::*;
use ferroui_base::media::effects::{CustomEffect, EffectParameterValue, ImmutableCustomEffect, ShaderEffectDescription};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::{
    AcrylicBackgroundSource, ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix, IExperimentalAcrylicMaterial,
    ImmutableLuminanceToAlphaEffect,
};
use ferroui_base::platform::{
    IDrawingContextImplWithLayerEffects, LightingParameters, MultiplyLayerFactors, PixelOperation,
};
use ferroui_base::rendering::composition::server::{LightColor, LightingMaterial};

const BLACK: [u8; 4] = [0, 0, 0, 255];

/// Red into blue and blue into red.
fn swap_red_and_blue() -> ColorMatrix {
    ColorMatrix::new([[0.0, 0.0, 1.0, 0.0], [0.0, 1.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [0.0; 4]])
}

fn layers(frame: &mut dyn IDrawingContextImpl) -> &mut dyn IDrawingContextImplWithLayerEffects {
    frame.as_drawing_context_impl_with_layer_effects().expect("layer effects")
}

/// Draws a 16 by 16 frame over white and hands it to `draw`.
fn frame_of(draw: impl FnOnce(&mut dyn IDrawingContextImpl)) -> Option<Arc<MemorySurface>> {
    let context = context()?;
    let surface = MemorySurface::new(16, 16);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(255, 255, 255));
    draw(&mut *frame);
    frame.dispose();
    target.dispose();
    Some(surface)
}

const AREA: Rect = Rect { x: 4.0, y: 4.0, width: 8.0, height: 8.0 };

#[test]
fn a_colour_matrix_changes_what_the_layer_holds_and_nothing_outside_it() {
    let Some(surface) = frame_of(|frame| {
        layers(frame).push_color_matrix_layer(AREA, &swap_red_and_blue(), ColorMatrixAlphaMode::Premultiplied);
        fill(frame, &solid(255, 0, 0), Rect::new(0.0, 0.0, 16.0, 16.0));
        layers(frame).pop_color_matrix_layer();
    }) else {
        return;
    };
    assert_eq!(BLUE, surface.rgba(8, 8), "inside the layer");
    assert_eq!(WHITE, surface.rgba(2, 2), "outside its bounds");
}

#[test]
fn a_layer_keeps_the_transform_the_drawing_sees() {
    let moved = Matrix::create_translation(3.0, 2.0);
    let Some(surface) = frame_of(|frame| {
        frame.set_transform(moved);
        layers(frame).push_color_matrix_layer(
            Rect::new(1.0, 2.0, 8.0, 8.0),
            &swap_red_and_blue(),
            ColorMatrixAlphaMode::Premultiplied,
        );
        assert_eq!(moved, frame.transform(), "inside the layer");
        fill(frame, &solid(255, 0, 0), Rect::new(1.0, 2.0, 4.0, 4.0));
        layers(frame).pop_color_matrix_layer();
        assert_eq!(moved, frame.transform(), "after it");
        frame.set_transform(Matrix::IDENTITY);
    }) else {
        return;
    };
    assert_eq!(BLUE, surface.rgba(5, 5), "where the transform puts the fill");
    assert_eq!(WHITE, surface.rgba(9, 9), "past the fill");
}

#[test]
fn layers_with_an_effect_nest() {
    let Some(surface) = frame_of(|frame| {
        layers(frame).push_color_matrix_layer(AREA, &swap_red_and_blue(), ColorMatrixAlphaMode::Premultiplied);
        layers(frame).push_color_matrix_layer(AREA, &swap_red_and_blue(), ColorMatrixAlphaMode::Premultiplied);
        fill(frame, &solid(255, 0, 0), Rect::new(0.0, 0.0, 16.0, 16.0));
        layers(frame).pop_color_matrix_layer();
        layers(frame).pop_color_matrix_layer();
    }) else {
        return;
    };
    assert_eq!(RED, surface.rgba(8, 8), "swapped twice");
}

#[test]
fn a_blend_layer_composes_with_its_mode() {
    let Some(surface) = frame_of(|frame| {
        fill(frame, &solid(255, 0, 0), Rect::new(0.0, 0.0, 16.0, 16.0));
        layers(frame).push_blend_layer(AREA, BitmapBlendingMode::Multiply);
        fill(frame, &solid(0, 0, 255), AREA);
        layers(frame).pop_blend_layer();
    }) else {
        return;
    };
    assert_eq!(BLACK, surface.rgba(8, 8), "red times blue");
    assert_eq!(RED, surface.rgba(2, 2), "outside what the layer drew");
}

#[test]
fn a_multiply_layer_scales_the_colour_and_keeps_the_alpha() {
    let factors = MultiplyLayerFactors {
        size: PixelSize::new(1, 1),
        rgba: Arc::new(vec![255, 0, 255, 255]),
        scale: 1.0,
    };
    let Some(surface) = frame_of(|frame| {
        layers(frame).push_multiply_layer(AREA, &factors);
        fill(frame, &solid(255, 255, 0), AREA);
        layers(frame).pop_multiply_layer();
    }) else {
        return;
    };
    assert_eq!(RED, surface.rgba(8, 8), "green is multiplied away");
    assert_eq!(WHITE, surface.rgba(2, 2), "outside the bounds");
}

#[test]
fn a_kernel_that_changes_nothing_keeps_the_content() {
    let Some(surface) = frame_of(|frame| {
        layers(frame).push_convolve_matrix_layer(AREA, &ConvolveMatrix::default());
        fill(frame, &solid(0, 0, 255), AREA);
        layers(frame).pop_convolve_matrix_layer();
    }) else {
        return;
    };
    assert_eq!(BLUE, surface.rgba(8, 8));
}

#[test]
fn a_pixel_operation_combines_its_sources() {
    let Some(surface) = frame_of(|frame| {
        assert!(layers(frame).push_pixel_operation(AREA, &PixelOperation::MaskInvert));
        fill(frame, &solid(255, 0, 0), AREA);
        layers(frame).next_pixel_operation_source();
        fill(frame, &solid(0, 0, 0), AREA);
        layers(frame).pop_pixel_operation();
    }) else {
        return;
    };
    assert_eq!([0, 255, 255, 255], surface.rgba(8, 8), "the first source inverted where the second has alpha");
}

#[test]
fn the_luminance_of_the_content_becomes_its_alpha() {
    let Some(surface) = frame_of(|frame| {
        let effects = frame.as_drawing_context_impl_with_effects().expect("effects");
        effects.push_effect(Some(AREA), &ImmutableLuminanceToAlphaEffect::default());
        fill(frame, &solid(255, 255, 255), AREA);
        frame.as_drawing_context_impl_with_effects().expect("effects").pop_effect();
    }) else {
        return;
    };
    assert_eq!(BLACK, surface.rgba(8, 8), "white content is black at full alpha");
}

#[test]
fn lighting_is_evaluated_on_the_gpu() {
    let lighting =
        LightingParameters { constant: LightColor::BLACK, lights: Vec::new(), material: LightingMaterial::VISUAL };
    let Some(surface) = frame_of(|frame| {
        assert!(layers(frame).draw_lighting(AREA, &lighting));
    }) else {
        return;
    };
    assert_eq!(BLACK, surface.rgba(8, 8), "no light");
    assert_eq!(WHITE, surface.rgba(2, 2), "outside the area");
}

macro_rules! shader_effect {
    ($type:ident, $name:literal, $wgsl:expr) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        struct $type;

        impl CustomEffect for $type {
            const NAME: &'static str = $name;

            fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
                f("amount", 1.0.into());
            }

            fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
                false
            }

            fn shader() -> Option<ShaderEffectDescription> {
                Some(ShaderEffectDescription::content_sampling("", $wgsl))
            }
        }
    };
}

shader_effect!(
    SwapShader,
    "progpu-test-swap",
    "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, }\n\
     fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord).bgra * uniforms.amount; }"
);

shader_effect!(
    BrokenShader,
    "progpu-test-broken",
    "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, }\n\
     fn effect(coord: vec2<f32>) -> vec4<f32> { return undefined; }"
);

shader_effect!(
    WrongBlockShader,
    "progpu-test-wrong-block",
    "struct Uniforms { layer_scale: f32, layer_size: vec2<f32>, amount: f32, }\n\
     fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord); }"
);

fn draw_through(effect: &ImmutableCustomEffect) -> Option<Arc<MemorySurface>> {
    frame_of(|frame| {
        frame.as_drawing_context_impl_with_effects().expect("effects").push_effect(Some(AREA), effect);
        fill(frame, &solid(255, 0, 0), AREA);
        frame.as_drawing_context_impl_with_effects().expect("effects").pop_effect();
    })
}

#[test]
fn a_custom_effect_draws_its_content_through_its_program() {
    let Some(surface) = draw_through(&ImmutableCustomEffect::new(SwapShader)) else { return };
    assert_eq!(BLUE, surface.rgba(8, 8));
    assert_eq!(WHITE, surface.rgba(2, 2));
}

#[test]
fn a_program_that_does_not_compile_leaves_the_content_as_it_is() {
    let Some(surface) = draw_through(&ImmutableCustomEffect::new(BrokenShader)) else { return };
    assert_eq!(RED, surface.rgba(8, 8));
}

#[test]
fn a_program_with_other_uniforms_leaves_the_content_as_it_is() {
    let Some(surface) = draw_through(&ImmutableCustomEffect::new(WrongBlockShader)) else { return };
    assert_eq!(RED, surface.rgba(8, 8));
}

struct Material(AcrylicBackgroundSource);

impl IExperimentalAcrylicMaterial for Material {
    fn background_source(&self) -> AcrylicBackgroundSource {
        self.0
    }

    fn tint_color(&self) -> Color {
        Color::from_rgb(0, 0, 255)
    }

    fn tint_opacity(&self) -> f64 {
        1.0
    }

    fn material_color(&self) -> Color {
        Color::from_rgb(255, 0, 0)
    }

    fn fallback_color(&self) -> Color {
        Color::from_rgb(0, 255, 0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn an_acrylic_material_is_its_tint_with_a_faint_noise() {
    let Some(surface) = frame_of(|frame| {
        frame
            .as_drawing_context_with_acrylic_like_support()
            .expect("acrylic")
            .draw_rectangle_with_material(&Material(AcrylicBackgroundSource::None), RoundedRect::from_rect(AREA));
    }) else {
        return;
    };
    let [r, g, b, a] = surface.rgba(8, 8);
    assert!(r <= 12 && g <= 12 && b >= 235 && a == 255, "the tint under the noise: {r} {g} {b} {a}");
    assert_eq!(WHITE, surface.rgba(2, 2), "outside the rectangle");
}
