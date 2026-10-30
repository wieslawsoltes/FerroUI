//! The class of the theme and its resources as a whole (not from upstream): the accent colours,
//! light/dark/high-contrast appearance, compact density and loading the theme through markup.

use super::support::*;
use crate::accents::SystemAccentColors;
use crate::{DensityStyle, MacOSTheme};
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::Color;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::{AccentColorShades, PlatformColorValues};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Ref, Thickness};
use ferroui_base::styling::{ControlTheme, Setter, SetterValue};
use ferroui_base::TypeInfo;
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::primitives::{OverlayPopupHost, PopupRoot};
use ferroui_controls::{
    Application, Border, Button, ContentControl, Control, StackPanel, TextBlock, TextLayoutRoundingPlacement, Window,
};

fn color_of(value: Option<Option<ferroui_base::BoxedValue>>) -> Option<Color> {
    from_markup_value::<Color>(&value?)
}

fn key(text: &str) -> ResourceKey {
    ResourceKey::from(text)
}

#[test]
fn system_accent_colors_provide_the_default_accent_without_an_owner() {
    let _app = start_application();
    let colors = SystemAccentColors::new();
    assert!(colors.has_resources());
    assert_eq!(Some(Color::from_rgb(0, 122, 255)), color_of(colors.try_get_resource(&key("SystemAccentColor"), None)));
    for shade in ["Dark1", "Dark2", "Dark3", "Light1", "Light2", "Light3"] {
        assert!(color_of(colors.try_get_resource(&key(&format!("SystemAccentColor{shade}")), None)).is_some());
    }
    assert!(colors.try_get_resource(&ResourceKey::Type(Button::TYPE), None).is_none());
}

#[test]
fn system_accent_colors_take_the_shades_the_platform_reports() {
    // An accent and shades of the kind UISettings reports (Dark1 and Light2 are the accent fills
    // of native WinUI on the default blue of Windows 11: #0067C0 in light, #4CC2FF in dark).
    let accent = Color::from_rgb(0, 120, 212);
    let shades = AccentColorShades {
        dark1: Color::from_rgb(0, 103, 192),
        dark2: Color::from_rgb(0, 62, 146),
        dark3: Color::from_rgb(0, 26, 104),
        light1: Color::from_rgb(0, 145, 248),
        light2: Color::from_rgb(76, 194, 255),
        light3: Color::from_rgb(153, 235, 255),
    };
    let values = PlatformColorValues::new().with_accent_color1(accent).with_accent_shades(Some(shades));
    assert_eq!((accent, shades), SystemAccentColors::accent_colors(Some(&values)));
}

#[test]
fn system_accent_colors_compute_the_shades_the_platform_does_not_report() {
    let accent = Color::from_rgb(0, 120, 212);
    let computed = |accent: Color| {
        let (dark1, dark2, dark3, light1, light2, light3) = SystemAccentColors::calculate_accent_shades(accent);
        AccentColorShades { dark1, dark2, dark3, light1, light2, light3 }
    };
    let values = PlatformColorValues::new().with_accent_color1(accent);
    assert_eq!((accent, computed(accent)), SystemAccentColors::accent_colors(Some(&values)));

    // Without platform settings: the default accent and its computed shades.
    let default_accent = Color::from_rgb(0, 122, 255);
    assert_eq!((default_accent, computed(default_accent)), SystemAccentColors::accent_colors(None));

    // The computed shades are ordered by lightness around the accent.
    let (_, shades) = SystemAccentColors::accent_colors(Some(&values));
    let lightness = |color: Color| color.to_hsl().l;
    assert!(lightness(shades.dark3) < lightness(shades.dark2) && lightness(shades.dark2) < lightness(shades.dark1));
    assert!(lightness(shades.dark1) < lightness(accent) && lightness(accent) < lightness(shades.light1));
    assert!(lightness(shades.light1) < lightness(shades.light2) && lightness(shades.light2) < lightness(shades.light3));
}

#[test]
fn high_contrast_is_a_variant_of_its_own_that_inherits_the_dark_one() {
    let high_contrast = MacOSTheme::high_contrast();
    assert_eq!(high_contrast, MacOSTheme::high_contrast());
    assert_ne!(high_contrast, ThemeVariant::dark());
    assert_ne!(high_contrast, ThemeVariant::default());
    assert_eq!(Some(ThemeVariant::dark()), high_contrast.inherit_variant());
}

#[test]
fn theme_loads_and_is_added_to_the_styles_of_an_application() {
    let _app = start_application();
    let application = Application::current().expect("an application");

    let theme = MacOSTheme::new();
    application.styles().add(theme.as_style());

    assert_eq!(1, application.styles().count());
    assert_eq!(1, theme.count());
    assert_eq!(DensityStyle::Normal, theme.density_style());
    // The compact resources are taken out of the resources by the constructor.
    assert!(!theme.resources().contains_key(&key("CompactStyles")));
    // The system accent colours stay a merged dictionary; the resource documents are merged into
    // the dictionary of the theme.
    assert_eq!(1, theme.resources().merged_dictionaries().count());
    let button_theme = application.try_get_resource(&ResourceKey::Type(Button::TYPE), None);
    assert!(button_theme.flatten().is_some());
    // Once attached, follow the owner's actual platform accent. The standalone
    // provider test above covers the macOS fallback in the absence of an owner.
    // TestServices installs platform settings whose accent need not be Apple blue.
    let platform_accent = application.platform_settings().expect("application platform settings")
        .get_color_values().accent_color1();
    assert_eq!(
        Some(platform_accent),
        color_of(application.try_get_resource(&key("SystemAccentColor"), None))
    );
}

#[test]
fn the_three_variants_resolve_their_resources() {
    let _app = start_themed_application();
    let application = Application::current().expect("an application");

    let resource = |name: &str, variant: ThemeVariant| -> Color {
        color_of(application.try_get_resource(&key(name), Some(&variant)))
            .unwrap_or_else(|| panic!("{name} is not a colour of {variant:?}"))
    };
    // The macOS semantic palette resolves independently for light and dark content.
    assert_eq!(Color::parse("#D9000000").unwrap(), resource("TextFillColorPrimary", ThemeVariant::light()));
    assert_eq!(Color::parse("#F2FFFFFF").unwrap(), resource("TextFillColorPrimary", ThemeVariant::dark()));
    assert_eq!(Color::parse("#FFF5F5F7").unwrap(), resource("SolidBackgroundFillColorBase", ThemeVariant::light()));
    assert_eq!(Color::parse("#FF242426").unwrap(), resource("SolidBackgroundFillColorBase", ThemeVariant::dark()));
    // The named high-contrast variant uses the dark macOS palette.
    assert_eq!(Color::parse("#FF1C1C1E").unwrap(), resource("SystemColorWindowColor", MacOSTheme::high_contrast()));
    assert_eq!(Color::parse("#FFFFFFFF").unwrap(), resource("SystemColorWindowColor", ThemeVariant::light()));

    // A brush of the high contrast dictionary takes the system colour of that variant: the
    // background of a window (SolidBackgroundFillColorBaseBrush, SystemColorWindowColor).
    let window = Window::new();
    window.set_content(Some(Control::boxed(&Border::new())));
    window.show();
    window.set_requested_theme_variant(Some(MacOSTheme::high_contrast()));
    let background = window.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()));
    assert_eq!(Some(Color::parse("#FF1C1C1E").unwrap()), background);
}

#[test]
fn compact_density_style_is_checked_first() {
    let _app = start_application();
    let application = Application::current().expect("an application");
    let theme = MacOSTheme::new();
    application.styles().add(theme.as_style());

    let padding = || {
        let value = application.try_get_resource(&key("TextControlThemePadding"), Some(&ThemeVariant::light()));
        from_markup_value::<Thickness>(&value.expect("TextControlThemePadding is defined")).expect("a thickness")
    };
    assert_eq!(Thickness::new(6.0, 3.0, 6.0, 3.0), padding());

    theme.set_density_style(DensityStyle::Compact);
    assert_eq!(DensityStyle::Compact, theme.get_direct_value(MacOSTheme::density_style_property()));
    assert_eq!(Thickness::new(2.0, 2.0, 6.0, 1.0), padding());

    theme.set_density_style(DensityStyle::Normal);
    assert_eq!(Thickness::new(6.0, 3.0, 6.0, 3.0), padding());
}

#[test]
fn theme_and_high_contrast_are_written_in_markup() {
    let _app = start_application();
    let control = load_text(
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:macos='using:FerroUI.Themes.MacOS'
                RequestedThemeVariant='{x:Static macos:MacOSTheme.HighContrast}'>
    <ContentControl.Styles>
        <macos:MacOSTheme DensityStyle='Compact' />
    </ContentControl.Styles>
</ContentControl>",
    );
    let control = from_markup_value::<Ref<ContentControl>>(&Some(control)).expect("a content control");
    let style = control.styles().get(0);
    let theme = style.as_object().and_then(|object| object.downcast_ref::<MacOSTheme>()).expect("the theme").to_ref();

    assert_eq!(DensityStyle::Compact, theme.density_style());
    assert_eq!(Some(MacOSTheme::high_contrast()), control.get_value(ThemeVariant::requested_theme_variant_property()));
    assert_eq!(
        Some(Color::parse("#FF0A84FF").unwrap()),
        color_of(style.try_get_resource(&key("SystemColorHighlightColor"), Some(&MacOSTheme::high_contrast())))
    );
}

#[test]
fn text_blocks_inherit_the_theme_layout_rounding_placement() {
    let _app = start_themed_application();
    let application = Application::current().expect("an application");

    // The window sets it, and the text blocks of its content inherit it, those of templates too
    // (the text of a button is a text block of its content presenter).
    let text = TextBlock::new();
    text.set_text(Some("Text"));
    let button = Button::new();
    button.set_content(Some(std::rc::Rc::new("Button".to_string()) as ferroui_base::BoxedValue));
    let panel = StackPanel::new();
    panel.children().add(text.clone().upcast::<Control>());
    panel.children().add(button.clone().upcast::<Control>());
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    window.show();

    assert_eq!(TextLayoutRoundingPlacement::Above, text.layout_rounding_placement());
    let button_text = button
        .get_visual_descendants()
        .into_iter()
        .find_map(|visual| visual.cast::<TextBlock>())
        .expect("the text block of the button");
    assert_eq!(TextLayoutRoundingPlacement::Above, button_text.layout_rounding_placement());
    // Outside the theme's top-levels a text block keeps the default of the reference.
    assert_eq!(TextLayoutRoundingPlacement::Below, TextBlock::new().layout_rounding_placement());

    // The control themes of the other roots of text set it as the window does.
    let sets_above = |type_: &'static TypeInfo| {
        let theme = application.try_get_resource(&ResourceKey::Type(type_), None).flatten();
        let theme = from_markup_value::<Ref<ControlTheme>>(&theme).unwrap_or_else(|| panic!("{}: a control theme", type_.name()));
        theme.setters().snapshot().iter().any(|setter| {
            let Some(setter) = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) else { return false };
            let is_placement = setter.property().is_some_and(|property| {
                std::ptr::eq(property, TextBlock::layout_rounding_placement_property().as_property())
            });
            let value = match setter.value() {
                Some(SetterValue::Value(value)) => from_markup_value::<TextLayoutRoundingPlacement>(&Some(value)),
                _ => None,
            };
            is_placement && value == Some(TextLayoutRoundingPlacement::Above)
        })
    };
    for type_ in [Window::TYPE, EmbeddableControlRoot::TYPE, PopupRoot::TYPE, OverlayPopupHost::TYPE] {
        assert!(sets_above(type_), "{}: TextBlock.LayoutRoundingPlacement is Above", type_.name());
    }
    window.close();
}

#[test]
fn a_command_bar_label_on_the_right_is_bounded_by_its_capitals_and_its_baseline() {
    let _app = start_themed_application();

    // G-29 (docs/porting/winui-theme/DEVIATIONS.md): the label is centred by its capitals.
    let button = ferroui_controls::CommandBarButton::new();
    button.set_label(Some("Screenshot"));
    button.set_label_position(ferroui_controls::CommandBarDefaultLabelPosition::Right);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&button)));
    window.show();

    let label = button
        .get_visual_descendants()
        .into_iter()
        .filter_map(|visual| visual.cast::<TextBlock>())
        .find(|text| text.name().as_deref() == Some("TextLabel"))
        .expect("the label of the button");
    assert_eq!(ferroui_base::media::TextLineBounds::Tight, label.text_line_bounds());

    // The text of the other controls keeps the box of the reference.
    assert_eq!(ferroui_base::media::TextLineBounds::Full, TextBlock::new().text_line_bounds());
    window.close();
}
