//! The ThemeShadow and Acrylic examples: their native controls and the source handlers. No
//! rendering parity with WinUI is claimed.

use crate::materials_samples::{
    FALLBACK_COLORS, IN_APP_BRUSH_KEY, LUMINOSITY_BRUSH_KEY, TINT_COLORS,
};
use crate::{examples, tests::start};
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{AcrylicBrush, Color};
use ferroui_base::Ref;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Border, ComboBox, Control, Grid, Slider, StackPanel, TextBlock};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

fn brush(root: &Ref<Control>, key: &str) -> Ref<AcrylicBrush> {
    root.try_find_resource(&ResourceKey::from(key), None)
        .flatten()
        .and_then(|value| value.downcast_ref::<Ref<AcrylicBrush>>().cloned())
        .unwrap_or_else(|| panic!("the page has the resource {key}"))
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} is not {expected}");
}

#[test]
fn theme_shadow_border_casts_its_shadow_on_the_receiver_at_the_slider_elevation() {
    let _application = start();
    let root = page("ThemeShadow");
    assert!(root.find_control::<StackPanel>("ThemeShadowExample1").is_some());
    let rect = root.get_control::<Border>("ShadowRect");
    let shadow = rect.shadow().expect("the Border has a Shadow");
    let theme_shadow = shadow.as_theme_shadow().expect("the Shadow is a ThemeShadow");
    // ShadowRect_Loaded: the grid behind the Border is a receiver.
    let receiver = root.get_control::<Grid>("ShadowCastGrid");
    assert!(theme_shadow.receivers().contains(&receiver));
    assert_eq!(theme_shadow.receivers().size(), 1);
    // Value="32" raises ValueChanged while the page loads.
    assert_eq!(rect.translation().z, 32.0);
    let slider = root.get_control::<Slider>("TranslationSliderInApp");
    for z in [0.0, 12.0, 64.0] {
        slider.set_range_value(z);
        assert_eq!(rect.translation().z, z);
        assert_eq!((rect.translation().x, rect.translation().y), (0.0, 0.0));
        assert_eq!(
            root.get_control::<TextBlock>("ShadowTranslationOutput").text().as_deref(),
            Some(format!("Translation.Z: {z}").as_str())
        );
    }
    // The slider's range bounds the elevation, as Minimum and Maximum do in the source.
    slider.set_range_value(100.0);
    assert_eq!(rect.translation().z, 64.0);
}

#[test]
fn acrylic_page_keeps_three_examples_and_the_loaded_state_of_its_brushes() {
    let _application = start();
    let root = page("Acrylic");
    for index in 1..=3 {
        assert!(root.find_control::<StackPanel>(&format!("AcrylicExample{index}")).is_some());
    }
    let in_app = brush(&root, IN_APP_BRUSH_KEY);
    let lumin = brush(&root, LUMINOSITY_BRUSH_KEY);
    // AcrylicPage_Loaded: the first colours, tint opacity and luminosity 0.8.
    assert_eq!(in_app.tint_color(), Color::from_rgb(0, 0, 0));
    assert_eq!(in_app.fallback_color(), Color::from_rgb(0x00, 0x80, 0x00));
    assert_close(in_app.tint_opacity(), 0.8);
    assert_eq!(lumin.tint_color(), Color::from_rgb(0x87, 0xCE, 0xEB));
    assert_eq!(lumin.fallback_color(), Color::from_rgb(0x87, 0xCE, 0xEB));
    assert_close(lumin.tint_opacity(), 0.8);
    assert_close(lumin.tint_luminosity_opacity().expect("a luminosity opacity"), 0.8);
    assert_eq!(root.get_control::<ComboBox>("ColorSelectorInApp").selected_index(), 0);
    assert_eq!(root.get_control::<ComboBox>("FallbackColorSelectorInApp").selected_index(), 0);
    for slider in ["OpacitySliderInApp", "OpacitySliderLumin", "LuminositySlider"] {
        assert_close(root.get_control::<Slider>(slider).value(), 0.8);
    }
    // The rectangles are filled with the page's brushes; the first with the theme's default.
    for name in ["CustomAcrylicShapeInApp", "CustomAcrylicShapeLumin"] {
        assert!(root.get_control::<Rectangle>(name).fill().is_some(), "{name} has a fill");
    }
    assert!(root.find_control::<Rectangle>("DefaultAcrylicShape").is_some());
}

#[test]
fn acrylic_options_change_tint_fallback_and_luminosity_of_their_own_brush() {
    let _application = start();
    let root = page("Acrylic");
    let in_app = brush(&root, IN_APP_BRUSH_KEY);
    let lumin = brush(&root, LUMINOSITY_BRUSH_KEY);

    // Slider_ValueChanged: each opacity slider changes the brush of its own example.
    root.get_control::<Slider>("OpacitySliderInApp").set_range_value(0.25);
    assert_close(in_app.tint_opacity(), 0.25);
    assert_close(lumin.tint_opacity(), 0.8);
    root.get_control::<Slider>("OpacitySliderLumin").set_range_value(0.5);
    assert_close(lumin.tint_opacity(), 0.5);
    assert_close(in_app.tint_opacity(), 0.25);

    // ColorSelector_SelectionChanged.
    let tint = root.get_control::<ComboBox>("ColorSelectorInApp");
    for (index, color) in TINT_COLORS.iter().enumerate().rev() {
        tint.set_selected_index(index as i32);
        assert_eq!(in_app.tint_color(), *color);
    }
    assert_eq!(TINT_COLORS[1], Color::from_rgb(0xFF, 0, 0));
    assert_eq!(TINT_COLORS[2], Color::from_rgb(0, 0, 0xFF));

    // FallbackColorSelector_SelectionChanged.
    let fallback = root.get_control::<ComboBox>("FallbackColorSelectorInApp");
    fallback.set_selected_index(1);
    assert_eq!(in_app.fallback_color(), FALLBACK_COLORS[1]);
    assert_eq!(in_app.fallback_color(), Color::from_rgb(0xFF, 0xFF, 0x00));
    fallback.set_selected_index(0);
    assert_eq!(in_app.fallback_color(), Color::from_rgb(0x00, 0x80, 0x00));
    // The luminosity brush keeps its own colours.
    assert_eq!(lumin.fallback_color(), Color::from_rgb(0x87, 0xCE, 0xEB));

    // LuminositySlider_ValueChanged.
    root.get_control::<Slider>("LuminositySlider").set_range_value(0.3);
    assert_close(lumin.tint_luminosity_opacity().expect("a luminosity opacity"), 0.3);
    assert_eq!(in_app.tint_luminosity_opacity(), None);
}
