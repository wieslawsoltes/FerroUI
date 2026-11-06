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
    InteractiveWaterRippleEffect, PointerSpotlightShaderEffect, ReactiveGridShaderEffect, RippleWave,
    WaterRippleShaderEffect,
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
    let modes: Vec<(ShaderEffectMode, BitmapBlendingMode, f64)> = interactive_effects()
        .iter()
        .map(|effect| effect.shader().map(|shader| (shader.mode, shader.blend_mode, shader.sample_radius)).unwrap())
        .collect();
    assert_eq!(
        vec![
            (ShaderEffectMode::Overlay, BitmapBlendingMode::Screen, 0.0),
            (ShaderEffectMode::Overlay, BitmapBlendingMode::SourceOver, 0.0),
            (ShaderEffectMode::ContentSampling, BitmapBlendingMode::SourceOver, WaterRippleShaderEffect::SAMPLE_RADIUS),
            (ShaderEffectMode::ContentSampling, BitmapBlendingMode::SourceOver, BurningFlameShaderEffect::SAMPLE_RADIUS),
        ],
        modes
    );
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

#[test]
fn the_reference_of_the_pointer_spotlight_follows_the_pointer() {
    let (width, height) = (40, 40);
    let lit = |effect: PointerSpotlightShaderEffect| {
        let mut pixels = vec![0u8; width * height * 4];
        assert!(ImmutableCustomEffect::new(effect).cpu_reference(&mut pixels, width, height, 1.0));
        pixels
    };
    let over =
        PointerSpotlightShaderEffect { pointer_x: 0.25, pointer_y: 0.25, is_pointer_over: true, ..PointerSpotlightShaderEffect::default() };
    let hovered = lit(over.clone());
    // Brightest under the pointer, nothing far from it.
    assert!(alpha(&hovered, width, 10, 10) > alpha(&hovered, width, 14, 10));
    assert_eq!(0, alpha(&hovered, width, 35, 35));
    // Pressed, the light is brighter.
    let pressed = lit(PointerSpotlightShaderEffect { is_pressed: true, ..over.clone() });
    assert!(alpha(&pressed, width, 10, 10) > alpha(&hovered, width, 10, 10));
    // The pointer away: nothing.
    assert!(lit(PointerSpotlightShaderEffect { is_pointer_over: false, ..over }).iter().all(|&value| value == 0));
}

#[test]
fn the_reference_of_the_reactive_grid_draws_lines_and_a_halo() {
    let (width, height) = (44, 44);
    let draw = |effect: ReactiveGridShaderEffect, scale: f64| {
        let mut pixels = vec![0u8; width * height * 4];
        assert!(ImmutableCustomEffect::new(effect).cpu_reference(&mut pixels, width, height, scale));
        pixels
    };
    let away = draw(ReactiveGridShaderEffect::default(), 1.0);
    // Lines along the first pixel of each cell of 22.
    for (x, y) in [(0, 5), (22, 5), (5, 0), (5, 22)] {
        assert!(alpha(&away, width, x, y) > 0, "({x}, {y})");
    }
    for (x, y) in [(1, 5), (21, 5), (5, 23), (10, 10)] {
        assert_eq!(0, alpha(&away, width, x, y), "({x}, {y})");
    }
    // At a scale of 2 a cell is 44 device pixels and a line 2.
    let scaled = draw(ReactiveGridShaderEffect::default(), 2.0);
    assert!(alpha(&scaled, width, 1, 5) > 0);
    assert_eq!(0, alpha(&scaled, width, 22, 5));
    // The halo under the pointer.
    let over = draw(ReactiveGridShaderEffect { is_pointer_over: true, ..ReactiveGridShaderEffect::default() }, 1.0);
    assert!(alpha(&over, width, 20, 20) > 0);
    assert_eq!(0, alpha(&away, width, 20, 20));
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
    // Below the top, where the flame moves the content up from beyond the area, the content
    // stays opaque.
    for y in 20..height {
        for x in 10..50 {
            assert_eq!(255, at(&pixels, x, y)[3], "({x}, {y})");
        }
    }
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
