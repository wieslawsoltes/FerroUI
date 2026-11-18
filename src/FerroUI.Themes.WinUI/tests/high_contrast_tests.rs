//! System high contrast (#560; docs/porting/winui-calculator-gaps.md, sections 7.2 and 7.3; not
//! from upstream): the system colours of `Accents/SystemColors`, `UseSystemHighContrast` and the
//! high contrast variant selected from a simulated contrast setting of the platform.

use super::support::*;
use crate::accents::SystemColors;
use crate::WinUITheme;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::Color;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::{
    ColorContrastPreference, DefaultPlatformSettings, PlatformColorValues, PlatformThemeVariant,
    SystemColors as Scheme,
};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Ref, StaticType};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Application, Border, Button, Control, Window};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

fn key(text: &str) -> ResourceKey {
    ResourceKey::from(text)
}

fn color_of(value: Option<Option<ferroui_base::BoxedValue>>) -> Option<Color> {
    from_markup_value::<Color>(&value?)
}

fn high_contrast(variant: PlatformThemeVariant) -> PlatformColorValues {
    PlatformColorValues::new().with_theme_variant(variant).with_contrast_preference(ColorContrastPreference::High)
}

/// A themed application whose platform settings simulate the colour values of the platform.
fn start_with_simulated_settings() -> (UnitTestApplicationScope, Rc<DefaultPlatformSettings>, Ref<WinUITheme>) {
    crate::register_types::register_types();
    let settings = Rc::new(DefaultPlatformSettings::new());
    let mut services = TestServices::styled_window()
        .with_global_clock(Rc::new(MockGlobalClock::new()))
        .with_platform_settings(settings.clone());
    services.theme = None;
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    let theme = WinUITheme::new();
    Application::current().expect("an application").styles().add(theme.as_style());
    (scope, settings, theme)
}

fn shown_window() -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(&Border::new())));
    window.show();
    window
}

fn background_of(window: &Window) -> Option<Color> {
    window.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()))
}

#[test]
fn system_colors_are_the_standard_ones_of_windows_outside_high_contrast() {
    let _app = start_application();
    let colors = SystemColors::new();
    assert!(colors.has_resources());
    for variant in [None, Some(ThemeVariant::default()), Some(ThemeVariant::light()), Some(ThemeVariant::dark())] {
        let resource = |name: &str| color_of(colors.try_get_resource(&key(name), variant.as_ref()));
        assert_eq!(Some(Color::from_rgb(0xFF, 0xFF, 0xFF)), resource("SystemColorWindowColor"));
        assert_eq!(Some(Color::from_rgb(0x00, 0x00, 0x00)), resource("SystemColorWindowTextColor"));
        assert_eq!(Some(Color::from_rgb(0xF0, 0xF0, 0xF0)), resource("SystemColorButtonFaceColor"));
        assert_eq!(Some(Color::from_rgb(0x00, 0x00, 0x00)), resource("SystemColorButtonTextColor"));
        assert_eq!(Some(Color::from_rgb(0x00, 0x78, 0xD7)), resource("SystemColorHighlightColor"));
        assert_eq!(Some(Color::from_rgb(0xFF, 0xFF, 0xFF)), resource("SystemColorHighlightTextColor"));
        assert_eq!(Some(Color::from_rgb(0x00, 0x66, 0xCC)), resource("SystemColorHotlightColor"));
        assert_eq!(Some(Color::from_rgb(0x6D, 0x6D, 0x6D)), resource("SystemColorGrayTextColor"));
    }
    // Without an owner, the high contrast variant has the colours of "High Contrast Black".
    let high = WinUITheme::high_contrast();
    assert_eq!(Some(Color::from_rgb(0x1A, 0xEB, 0xFF)), color_of(colors.try_get_resource(&key("SystemColorHighlightColor"), Some(&high))));
    // Nothing else.
    assert!(colors.try_get_resource(&key("SystemColorBackgroundColor"), None).is_none());
    assert!(colors.try_get_resource(&key("SystemAccentColor"), None).is_none());
    assert!(colors.try_get_resource(&ResourceKey::Type(Button::TYPE), None).is_none());
}

#[test]
fn high_contrast_colors_are_the_platform_ones_or_a_scheme_of_its_variant() {
    assert_eq!(Scheme::high_contrast_black(), SystemColors::high_contrast_colors_of(None));
    assert_eq!(
        Scheme::high_contrast_white(),
        SystemColors::high_contrast_colors_of(Some(&high_contrast(PlatformThemeVariant::Light)))
    );
    assert_eq!(
        Scheme::high_contrast_black(),
        SystemColors::high_contrast_colors_of(Some(&high_contrast(PlatformThemeVariant::Dark)))
    );
    let aquatic = Scheme::new(Color::from_rgb(0x20, 0x20, 0x20), Color::from_rgb(0xFF, 0xFF, 0xFF))
        .with_highlight(Color::from_rgb(0x8E, 0xE3, 0xF0))
        .with_hotlight(Color::from_rgb(0x75, 0xE9, 0xFC));
    let values = high_contrast(PlatformThemeVariant::Light).with_system_colors(Some(aquatic));
    assert_eq!(aquatic, SystemColors::high_contrast_colors_of(Some(&values)));
}

#[test]
fn the_theme_sets_its_high_contrast_variant_while_it_is_in_use() {
    let _app = start_application();
    let application = Application::current().expect("an application");
    assert_eq!(None, ThemeVariant::high_contrast_variant());

    let theme = WinUITheme::new();
    assert!(theme.use_system_high_contrast());
    assert_eq!(None, ThemeVariant::high_contrast_variant());

    let style = theme.as_style();
    application.styles().add(style.clone());
    assert_eq!(Some(WinUITheme::high_contrast()), ThemeVariant::high_contrast_variant());

    theme.set_use_system_high_contrast(false);
    assert_eq!(None, ThemeVariant::high_contrast_variant());
    theme.set_use_system_high_contrast(true);
    assert_eq!(Some(WinUITheme::high_contrast()), ThemeVariant::high_contrast_variant());

    application.styles().remove(style.clone());
    assert_eq!(None, ThemeVariant::high_contrast_variant());
}

#[test]
fn the_application_and_its_windows_follow_the_simulated_contrast_setting() {
    let (_app, settings, _theme) = start_with_simulated_settings();
    let application = Application::current().expect("an application");
    let window = shown_window();
    assert_eq!(Some(ThemeVariant::light()), application.actual_theme_variant());
    assert_eq!(Some(Color::parse("#F3F3F3").unwrap()), background_of(&window));

    // High contrast on a dark platform without system colours: "High Contrast Black".
    settings.set_color_values(high_contrast(PlatformThemeVariant::Dark));
    assert_eq!(Some(WinUITheme::high_contrast()), application.actual_theme_variant());
    assert_eq!(Some(WinUITheme::high_contrast()), window.actual_theme_variant());
    assert_eq!(Some(Color::from_rgb(0, 0, 0)), background_of(&window));

    // Decision D10: a requested light or dark variant does not win over system high contrast.
    application.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert_eq!(Some(WinUITheme::high_contrast()), application.actual_theme_variant());

    // The system colours of the platform, updated live.
    let dusk = Scheme::new(Color::from_rgb(0x2D, 0x32, 0x36), Color::from_rgb(0xFF, 0xFF, 0xFF));
    settings.set_color_values(high_contrast(PlatformThemeVariant::Dark).with_system_colors(Some(dusk)));
    assert_eq!(Some(WinUITheme::high_contrast()), window.actual_theme_variant());
    assert_eq!(Some(Color::from_rgb(0x2D, 0x32, 0x36)), background_of(&window));

    // High contrast off: the requested variant again.
    settings.set_color_values(PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Dark));
    assert_eq!(Some(ThemeVariant::light()), application.actual_theme_variant());
    assert_eq!(Some(Color::parse("#F3F3F3").unwrap()), background_of(&window));
}

#[test]
fn without_use_system_high_contrast_the_contrast_setting_selects_nothing() {
    let (_app, settings, theme) = start_with_simulated_settings();
    let application = Application::current().expect("an application");
    theme.set_use_system_high_contrast(false);

    settings.set_color_values(high_contrast(PlatformThemeVariant::Dark));
    assert_eq!(Some(ThemeVariant::dark()), application.actual_theme_variant());

    // Turned on while high contrast is on: the variant applies at once.
    theme.set_use_system_high_contrast(true);
    assert_eq!(Some(WinUITheme::high_contrast()), application.actual_theme_variant());
}
