//! The tiles of the gallery: one effect each, shown on a sample with what sets it.
//!
//! A tile of a value ([`ValueTile`]) holds the value of a [`CustomEffect`] and sets its snapshot as
//! the `Effect` of its host: a slider for each of its numbers and a button for each of its colours
//! change the value, and the host takes the new snapshot. The tile shows the code that makes the
//! value, the effect string markup writes for the current value (`Effect="tint(...)"`, which
//! `Effect::parse` reads back), and two animations of `Visual.Effect`: an `EffectTransition` on
//! the host (every change of the effect is animated, and turning the effect off or on fades it
//! through its neutral value) and key frames that sweep the first number of the effect.
//!
//! The values change only the uniforms of the programs and the members of the graphs the backend
//! compiled once; nothing of a tile is drawn on the processor where the backend has a GPU.

use ferroui_base::animation::{
    Animation, AnimationTask, Cue, EffectTransition, IAnimationSetter, IClock, IterationCount, KeyFrame, PlaybackDirection,
    SignedDuration, Transitions,
};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::effects::{
    CustomEffect, CustomEffectClass, Effect, EffectExtensions, EffectParameterValue, IEffect, ImmutableCustomEffect,
};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, FontFamily, FontWeight, IBrush, TextWrapping};
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{BoxedValue, CornerRadius, Ref, Thickness, Upcast, Visual};
use ferroui_controls::{Border, Button, Control, Slider, StackPanel, TextBlock, WrapPanel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The width of a tile.
pub(crate) const TILE_WIDTH: f64 = 268.0;

/// The duration of the transition of the effect of a host.
pub const TRANSITION_MS: f64 = 300.0;

/// The duration of one sweep of the key frames of a tile.
pub const SWEEP_MS: f64 = 1600.0;

/// The colours of the swatches of the sample.
const SWATCHES: [(u8, u8, u8); 4] = [(0xE8, 0x5D, 0x3A), (0x2E, 0x86, 0xDE), (0x3F, 0x9E, 0x5A), (0xD9, 0xA4, 0x41)];

/// A colour a tile offers for a colour parameter: its name and its red, green and blue.
pub(crate) type Swatch = (&'static str, (u8, u8, u8));

pub(crate) fn brush(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

pub(crate) fn text(value: &str, size: f64, weight: FontWeight) -> Ref<TextBlock> {
    let block = TextBlock::new();
    block.set_text(Some(value));
    block.set_font_size(size);
    block.set_font_weight(weight);
    block.set_text_wrapping(TextWrapping::Wrap);
    block
}

pub(crate) fn caption(value: &str) -> Ref<TextBlock> {
    let block = text(value, 12.0, FontWeight::Normal);
    block.set_opacity(0.75);
    block
}

/// Code, in a monospaced font on a faint panel.
pub(crate) fn code_block(value: &str) -> Ref<Border> {
    let block = text(value, 11.0, FontWeight::Normal);
    block.set_font_family(FontFamily::new("Cascadia Mono,Consolas,Menlo,monospace"));
    let panel = Border::new();
    panel.set_padding(Thickness::new(8.0, 6.0, 8.0, 6.0));
    panel.set_corner_radius(CornerRadius::uniform(4.0));
    panel.set_background(Some(brush(Color::from_argb(0x18, 0x80, 0x80, 0x80))));
    panel.set_child(block);
    panel
}

pub(crate) fn column(spacing: f64) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_spacing(spacing);
    panel
}

pub(crate) fn row(spacing: f64) -> Ref<StackPanel> {
    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Horizontal);
    panel.set_spacing(spacing);
    panel
}

pub(crate) fn button(label: &str) -> Ref<Button> {
    let button = Button::new();
    set_label(&button, label);
    button
}

pub(crate) fn set_label(button: &Button, label: &str) {
    button.set_content(Some(Rc::new(label.to_owned()) as BoxedValue));
}

/// The text of a button made by [`button`].
pub fn label_of(button: &Button) -> Option<String> {
    button.content().and_then(|content| content.downcast_ref::<String>().cloned())
}

/// A section of the gallery: its title, its description and its tiles.
pub(crate) fn section(title: &str, description: &str, tiles: &[EffectTile]) -> Ref<StackPanel> {
    let panel = column(8.0);
    panel.children().add(text(title, 18.0, FontWeight::SemiBold));
    let about = text(description, 13.0, FontWeight::Normal);
    about.set_opacity(0.7);
    panel.children().add(about);
    let wrap = WrapPanel::new();
    wrap.set_orientation(Orientation::Horizontal);
    for tile in tiles {
        wrap.children().add(tile.root.clone());
    }
    panel.children().add(wrap);
    panel
}

/// The content every effect is shown on: four round swatches and a title, on no background, so
/// that a glow shows around their shapes.
pub(crate) fn sample() -> Ref<Border> {
    let swatches = row(8.0);
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

/// The sample on a dark panel of a fixed size: for the effects drawn over an area (the shaders,
/// the transitions), and for the interactive effects, so that the whole panel takes the pointer
/// (an element without a background is not hit by it).
pub(crate) fn surface() -> Ref<Border> {
    let surface = sample();
    surface.set_width(240.0);
    surface.set_height(150.0);
    surface.set_corner_radius(CornerRadius::uniform(8.0));
    surface.set_background(Some(brush(Color::from_rgb(0x1E, 0x27, 0x3A))));
    surface
}

/// A slider with its name and value above it; `changed` gets the new value.
fn labelled_slider(name: &'static str, range: (f64, f64), value: f64, changed: impl Fn(f64) + 'static) -> (Ref<StackPanel>, Ref<Slider>) {
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
    panel.children().add(slider.clone());
    (panel, slider)
}

/// The effect string of a snapshot as markup writes it.
pub fn markup_of(effect: &ImmutableCustomEffect) -> String {
    format!("Effect=\"{effect}\"")
}

/// Runs an animation on a host, on `clock` (the clock of the host when `None`).
pub(crate) type Play = Rc<dyn Fn(Option<Rc<dyn IClock>>) -> AnimationTask>;

/// A tile of the gallery: its title, the element its effect is set on, the effect string of the
/// current snapshot, and what changes and animates it.
pub struct EffectTile {
    /// The title of the tile.
    pub title: &'static str,
    /// The name of the effect in effect strings ([`CustomEffect::NAME`]).
    pub name: &'static str,
    /// The tile.
    pub root: Ref<StackPanel>,
    /// The element whose `Effect` the tile sets.
    pub host: Ref<Control>,
    /// The effect string of the current snapshot, as markup writes it.
    pub markup: Ref<TextBlock>,
    /// The code that makes the effect.
    pub code: &'static str,
    /// A slider for each number of the effect, with the name of the number.
    pub sliders: Vec<(&'static str, Ref<Slider>)>,
    /// A button for each colour the tile offers, with the name of the colour.
    pub swatches: Vec<(&'static str, Ref<Button>)>,
    /// The button that turns the effect off and on again.
    pub toggle: Option<Ref<Button>>,
    /// The button that plays the key frames, or the next page of a presenter.
    pub play_button: Option<Ref<Button>>,
    play: Option<Play>,
}

impl EffectTile {
    /// The slider of the number `name`.
    pub fn slider(&self, name: &str) -> Option<&Ref<Slider>> {
        self.sliders.iter().find(|(number, _)| *number == name).map(|(_, slider)| slider)
    }

    /// The snapshot of the effect of the host: the current value, or the animated one while an
    /// animation runs.
    pub fn effect(&self) -> Option<ImmutableCustomEffect> {
        self.host.effect().and_then(|effect| EffectExtensions::to_custom_effect(&*effect))
    }

    /// Plays the key frames of the tile (what its play button does) on `clock`, or on the clock
    /// of the host when `None`. `None` for a tile without key frames.
    pub fn play(&self, clock: Option<Rc<dyn IClock>>) -> Option<AnimationTask> {
        self.play.as_ref().map(|play| play(clock))
    }
}

/// A number of an effect with the range of its slider.
#[derive(Clone, Copy, Debug)]
struct Number {
    name: &'static str,
    minimum: f64,
    maximum: f64,
}

/// How the key frames of a tile sweep its first number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sweep {
    /// From the minimum to the maximum and back.
    ThereAndBack,
    /// From the minimum to the maximum (the progress of a transition).
    Forward,
}

/// Key frames of `Visual.Effect` from `from` (cue 0) to `to` (cue 1) over `duration_ms`; the
/// effect between them is interpolated parameter by parameter (`CustomEffect::interpolate`).
pub(crate) fn key_frames<T: CustomEffect>(from: T, to: T, duration_ms: f64, sweep: Sweep) -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_milliseconds(duration_ms));
    for (cue, value) in [(0.0, from), (1.0, to)] {
        let effect: Option<Rc<dyn IEffect>> = Some(ImmutableCustomEffect::new(value).into());
        let setter = Setter::new(Visual::effect_property(), effect) as Rc<dyn IAnimationSetter>;
        animation.children().add(KeyFrame::with_cue(Cue::new(cue), [setter]));
    }
    if sweep == Sweep::ThereAndBack {
        animation.set_iteration_count(IterationCount::new(2));
        animation.set_playback_direction(PlaybackDirection::Alternate);
    }
    animation
}

/// The key frames that sweep the number `number` of `value` from its minimum to its maximum.
fn sweep_animation<T: CustomEffect>(value: &T, number: Number, sweep: Sweep) -> Ref<Animation> {
    let at = |x: f64| {
        let mut value = value.clone();
        value.set_parameter(number.name, EffectParameterValue::Float(x));
        value
    };
    key_frames(at(number.minimum), at(number.maximum), SWEEP_MS, sweep)
}

/// The transitions of a host: its effect, over [`TRANSITION_MS`].
fn effect_transitions() -> Transitions {
    let transition = EffectTransition::new();
    transition.set_property(Some(Visual::effect_property()));
    transition.set_duration(SignedDuration::from_milliseconds(TRANSITION_MS));
    Transitions::from_items([transition.into()])
}

/// The builder of the tile of a value of a custom effect.
pub(crate) struct ValueTile<T: CustomEffect> {
    title: &'static str,
    value: T,
    code: &'static str,
    numbers: Vec<Number>,
    colors: Option<(&'static str, &'static [Swatch])>,
    host: Option<Ref<Border>>,
    sweep: Sweep,
}

impl<T: CustomEffect> ValueTile<T> {
    /// The tile `title` of `value`, made by `code`.
    pub(crate) fn new(title: &'static str, value: T, code: &'static str) -> Self {
        Self { title, value, code, numbers: Vec::new(), colors: None, host: None, sweep: Sweep::ThereAndBack }
    }

    /// A slider for the number `name`, from `minimum` to `maximum`. The key frames sweep the
    /// first one.
    pub(crate) fn number(mut self, name: &'static str, minimum: f64, maximum: f64) -> Self {
        self.numbers.push(Number { name, minimum, maximum });
        self
    }

    /// A button for each of `swatches`, which sets the colour parameter `name`.
    pub(crate) fn colors(mut self, name: &'static str, swatches: &'static [Swatch]) -> Self {
        self.colors = Some((name, swatches));
        self
    }

    /// The element the effect is set on, in place of [`sample`].
    pub(crate) fn host(mut self, host: Ref<Border>) -> Self {
        self.host = Some(host);
        self
    }

    /// How the key frames sweep the first number.
    pub(crate) fn sweep(mut self, sweep: Sweep) -> Self {
        self.sweep = sweep;
        self
    }

    pub(crate) fn build(self) -> EffectTile {
        let ValueTile { title, value, code, numbers, colors, host, sweep } = self;
        let host = host.unwrap_or_else(sample);
        host.set_transitions(Some(effect_transitions()));
        let markup = caption("");
        let value = Rc::new(RefCell::new(value));
        let enabled = Rc::new(Cell::new(true));
        // The snapshot of the value is the effect of the host; nothing is borrowed while the
        // host and the caption change.
        let apply: Rc<dyn Fn()> = {
            let (host, markup, value, enabled) = (host.downgrade(), markup.downgrade(), value.clone(), enabled.clone());
            Rc::new(move || {
                let effect = ImmutableCustomEffect::new(value.borrow().clone());
                if let Some(markup) = markup.upgrade() {
                    markup.set_text(Some(&markup_of(&effect)));
                }
                if let Some(host) = host.upgrade() {
                    let effect: Option<Rc<dyn IEffect>> = if enabled.get() { Some(effect.into()) } else { None };
                    host.set_effect(effect);
                }
            })
        };
        apply();

        let root = column(6.0);
        root.set_width(TILE_WIDTH);
        root.set_margin(Thickness::new(0.0, 0.0, 16.0, 16.0));
        root.children().add(text(title, 15.0, FontWeight::SemiBold));
        root.children().add(host.clone());

        let current = ImmutableCustomEffect::new(value.borrow().clone());
        let mut sliders = Vec::new();
        for number in &numbers {
            let name = number.name;
            let start = current.parameter(name).and_then(|parameter| parameter.as_float()).unwrap_or(number.minimum);
            let (value, apply) = (value.clone(), apply.clone());
            let (panel, slider) = labelled_slider(name, (number.minimum, number.maximum), start, move |new| {
                let changed = value.borrow_mut().set_parameter(name, EffectParameterValue::Float(new));
                if changed {
                    apply();
                }
            });
            root.children().add(panel);
            sliders.push((name, slider));
        }

        let mut swatches = Vec::new();
        if let Some((name, choices)) = colors {
            root.children().add(caption(name));
            let buttons = row(4.0);
            for &(label, (r, g, b)) in choices {
                let color = Color::from_rgb(r, g, b);
                let swatch = button(label);
                let (value, apply) = (value.clone(), apply.clone());
                let _ = swatch.click(move |_, _| {
                    let changed = value.borrow_mut().set_parameter(name, EffectParameterValue::Color(color));
                    if changed {
                        apply();
                    }
                });
                buttons.children().add(swatch.clone());
                swatches.push((label, swatch));
            }
            root.children().add(buttons);
        }

        // The animations: the effect off and on again through the transition of the host, and the
        // key frames of the first number.
        let toggle = button("Turn off");
        {
            let (apply, enabled, weak) = (apply.clone(), enabled.clone(), toggle.downgrade());
            let _ = toggle.click(move |_, _| {
                enabled.set(!enabled.get());
                if let Some(toggle) = weak.upgrade() {
                    set_label(&toggle, if enabled.get() { "Turn off" } else { "Turn on" });
                }
                apply();
            });
        }
        let play: Option<Play> = numbers.first().copied().map(|number| {
            let (host, value) = (host.downgrade(), value.clone());
            Rc::new(move |clock: Option<Rc<dyn IClock>>| {
                let Some(host) = host.upgrade() else { return AnimationTask::completed() };
                let animation = sweep_animation(&*value.borrow(), number, sweep);
                animation.run_async_with_clock(&host, clock, CancellationToken::none())
            }) as Play
        });
        let actions = row(8.0);
        actions.children().add(toggle.clone());
        let play_button = play.as_ref().map(|play| {
            let label = match sweep {
                Sweep::ThereAndBack => format!("Sweep {}", numbers[0].name),
                Sweep::Forward => String::from("Play"),
            };
            let play_button = button(&label);
            let play = play.clone();
            let _ = play_button.click(move |_, _| drop(play(None)));
            actions.children().add(play_button.clone());
            play_button
        });
        root.children().add(actions);
        root.children().add(markup.clone());
        root.children().add(code_block(code));

        EffectTile {
            title,
            name: T::NAME,
            root,
            host: host.upcast(),
            markup,
            code,
            sliders,
            swatches,
            toggle: Some(toggle),
            play_button,
            play,
        }
    }
}

/// A caption with the effect string of the current snapshot of `effect`, kept up to date as the
/// pointer changes it.
pub(crate) fn snapshot_caption<T: CustomEffectClass + Upcast<Effect>>(effect: &Ref<T>) -> Ref<TextBlock> {
    let markup = caption(&markup_of(&effect.to_custom_effect()));
    let (weak_effect, weak_markup) = (effect.downgrade(), markup.downgrade());
    let base: &Effect = Upcast::<Effect>::upcast(&**effect);
    base.invalidated(move || {
        if let (Some(effect), Some(markup)) = (weak_effect.upgrade(), weak_markup.upgrade()) {
            markup.set_text(Some(&markup_of(&effect.to_custom_effect())));
        }
    })
    .detach();
    markup
}

/// The tile of an interactive effect class: its title, what to do, the host and the effect string
/// of the snapshot the pointer changes.
pub(crate) fn interactive_tile<T: CustomEffectClass + Upcast<Effect>>(
    title: &'static str,
    name: &'static str,
    hint: &str,
    code: &'static str,
    effect: &Ref<T>,
    host: Ref<Control>,
) -> EffectTile {
    let markup = snapshot_caption(effect);
    let root = column(6.0);
    root.set_width(TILE_WIDTH);
    root.set_margin(Thickness::new(0.0, 0.0, 16.0, 16.0));
    root.children().add(text(title, 15.0, FontWeight::SemiBold));
    root.children().add(caption(hint));
    root.children().add(host.clone());
    root.children().add(markup.clone());
    root.children().add(code_block(code));
    EffectTile {
        title,
        name,
        root,
        host,
        markup,
        code,
        sliders: Vec::new(),
        swatches: Vec::new(),
        toggle: None,
        play_button: None,
        play: None,
    }
}

/// A tile whose element and action the caller made: the presenter of the page transitions, the
/// ways of setting an effect of the gallery. `play` is the button of the action and the action.
pub(crate) fn custom_tile(
    title: &'static str,
    name: &'static str,
    hint: &str,
    code: &'static str,
    host: Ref<Control>,
    markup: Ref<TextBlock>,
    play: Option<(Ref<Button>, Play)>,
) -> EffectTile {
    let root = column(6.0);
    root.set_width(TILE_WIDTH);
    root.set_margin(Thickness::new(0.0, 0.0, 16.0, 16.0));
    root.children().add(text(title, 15.0, FontWeight::SemiBold));
    root.children().add(caption(hint));
    root.children().add(host.clone());
    let (play_button, play) = match play {
        Some((play_button, play)) => {
            root.children().add(play_button.clone());
            (Some(play_button), Some(play))
        }
        None => (None, None),
    };
    root.children().add(markup.clone());
    root.children().add(code_block(code));
    EffectTile {
        title,
        name,
        root,
        host,
        markup,
        code,
        sliders: Vec::new(),
        swatches: Vec::new(),
        toggle: None,
        play_button,
        play,
    }
}
