//! The families of the gallery, one section each (section 3.7 of `docs/porting/custom-effects.md`):
//! the colour effects, the image effects, the shader effects, the interactive effects and the page
//! transitions, each effect of the library (`ferroui_base::media::effects::library`, Effector's
//! sample set) in a tile of its own.

use crate::tiles::{
    brush, button, caption, custom_tile, interactive_tile, key_frames, markup_of, row, section, surface, text, EffectTile,
    Play, Swatch, Sweep, ValueTile,
};
use crate::transition_images::TransitionImages;
use ferroui_base::animation::{AnimationTask, IClock};
use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::media::effects::library::{
    BrightnessContrastEffect, DissolveTransitionEffect, EdgeDetectEffect, GlowEffect, GrayscaleEffect,
    InteractiveBurningFlameEffect, InteractivePointerSpotlightEffect, InteractiveReactiveGridEffect,
    InteractiveWaterRippleEffect, InvertEffect, MaskTransitionEffect, PixelateEffect, ReactiveGridShaderEffect,
    SaturationEffect, ScanlineShaderEffect, SepiaEffect, SharpenEffect, SpotlightShaderEffect, TintEffect,
};
use ferroui_base::media::effects::{CustomEffect, EffectImage, ImmutableCustomEffect};
use ferroui_base::media::{Color, FontWeight};
use ferroui_base::threading::CancellationToken;
use ferroui_base::{CornerRadius, Ref, Thickness, WeakRef};
use ferroui_controls::{Border, Button, StackPanel, TextBlock};
use std::cell::Cell;
use std::rc::Rc;

/// The duration of a transition of the presenter.
pub const PRESENTER_MS: f64 = 1200.0;

const TINTS: &[Swatch] = &[("Teal", (0x0F, 0x9D, 0x8E)), ("Sky", (0x00, 0xBF, 0xFF)), ("Amber", (0xF5, 0x9E, 0x0B)), ("Rose", (0xF0, 0x6B, 0x92))];
const GLOWS: &[Swatch] = &[("Gold", (0xFF, 0xD7, 0x00)), ("Cyan", (0x3F, 0xC3, 0xFF)), ("Lime", (0x74, 0xD6, 0x4F)), ("Rose", (0xFF, 0x9E, 0xC8))];
const GRIDS: &[Swatch] = &[("Cyan", (0x3F, 0xC3, 0xFF)), ("Lime", (0x74, 0xD6, 0x4F)), ("Amber", (0xFF, 0xB3, 0x47)), ("Rose", (0xF0, 0x6B, 0x92))];
const LIGHTS: &[Swatch] = &[("Gold", (0xFF, 0xD2, 0x6B)), ("Ice", (0xB9, 0xEC, 0xFF)), ("Mint", (0x9D, 0xFF, 0xC2)), ("Rose", (0xFF, 0x9E, 0xC8))];

/// A family of custom effects: a section of the gallery and a page of the ControlCatalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// Tint, grayscale, sepia, saturation, brightness and contrast, invert: colour matrices.
    Colour,
    /// Glow, sharpen, edge detect, pixelate: what reads the neighbourhood of a pixel.
    Image,
    /// Scan lines, a grid and a spotlight: runtime shaders over an area.
    Shader,
    /// The pointer spotlight, the reactive grid, the water ripple and the burn-away button.
    Interactive,
    /// The dissolve and the mask transitions between two pages.
    Transitions,
}

impl Family {
    /// Every family, in the order of the gallery and of the pages of the catalog.
    pub const ALL: [Family; 5] = [Family::Colour, Family::Image, Family::Shader, Family::Interactive, Family::Transitions];

    /// The title of the section, the header of the page of the catalog.
    pub fn title(self) -> &'static str {
        match self {
            Family::Colour => "Colour effects",
            Family::Image => "Image effects",
            Family::Shader => "Shader effects",
            Family::Interactive => "Interactive effects",
            Family::Transitions => "Page transitions",
        }
    }

    /// What the section shows and how it is drawn.
    pub fn description(self) -> &'static str {
        match self {
            Family::Colour => "Custom effects that describe themselves as effect graphs of colour matrices (CustomEffect::graph): the backend compiles each graph once and draws it around the content on the GPU; a slider changes only the values of its members. Each is a value set as Visual.Effect from code, or from markup as the effect string under it. The transition of the host fades a change in, and turning an effect off fades it out through its neutral value.",
            Family::Image => "Effects that read the neighbourhood of a pixel: the glow is a drop shadow of the content in a colour, sharpen and edge detect are graphs of a convolution, and pixelate is a runtime shader that samples the content (SkSL on Skia, WGSL on Vello). Each is drawn on the GPU; Vello's CPU mode draws its CPU reference.",
            Family::Shader => "Runtime shaders drawn over the area of their host: scan lines, a grid and a spotlight, each an SkSL program Skia compiles once and a WGSL program Vello compiles once; the parameters are their uniforms. The last tile is a spotlight over cards that have tints of their own: the effect of a visual is drawn around the effects of its children.",
            Family::Interactive => "Effect classes that implement IEffectInputHandler: set as Visual.Effect on an input element, they receive its pointer input and change their value, and the element takes a new snapshot, so the program, compiled once, is drawn with new uniforms. The water ripple and the flame advance on the frames of the animation clock while they move.",
            Family::Transitions => "Runtime shaders that sample two pages and a pattern as images besides the content: each backend uploads an image once into a texture and binds it as a child of the program. Their progress is a number, so key frames of Visual.Effect play a transition. The presenter goes to the next page from where it is pressed, as Effector's compiz-style sample does.",
        }
    }

    /// The tiles of the family; the page transitions sample `images`.
    pub fn tiles(self, images: &TransitionImages) -> Vec<EffectTile> {
        match self {
            Family::Colour => colour_tiles(),
            Family::Image => image_tiles(),
            Family::Shader => shader_tiles(),
            Family::Interactive => interactive_tiles(),
            Family::Transitions => transition_tiles(images),
        }
    }

    /// The section of the family: its title, its description and its tiles.
    pub fn section(self, images: &TransitionImages) -> Ref<StackPanel> {
        section(self.title(), self.description(), &self.tiles(images))
    }
}

// --- the colour effects -------------------------------------------------------------------------

fn colour_tiles() -> Vec<EffectTile> {
    vec![
        ValueTile::new(
            "Tint",
            TintEffect { color: Color::from_rgb(0x0F, 0x9D, 0x8E), strength: 0.55 },
            "TintEffect { color: Color::from_rgb(0x0F, 0x9D, 0x8E), strength: 0.55 }",
        )
        .number("strength", 0.0, 1.0)
        .colors("color", TINTS)
        .build(),
        ValueTile::new("Grayscale", GrayscaleEffect::default(), "GrayscaleEffect { amount: 1.0 }").number("amount", 0.0, 1.0).build(),
        ValueTile::new("Sepia", SepiaEffect::default(), "SepiaEffect { amount: 1.0 }").number("amount", 0.0, 1.0).build(),
        ValueTile::new("Saturation", SaturationEffect { saturation: 1.8 }, "SaturationEffect { saturation: 1.8 }")
            .number("saturation", 0.0, SaturationEffect::MAXIMUM)
            .build(),
        ValueTile::new(
            "Brightness and contrast",
            BrightnessContrastEffect { brightness: 0.1, contrast: 1.4 },
            "BrightnessContrastEffect { brightness: 0.1, contrast: 1.4 }",
        )
        .number("brightness", -1.0, 1.0)
        .number("contrast", 0.0, BrightnessContrastEffect::MAXIMUM_CONTRAST)
        .build(),
        ValueTile::new("Invert", InvertEffect::default(), "InvertEffect { amount: 1.0 }").number("amount", 0.0, 1.0).build(),
    ]
}

// --- the image effects --------------------------------------------------------------------------

fn image_tiles() -> Vec<EffectTile> {
    vec![
        ValueTile::new("Glow", GlowEffect::default(), "GlowEffect { color: Colors::GOLD, blur_radius: 12.0, intensity: 0.9 }")
            .number("blur-radius", 0.0, 24.0)
            .number("intensity", 0.0, 1.0)
            .colors("color", GLOWS)
            .build(),
        ValueTile::new("Sharpen", SharpenEffect::default(), "SharpenEffect { strength: 1.0 }").number("strength", 0.0, 2.0).build(),
        ValueTile::new("Edge detect", EdgeDetectEffect::default(), "EdgeDetectEffect { strength: 1.0 }")
            .number("strength", 0.0, 2.0)
            .build(),
        ValueTile::new("Pixelate", PixelateEffect { cell_size: 6.0 }, "PixelateEffect { cell_size: 6.0 }")
            .number("cell-size", 0.0, PixelateEffect::MAXIMUM_CELL_SIZE)
            .build(),
    ]
}

// --- the shader effects -------------------------------------------------------------------------

/// Three cards with tints of their own, on a dark panel: the host of the nested spotlight.
fn tinted_cards() -> Ref<Border> {
    let cards = row(8.0);
    for (title, (r, g, b)) in [("Ocean", (0x2E, 0x86, 0xDE)), ("Moss", (0x3F, 0x9E, 0x5A)), ("Ember", (0xE8, 0x5D, 0x3A))] {
        let card = Border::new();
        card.set_width(60.0);
        card.set_height(96.0);
        card.set_padding(Thickness::uniform(6.0));
        card.set_corner_radius(CornerRadius::uniform(8.0));
        card.set_background(Some(brush(Color::from_rgb(0xF4, 0xF1, 0xEA))));
        card.set_child(text(title, 12.0, FontWeight::SemiBold));
        card.set_effect(Some(ImmutableCustomEffect::new(TintEffect { color: Color::from_rgb(r, g, b), strength: 0.6 }).into()));
        cards.children().add(card);
    }
    let host = Border::new();
    host.set_width(240.0);
    host.set_height(150.0);
    host.set_padding(Thickness::uniform(16.0));
    host.set_corner_radius(CornerRadius::uniform(8.0));
    host.set_background(Some(brush(Color::from_rgb(0x1E, 0x27, 0x3A))));
    host.set_child(cards);
    host
}

fn shader_tiles() -> Vec<EffectTile> {
    vec![
        ValueTile::new("Scan lines", ScanlineShaderEffect { spacing: 4.0, strength: 0.4 }, "ScanlineShaderEffect { spacing: 4.0, strength: 0.4 }")
            .host(surface())
            .number("strength", 0.0, 1.0)
            .number("spacing", 2.0, 32.0)
            .build(),
        ValueTile::new(
            "Grid",
            ReactiveGridShaderEffect { cell_size: 22.0, strength: 0.24, color: Color::from_rgb(0x3F, 0xC3, 0xFF), ..ReactiveGridShaderEffect::default() },
            "ReactiveGridShaderEffect { cell_size: 22.0, strength: 0.24, ..Default::default() }",
        )
        .host(surface())
        .number("strength", 0.0, 0.8)
        .number("cell-size", 8.0, 64.0)
        .colors("color", GRIDS)
        .build(),
        ValueTile::new(
            "Spotlight",
            SpotlightShaderEffect { strength: 0.8, ..SpotlightShaderEffect::default() },
            "SpotlightShaderEffect { strength: 0.8, ..Default::default() }",
        )
        .host(surface())
        .number("center-x", 0.0, 1.0)
        .number("center-y", 0.0, 1.0)
        .number("radius", 0.05, 1.0)
        .number("strength", 0.0, 1.0)
        .colors("color", LIGHTS)
        .build(),
        ValueTile::new(
            "Spotlight over tinted cards",
            SpotlightShaderEffect { center_x: 0.78, center_y: 0.3, radius: 0.5, strength: 0.6, ..SpotlightShaderEffect::default() },
            "SpotlightShaderEffect { center_x: 0.78, center_y: 0.3, radius: 0.5, strength: 0.6, ..Default::default() }",
        )
        .host(tinted_cards())
        .number("center-x", 0.0, 1.0)
        .number("center-y", 0.0, 1.0)
        .build(),
    ]
}

// --- the interactive effects --------------------------------------------------------------------

fn interactive_tiles() -> Vec<EffectTile> {
    let spotlight = InteractivePointerSpotlightEffect::new();
    let spotlight_host = surface();
    spotlight_host.set_effect(Some(spotlight.clone().into()));

    let grid = InteractiveReactiveGridEffect::new();
    let grid_host = surface();
    grid_host.set_effect(Some(grid.clone().into()));

    let ripple = InteractiveWaterRippleEffect::new();
    let ripple_host = surface();
    ripple_host.set_effect(Some(ripple.clone().into()));

    // The burn-away button: the flame takes the press the button handles.
    let flame = InteractiveBurningFlameEffect::new();
    let burn = button("Burn away");
    burn.set_width(240.0);
    burn.set_height(72.0);
    burn.set_effect(Some(flame.clone().into()));

    vec![
        interactive_tile(
            "Pointer spotlight",
            "pointer-spotlight",
            "Move the pointer over the panel; press to brighten the light.",
            "let spotlight = InteractivePointerSpotlightEffect::new();\npanel.set_effect(Some(spotlight.into()));",
            &spotlight,
            spotlight_host.upcast(),
        ),
        interactive_tile(
            "Reactive grid",
            "reactive-grid",
            "The halo follows the pointer and widens while pressed.",
            "let grid = InteractiveReactiveGridEffect::new();\npanel.set_effect(Some(grid.into()));",
            &grid,
            grid_host.upcast(),
        ),
        interactive_tile(
            "Water ripple",
            "water-ripple",
            "Press, drag and release: rings spread from the pointer and bend the content.",
            "let ripple = InteractiveWaterRippleEffect::new();\npanel.set_effect(Some(ripple.into()));",
            &ripple,
            ripple_host.upcast(),
        ),
        interactive_tile(
            "Burn-away button",
            "burning-flame",
            "Press the button: it burns where it is pressed and burns down after the release.",
            "let flame = InteractiveBurningFlameEffect::new();\nbutton.set_effect(Some(flame.into()));",
            &flame,
            burn.upcast(),
        ),
    ]
}

// --- the page transitions -----------------------------------------------------------------------

fn transition_tiles(images: &TransitionImages) -> Vec<EffectTile> {
    let TransitionImages { first_page, second_page, pattern } = images.clone();
    vec![
        ValueTile::new(
            "Dissolve",
            DissolveTransitionEffect {
                from_page: first_page,
                to_page: second_page.clone(),
                pattern: pattern.clone(),
                progress: 0.5,
                ..DissolveTransitionEffect::default()
            },
            "DissolveTransitionEffect { from_page, to_page, pattern, progress: 0.5, ..Default::default() }",
        )
        .host(surface())
        .number("progress", 0.0, 1.0)
        .number("center-x", 0.0, 1.0)
        .number("center-y", 0.0, 1.0)
        .sweep(Sweep::Forward)
        .build(),
        ValueTile::new(
            "Mask",
            MaskTransitionEffect { to_page: second_page, mask: pattern, progress: 0.5, softness: 0.15 },
            "MaskTransitionEffect { to_page, mask, progress: 0.5, softness: 0.15 }",
        )
        .host(surface())
        .number("progress", 0.0, 1.0)
        .number("softness", 0.001, 1.0)
        .sweep(Sweep::Forward)
        .build(),
        presenter_tile(images),
    ]
}

/// The presenter of the page transitions: it shows one of two pages, and a press (or its button)
/// dissolves to the other from where it was pressed, by key frames of the progress of a dissolve.
struct Presenter {
    pages: [EffectImage; 2],
    pattern: EffectImage,
    /// The index of the page shown.
    shown: Cell<usize>,
    host: WeakRef<Border>,
    markup: WeakRef<TextBlock>,
}

impl Presenter {
    /// The dissolve from the page `from` to the other one at `progress`, spreading from `center`.
    fn dissolve(&self, from: usize, progress: f64, center: (f64, f64)) -> DissolveTransitionEffect {
        DissolveTransitionEffect {
            from_page: self.pages[from].clone(),
            to_page: self.pages[1 - from].clone(),
            pattern: self.pattern.clone(),
            progress,
            center_x: center.0,
            center_y: center.1,
            ..DissolveTransitionEffect::default()
        }
    }

    /// Sets the resting effect of the host: the page `shown`.
    fn rest(&self, center: (f64, f64)) -> Option<Ref<Border>> {
        let host = self.host.upgrade()?;
        let effect = ImmutableCustomEffect::new(self.dissolve(1 - self.shown.get(), 1.0, center));
        if let Some(markup) = self.markup.upgrade() {
            markup.set_text(Some(&markup_of(&effect)));
        }
        host.set_effect(Some(effect.into()));
        Some(host)
    }

    /// Goes to the other page from `center` (fractions of the width and the height of the
    /// host): the host rests on the other page, and the key frames play the dissolve to it, on
    /// `clock` (the clock of the host when `None`).
    fn advance(&self, center: (f64, f64), clock: Option<Rc<dyn IClock>>) -> AnimationTask {
        let from = self.shown.get();
        self.shown.set(1 - from);
        let Some(host) = self.rest(center) else { return AnimationTask::completed() };
        let animation = key_frames(self.dissolve(from, 0.0, center), self.dissolve(from, 1.0, center), PRESENTER_MS, Sweep::Forward);
        animation.run_async_with_clock(&host, clock, CancellationToken::none())
    }
}

/// The corners and the middle the button of the presenter dissolves from, in turn.
const PRESENTER_CENTERS: [(f64, f64); 5] = [(0.5, 0.5), (0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];

fn presenter_tile(images: &TransitionImages) -> EffectTile {
    let host = surface();
    let markup = caption("");
    let presenter = Rc::new(Presenter {
        pages: [images.first_page.clone(), images.second_page.clone()],
        pattern: images.pattern.clone(),
        shown: Cell::new(0),
        host: host.downgrade(),
        markup: markup.downgrade(),
    });
    let _ = presenter.rest((0.5, 0.5));

    // A press dissolves from where it is.
    {
        let presenter = presenter.clone();
        let _ = host.add_handler(InputElement::pointer_pressed_event(), move |_, e: &PointerPressedEventArgs| {
            let Some(host) = presenter.host.upgrade() else { return };
            let position = e.get_position(Some(&host));
            let bounds = host.bounds();
            let center = ((position.x / bounds.width.max(1.0)).clamp(0.0, 1.0), (position.y / bounds.height.max(1.0)).clamp(0.0, 1.0));
            drop(presenter.advance(center, None));
        });
    }
    let turn = Rc::new(Cell::new(0usize));
    let play: Play = {
        let (presenter, turn) = (presenter.clone(), turn.clone());
        Rc::new(move |clock: Option<Rc<dyn IClock>>| {
            let center = PRESENTER_CENTERS[turn.get() % PRESENTER_CENTERS.len()];
            turn.set(turn.get() + 1);
            presenter.advance(center, clock)
        })
    };
    let next: Ref<Button> = button("Next page");
    {
        let play = play.clone();
        let _ = next.click(move |_, _| drop(play(None)));
    }
    custom_tile(
        "Presenter",
        DissolveTransitionEffect::NAME,
        "Press the page, or the button: the next page dissolves in from that point.",
        "let start = DissolveTransitionEffect { from_page, to_page, pattern, progress: 0.0, center_x, center_y, ..Default::default() };\nlet end = DissolveTransitionEffect { progress: 1.0, ..start.clone() };\nhost.set_effect(Some(ImmutableCustomEffect::new(end.clone()).into()));\nkey_frames(start, end).run_async(&host, CancellationToken::none());",
        host.upcast(),
        markup,
        Some((next, play)),
    )
}
