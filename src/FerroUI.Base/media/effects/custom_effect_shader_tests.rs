//! Tests of the runtime shader effects (#455, E4; not from upstream): the
//! description and its precedence below a graph, the uniform convention (the
//! value of each kind of parameter, the block of an effect, the names), the
//! padding of a sample radius, and the shader effects of the library: their
//! SkSL and WGSL declare the uniforms of the convention, they print and parse
//! back, are registered and prewarmed, and have CPU references. What Skia
//! draws is tested against those references in `ferroui-skia`.

use super::library::{PixelateEffect, ScanlineShaderEffect, SpotlightShaderEffect};
use super::*;
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{Color, Colors};
use crate::rendering::composition::effects::{
    effect_prewarm_names, CompositionEffectSourceParameter, GraphicsEffect, OpacityEffect,
};
use crate::{Point, Rect, Size, Thickness, Vector};

const SKSL: &str = "uniform float2 layer_size; uniform float layer_scale; uniform shader content;\n\
                    half4 main(float2 coord) { return content.eval(coord); }";
const WGSL: &str = "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, }\n\
                    fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord); }";

/// An effect with a parameter of every kind.
#[derive(Clone, Debug, PartialEq)]
struct Every {
    on: bool,
    count: i32,
    amount: f64,
    color: Color,
    point: Point,
    vector: Vector,
    size: Size,
    rect: Rect,
    label: std::sync::Arc<str>,
}

impl Default for Every {
    fn default() -> Self {
        Self {
            on: true,
            count: 3,
            amount: 0.25,
            color: Color::from_argb(128, 255, 0, 64),
            point: Point::new(1.0, 2.0),
            vector: Vector::new(-3.0, 4.5),
            size: Size::new(10.0, 20.0),
            rect: Rect::new(1.0, 2.0, 3.0, 4.0),
            label: "text".into(),
        }
    }
}

impl CustomEffect for Every {
    const NAME: &'static str = "shader-test-every";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("on", self.on.into());
        f("count", self.count.into());
        f("amount", self.amount.into());
        f("tint-color", self.color.into());
        f("point", self.point.into());
        f("vector", self.vector.into());
        f("size", self.size.into());
        f("rect", self.rect.into());
        f("label", self.label.clone().into());
    }

    fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
        false
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(ShaderEffectDescription::content_sampling(SKSL, WGSL))
    }
}

#[test]
fn a_description_is_content_sampling_or_an_overlay() {
    let sampling = ShaderEffectDescription::content_sampling(SKSL, WGSL);
    assert_eq!(ShaderEffectMode::ContentSampling, sampling.mode);
    assert!(sampling.samples_content());
    assert_eq!(SKSL, sampling.sksl);
    assert_eq!(WGSL, sampling.wgsl);
    assert_eq!(BitmapBlendingMode::SourceOver, sampling.blend_mode);
    assert_eq!(0.0, sampling.sample_radius);
    assert_eq!(ShaderEffectPadding::Effect, sampling.padding);
    assert!(sampling.children.is_empty());

    let overlay = ShaderEffectDescription::overlay(SKSL, WGSL, BitmapBlendingMode::Screen);
    assert_eq!(ShaderEffectMode::Overlay, overlay.mode);
    assert!(!overlay.samples_content());
    assert_eq!(BitmapBlendingMode::Screen, overlay.blend_mode);

    let built = sampling
        .clone()
        .with_sample_radius(12.5)
        .with_padding(ShaderEffectPadding::SampleRadius)
        .with_children(["page"]);
    assert_eq!(12.5, built.sample_radius);
    assert_eq!(ShaderEffectPadding::SampleRadius, built.padding);
    assert_eq!(vec![std::borrow::Cow::Borrowed("page")], built.children);
    assert_eq!(0.0, sampling.clone().with_sample_radius(-1.0).sample_radius);
    assert_eq!(0.0, sampling.with_sample_radius(f64::NAN).sample_radius);
}

#[test]
fn each_kind_of_parameter_is_a_uniform_of_the_convention() {
    let uniform = |value: EffectParameterValue| ShaderUniformValue::from_parameter(&value);
    assert_eq!(Some(ShaderUniformValue::Int(1)), uniform(EffectParameterValue::Bool(true)));
    assert_eq!(Some(ShaderUniformValue::Int(0)), uniform(EffectParameterValue::Bool(false)));
    assert_eq!(Some(ShaderUniformValue::Int(-7)), uniform(EffectParameterValue::Int(-7)));
    assert_eq!(Some(ShaderUniformValue::Float(0.5)), uniform(EffectParameterValue::Float(0.5)));
    // A colour is premultiplied, 0 to 1.
    let Some(ShaderUniformValue::Float4(color)) = uniform(Color::from_argb(51, 255, 0, 255).into()) else {
        panic!("a colour is a float4")
    };
    let expected = [0.2, 0.0, 0.2, 0.2];
    assert!(color.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-6), "{color:?}");
    assert_eq!(Some(ShaderUniformValue::Float4([0.0, 0.0, 0.0, 0.0])), uniform(Colors::TRANSPARENT.into()));
    assert_eq!(Some(ShaderUniformValue::Float2([1.0, 2.0])), uniform(Point::new(1.0, 2.0).into()));
    assert_eq!(Some(ShaderUniformValue::Float2([-3.0, 4.5])), uniform(Vector::new(-3.0, 4.5).into()));
    assert_eq!(Some(ShaderUniformValue::Float2([10.0, 20.0])), uniform(Size::new(10.0, 20.0).into()));
    assert_eq!(Some(ShaderUniformValue::Float4([1.0, 2.0, 3.0, 4.0])), uniform(Rect::new(1.0, 2.0, 3.0, 4.0).into()));
    // Text has no uniform.
    assert_eq!(None, uniform(EffectParameterValue::from("name")));

    assert_eq!(ShaderUniformType::Float2, ShaderUniformValue::Float2([0.0; 2]).ty());
    assert_eq!(&[1.0, 2.0][..], ShaderUniformValue::Float2([1.0, 2.0]).floats());
    assert!(ShaderUniformValue::Int(3).floats().is_empty());
    assert_eq!(("float4", "vec4<f32>", 4), {
        let ty = ShaderUniformType::Float4;
        (ty.sksl(), ty.wgsl(), ty.components())
    });
}

#[test]
fn the_uniform_block_is_the_layer_then_the_parameters_in_order() {
    let effect = ImmutableCustomEffect::new(Every::default());
    let mut uniforms = Vec::new();
    effect.for_each_shader_uniform([200.0, 100.0], 2.0, &mut |name, value| uniforms.push((name, value)));
    let alpha = 128.0 / 255.0;
    assert_eq!(
        vec![
            (SHADER_LAYER_SIZE, ShaderUniformValue::Float2([200.0, 100.0])),
            (SHADER_LAYER_SCALE, ShaderUniformValue::Float(2.0)),
            ("on", ShaderUniformValue::Int(1)),
            ("count", ShaderUniformValue::Int(3)),
            ("amount", ShaderUniformValue::Float(0.25)),
            ("tint-color", ShaderUniformValue::Float4([alpha, 0.0, 64.0 / 255.0 * alpha, alpha])),
            ("point", ShaderUniformValue::Float2([1.0, 2.0])),
            ("vector", ShaderUniformValue::Float2([-3.0, 4.5])),
            ("size", ShaderUniformValue::Float2([10.0, 20.0])),
            ("rect", ShaderUniformValue::Float4([1.0, 2.0, 3.0, 4.0])),
        ],
        uniforms
    );

    // The declarations a program checks against: `-` is written `_`.
    use ShaderUniformType::*;
    let expected: Vec<(String, ShaderUniformType)> = [
        ("layer_size", Float2),
        ("layer_scale", Float),
        ("on", Int),
        ("count", Int),
        ("amount", Float),
        ("tint_color", Float4),
        ("point", Float2),
        ("vector", Float2),
        ("size", Float2),
        ("rect", Float4),
    ]
    .into_iter()
    .map(|(name, ty)| (name.to_owned(), ty))
    .collect();
    assert_eq!(expected, effect.shader_uniform_layout());
}

#[test]
fn a_uniform_is_named_as_its_parameter_with_underscores() {
    assert_eq!("cell_size", shader_uniform_name("cell-size"));
    assert_eq!("amount", shader_uniform_name("amount"));
    assert!(shader_uniform_name_matches("cell_size", "cell-size"));
    assert!(shader_uniform_name_matches("cell_size", "cell_size"));
    assert!(!shader_uniform_name_matches("cell-size", "cell_size"));
    assert!(!shader_uniform_name_matches("cellsize", "cell-size"));
    assert!(!shader_uniform_name_matches("cell_sizes", "cell-size"));
}

/// An effect with both a graph and a shader: the graph is drawn.
#[derive(Clone, Debug, Default, PartialEq)]
struct Both;

impl CustomEffect for Both {
    const NAME: &'static str = "shader-test-both";

    fn for_each_parameter(&self, _: &mut dyn FnMut(&'static str, EffectParameterValue)) {}

    fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
        false
    }

    fn graph() -> Option<GraphicsEffect> {
        let content = CompositionEffectSourceParameter::new(CUSTOM_EFFECT_CONTENT);
        Some(OpacityEffect { name: "Opacity".into(), opacity: 0.5, source: Some(content.into()) }.into())
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(ShaderEffectDescription::content_sampling(SKSL, WGSL))
    }
}

/// An effect without a graph or a shader.
#[derive(Clone, Debug, Default, PartialEq)]
struct Neither;

impl CustomEffect for Neither {
    const NAME: &'static str = "shader-test-neither";

    fn for_each_parameter(&self, _: &mut dyn FnMut(&'static str, EffectParameterValue)) {}

    fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
        false
    }
}

#[test]
fn a_graph_takes_precedence_over_a_shader() {
    let both = ImmutableCustomEffect::new(Both);
    assert!(both.has_graph());
    assert!(!both.has_shader());
    assert_eq!(None, both.shader());

    let every = ImmutableCustomEffect::new(Every::default());
    assert!(!every.has_graph());
    assert!(every.has_shader());
    assert_eq!(Some(ShaderEffectDescription::content_sampling(SKSL, WGSL)), every.shader().as_deref().cloned());
    // Read once per type: the same description.
    assert!(std::sync::Arc::ptr_eq(&every.shader().unwrap(), &every.shader().unwrap()));

    assert!(!ImmutableCustomEffect::new(Neither).has_shader());
}

/// An effect that moves its content outward by its sample radius.
#[derive(Clone, Debug, Default, PartialEq)]
struct Spread;

impl CustomEffect for Spread {
    const NAME: &'static str = "shader-test-spread";

    fn for_each_parameter(&self, _: &mut dyn FnMut(&'static str, EffectParameterValue)) {}

    fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
        false
    }

    fn padding(&self) -> Thickness {
        Thickness::new(1.0, 2.0, 3.0, 4.0)
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::content_sampling(SKSL, WGSL)
                .with_sample_radius(5.0)
                .with_padding(ShaderEffectPadding::SampleRadius),
        )
    }
}

#[test]
fn a_sample_radius_padding_adds_the_radius_to_each_side() {
    assert_eq!(Thickness::new(6.0, 7.0, 8.0, 9.0), ImmutableCustomEffect::new(Spread).padding());
    // The padding of the effect alone otherwise.
    assert_eq!(Thickness::default(), ImmutableCustomEffect::new(PixelateEffect::default()).padding());
}

// --- the library ---------------------------------------------------------------

/// The shader effects of the library at values that are not neutral.
fn library_shader_effects() -> Vec<ImmutableCustomEffect> {
    vec![
        ImmutableCustomEffect::new(PixelateEffect { cell_size: 6.0 }),
        ImmutableCustomEffect::new(ScanlineShaderEffect { spacing: 4.0, strength: 0.5 }),
        ImmutableCustomEffect::new(SpotlightShaderEffect {
            center_x: 0.5,
            center_y: 0.5,
            radius: 0.5,
            strength: 0.8,
            color: Colors::ORANGE,
        }),
    ]
}

/// The uniforms an SkSL program declares, in order (`uniform <type> <name>;`),
/// and the names of its children (`uniform shader <name>;`).
fn sksl_declarations(source: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut uniforms = Vec::new();
    let mut children = Vec::new();
    for statement in source.split(';') {
        let words: Vec<&str> = statement.split_whitespace().collect();
        if let ["uniform", ty, name] = words[..] {
            if ty == "shader" {
                children.push(name.to_owned());
            } else {
                uniforms.push((name.to_owned(), ty.to_owned()));
            }
        }
    }
    (uniforms, children)
}

/// The members of `struct Uniforms` of a WGSL program, in order.
fn wgsl_uniforms(source: &str) -> Vec<(String, String)> {
    let start = source.find("struct Uniforms {").expect("a struct Uniforms") + "struct Uniforms {".len();
    let end = start + source[start..].find('}').expect("the end of the struct");
    source[start..end]
        .split(',')
        .filter_map(|member| {
            let (name, ty) = member.split_once(':')?;
            Some((name.trim().to_owned(), ty.trim().to_owned()))
        })
        .collect()
}

#[test]
fn the_programs_of_the_library_declare_the_uniforms_of_their_parameters() {
    for effect in library_shader_effects() {
        let shader = effect.shader().unwrap_or_else(|| panic!("{effect} is a shader effect"));
        let layout = effect.shader_uniform_layout();
        let (uniforms, children) = sksl_declarations(&shader.sksl);
        let expected: Vec<(String, String)> =
            layout.iter().map(|(name, ty)| (name.clone(), ty.sksl().to_owned())).collect();
        assert_eq!(expected, uniforms, "{effect}: the uniforms of the SkSL");
        let wanted_children = if shader.samples_content() { vec![CUSTOM_EFFECT_CONTENT.to_owned()] } else { Vec::new() };
        assert_eq!(wanted_children, children, "{effect}: the children of the SkSL");
        assert!(shader.sksl.contains("half4 main(float2 coord)"), "{effect}");

        let expected: Vec<(String, String)> =
            layout.iter().map(|(name, ty)| (name.clone(), ty.wgsl().to_owned())).collect();
        assert_eq!(expected, wgsl_uniforms(&shader.wgsl), "{effect}: the uniforms of the WGSL");
        assert!(shader.wgsl.contains("fn effect(coord: vec2<f32>) -> vec4<f32>"), "{effect}");
        assert_eq!(shader.samples_content(), shader.wgsl.contains("sample_content("), "{effect}");
        assert!(shader.children.is_empty(), "{effect}");
    }
    let modes: Vec<(ShaderEffectMode, BitmapBlendingMode)> = library_shader_effects()
        .iter()
        .map(|effect| effect.shader().map(|shader| (shader.mode, shader.blend_mode)).unwrap())
        .collect();
    assert_eq!(
        vec![
            (ShaderEffectMode::ContentSampling, BitmapBlendingMode::SourceOver),
            (ShaderEffectMode::Overlay, BitmapBlendingMode::SourceOver),
            (ShaderEffectMode::Overlay, BitmapBlendingMode::Screen),
        ],
        modes
    );
}

#[test]
fn the_shader_effects_print_parse_back_and_are_registered() {
    let neutral = library_shader_effects().into_iter().map(|effect| effect.neutral().expect("a neutral value"));
    for effect in library_shader_effects().into_iter().chain(neutral) {
        let text = effect.to_string();
        let parsed = Effect::parse(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
        assert_eq!(Some(&effect), parsed.as_custom_effect(), "{text}");
    }
    let parsed = Effect::parse("pixelate(cell_size=12)").unwrap();
    let pixelate = parsed.as_custom_effect().and_then(|effect| effect.downcast_ref::<PixelateEffect>()).cloned();
    assert_eq!(Some(PixelateEffect { cell_size: 12.0 }), pixelate);

    let names = CustomEffectRegistry::names();
    for name in ["pixelate", "scanline", "spotlight"] {
        assert!(names.contains(&name), "{name}: {names:?}");
        assert!(effect_prewarm_names().contains(&format!("custom-effect:{name}")), "{name}");
    }
}

/// A 6 x 4 image of premultiplied colours: opaque, translucent and
/// transparent pixels.
fn sample_pixels() -> Vec<u8> {
    (0..24u8)
        .flat_map(|index| match index % 4 {
            0 => [255, index * 10, 0, 255],
            1 => [0, 0, index * 10, index * 10],
            2 => [0, 0, 0, 0],
            _ => [index * 5, index * 5, index * 5, 200],
        })
        .collect()
}

#[test]
fn a_neutral_shader_effect_leaves_the_content_unchanged() {
    for effect in library_shader_effects() {
        let neutral = effect.neutral().expect("a neutral value");
        for scale in [1.0, 2.0] {
            let mut pixels = sample_pixels();
            assert!(neutral.cpu_reference(&mut pixels, 6, 4, scale), "{neutral}: a reference");
            assert_eq!(sample_pixels(), pixels, "{neutral} at {scale}");
        }
    }
}

#[test]
fn the_reference_of_pixelate_repeats_the_pixel_at_the_centre_of_each_cell() {
    let width = 6;
    let pixel = |pixels: &[u8], x: usize, y: usize| -> [u8; 4] { pixels[(y * width + x) * 4..][..4].try_into().unwrap() };
    let original = sample_pixels();
    let mut pixels = original.clone();
    ImmutableCustomEffect::new(PixelateEffect { cell_size: 2.0 }).cpu_reference(&mut pixels, 6, 4, 1.0);
    // Cells of 2 x 2 pixels: the centre (1, 1) of the first is its pixel (1, 1).
    for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(pixel(&original, 1, 1), pixel(&pixels, x, y));
    }
    assert_eq!(pixel(&original, 3, 3), pixel(&pixels, 2, 2));
    // At a scale of 2 the cell is 4 device pixels; the last cell is cut by
    // the edge and shows the pixel inside it nearest to its centre.
    let mut scaled = original.clone();
    ImmutableCustomEffect::new(PixelateEffect { cell_size: 2.0 }).cpu_reference(&mut scaled, 6, 4, 2.0);
    assert_eq!(pixel(&original, 2, 2), pixel(&scaled, 0, 0));
    assert_eq!(pixel(&original, 2, 2), pixel(&scaled, 3, 3));
    assert_eq!(pixel(&original, 5, 2), pixel(&scaled, 4, 0));
}

#[test]
fn the_reference_of_the_scan_lines_darkens_the_lower_half_of_each_band() {
    let (width, height) = (2, 8);
    let white = vec![255u8; width * height * 4];
    let mut pixels = white.clone();
    let effect = ImmutableCustomEffect::new(ScanlineShaderEffect { spacing: 4.0, strength: 0.5 });
    assert!(effect.cpu_reference(&mut pixels, width, height, 1.0));
    for y in 0..height {
        let row = &pixels[y * width * 4..][..4];
        if y % 4 >= 2 {
            assert_eq!([128, 128, 128, 255], row, "row {y}");
        } else {
            assert_eq!([255, 255, 255, 255], row, "row {y}");
        }
    }
    // Over transparent pixels the band is translucent black.
    let mut clear = vec![0u8; 4 * 4];
    effect.cpu_reference(&mut clear, 1, 4, 1.0);
    assert_eq!(&[0, 0, 0, 128], &clear[8..12]);
}

#[test]
fn the_reference_of_the_spotlight_is_brightest_at_its_centre() {
    let (width, height) = (20, 20);
    let mut pixels = vec![0u8; width * height * 4];
    let effect = ImmutableCustomEffect::new(SpotlightShaderEffect {
        center_x: 0.5,
        center_y: 0.5,
        radius: 0.5,
        strength: 1.0,
        color: Colors::WHITE,
    });
    assert!(effect.cpu_reference(&mut pixels, width, height, 1.0));
    let alpha = |x: usize, y: usize| pixels[(y * width + x) * 4 + 3];
    assert!(alpha(10, 10) > 230, "{}", alpha(10, 10));
    assert!(alpha(10, 10) > alpha(14, 10) && alpha(14, 10) > alpha(18, 10));
    // Beyond the radius, nothing.
    assert_eq!(0, alpha(0, 0));
    // Screened over opaque black, the light is grey; over white it is white.
    let mut black = [0, 0, 0, 255].repeat(width * height);
    effect.cpu_reference(&mut black, width, height, 1.0);
    let center = &black[(10 * width + 10) * 4..][..4];
    assert!(center[0] > 230 && center[0] == center[1] && center[3] == 255, "{center:?}");
    let mut white = vec![255u8; width * height * 4];
    effect.cpu_reference(&mut white, width, height, 1.0);
    assert!(white.iter().all(|&value| value == 255));
}
