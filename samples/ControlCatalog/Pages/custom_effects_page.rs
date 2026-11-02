//! The page "Custom effects" of the section "Composition" of the catalog (#455, E2 and E3; not from
//! upstream; `docs/porting/custom-effects.md`): the custom effects FerroUI ships
//! (`ferroui_base::media::effects::library`), each set as `Visual.Effect` on the same sample
//! content, with a slider for each of its numbers and the effect string markup writes for it.
//! Each effect is an effect graph drawn by the backend on the GPU; a slider changes only the
//! values of its members.

use super::winui_composition_page::{brush, section, text};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::effects::library::{
    BrightnessContrastEffect, EdgeDetectEffect, GlowEffect, GrayscaleEffect, InvertEffect, SaturationEffect,
    SepiaEffect, SharpenEffect, TintEffect,
};
use ferroui_base::media::effects::{CustomEffect, EffectParameterValue, ImmutableCustomEffect};
use ferroui_base::media::{Color, FontWeight};
use ferroui_base::{CornerRadius, Ref, Thickness};
use ferroui_controls::{Border, Slider, StackPanel, TextBlock, WrapPanel};
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
    section(
        "Custom effects",
        "Effects defined outside the framework as values of CustomEffect, set as Visual.Effect from code or markup (Effect=\"tint(color=#0F9D8E, strength=0.55)\"). The effects of the library describe themselves as effect graphs: colour matrices, convolutions and a drop shadow drawn by the backend on the GPU, the graph compiled once, a slider changing only the values of its members.",
        &tiles.upcast(),
    )
}
