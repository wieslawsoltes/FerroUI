//! The page "Custom effects" of the section "Composition" of the catalog (#455, E2 to E7; not from
//! upstream; `docs/porting/custom-effects.md`): the custom effects FerroUI ships
//! (`ferroui_base::media::effects::library`), each set as `Visual.Effect` on the same sample
//! content, with a slider for each of its numbers and the effect string markup writes for it.
//! The colour effects and the glow are effect graphs, pixelate, scan lines and spotlight runtime
//! shaders (SkSL on Skia, WGSL on Vello; Vello's CPU mode draws their CPU references), all drawn by
//! the backend on the GPU; a slider changes only the values of the members or the uniforms. The
//! page transitions sample photographs of the catalog's assets as images (E6), uploaded once by
//! the backend; their images are set from code (an effect string cannot name an image). The
//! interactive effects (E7) are effect classes that follow the pointer over their host: the
//! pointer spotlight, the reactive grid, the water ripple and the burn-away button, each a runtime
//! shader whose uniforms the pointer changes.

use super::winui_composition_page::{brush, section, text};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::effects::library::{
    BrightnessContrastEffect, DissolveTransitionEffect, EdgeDetectEffect, GlowEffect, GrayscaleEffect,
    InteractiveBurningFlameEffect, InteractivePointerSpotlightEffect, InteractiveReactiveGridEffect,
    InteractiveWaterRippleEffect, InvertEffect, MaskTransitionEffect, PixelateEffect, SaturationEffect,
    ScanlineShaderEffect, SepiaEffect, SharpenEffect, SpotlightShaderEffect, TintEffect,
};
use ferroui_base::media::effects::{
    CustomEffect, CustomEffectClass, Effect, EffectImage, EffectParameterValue, ImmutableCustomEffect,
};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Color, FontWeight};
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, CornerRadius, Ref, Thickness, Upcast};
use ferroui_controls::{Border, Button, Slider, StackPanel, TextBlock, WrapPanel};
use std::cell::RefCell;
use std::rc::Rc;

/// The colours of the swatches of the sample.
const SWATCHES: [(u8, u8, u8); 4] = [(0xE8, 0x5D, 0x3A), (0x2E, 0x86, 0xDE), (0x3F, 0x9E, 0x5A), (0xD9, 0xA4, 0x41)];

fn caption(value: &str) -> Ref<TextBlock> {
    let block = text(value, 12.0, FontWeight::Normal);
    block.set_opacity(0.75);
    block
}

fn column(spacing: f64) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(spacing);
    panel
}

/// The content every effect is shown on: four round swatches and a title, on no background, so
/// that a glow shows around their shapes.
fn sample() -> Ref<Border> {
    let swatches = StackPanel::new();
    swatches.set_orientation(Orientation::Horizontal);
    swatches.set_spacing(8.0);
    for (r, g, b) in SWATCHES {
        let swatch = Border::new();
        swatch.set_width(36.0);
        swatch.set_height(36.0);
        swatch.set_corner_radius(CornerRadius::uniform(18.0));
        swatch.set_background(Some(brush(Color::from_rgb(r, g, b))));
        swatches.children().add(swatch);
    }
    let title = text("FerroUI", 24.0, FontWeight::Bold);
    title.set_foreground(Some(brush(Color::from_rgb(0x9B, 0x59, 0xB6))));
    let content = column(8.0);
    content.children().add(swatches);
    content.children().add(title);
    let sample = Border::new();
    sample.set_padding(Thickness::uniform(16.0));
    sample.set_horizontal_alignment(HorizontalAlignment::Left);
    sample.set_child(content);
    sample
}

/// The image of the asset at `uri` as the handle of an effect image: the bitmap decoded once
/// (the handle holds it); the empty image, which the transitions draw transparent, when it does
/// not load.
fn asset_image(uri: &str) -> EffectImage {
    let bitmap = Uri::absolute(uri)
        .ok()
        .and_then(|uri| AssetLoader::open(&uri, None).ok())
        .and_then(|mut stream| Bitmap::from_stream(&mut stream).ok());
    bitmap.map(|bitmap| EffectImage::new(&bitmap)).unwrap_or_default()
}

/// A slider with its name and value above it; `changed` gets the new value.
fn slider(name: &'static str, range: (f64, f64), value: f64, changed: impl Fn(f64) + 'static) -> Ref<StackPanel> {
    let panel = column(2.0);
    let title = caption(&format!("{name}: {value:.2}"));
    let slider = Slider::new();
    slider.set_minimum(range.0);
    slider.set_maximum(range.1);
    slider.set_range_value(value);
    slider.set_width(200.0);
    slider.set_horizontal_alignment(HorizontalAlignment::Left);
    let weak = title.downgrade();
    let _ = slider.value_changed(move |_, e| {
        if let Some(title) = weak.upgrade() {
            title.set_text(Some(&format!("{name}: {:.2}", e.new_value())));
        }
        changed(e.new_value());
    });
    panel.children().add(title);
    panel.children().add(slider);
    panel
}

/// The tile of one effect: its name, the sample with the effect, a slider for each of `numbers`
/// (a parameter and its range), and the effect string of the current value.
fn effect_tile<T: CustomEffect>(initial: T, numbers: &[(&'static str, f64, f64)]) -> Ref<StackPanel> {
    let tile = column(6.0);
    tile.set_width(260.0);
    tile.set_margin(Thickness::new(0.0, 0.0, 16.0, 16.0));
    let sample = sample();
    let markup = caption("");
    let value = Rc::new(RefCell::new(initial));
    // The snapshot of the value is the effect of the sample; nothing is borrowed while the
    // sample and the caption change.
    let apply: Rc<dyn Fn()> = {
        let (sample, markup, value) = (sample.downgrade(), markup.downgrade(), value.clone());
        Rc::new(move || {
            let effect = ImmutableCustomEffect::new(value.borrow().clone());
            if let Some(markup) = markup.upgrade() {
                markup.set_text(Some(&format!("Effect=\"{effect}\"")));
            }
            if let Some(sample) = sample.upgrade() {
                sample.set_effect(Some(effect.into()));
            }
        })
    };
    apply();

    tile.children().add(text(T::NAME, 15.0, FontWeight::SemiBold));
    tile.children().add(sample);
    let current = ImmutableCustomEffect::new(value.borrow().clone());
    for &(name, minimum, maximum) in numbers {
        let start = current.parameter(name).and_then(|parameter| parameter.as_float()).unwrap_or(minimum);
        let (value, apply) = (value.clone(), apply.clone());
        tile.children().add(slider(name, (minimum, maximum), start, move |new| {
            let changed = value.borrow_mut().set_parameter(name, EffectParameterValue::Float(new));
            if changed {
                apply();
            }
        }));
    }
    tile.children().add(markup);
    tile
}

/// The section of the page "Custom effects".
pub(super) fn custom_effects_section() -> Ref<StackPanel> {
    let tiles = WrapPanel::new();
    tiles.set_orientation(Orientation::Horizontal);
    tiles.children().add(effect_tile(TintEffect::default(), &[("strength", 0.0, 1.0)]));
    tiles.children().add(effect_tile(GrayscaleEffect::default(), &[("amount", 0.0, 1.0)]));
    tiles.children().add(effect_tile(SepiaEffect::default(), &[("amount", 0.0, 1.0)]));
    tiles.children().add(effect_tile(
        SaturationEffect { saturation: 1.8 },
        &[("saturation", 0.0, SaturationEffect::MAXIMUM)],
    ));
    tiles.children().add(effect_tile(
        BrightnessContrastEffect { brightness: 0.1, contrast: 1.4 },
        &[("brightness", -1.0, 1.0), ("contrast", 0.0, BrightnessContrastEffect::MAXIMUM_CONTRAST)],
    ));
    tiles.children().add(effect_tile(InvertEffect::default(), &[("amount", 0.0, 1.0)]));
    tiles.children().add(effect_tile(GlowEffect::default(), &[("blur-radius", 0.0, 24.0), ("intensity", 0.0, 1.0)]));
    tiles.children().add(effect_tile(SharpenEffect::default(), &[("strength", 0.0, 2.0)]));
    tiles.children().add(effect_tile(EdgeDetectEffect::default(), &[("strength", 0.0, 2.0)]));
    tiles.children().add(effect_tile(
        PixelateEffect { cell_size: 6.0 },
        &[("cell-size", 0.0, PixelateEffect::MAXIMUM_CELL_SIZE)],
    ));
    tiles.children().add(effect_tile(
        ScanlineShaderEffect { spacing: 4.0, strength: 0.4 },
        &[("spacing", 2.0, 32.0), ("strength", 0.0, 1.0)],
    ));
    tiles.children().add(effect_tile(
        SpotlightShaderEffect { strength: 0.8, ..SpotlightShaderEffect::default() },
        &[("center-x", 0.0, 1.0), ("center-y", 0.0, 1.0), ("radius", 0.05, 1.0), ("strength", 0.0, 1.0)],
    ));
    // The page transitions: two photographs as the pages, a third one's luminance as the pattern
    // of the dissolve and the mask of the reveal.
    let arch = asset_image("ferres://ControlCatalog/Assets/delicate-arch-896885_640.jpg");
    let deer = asset_image("ferres://ControlCatalog/Assets/hirsch-899118_640.jpg");
    let leaf = asset_image("ferres://ControlCatalog/Assets/maple-leaf-888807_640.jpg");
    tiles.children().add(effect_tile(
        DissolveTransitionEffect {
            from_page: arch,
            to_page: deer.clone(),
            pattern: leaf.clone(),
            progress: 0.5,
            ..DissolveTransitionEffect::default()
        },
        &[("progress", 0.0, 1.0), ("center-x", 0.0, 1.0), ("center-y", 0.0, 1.0)],
    ));
    tiles.children().add(effect_tile(
        MaskTransitionEffect { to_page: deer, mask: leaf, progress: 0.5, softness: 0.15 },
        &[("progress", 0.0, 1.0), ("softness", 0.001, 1.0)],
    ));
    let page = section(
        "Custom effects",
        "Effects defined outside the framework as values of CustomEffect, set as Visual.Effect from code or markup (Effect=\"tint(color=#0F9D8E, strength=0.55)\"). The colour effects, the glow, sharpen and edge detect describe themselves as effect graphs: colour matrices, convolutions and a drop shadow drawn by the backend on the GPU. Pixelate, scan lines and spotlight are runtime shaders: an SkSL program Skia compiles once and a WGSL program Vello compiles once, each drawn around the content on the GPU (Vello's CPU mode draws the CPU reference of the effect). The dissolve and mask transitions are runtime shaders that sample images, here photographs set from code: each backend uploads an image once into a texture and binds it as a child of the program. A slider changes only the values of the members or the uniforms.",
        &tiles.upcast(),
    );
    page.children().add(interactive_effects_section());
    page
}

// --- the interactive effects (E7) --------------------------------------------------------------

/// A caption with the effect string of the current snapshot of `effect`, kept up to date as the
/// pointer changes it.
fn snapshot_caption<T: CustomEffectClass + Upcast<Effect>>(effect: &Ref<T>) -> Ref<TextBlock> {
    let markup = caption(&format!("Effect=\"{}\"", effect.to_custom_effect()));
    let (weak_effect, weak_markup) = (effect.downgrade(), markup.downgrade());
    let base: &Effect = Upcast::<Effect>::upcast(&**effect);
    base.invalidated(move || {
        if let (Some(effect), Some(markup)) = (weak_effect.upgrade(), weak_markup.upgrade()) {
            markup.set_text(Some(&format!("Effect=\"{}\"", effect.to_custom_effect())));
        }
    })
    .detach();
    markup
}

/// The surface an interactive effect is shown on: the sample on a dark panel, so that the whole
/// panel takes the pointer (an element without a background is not hit by it).
fn interactive_surface() -> Ref<Border> {
    let surface = sample();
    surface.set_width(240.0);
    surface.set_height(150.0);
    surface.set_corner_radius(CornerRadius::uniform(8.0));
    surface.set_background(Some(brush(Color::from_rgb(0x1E, 0x27, 0x3A))));
    surface
}

/// The tile of an interactive effect: its name, what to do, the host and the effect string of
/// the snapshot the pointer changes.
fn interactive_tile(name: &str, hint: &str, host: Ref<ferroui_controls::Control>, markup: Ref<TextBlock>) -> Ref<StackPanel> {
    let tile = column(6.0);
    tile.set_width(260.0);
    tile.set_margin(Thickness::new(0.0, 0.0, 16.0, 16.0));
    tile.children().add(text(name, 15.0, FontWeight::SemiBold));
    tile.children().add(caption(hint));
    tile.children().add(host);
    tile.children().add(markup);
    tile
}

/// The section of the interactive effects: effect classes that receive the pointer input of the
/// element they are the effect of, and change the uniforms of their programs from it.
fn interactive_effects_section() -> Ref<StackPanel> {
    let tiles = WrapPanel::new();
    tiles.set_orientation(Orientation::Horizontal);

    let spotlight = InteractivePointerSpotlightEffect::new();
    let surface = interactive_surface();
    surface.set_effect(Some(spotlight.clone().into()));
    tiles.children().add(interactive_tile(
        "pointer-spotlight",
        "Move the pointer over the panel; press to brighten the light.",
        surface.upcast(),
        snapshot_caption(&spotlight),
    ));

    let grid = InteractiveReactiveGridEffect::new();
    let surface = interactive_surface();
    surface.set_effect(Some(grid.clone().into()));
    tiles.children().add(interactive_tile(
        "reactive-grid",
        "The halo follows the pointer and widens while pressed.",
        surface.upcast(),
        snapshot_caption(&grid),
    ));

    let ripple = InteractiveWaterRippleEffect::new();
    let surface = interactive_surface();
    surface.set_effect(Some(ripple.clone().into()));
    tiles.children().add(interactive_tile(
        "water-ripple",
        "Press, drag and release: rings spread from the pointer and bend the content.",
        surface.upcast(),
        snapshot_caption(&ripple),
    ));

    // The burn-away button: the flame takes the press the button handles.
    let flame = InteractiveBurningFlameEffect::new();
    let button = Button::new();
    button.set_content(Some(Rc::new("Burn away".to_owned()) as BoxedValue));
    button.set_width(240.0);
    button.set_height(72.0);
    button.set_effect(Some(flame.clone().into()));
    tiles.children().add(interactive_tile(
        "burning-flame",
        "Press the button: it burns where it is pressed and burns down after the release.",
        button.upcast(),
        snapshot_caption(&flame),
    ));

    section(
        "Interactive effects",
        "Effect classes that implement IEffectInputHandler: set as Visual.Effect on an input element, the element routes its pointer events to the effect (after its own handlers and those of its children, handled events only to an effect that asks for them, captured pointers wherever they are), and the effect changes its value and raises Invalidated. The visual takes a new snapshot, so the program, compiled once, is drawn with new uniforms: the pointer position normalized to the element, the press, the age of a ripple, the phase of the flame. The water ripple and the flame advance on the frames of the animation clock while they move.",
        &tiles.upcast(),
    )
}
