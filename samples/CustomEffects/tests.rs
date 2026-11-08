//! The tests of the gallery, headless: the sections are built on the test application of
//! `ferroui-controls` with the Skia render interface of `sample-testing` (so that the pages of the
//! transitions render into images) and a global clock the tests tick, and their tiles are driven
//! as a user would (a slider moved, a button clicked).
//!
//! Not ports: Effector's sample applications have no tests of their own.

use crate::{
    authoring_tiles, gallery, label_of, markup_of, EffectTile, Family, TransitionImages, GLOW_MS, PAGE_HEIGHT, PAGE_WIDTH,
    PRESENTER_MS, SWEEP_MS, TINT_MARKUP,
};
use ferroui_base::animation::{IClock, SignedDuration};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::effects::library::{
    DissolveTransitionEffect, GlowEffect, InteractivePointerSpotlightEffect, InteractiveWaterRippleEffect, TintEffect,
};
use ferroui_base::media::effects::{Effect, EffectExtensions, ImmutableCustomEffect};
use ferroui_base::media::Color;
use ferroui_base::threading::Dispatcher;
use ferroui_base::PixelSize;
use ferroui_controls::primitives::UniformGrid;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Button, Control, Image};
use sample_testing::TestGlobalClock;
use std::rc::Rc;

/// The test application and its global clock.
fn start() -> (UnitTestApplicationScope, Rc<TestGlobalClock>) {
    let clock = Rc::new(TestGlobalClock::default());
    let scope = UnitTestApplication::start(sample_testing::services(clock.clone()));
    (scope, clock)
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);
}

fn milliseconds(value: f64) -> SignedDuration {
    SignedDuration::from_milliseconds(value)
}

/// The number `name` of the effect of the host of `tile`.
fn number(tile: &EffectTile, name: &str) -> Option<f64> {
    tile.effect().and_then(|effect| effect.parameter(name)).and_then(|value| value.as_float())
}

fn assert_close(expected: f64, actual: Option<f64>, what: &str) {
    let actual = actual.unwrap_or_else(|| panic!("{what}: no value"));
    assert!((expected - actual).abs() < 1e-6, "{what}: {actual}, expected {expected}");
}

/// The tile of `family` whose title is `title`.
fn tile(family: Family, images: &TransitionImages, title: &str) -> EffectTile {
    family.tiles(images).into_iter().find(|tile| tile.title == title).unwrap_or_else(|| panic!("the tile {title}"))
}

/// The dissolve of the host of `tile`.
fn dissolve(tile: &EffectTile) -> DissolveTransitionEffect {
    tile.effect().and_then(|effect| effect.downcast_ref::<DissolveTransitionEffect>().cloned()).expect("a dissolve")
}

#[test]
fn every_family_has_a_tile_per_effect_of_the_library() {
    let _app = start();
    let images = TransitionImages::default();
    let expected: [(Family, &[&str]); 5] = [
        (Family::Colour, &["tint", "grayscale", "sepia", "saturation", "brightness-contrast", "invert"]),
        (Family::Image, &["glow", "sharpen", "edge-detect", "pixelate"]),
        (Family::Shader, &["scanline", "grid", "spotlight", "spotlight"]),
        (Family::Interactive, &["pointer-spotlight", "reactive-grid", "water-ripple", "burning-flame"]),
        (Family::Transitions, &["dissolve-transition", "mask-transition", "dissolve-transition"]),
    ];
    assert_eq!(Family::ALL.to_vec(), expected.iter().map(|(family, _)| *family).collect::<Vec<_>>());
    for (family, names) in expected {
        let tiles = family.tiles(&images);
        assert_eq!(names.to_vec(), tiles.iter().map(|tile| tile.name).collect::<Vec<_>>(), "{}", family.title());
        for tile in &tiles {
            // The host shows the effect of the tile: a value, or the snapshot of an effect class.
            let effect = tile.effect().unwrap_or_else(|| panic!("{}: the host has no custom effect", tile.title));
            assert_eq!(tile.name, effect.name(), "{}", tile.title);
            assert_eq!(Some(markup_of(&effect)), tile.markup.text(), "{}", tile.title);
            assert!(!tile.code.is_empty(), "{}", tile.title);
        }
    }
    let mut titles: Vec<&str> = Family::ALL.iter().map(|family| family.title()).collect();
    titles.sort_unstable();
    titles.dedup();
    assert_eq!(5, titles.len());
}

/// The effect string a tile shows is what markup writes: `Effect::parse` reads it back as the
/// effect of the host.
#[test]
fn the_effect_string_of_a_tile_parses_back_to_its_effect() {
    let _app = start();
    let images = TransitionImages::default();
    for family in [Family::Colour, Family::Image, Family::Shader] {
        for tile in family.tiles(&images) {
            let effect = tile.effect().expect("a custom effect");
            let markup = tile.markup.text().expect("the effect string");
            let text = markup.strip_prefix("Effect=\"").and_then(|text| text.strip_suffix('"')).expect("Effect=\"...\"");
            let parsed = Effect::parse(text).unwrap_or_else(|error| panic!("{}: {text}: {error}", tile.title));
            assert_eq!(Some(effect), EffectExtensions::to_custom_effect(&*parsed), "{}: {text}", tile.title);
        }
    }
}

#[test]
fn a_slider_changes_its_number_and_the_effect_string() {
    let _app = start();
    let tint = tile(Family::Colour, &TransitionImages::default(), "Tint");
    assert_close(0.55, number(&tint, "strength"), "the strength");
    tint.slider("strength").expect("the slider of the strength").set_range_value(0.25);
    assert_close(0.25, number(&tint, "strength"), "the strength after the slider");
    let markup = tint.markup.text().expect("the effect string");
    assert!(markup.contains("strength=0.25"), "{markup}");

    // A second number of the same effect.
    let spotlight = tile(Family::Shader, &TransitionImages::default(), "Spotlight");
    spotlight.slider("radius").expect("the slider of the radius").set_range_value(0.3);
    assert_close(0.3, number(&spotlight, "radius"), "the radius");
    assert_close(0.55, number(&spotlight, "center-x"), "the centre is unchanged");
}

#[test]
fn a_swatch_sets_the_colour_of_the_effect() {
    let _app = start();
    let tint = tile(Family::Colour, &TransitionImages::default(), "Tint");
    let (_, rose) = tint.swatches.iter().find(|(name, _)| *name == "Rose").expect("the swatch Rose");
    click(rose);
    let color = tint.effect().and_then(|effect| effect.parameter("color")).and_then(|value| value.as_color());
    assert_eq!(Some(Color::from_rgb(0xF0, 0x6B, 0x92)), color);
}

#[test]
fn the_toggle_turns_the_effect_off_and_on_again() {
    let _app = start();
    let sepia = tile(Family::Colour, &TransitionImages::default(), "Sepia");
    let before = sepia.effect().expect("the sepia");
    let toggle = sepia.toggle.clone().expect("the toggle");
    assert_eq!(Some(String::from("Turn off")), label_of(&toggle));

    click(&toggle);
    assert!(sepia.host.effect().is_none(), "the effect is off");
    assert_eq!(Some(String::from("Turn on")), label_of(&toggle));

    click(&toggle);
    assert_eq!(Some(before), sepia.effect());
    assert_eq!(Some(String::from("Turn off")), label_of(&toggle));
}

/// The key frames of a tile sweep its first number from the minimum of its slider to the
/// maximum and back, then the effect is the value of the tile again.
#[test]
fn the_key_frames_sweep_the_first_number_there_and_back() {
    let (_app, clock) = start();
    let tint = tile(Family::Colour, &TransitionImages::default(), "Tint");
    let run = tint.play(Some(clock.clone() as Rc<dyn IClock>)).expect("key frames");
    clock.pulse(milliseconds(0.0));
    clock.pulse(milliseconds(SWEEP_MS / 4.0));
    assert_close(0.25, number(&tint, "strength"), "a quarter of the way there");
    clock.pulse(milliseconds(SWEEP_MS * 1.5));
    assert_close(0.5, number(&tint, "strength"), "half of the way back");
    clock.pulse(milliseconds(SWEEP_MS * 2.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(run.is_completed(), "the key frames have ended");
    assert_close(0.55, number(&tint, "strength"), "the value of the tile");
}

#[test]
fn the_transitions_play_their_progress_forward() {
    let (_app, clock) = start();
    let mask = tile(Family::Transitions, &TransitionImages::default(), "Mask");
    let run = mask.play(Some(clock.clone() as Rc<dyn IClock>)).expect("key frames");
    clock.pulse(milliseconds(0.0));
    clock.pulse(milliseconds(SWEEP_MS / 2.0));
    assert_close(0.5, number(&mask, "progress"), "half way");
    clock.pulse(milliseconds(SWEEP_MS));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(run.is_completed(), "a transition plays once");
}

#[test]
fn the_pages_of_the_transitions_are_rendered_once_into_images() {
    let _app = start();
    let images = TransitionImages::render();
    let all = [&images.first_page, &images.second_page, &images.pattern];
    for image in all {
        assert!(!image.is_empty());
        assert_eq!(PixelSize::new(PAGE_WIDTH, PAGE_HEIGHT), image.pixel_size());
    }
    assert_ne!(images.first_page, images.second_page);
    assert_ne!(images.second_page, images.pattern);

    // The transitions sample them; their effect strings leave the images out.
    let dissolve_tile = tile(Family::Transitions, &images, "Dissolve");
    let value = dissolve(&dissolve_tile);
    assert_eq!((&images.first_page, &images.second_page, &images.pattern), (&value.from_page, &value.to_page, &value.pattern));
    let markup = dissolve_tile.markup.text().expect("the effect string");
    assert!(!markup.contains("page"), "{markup}");
}

/// The presenter shows a page; its action dissolves to the other one, and it rests there.
#[test]
fn the_presenter_dissolves_to_the_other_page_and_back() {
    let (_app, clock) = start();
    let images = TransitionImages::render();
    let presenter = tile(Family::Transitions, &images, "Presenter");
    let resting = dissolve(&presenter);
    assert_eq!((1.0, &images.first_page), (resting.progress, &resting.to_page), "the first page is shown");

    let run = presenter.play(Some(clock.clone() as Rc<dyn IClock>)).expect("the next page");
    clock.pulse(milliseconds(0.0));
    clock.pulse(milliseconds(PRESENTER_MS / 2.0));
    let half = dissolve(&presenter);
    assert!((half.progress - 0.5).abs() < 1e-6, "half way: {}", half.progress);
    assert_eq!((&images.first_page, &images.second_page), (&half.from_page, &half.to_page));
    clock.pulse(milliseconds(PRESENTER_MS));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(run.is_completed());
    let resting = dissolve(&presenter);
    assert_eq!((1.0, &images.second_page), (resting.progress, &resting.to_page), "the second page is shown");

    // The next turn dissolves back from the top left corner.
    drop(presenter.play(Some(clock.clone() as Rc<dyn IClock>)));
    let resting = dissolve(&presenter);
    assert_eq!((&images.first_page, 0.0, 0.0), (&resting.to_page, resting.center_x, resting.center_y));
    let markup = presenter.markup.text().expect("the effect string");
    assert!(markup.contains("center-x=0, center-y=0,"), "{markup}");
}

#[test]
fn an_interactive_tile_shows_the_snapshot_its_effect_takes() {
    let _app = start();
    let spotlight = tile(Family::Interactive, &TransitionImages::default(), "Pointer spotlight");
    let effect = spotlight.host.interactive_effect().expect("the host routes its pointer input to the effect");
    let class = effect.cast::<InteractivePointerSpotlightEffect>().expect("the pointer spotlight");
    class.update(|value| {
        value.pointer_x = 0.25;
        value.is_pointer_over = true;
    });
    let markup = spotlight.markup.text().expect("the effect string");
    assert!(markup.contains("pointer-x=0.25"), "{markup}");
    assert!(markup.contains("is-pointer-over=true"), "{markup}");
    assert_eq!(Some(markup), spotlight.effect().map(|effect| markup_of(&effect)));
}

#[test]
fn the_ways_of_setting_an_effect() {
    let (_app, clock) = start();
    let tiles = authoring_tiles();
    assert_eq!(vec!["From code", "From an effect string", "Key frames"], tiles.iter().map(|tile| tile.title).collect::<Vec<_>>());

    // The effect string is read by the converter markup uses.
    let parsed = tiles[1].effect().expect("the parsed tint");
    assert_eq!(ImmutableCustomEffect::new(TintEffect { color: Color::from_rgb(0x0F, 0x9D, 0x8E), strength: 0.55 }), parsed);
    assert_eq!(Some(format!("Effect=\"{parsed}\"")), tiles[1].markup.text());
    assert!(tiles[1].code.contains(TINT_MARKUP));

    // The key frames grow the glow and bring it back.
    let resting = number(&tiles[2], "blur-radius").expect("the blur radius");
    let run = tiles[2].play(Some(clock.clone() as Rc<dyn IClock>)).expect("key frames");
    clock.pulse(milliseconds(0.0));
    clock.pulse(milliseconds(GLOW_MS));
    let glow = tiles[2].effect().and_then(|effect| effect.downcast_ref::<GlowEffect>().cloned()).expect("a glow");
    assert!(glow.blur_radius > resting, "{} after a run, {resting} at rest", glow.blur_radius);
    clock.pulse(milliseconds(GLOW_MS * 2.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(run.is_completed());
    assert_close(resting, number(&tiles[2], "blur-radius"), "at rest again");
}

#[test]
fn the_gallery_has_the_ways_of_setting_an_effect_and_a_section_per_family() {
    let _app = start();
    let view = gallery(&TransitionImages::default());
    let scroller = view.cast::<ferroui_controls::ScrollViewer>().expect("a scroll viewer");
    let content = scroller
        .content()
        .and_then(|content| ferroui_controls::Control::from_boxed(&content))
        .and_then(|content| content.cast::<ferroui_controls::StackPanel>())
        .expect("the panel of the sections");
    // The header, the ways of setting an effect and the families.
    assert_eq!(2 + Family::ALL.len(), content.children().count());
}

/// The burn-away button is opaque, as Effector's (#580): the flame burns the content in
/// proportion to its alpha, so on a translucent button it would show on the text only.
#[test]
fn the_burn_away_button_is_opaque() {
    let _app = start();
    let burn = tile(Family::Interactive, &TransitionImages::default(), "Burn-away button");
    let button = burn.host.cast::<Button>().expect("the host is a button");
    let face = button
        .content()
        .and_then(|content| Control::from_boxed(&content))
        .and_then(|content| content.cast::<Border>())
        .expect("the face of the button");
    let color = face.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()));
    assert_eq!(Some(Color::from_rgb(0x1E, 0x23, 0x28)), color);
    assert_eq!(Some(Color::from_rgb(0x1E, 0x23, 0x28)), button.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color())));
}

/// The water ripple bends image content (#580): the picture of the images it is given, or a
/// picture the gallery draws, never a flat panel.
#[test]
fn the_water_ripple_bends_a_picture() {
    let _app = start();
    let child = |images: &TransitionImages| {
        let ripple = tile(Family::Interactive, images, "Water ripple");
        assert!(ripple.host.interactive_effect().is_some(), "the picture takes the pointer");
        ripple.host.cast::<Border>().and_then(|host| host.child()).expect("the picture")
    };
    assert!(child(&TransitionImages::default()).cast::<UniformGrid>().is_some(), "the picture the gallery draws");
    let images = TransitionImages::render();
    assert!(images.picture.image().is_some());
    assert!(child(&images).cast::<Image>().is_some(), "the picture it is given");
}

/// The caption of an interactive tile shows the first change at once and is not rewritten on
/// every frame of an animation (#580): changes within an interval wait for its end.
#[test]
fn the_caption_of_an_interactive_tile_is_throttled() {
    let _app = start();
    let ripple = tile(Family::Interactive, &TransitionImages::default(), "Water ripple");
    let effect = ripple.host.interactive_effect().expect("the interactive effect");
    let class = effect.cast::<InteractiveWaterRippleEffect>().expect("the water ripple");
    class.update(|value| value.pointer_x = 0.25);
    let first = ripple.markup.text().expect("the effect string");
    assert!(first.contains("pointer-x=0.25"), "{first}");
    // The next frames within the interval leave the caption as it is.
    class.update(|value| value.pointer_x = 0.5);
    class.update(|value| value.pointer_x = 0.75);
    assert_eq!(Some(first), ripple.markup.text());
}
