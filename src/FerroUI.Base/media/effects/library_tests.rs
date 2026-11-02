//! Tests of the custom effects FerroUI ships (#455, E2 and E3; not from upstream):
//! each loads its graph with members that match its graph values, prints
//! and parses back, leaves the content unchanged at its neutral value, and
//! has a CPU reference. How the render walk draws them is tested with the
//! compositor (`composition_custom_effect_tests.rs`), what the backends draw
//! against the CPU reference in `ferroui-skia` and `ferroui-vello`.

use super::library::*;
use super::{CustomEffect, CustomEffectRegistry, Effect, EffectParameterValue, ImmutableCustomEffect};
use crate::media::{ColorMatrixAlphaMode, Colors};
use crate::rendering::composition::effects::effect_prewarm_names;

/// The library effects at a value that is not their neutral one.
fn library_effects() -> Vec<ImmutableCustomEffect> {
    vec![
        ImmutableCustomEffect::new(TintEffect { color: Colors::TEAL, strength: 0.4 }),
        ImmutableCustomEffect::new(GrayscaleEffect { amount: 0.7 }),
        ImmutableCustomEffect::new(SepiaEffect { amount: 0.8 }),
        ImmutableCustomEffect::new(SaturationEffect { saturation: 1.6 }),
        ImmutableCustomEffect::new(BrightnessContrastEffect { brightness: -0.1, contrast: 1.3 }),
        ImmutableCustomEffect::new(InvertEffect { amount: 0.9 }),
        ImmutableCustomEffect::new(GlowEffect { color: Colors::ORANGE, blur_radius: 4.0, intensity: 0.7 }),
        ImmutableCustomEffect::new(SharpenEffect { strength: 0.8 }),
        ImmutableCustomEffect::new(EdgeDetectEffect { strength: 1.2 }),
    ]
}

/// The neutral values of the library effects that have one (all but edge
/// detect).
fn neutral_effects() -> Vec<ImmutableCustomEffect> {
    let neutral: Vec<ImmutableCustomEffect> = library_effects().iter().filter_map(|effect| effect.neutral()).collect();
    assert_eq!(library_effects().len() - 1, neutral.len());
    neutral
}

/// A 6 x 2 image of premultiplied colours: opaque, translucent and
/// transparent pixels.
fn sample_pixels() -> Vec<u8> {
    vec![
        255, 0, 0, 255, 0, 128, 0, 255, 20, 40, 200, 255, 60, 30, 10, 128, 0, 0, 0, 0, 100, 100, 100, 100, //
        255, 255, 255, 255, 0, 0, 0, 255, 128, 128, 0, 128, 10, 20, 30, 40, 250, 200, 150, 255, 1, 2, 3, 4,
    ]
}

#[test]
fn each_effect_loads_its_graph_with_its_members() {
    for effect in library_effects() {
        assert!(effect.has_graph(), "{effect}: {:?}", effect.graph_error());
        assert_eq!(None, effect.graph_error(), "{effect}");
        // A member for each graph value, in order, named as an animatable
        // property of an effect factory.
        let mut values = Vec::new();
        effect.for_each_graph_value(&mut |name, _| values.push(name));
        let members = effect.graph_members();
        assert_eq!(values, members.iter().map(|(name, _)| *name).collect::<Vec<_>>(), "{effect}");
        for (name, member) in &members {
            assert!(member.contains('.'), "{effect}: {name} sets {member}");
            if name.contains('.') {
                assert_eq!(*name, member.as_str(), "{effect}");
            }
        }
    }
}

#[test]
fn the_members_of_the_graphs() {
    let members = |effect: ImmutableCustomEffect| -> Vec<String> {
        effect.graph_members().into_iter().map(|(_, member)| member).collect()
    };
    assert_eq!(vec!["Tint.Color"], members(ImmutableCustomEffect::new(TintEffect::default())));
    assert_eq!(vec!["Grayscale.Saturation"], members(ImmutableCustomEffect::new(GrayscaleEffect::default())));
    // The parameter is the value of the effect named after it.
    assert_eq!(vec!["amount.Intensity"], members(ImmutableCustomEffect::new(SepiaEffect::default())));
    assert_eq!(vec!["Saturation.Saturation"], members(ImmutableCustomEffect::new(SaturationEffect::default())));
    let transfer = vec![
        "Transfer.RedSlope",
        "Transfer.GreenSlope",
        "Transfer.BlueSlope",
        "Transfer.RedOffset",
        "Transfer.GreenOffset",
        "Transfer.BlueOffset",
    ];
    assert_eq!(transfer, members(ImmutableCustomEffect::new(BrightnessContrastEffect::default())));
    assert_eq!(transfer, members(ImmutableCustomEffect::new(InvertEffect::default())));
    assert_eq!(vec!["Glow.BlurAmount", "Glow.ShadowColor"], members(ImmutableCustomEffect::new(GlowEffect::default())));
    let kernel = |name: &str| -> Vec<String> { (0..9).map(|index| format!("{name}.KernelMatrix{index}")).collect() };
    assert_eq!(kernel("Sharpen"), members(ImmutableCustomEffect::new(SharpenEffect::default())));
    assert_eq!(kernel("Edges"), members(ImmutableCustomEffect::new(EdgeDetectEffect::default())));
}

#[test]
fn the_graph_values_follow_the_parameters() {
    let value = |effect: &ImmutableCustomEffect, wanted: &str| -> EffectParameterValue {
        let mut found = None;
        effect.for_each_graph_value(&mut |name, value| {
            if name == wanted {
                found = Some(value);
            }
        });
        found.unwrap_or_else(|| panic!("{effect} has the value {wanted}"))
    };
    let tint = ImmutableCustomEffect::new(TintEffect { color: Colors::RED, strength: 0.5 });
    let pink = crate::media::Color::from_argb(255, 255, 128, 128);
    assert_eq!(EffectParameterValue::Color(pink), value(&tint, "Tint.Color"));
    let grayscale = ImmutableCustomEffect::new(GrayscaleEffect { amount: 2.0 });
    assert_eq!(EffectParameterValue::Float(0.0), value(&grayscale, "Grayscale.Saturation"));
    let sepia = ImmutableCustomEffect::new(SepiaEffect { amount: 0.3 });
    assert_eq!(EffectParameterValue::Float(0.3), value(&sepia, "amount"));
    let saturation = ImmutableCustomEffect::new(SaturationEffect { saturation: 9.0 });
    assert_eq!(EffectParameterValue::Float(SaturationEffect::MAXIMUM), value(&saturation, "Saturation.Saturation"));
    let contrast = ImmutableCustomEffect::new(BrightnessContrastEffect { brightness: 0.25, contrast: 2.0 });
    assert_eq!(EffectParameterValue::Float(2.0), value(&contrast, "Transfer.RedSlope"));
    assert_eq!(EffectParameterValue::Float(-0.25), value(&contrast, "Transfer.BlueOffset"));
    let invert = ImmutableCustomEffect::new(InvertEffect { amount: 1.0 });
    assert_eq!(EffectParameterValue::Float(-1.0), value(&invert, "Transfer.GreenSlope"));
    assert_eq!(EffectParameterValue::Float(1.0), value(&invert, "Transfer.GreenOffset"));
    let glow = ImmutableCustomEffect::new(GlowEffect { color: Colors::BLUE, blur_radius: 10.0, intensity: 0.5 });
    let halo = crate::media::Color::from_argb(128, 0, 0, 255);
    assert_eq!(EffectParameterValue::Color(halo), value(&glow, "Glow.ShadowColor"));
    let amount = value(&glow, "Glow.BlurAmount").as_float().unwrap();
    assert!((crate::rendering::composition::effects::blur_amount_to_radius(amount) - 10.0).abs() < 1e-9);
}

#[test]
fn the_effects_print_and_parse_back() {
    for effect in library_effects().into_iter().chain(neutral_effects()) {
        let text = effect.to_string();
        let parsed = Effect::parse(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
        let parsed = parsed.as_custom_effect().expect("a custom effect").clone();
        assert_eq!(effect, parsed, "{text}");
    }
    let parsed = Effect::parse("tint(color=#0F9D8E, strength=0.55)").unwrap();
    let tint = parsed.as_custom_effect().and_then(|effect| effect.downcast_ref::<TintEffect>()).cloned();
    assert_eq!(Some(TintEffect { color: crate::media::Color::from_rgb(0x0F, 0x9D, 0x8E), strength: 0.55 }), tint);
    let parsed = Effect::parse("brightness-contrast(Contrast=1.5)").unwrap();
    let effect = parsed.as_custom_effect().and_then(|effect| effect.downcast_ref::<BrightnessContrastEffect>()).cloned();
    assert_eq!(Some(BrightnessContrastEffect { brightness: 0.0, contrast: 1.5 }), effect);
    let parsed = Effect::parse("glow(blur_radius=8)").unwrap();
    let effect = parsed.as_custom_effect().and_then(|effect| effect.downcast_ref::<GlowEffect>()).cloned();
    assert_eq!(Some(GlowEffect { blur_radius: 8.0, ..GlowEffect::default() }), effect);
}

#[test]
fn the_effects_are_registered_and_their_graphs_prewarmed() {
    let names = CustomEffectRegistry::names();
    let library =
        ["tint", "grayscale", "sepia", "saturation", "brightness-contrast", "invert", "glow", "sharpen", "edge-detect"];
    for name in library {
        assert!(names.contains(&name), "{name}: {names:?}");
        assert!(CustomEffectRegistry::is_registered(name), "{name}");
        assert!(effect_prewarm_names().contains(&format!("custom-effect:{name}")), "{name}");
    }
}

#[test]
fn a_neutral_effect_leaves_the_content_unchanged() {
    for effect in neutral_effects() {
        let mut pixels = sample_pixels();
        assert!(effect.cpu_reference(&mut pixels, 6, 2, 1.0), "{effect}: a reference");
        for (index, (after, before)) in pixels.iter().zip(sample_pixels()).enumerate() {
            assert!(after.abs_diff(before) <= 1, "{effect}: byte {index}: {before} became {after}");
        }
    }
}

#[test]
fn the_reference_of_a_colour_effect_is_its_matrix() {
    let tint = TintEffect { color: Colors::TEAL, strength: 0.4 };
    let grayscale = GrayscaleEffect { amount: 0.7 };
    let sepia = SepiaEffect { amount: 0.8 };
    let saturation = SaturationEffect { saturation: 1.6 };
    let contrast = BrightnessContrastEffect { brightness: -0.1, contrast: 1.3 };
    let invert = InvertEffect { amount: 0.9 };
    let matrices = [
        (tint.matrix(), ImmutableCustomEffect::new(tint)),
        (grayscale.matrix(), ImmutableCustomEffect::new(grayscale)),
        (sepia.matrix(), ImmutableCustomEffect::new(sepia)),
        (saturation.matrix(), ImmutableCustomEffect::new(saturation)),
        (contrast.matrix(), ImmutableCustomEffect::new(contrast)),
        (invert.matrix(), ImmutableCustomEffect::new(invert)),
    ];
    for (matrix, effect) in matrices {
        let mut pixels = sample_pixels();
        assert!(effect.cpu_reference(&mut pixels, 6, 2, 1.0));
        let mut expected = sample_pixels();
        matrix.apply_to_rgba8(&mut expected, ColorMatrixAlphaMode::Premultiplied);
        assert_eq!(expected, pixels, "{effect}");
    }
    // Opaque red inverted is opaque cyan; grey is kept by the grayscale.
    let mut red = vec![255, 0, 0, 255];
    ImmutableCustomEffect::new(InvertEffect { amount: 1.0 }).cpu_reference(&mut red, 1, 1, 1.0);
    assert_eq!(vec![0, 255, 255, 255], red);
    let mut grey = vec![100, 100, 100, 255];
    ImmutableCustomEffect::new(GrayscaleEffect { amount: 1.0 }).cpu_reference(&mut grey, 1, 1, 1.0);
    assert!(grey.iter().zip([100, 100, 100, 255]).all(|(a, b)| a.abs_diff(b) <= 1), "{grey:?}");
}

#[test]
fn the_reference_of_a_glow_is_a_halo_under_the_content() {
    let (width, height) = (40, 40);
    let mut pixels = vec![0u8; width * height * 4];
    for y in 15..25 {
        for x in 15..25 {
            pixels[(y * width + x) * 4..][..4].copy_from_slice(&[255, 0, 0, 255]);
        }
    }
    let content = pixels.clone();
    let glow = ImmutableCustomEffect::new(GlowEffect { color: Colors::BLUE, blur_radius: 6.0, intensity: 1.0 });
    assert!(glow.cpu_reference(&mut pixels, width, height, 1.0));
    let at = |pixels: &[u8], x: usize, y: usize| -> [u8; 4] { pixels[(y * width + x) * 4..][..4].try_into().unwrap() };
    // The content is over the halo.
    assert_eq!(at(&content, 20, 20), at(&pixels, 20, 20));
    // Beside the content, the halo: blue, fading with the distance.
    let near = at(&pixels, 13, 20);
    let far = at(&pixels, 10, 20);
    assert!(near[2] > far[2] && far[2] > 0, "{near:?} {far:?}");
    assert_eq!([0, 0], [near[0], near[1]]);
    assert_eq!(near[2], near[3], "the halo is opaque blue, premultiplied");
    // Far from the content, nothing.
    assert_eq!([0, 0, 0, 0], at(&pixels, 0, 0));
    // An application's effect without a reference leaves the pixels alone.
    let mut untouched = content.clone();
    let plain = ImmutableCustomEffect::new(Plain);
    assert!(!plain.cpu_reference(&mut untouched, width, height, 1.0));
    assert_eq!(content, untouched);
}

#[test]
fn the_reference_of_an_image_effect_is_its_convolution() {
    let sharpen = SharpenEffect { strength: 0.8 };
    let edges = EdgeDetectEffect { strength: 1.2 };
    let effects = [
        (sharpen.convolution(), ImmutableCustomEffect::new(sharpen)),
        (edges.convolution(), ImmutableCustomEffect::new(edges)),
    ];
    for (convolution, effect) in effects {
        let mut pixels = sample_pixels();
        assert!(effect.cpu_reference(&mut pixels, 6, 2, 1.0), "{effect}");
        let mut expected = sample_pixels();
        convolution.apply_to_premultiplied_rgba8(&mut expected, 6, 2);
        assert_eq!(expected, pixels, "{effect}");
        assert_ne!(sample_pixels(), pixels, "{effect} changes the content");
    }
    // The kernels, with the strength clamped to 0..2.
    assert_eq!([0.0, -2.0, 0.0, -2.0, 9.0, -2.0, 0.0, -2.0, 0.0], SharpenEffect { strength: 5.0 }.kernel());
    assert_eq!([-1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0], EdgeDetectEffect::default().kernel());
    assert!(!SharpenEffect::default().convolution().preserve_alpha);
    assert!(EdgeDetectEffect::default().convolution().preserve_alpha);
    // A flat opaque area has no edges: black, opaque.
    let mut flat = vec![120u8, 60, 30, 255].repeat(9);
    ImmutableCustomEffect::new(EdgeDetectEffect::default()).cpu_reference(&mut flat, 3, 3, 1.0);
    assert_eq!([0, 0, 0, 255], flat[16..20]);
    // Edge detect has no neutral value.
    assert!(ImmutableCustomEffect::new(EdgeDetectEffect::default()).neutral().is_none());
}

/// An effect without a graph and without a reference.
#[derive(Clone, Debug, Default, PartialEq)]
struct Plain;

impl CustomEffect for Plain {
    const NAME: &'static str = "library-test-plain";

    fn for_each_parameter(&self, _: &mut dyn FnMut(&'static str, EffectParameterValue)) {}

    fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
        false
    }
}

#[test]
fn an_effect_without_a_graph_has_no_members() {
    let plain = ImmutableCustomEffect::new(Plain);
    assert!(!plain.has_graph());
    assert_eq!(None, plain.graph_error());
    assert!(plain.graph_members().is_empty());
}
