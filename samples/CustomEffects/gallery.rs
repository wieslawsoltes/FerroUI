//! The main view of the gallery application (Effector's `Effector.Sample.App`): a header, the ways
//! of setting a custom effect (from code, from an effect string as markup writes it, and by key
//! frames of `Visual.Effect`), and a section per family.

use crate::families::Family;
use crate::tiles::{
    brush, button, caption, column, custom_tile, key_frames, markup_of, row, sample, section, text, EffectTile, Play, Sweep,
    ValueTile,
};
use crate::transition_images::TransitionImages;
use ferroui_base::animation::{AnimationTask, IClock};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::effects::library::{GlowEffect, TintEffect};
use ferroui_base::media::effects::{CustomEffect, CustomEffectRegistry, Effect, EffectExtensions, ImmutableCustomEffect};
use ferroui_base::media::{Color, FontWeight};
use ferroui_base::threading::CancellationToken;
use ferroui_base::{CornerRadius, Ref, Thickness};
use ferroui_controls::{Border, Control, ScrollViewer, StackPanel, Window};
use std::rc::Rc;

/// The title of the section of the ways of setting an effect.
pub const AUTHORING_TITLE: &str = "Setting an effect";

/// The effect string of the tile "From an effect string", as markup writes it.
pub const TINT_MARKUP: &str = "tint(color=#0F9D8E, strength=0.55)";

/// The duration of one run of the key frames of the tile "Key frames".
pub const GLOW_MS: f64 = 1800.0;

/// The tiles of the ways of setting an effect: a value from code, an effect string read by
/// `Effect::parse` (the converter markup uses for `Effect="..."`), and key frames of
/// `Visual.Effect`.
pub fn authoring_tiles() -> Vec<EffectTile> {
    // From code: a value of the effect, set as the effect of the host.
    let from_code = ValueTile::new(
        "From code",
        TintEffect { color: Color::from_rgb(0x0F, 0x9D, 0x8E), strength: 0.7 },
        "border.set_effect(Some(ImmutableCustomEffect::new(TintEffect {\n    color: Color::from_rgb(0x0F, 0x9D, 0x8E),\n    strength: 0.7,\n}).into()));",
    )
    .number("strength", 0.0, 1.0)
    .build();

    // From an effect string: what `<Border Effect="tint(...)">` does.
    let parsed = sample();
    let effect = Effect::parse(TINT_MARKUP).ok();
    let snapshot = effect.as_ref().and_then(|effect| EffectExtensions::to_custom_effect(&**effect));
    let markup = caption(&snapshot.map(|snapshot| markup_of(&snapshot)).unwrap_or_default());
    parsed.set_effect(effect);
    let from_markup = custom_tile(
        "From an effect string",
        TintEffect::NAME,
        "The effect string markup writes, read by the converter of IEffect.",
        "<Border Effect=\"tint(color=#0F9D8E, strength=0.55)\" />\n\nborder.set_effect(Some(Effect::parse(\"tint(color=#0F9D8E, strength=0.55)\")?));",
        parsed.upcast(),
        markup,
        None,
    );

    // Key frames: the glow grows, takes another colour and comes back.
    let glowing = sample();
    let resting = GlowEffect { blur_radius: 4.0, intensity: 0.5, ..GlowEffect::default() };
    let glow_markup = caption(&markup_of(&ImmutableCustomEffect::new(resting.clone())));
    glowing.set_effect(Some(ImmutableCustomEffect::new(resting.clone()).into()));
    let play: Play = {
        let host = glowing.downgrade();
        Rc::new(move |clock: Option<Rc<dyn IClock>>| {
            let Some(host) = host.upgrade() else { return AnimationTask::completed() };
            let bright = GlowEffect { color: Color::from_rgb(0x3F, 0xC3, 0xFF), blur_radius: 20.0, intensity: 1.0 };
            key_frames(resting.clone(), bright, GLOW_MS, Sweep::ThereAndBack).run_async_with_clock(&host, clock, CancellationToken::none())
        })
    };
    let play_button = button("Play the key frames");
    {
        let play = play.clone();
        let _ = play_button.click(move |_, _| drop(play(None)));
    }
    let key_frames_tile = custom_tile(
        "Key frames",
        GlowEffect::NAME,
        "Key frames of Visual.Effect: the glow is interpolated parameter by parameter.",
        "let animation = Animation::new();\nanimation.children().add(KeyFrame::with_cue(Cue::new(0.0),\n    [Setter::new(Visual::effect_property(), resting)]));\nanimation.children().add(KeyFrame::with_cue(Cue::new(1.0),\n    [Setter::new(Visual::effect_property(), bright)]));\nanimation.run_async(&border, CancellationToken::none());",
        glowing.upcast(),
        glow_markup,
        Some((play_button, play)),
    );

    vec![from_code, from_markup, key_frames_tile]
}

/// A number and what it counts, on a tinted card.
fn stat(value: &str, what: &str, background: Color) -> Ref<Border> {
    let content = column(2.0);
    content.children().add(text(value, 26.0, FontWeight::Bold));
    content.children().add(caption(what));
    let card = Border::new();
    card.set_padding(Thickness::uniform(14.0));
    card.set_corner_radius(CornerRadius::uniform(16.0));
    card.set_background(Some(brush(background)));
    card.set_child(content);
    card
}

/// The header of the gallery.
fn header() -> Ref<StackPanel> {
    let panel = column(10.0);
    panel.children().add(text("Custom effects", 34.0, FontWeight::SemiBold));
    let about = text(
        "Effects defined outside the framework as values of CustomEffect and set as Visual.Effect, from code or from markup. Each family below is drawn by the backend on the GPU: effect graphs compiled once, runtime shaders compiled once (SkSL on Skia, WGSL on Vello), images uploaded once; a change of a value changes only the uniforms. Every effect of the library has a CPU reference, which Vello's CPU mode draws and the render tests compare the GPU with.",
        14.0,
        FontWeight::Normal,
    );
    about.set_opacity(0.8);
    panel.children().add(about);
    let stats = row(12.0);
    let effects = CustomEffectRegistry::names().len().to_string();
    stats.children().add(stat(&effects, "effects in the library", Color::from_argb(0x30, 0x2D, 0x58, 0x4D)));
    stats.children().add(stat(&Family::ALL.len().to_string(), "families", Color::from_argb(0x30, 0x9A, 0x5B, 0x20)));
    stats.children().add(stat("Skia", "Ganesh and Graphite", Color::from_argb(0x30, 0x27, 0x54, 0x6A)));
    stats.children().add(stat("Vello", "GPU, hybrid and CPU", Color::from_argb(0x30, 0x6A, 0x3D, 0x74)));
    panel.children().add(stats);
    panel
}

/// The gallery: the header, the ways of setting an effect and a section per family, in a scroll
/// viewer. The page transitions sample `images`.
pub fn gallery(images: &TransitionImages) -> Ref<Control> {
    let content = column(28.0);
    content.set_margin(Thickness::uniform(28.0));
    content.set_vertical_alignment(VerticalAlignment::Top);
    content.set_horizontal_alignment(HorizontalAlignment::Stretch);
    content.children().add(header());
    content.children().add(section(
        AUTHORING_TITLE,
        "A custom effect is a value: set it from code, write it as an effect string in markup, or animate it with key frames and transitions of Visual.Effect, as the built-in blur and drop shadow are.",
        &authoring_tiles(),
    ));
    for family in Family::ALL {
        content.children().add(family.section(images));
    }
    let scroller = ScrollViewer::new();
    scroller.set_content(Some(Control::boxed(content)));
    scroller.upcast()
}

/// The main window of the gallery application.
pub fn main_window(images: &TransitionImages) -> Ref<Window> {
    let window = Window::new();
    window.set_title(Some(String::from("Custom effects")));
    window.set_width(1180.0);
    window.set_height(900.0);
    window.set_content(Some(Control::boxed(gallery(images))));
    window
}
