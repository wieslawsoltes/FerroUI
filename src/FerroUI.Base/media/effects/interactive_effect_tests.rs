//! Tests of the interactive effects of the library (#455, E7; not from upstream): the four values
//! are shader effects of the convention (their SkSL and WGSL declare the uniforms of their
//! parameters), print and parse back, are registered and prewarmed, leave the content unchanged at
//! their neutral values, and have CPU references that draw what they describe; the classes take
//! their snapshots from their values, invalidate only on a change, and advance their animations.
//! The routing of the pointer input to the classes is tested with a test root in
//! `input/effect_input_tests.rs`; what the backends draw against the CPU references in
//! `ferroui-skia` and `ferroui-vello`.

use super::library::{
    BurningFlameShaderEffect, InteractiveBurningFlameEffect, InteractivePointerSpotlightEffect,
    InteractiveReactiveGridEffect, InteractiveWaterRippleEffect, PointerSpotlightShaderEffect, ReactiveGridShaderEffect,
    RippleWave, WaterRippleShaderEffect,
};
use super::*;
use crate::media::imaging::BitmapBlendingMode;
use crate::media::Colors;
use crate::rendering::composition::effects::effect_prewarm_names;
use crate::Point;
use std::cell::Cell;
use std::rc::Rc;

/// The interactive effects at values that draw something.
fn interactive_effects() -> Vec<ImmutableCustomEffect> {
    let mut ripple = WaterRippleShaderEffect { is_pointer_over: true, pointer_x: 0.3, ..WaterRippleShaderEffect::default() };
    ripple.spawn_ripple(Point::new(0.5, 0.5), 1.0);
    ripple.ripples[0].age = 0.3;
    vec![
        ImmutableCustomEffect::new(PointerSpotlightShaderEffect {
            pointer_x: 0.4,
            pointer_y: 0.6,
            is_pointer_over: true,
            ..PointerSpotlightShaderEffect::default()
        }),
        ImmutableCustomEffect::new(ReactiveGridShaderEffect {
            is_pointer_over: true,
            is_pressed: true,
            ..ReactiveGridShaderEffect::default()
        }),
        ImmutableCustomEffect::new(ripple),
        ImmutableCustomEffect::new(BurningFlameShaderEffect {
            burn_amount: 0.8,
            flame_phase: 1.2,
            ..BurningFlameShaderEffect::default()
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
fn the_interactive_effects_are_shader_effects_of_the_convention() {
    for effect in interactive_effects() {
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
        // The flags are `int` uniforms.
        assert!(layout.iter().any(|(name, ty)| name == "is_pressed" && *ty == ShaderUniformType::Int), "{effect}");
    }
    let modes: Vec<(ShaderEffectMode, BitmapBlendingMode, f64, bool)> = interactive_effects()
        .iter()
        .map(|effect| {
            effect
                .shader()
                .map(|shader| (shader.mode, shader.blend_mode, shader.sample_radius, shader.masked_by_content))
                .unwrap()
        })
        .collect();
    // The overlays are masked by the content (#580); the programs that move the content compose
    // with it themselves.
    assert_eq!(
        vec![
            (ShaderEffectMode::Overlay, BitmapBlendingMode::Screen, 0.0, true),
            (ShaderEffectMode::Overlay, BitmapBlendingMode::SourceOver, 0.0, true),
            (ShaderEffectMode::ContentSampling, BitmapBlendingMode::SourceOver, WaterRippleShaderEffect::SAMPLE_RADIUS, false),
            (ShaderEffectMode::ContentSampling, BitmapBlendingMode::SourceOver, BurningFlameShaderEffect::SAMPLE_RADIUS, false),
        ],
        modes
    );
    // Effector's sample radii: the largest moves of the programs.
    assert_eq!(56.0, WaterRippleShaderEffect::SAMPLE_RADIUS);
    assert_eq!(21.0, BurningFlameShaderEffect::SAMPLE_RADIUS);
    // The ripples are a point and two numbers each, in order.
    let layout = ImmutableCustomEffect::new(WaterRippleShaderEffect::default()).shader_uniform_layout();
    let names: Vec<&str> = layout.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        &["ripple_1_center", "ripple_1_age", "ripple_1_strength", "ripple_2_center"],
        &names[6..10],
        "{names:?}"
    );
    assert_eq!(ShaderUniformType::Float2, layout[6].1);
}

#[test]
fn the_interactive_effects_print_parse_back_and_are_registered() {
    let neutral = interactive_effects().into_iter().map(|effect| effect.neutral().expect("a neutral value"));
    for effect in interactive_effects().into_iter().chain(neutral) {
        let text = effect.to_string();
        let parsed = Effect::parse(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
        assert_eq!(Some(&effect), parsed.as_custom_effect(), "{text}");
    }
    let parsed = Effect::parse("water-ripple(ripple-1-center=0.25,0.75, ripple_1_age=0.5)").unwrap();
    let ripple = parsed.as_custom_effect().and_then(|effect| effect.downcast_ref::<WaterRippleShaderEffect>()).cloned();
    let ripple = ripple.expect("a water ripple");
    assert_eq!(Point::new(0.25, 0.75), ripple.ripples[0].center);
    assert_eq!(0.5, ripple.ripples[0].age);
    let parsed = Effect::parse("pointer-spotlight(is-pointer-over=true, pointer-x=0.2)").unwrap();
    let spotlight =
        parsed.as_custom_effect().and_then(|effect| effect.downcast_ref::<PointerSpotlightShaderEffect>()).cloned();
    assert_eq!(
        Some(PointerSpotlightShaderEffect { is_pointer_over: true, pointer_x: 0.2, ..PointerSpotlightShaderEffect::default() }),
        spotlight
    );

    let names = CustomEffectRegistry::names();
    for name in ["pointer-spotlight", "reactive-grid", "water-ripple", "burning-flame"] {
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
fn a_neutral_interactive_effect_leaves_the_content_unchanged() {
    let defaults = [
        ImmutableCustomEffect::new(PointerSpotlightShaderEffect::default()),
        ImmutableCustomEffect::new(WaterRippleShaderEffect::default()),
        ImmutableCustomEffect::new(BurningFlameShaderEffect::default()),
    ];
    let neutral = interactive_effects().into_iter().map(|effect| effect.neutral().expect("a neutral value"));
    // The pointer away and nothing burning or rippling is neutral too.
    for effect in neutral.chain(defaults) {
        for scale in [1.0, 2.0] {
            let mut pixels = sample_pixels();
            assert!(effect.cpu_reference(&mut pixels, 6, 4, scale), "{effect}: a reference");
            assert_eq!(sample_pixels(), pixels, "{effect} at {scale}");
        }
    }
}

/// The alpha of the pixel (`x`, `y`) of an image `width` wide.
fn alpha(pixels: &[u8], width: usize, x: usize, y: usize) -> u8 {
    pixels[(y * width + x) * 4 + 3]
}

/// The red channel of the pixel (`x`, `y`) of an image `width` wide.
fn red(pixels: &[u8], width: usize, x: usize, y: usize) -> u8 {
    pixels[(y * width + x) * 4]
}

/// A `width` by `height` image of opaque dark grey: the content the overlays are drawn over.
fn dark(width: usize, height: usize) -> Vec<u8> {
    [40u8, 40, 40, 255].repeat(width * height)
}

#[test]
fn the_reference_of_the_pointer_spotlight_follows_the_pointer() {
    let (width, height) = (40, 40);
    let lit = |effect: PointerSpotlightShaderEffect| {
        let mut pixels = dark(width, height);
        assert!(ImmutableCustomEffect::new(effect).cpu_reference(&mut pixels, width, height, 1.0));
        pixels
    };
    let over =
        PointerSpotlightShaderEffect { pointer_x: 0.25, pointer_y: 0.25, is_pointer_over: true, ..PointerSpotlightShaderEffect::default() };
    let hovered = lit(over.clone());
    // Brightest under the pointer, nothing far from it.
    assert!(red(&hovered, width, 10, 10) > red(&hovered, width, 14, 10));
    assert_eq!(40, red(&hovered, width, 35, 35));
    // Pressed, the light is brighter.
    let pressed = lit(PointerSpotlightShaderEffect { is_pressed: true, ..over.clone() });
    assert!(red(&pressed, width, 10, 10) > red(&hovered, width, 10, 10));
    // The pointer away: nothing.
    assert_eq!(dark(width, height), lit(PointerSpotlightShaderEffect { is_pointer_over: false, ..over }));
}

/// The overlays are masked by the content (#580): over transparent pixels they draw nothing,
/// over translucent ones in proportion to their alpha, so a light or a line never shows in the
/// transparent corners of a rounded host or the gaps between its children.
#[test]
fn an_interactive_overlay_over_transparent_content_leaves_it_transparent() {
    let (width, height) = (44, 44);
    let overlays = [
        ImmutableCustomEffect::new(PointerSpotlightShaderEffect {
            is_pointer_over: true,
            is_pressed: true,
            strength: 1.0,
            ..PointerSpotlightShaderEffect::default()
        }),
        ImmutableCustomEffect::new(ReactiveGridShaderEffect {
            is_pointer_over: true,
            is_pressed: true,
            strength: 1.0,
            ..ReactiveGridShaderEffect::default()
        }),
    ];
    for effect in overlays {
        for scale in [1.0, 2.0] {
            let mut clear = vec![0u8; width * height * 4];
            assert!(effect.cpu_reference(&mut clear, width, height, scale));
            assert!(clear.iter().all(|&value| value == 0), "{effect} at {scale}: drawn over transparent content");
            // Over opaque content it draws, over translucent content less.
            let mut opaque = dark(width, height);
            effect.cpu_reference(&mut opaque, width, height, scale);
            let mut translucent = [20u8, 20, 20, 128].repeat(width * height);
            effect.cpu_reference(&mut translucent, width, height, scale);
            let (x, y) = (22, 22);
            assert!(red(&opaque, width, x, y) > 40, "{effect} at {scale}");
            assert!(red(&translucent, width, x, y) > 20, "{effect} at {scale}");
            assert!(red(&translucent, width, x, y) - 20 < red(&opaque, width, x, y) - 40, "{effect} at {scale}");
        }
    }
}

#[test]
fn the_reference_of_the_reactive_grid_draws_lines_and_a_halo() {
    let (width, height) = (44, 44);
    let draw = |effect: ReactiveGridShaderEffect, scale: f64| {
        let mut pixels = dark(width, height);
        assert!(ImmutableCustomEffect::new(effect).cpu_reference(&mut pixels, width, height, scale));
        pixels
    };
    let lit = |pixels: &[u8], x: usize, y: usize| red(pixels, width, x, y) > 40;
    let away = draw(ReactiveGridShaderEffect::default(), 1.0);
    // Lines along the first pixel of each cell of 22 (5 percent of 22 is 1.1 pixels).
    for (x, y) in [(0, 5), (22, 5), (5, 0), (5, 22)] {
        assert!(lit(&away, x, y), "({x}, {y})");
    }
    for (x, y) in [(1, 5), (21, 5), (5, 23), (10, 10)] {
        assert!(!lit(&away, x, y), "({x}, {y})");
    }
    // At a scale of 2 a cell is 44 device pixels and a line 2.
    let scaled = draw(ReactiveGridShaderEffect::default(), 2.0);
    assert!(lit(&scaled, 1, 5));
    assert!(!lit(&scaled, 2, 5));
    assert!(!lit(&scaled, 22, 5));
    // The halo under the pointer.
    let over = draw(ReactiveGridShaderEffect { is_pointer_over: true, ..ReactiveGridShaderEffect::default() }, 1.0);
    assert!(lit(&over, 20, 20));
    assert!(!lit(&away, 20, 20));
}

/// The lines of the reactive grid are 5 percent of a cell wide, as Effector's (`gridX < 0.05`),
/// not a pixel: a cell of 40 has lines of 2 pixels, one of 64 lines of 3 (#580).
#[test]
fn the_lines_of_the_reactive_grid_are_a_twentieth_of_a_cell() {
    let (width, height) = (130, 4);
    let columns = |cell_size: f64, scale: f64| -> Vec<usize> {
        let mut pixels = dark(width, height);
        let effect = ReactiveGridShaderEffect { cell_size, strength: 1.0, ..ReactiveGridShaderEffect::default() };
        assert!(ImmutableCustomEffect::new(effect).cpu_reference(&mut pixels, width, height, scale));
        // The second row: below the horizontal line at the top of the first cells.
        (0..width).filter(|&x| red(&pixels, width, x, height - 1) > 40).collect()
    };
    assert_eq!(vec![0, 1, 40, 41, 80, 81, 120, 121], columns(40.0, 1.0));
    assert_eq!(vec![0, 1, 2, 64, 65, 66, 128, 129], columns(64.0, 1.0));
    // At a scale of 2 a cell of 22 is 44 device pixels and its lines 2.2: 2 pixels.
    assert_eq!(vec![0, 1, 44, 45, 88, 89], columns(22.0, 2.0));
    assert_eq!(0.05, ReactiveGridShaderEffect::LINE_WIDTH);
}

/// A `width` by `height` image of opaque vertical stripes, two pixels wide.
fn stripes(width: usize, height: usize) -> Vec<u8> {
    (0..width * height).flat_map(|index| if (index % width) / 2 % 2 == 0 { [255, 0, 0, 255] } else { [0, 0, 255, 255] }).collect()
}

#[test]
fn the_reference_of_the_water_ripple_bends_the_content_under_its_rings() {
    let (width, height) = (60, 60);
    let mut effect = WaterRippleShaderEffect::default();
    effect.spawn_ripple(Point::new(0.5, 0.5), 1.0);
    effect.ripples[0].age = 0.3;
    let mut pixels = stripes(width, height);
    assert!(ImmutableCustomEffect::new(effect.clone()).cpu_reference(&mut pixels, width, height, 1.0));
    let original = stripes(width, height);
    // The ring is at 0.3 of 0.72 of the side from the centre: about 13 pixels; the content there
    // changes, the corners do not.
    let at = |pixels: &[u8], x: usize, y: usize| pixels[(y * width + x) * 4..][..4].to_vec();
    let ring_changed = (0..width).any(|x| at(&pixels, x, 30 - 13) != at(&original, x, 30 - 13));
    assert!(ring_changed, "the ring moves or tints the content");
    assert_eq!(at(&original, 1, 1), at(&pixels, 1, 1));
    // The content stays opaque, and transparent content stays transparent.
    assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
    let mut clear = vec![0u8; width * height * 4];
    ImmutableCustomEffect::new(effect).cpu_reference(&mut clear, width, height, 1.0);
    assert!(clear.iter().all(|&value| value == 0));
}

/// The water ripple samples the moved content clamped to the area and composes over the content
/// masked by it (#580): a ring near the edge of opaque content moves the texels of the edge in,
/// where they were transparent before, and what is transparent stays transparent however the
/// content beside it is moved.
#[test]
fn a_ripple_samples_inside_the_area_and_stays_within_the_content() {
    let (width, height) = (60, 40);
    let green = [0u8, 255, 0, 255];
    // Stripes with a green first column: the edge the ring pushes the content beyond.
    let content: Vec<u8> = stripes(width, height)
        .chunks_exact(4)
        .enumerate()
        .flat_map(|(index, pixel)| if index % width == 0 { green } else { [pixel[0], pixel[1], pixel[2], pixel[3]] })
        .collect();
    let mut effect = WaterRippleShaderEffect { distortion: 18.0, ..WaterRippleShaderEffect::default() };
    // A ring of about 5 pixels around (6, 20): left of its centre it moves the content 10 to 18
    // pixels to the left, beyond the left edge.
    effect.spawn_ripple(Point::new(0.1, 0.5), 1.0);
    effect.ripples[0].age = 0.12;
    let mut pixels = content.clone();
    assert!(ImmutableCustomEffect::new(effect).cpu_reference(&mut pixels, width, height, 1.0));
    let at = |pixels: &[u8], x: usize, y: usize| -> [u8; 4] { pixels[(y * width + x) * 4..][..4].try_into().unwrap() };
    assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 255), "opaque content stays opaque");
    // The pixels the ring moves beyond the edge show the edge (green), not themselves.
    let edge_shown = (16..25).any(|y| (1..4).any(|x| {
        let pixel = at(&pixels, x, y);
        at(&content, x, y) != green && pixel[1] > pixel[0] && pixel[1] > pixel[2]
    }));
    assert!(edge_shown, "the moved content is sampled at the edge of the area");

    // The left half opaque, the right half transparent, a ring in the right half: it pulls the
    // opaque content towards the transparent half, which stays transparent.
    let content: Vec<u8> = (0..width * height)
        .flat_map(|index| if index % width < width / 2 { [0, 160, 80, 255] } else { [0, 0, 0, 0] })
        .collect();
    let mut ripple = WaterRippleShaderEffect { distortion: 18.0, ..WaterRippleShaderEffect::default() };
    ripple.spawn_ripple(Point::new(0.6, 0.5), 1.0);
    ripple.ripples[0].age = 0.15;
    let mut pixels = content.clone();
    assert!(ImmutableCustomEffect::new(ripple).cpu_reference(&mut pixels, width, height, 1.0));
    for (index, (after, before)) in pixels.chunks_exact(4).zip(content.chunks_exact(4)).enumerate() {
        let (x, y) = (index % width, index / width);
        if before[3] == 0 {
            assert_eq!([0, 0, 0, 0], after, "pixel ({x}, {y}) stays transparent");
        } else {
            assert_eq!(255, after[3], "pixel ({x}, {y}) stays opaque");
        }
    }
}

#[test]
fn the_reference_of_the_burning_flame_burns_above_the_ignition() {
    let (width, height) = (60, 60);
    let effect = BurningFlameShaderEffect { burn_amount: 1.0, flame_phase: 0.7, ..BurningFlameShaderEffect::default() };
    let original = [40u8, 60, 200, 255].repeat(width * height);
    let mut pixels = original.clone();
    assert!(ImmutableCustomEffect::new(effect.clone()).cpu_reference(&mut pixels, width, height, 1.0));
    let at = |pixels: &[u8], x: usize, y: usize| pixels[(y * width + x) * 4..][..4].to_vec();
    // At the ignition (0.5, 0.68) the content takes the colours of the flame: redder.
    let ignition = at(&pixels, 30, 41);
    assert!(ignition[0] > 40 + 60, "{ignition:?}");
    // The flame does not reach the bottom corners.
    assert_eq!(at(&original, 1, 58), at(&pixels, 1, 58));
    // The content stays opaque everywhere: where the flame moves the content up from beyond the
    // top, it samples the top row (clamped to the area, #580), so it cuts no notches into it.
    for y in 0..height {
        for x in 0..width {
            assert_eq!(255, at(&pixels, x, y)[3], "({x}, {y})");
        }
    }
    // Ignited at the top, the flame still keeps the top edge opaque, and it burns nothing
    // beyond the coverage of the content.
    let top = BurningFlameShaderEffect { ignition_y: 0.18, ..effect.clone() };
    let mut pixels = original.clone();
    ImmutableCustomEffect::new(top.clone()).cpu_reference(&mut pixels, width, height, 1.0);
    assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
    let banded: Vec<u8> = (0..width * height).flat_map(|index| if index / width < 30 { [0, 0, 0, 0] } else { [40, 60, 200, 255] }).collect();
    let mut pixels = banded.clone();
    ImmutableCustomEffect::new(top).cpu_reference(&mut pixels, width, height, 1.0);
    assert!(pixels[..30 * width * 4].iter().all(|&value| value == 0), "the transparent top half stays transparent");
    let mut clear = vec![0u8; width * height * 4];
    ImmutableCustomEffect::new(effect).cpu_reference(&mut clear, width, height, 1.0);
    assert!(clear.iter().all(|&value| value == 0));
}

#[test]
fn ripples_spawn_age_and_fade() {
    let ripple = RippleWave::new(Point::new(1.5, -0.5), 2.0);
    assert_eq!(RippleWave { center: Point::new(1.0, 0.0), age: 0.0, strength: 1.0 }, ripple);
    assert!(ripple.is_active());
    assert_eq!(0.5, ripple.advance(0.5).age);
    assert!(!ripple.advance(RippleWave::LIFETIME + 0.1).is_active());
    assert!(!RippleWave::INACTIVE.is_active());
    assert_eq!(RippleWave::INACTIVE, RippleWave::INACTIVE.advance(1.0));
    assert_eq!(RippleWave::INACTIVE, RippleWave::default());

    let mut effect = WaterRippleShaderEffect::default();
    assert!(!effect.has_active_ripples());
    effect.spawn_ripple(Point::new(0.1, 0.1), 1.0);
    effect.spawn_ripple(Point::new(0.2, 0.2), 0.5);
    effect.spawn_ripple(Point::new(0.3, 0.3), 0.25);
    effect.spawn_ripple(Point::new(0.4, 0.4), 0.125);
    let centers: Vec<f64> = effect.ripples.iter().map(|ripple| ripple.center.x).collect();
    assert_eq!(vec![0.4, 0.3, 0.2], centers, "the newest first, the oldest dropped");
    assert!(effect.has_active_ripples());
}

#[test]
fn the_snapshot_of_an_interactive_class_is_its_value_and_a_change_invalidates_it() {
    let effect = InteractivePointerSpotlightEffect::new();
    assert!(effect.is_interactive() && effect.is_custom());
    assert!(effect.input_handler().is_some());
    let invalidations = Rc::new(Cell::new(0));
    let counted = invalidations.clone();
    let _subscription = effect.invalidated(move || counted.set(counted.get() + 1));

    let handle: Rc<dyn IEffect> = effect.clone().into();
    let snapshot = || EffectExtensions::to_custom_effect(&*handle).expect("a custom effect");
    assert_eq!(ImmutableCustomEffect::new(PointerSpotlightShaderEffect::default()), snapshot());

    effect.update(|value| value.pointer_x = 0.25);
    assert_eq!(1, invalidations.get());
    assert_eq!(Some(0.25), snapshot().downcast_ref::<PointerSpotlightShaderEffect>().map(|value| value.pointer_x));
    // The same value again: no invalidation.
    effect.update(|value| value.pointer_x = 0.25);
    effect.set_value(effect.value());
    assert_eq!(1, invalidations.get());
    effect.set_value(PointerSpotlightShaderEffect { color: Colors::RED, ..effect.value() });
    assert_eq!(2, invalidations.get());

}

#[test]
fn the_water_ripple_class_ages_its_ripples_until_they_are_gone() {
    let effect = InteractiveWaterRippleEffect::new();
    assert!(!effect.advance(0.1), "nothing moves");
    effect.update(|value| value.spawn_ripple(Point::new(0.5, 0.5), 1.0));
    assert!(effect.advance(0.23));
    let age = effect.value().ripples[0].age;
    assert!((age - 0.2).abs() < 1e-9, "{age}");
    // A held press keeps the frames running.
    effect.update(|value| value.is_pressed = true);
    assert!(effect.advance(2.0));
    assert!(!effect.value().has_active_ripples());
    effect.update(|value| value.is_pressed = false);
    assert!(!effect.advance(0.016));
    assert!(!effect.is_animating());
}

#[test]
fn the_burning_flame_class_burns_while_pressed_and_burns_down_after() {
    let effect = InteractiveBurningFlameEffect::new();
    assert!(effect.handled_events_too());
    assert!(!effect.advance(0.1), "nothing burns");
    effect.update(|value| {
        value.is_pressed = true;
        value.burn_amount = 0.5;
        value.flame_phase = 0.0;
    });
    assert!(effect.advance(0.25));
    let value = effect.value();
    assert!((value.burn_amount - 0.7).abs() < 1e-9, "{}", value.burn_amount);
    assert!((value.flame_phase - 0.7).abs() < 1e-9, "{}", value.flame_phase);
    // The phase wraps at its period.
    effect.update(|value| value.flame_phase = BurningFlameShaderEffect::PHASE_PERIOD - 0.1);
    effect.advance(0.25);
    assert!((effect.value().flame_phase - 0.6).abs() < 1e-9, "{}", effect.value().flame_phase);
    assert!((effect.value().burn_amount - 0.9).abs() < 1e-9, "{}", effect.value().burn_amount);
    // Released, it burns down over 2.6 seconds.
    effect.update(|value| value.is_pressed = false);
    assert!(effect.advance(1.3));
    assert!((effect.value().burn_amount - 0.4).abs() < 1e-9, "{}", effect.value().burn_amount);
    assert!(!effect.advance(1.1));
    assert_eq!(0.0, effect.value().burn_amount);
}

/// Every interactive effect class takes handled events (#580), as Effector's input effects use
/// `handledEventsToo`: the pointer over a button, which handles its press, still reaches them.
#[test]
fn every_interactive_class_takes_handled_events() {
    assert!(InteractivePointerSpotlightEffect::new().handled_events_too());
    assert!(InteractiveReactiveGridEffect::new().handled_events_too());
    assert!(InteractiveWaterRippleEffect::new().handled_events_too());
    assert!(InteractiveBurningFlameEffect::new().handled_events_too());
}
