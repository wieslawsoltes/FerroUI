//! The classes of the theme (not from upstream, which has no tests of
//! them): the accent shades, the colour palettes, the density style, and
//! the theme as a whole.

use super::support::*;
use crate::accents::SystemAccentColors;
use crate::{ColorPaletteResources, ColorPaletteResourcesCollection, DensityStyle, FluentTheme};
use ferroui_base::controls::{IResourceProvider, ResourceKey};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{Application, Button, ContentControl, Control, Window};
use std::rc::Rc;

fn color_of(value: Option<Option<ferroui_base::BoxedValue>>) -> Option<Color> {
    from_markup_value::<Color>(&value?)
}

fn key(text: &str) -> ResourceKey {
    ResourceKey::from(text)
}

#[test]
fn accent_shades_step_the_lightness_of_the_accent() {
    let accent = Color::from_rgb(0, 120, 215);
    let (dark1, dark2, dark3, light1, light2, light3) = SystemAccentColors::calculate_accent_shades(accent);
    let lightness = accent.to_hsl().l;
    let tolerance = 1.0 / 255.0;
    for (shade, step) in [(dark1, -28.5), (dark2, -49.0), (dark3, -74.5), (light1, 39.0), (light2, 70.0), (light3, 103.0)]
    {
        let expected = (lightness + step / 255.0).clamp(0.0, 1.0);
        assert!((shade.to_hsl().l - expected).abs() <= tolerance, "{shade:?} for the step {step}");
        assert_eq!(accent.a, shade.a);
    }
}

#[test]
fn system_accent_colors_provide_the_default_accent_without_an_owner() {
    let _app = start_application();
    let colors = SystemAccentColors::new();
    assert!(colors.has_resources());
    assert_eq!(Some(Color::from_rgb(0, 120, 215)), color_of(colors.try_get_resource(&key("SystemAccentColor"), None)));
    for shade in ["Dark1", "Dark2", "Dark3", "Light1", "Light2", "Light3"] {
        assert!(color_of(colors.try_get_resource(&key(&format!("SystemAccentColor{shade}")), None)).is_some());
    }
    assert!(colors.try_get_resource(&key("SystemAltHighColor"), None).is_none());
    assert!(colors.try_get_resource(&ResourceKey::Type(Button::TYPE), None).is_none());
}

#[test]
fn color_palette_resources_hold_the_colors_that_are_set() {
    let _app = start_application();
    let palette = ColorPaletteResources::new();
    assert!(!palette.has_resources());
    assert_eq!(Color::default(), palette.alt_high());
    assert!(palette.try_get_resource(&key("SystemAltHighColor"), None).is_none());

    let red = Color::from_rgb(255, 0, 0);
    palette.set_alt_high(red);
    palette.set_region_color(Color::from_rgb(1, 2, 3));
    assert!(palette.has_resources());
    assert_eq!(red, palette.alt_high());
    assert_eq!(Some(red), color_of(palette.try_get_resource(&key("SystemAltHighColor"), None)));
    assert_eq!(Some(Color::from_rgb(1, 2, 3)), color_of(palette.try_get_resource(&key("SystemRegionColor"), None)));

    // The default colour removes the entry.
    palette.set_alt_high(Color::default());
    palette.set_region_color(Color::default());
    assert!(!palette.has_resources());
    assert!(palette.try_get_resource(&key("SystemAltHighColor"), None).is_none());
}

#[test]
fn color_palette_resources_compute_the_shades_of_their_accent() {
    let _app = start_application();
    let palette = ColorPaletteResources::new();
    assert!(palette.try_get_resource(&key("SystemAccentColor"), None).is_none());

    let accent = Color::from_rgb(0, 128, 0);
    palette.set_accent(accent);
    assert!(palette.has_resources());
    assert_eq!(accent, palette.get_direct_value(ColorPaletteResources::accent_property()));
    assert_eq!(Some(accent), color_of(palette.try_get_resource(&key("SystemAccentColor"), None)));
    let (dark1, _, _, _, _, light3) = SystemAccentColors::calculate_accent_shades(accent);
    assert_eq!(Some(dark1), color_of(palette.try_get_resource(&key("SystemAccentColorDark1"), None)));
    assert_eq!(Some(light3), color_of(palette.try_get_resource(&key("SystemAccentColorLight3"), None)));

    palette.set_accent(Color::default());
    assert!(!palette.has_resources());
    assert!(palette.try_get_resource(&key("SystemAccentColor"), None).is_none());
}

#[test]
fn palette_collection_answers_for_the_palette_of_the_variant() {
    let _app = start_application();
    let collection = ColorPaletteResourcesCollection::new();
    assert!(!collection.has_resources());

    let light = ColorPaletteResources::new();
    light.set_alt_high(Color::from_rgb(1, 1, 1));
    let dark = ColorPaletteResources::new();
    dark.set_alt_high(Color::from_rgb(2, 2, 2));
    collection.add(ThemeVariant::light(), light);
    collection.add(ThemeVariant::dark(), dark);
    assert!(collection.has_resources());
    assert_eq!(2, collection.count());

    let alt_high = |theme: Option<&ThemeVariant>| color_of(collection.try_get_resource(&key("SystemAltHighColor"), theme));
    assert_eq!(Some(Color::from_rgb(1, 1, 1)), alt_high(Some(&ThemeVariant::light())));
    assert_eq!(Some(Color::from_rgb(2, 2, 2)), alt_high(Some(&ThemeVariant::dark())));
    // No variant and the default variant are the light one.
    assert_eq!(Some(Color::from_rgb(1, 1, 1)), alt_high(None));
    assert_eq!(Some(Color::from_rgb(1, 1, 1)), alt_high(Some(&ThemeVariant::default())));

    assert!(collection.remove(&ThemeVariant::dark()));
    assert_eq!(None, alt_high(Some(&ThemeVariant::dark())));
}

#[test]
fn palette_collection_only_supports_light_and_dark() {
    let _app = start_application();
    let collection = ColorPaletteResourcesCollection::new();
    let error = collection.try_add(ThemeVariant::default(), ColorPaletteResources::new()).unwrap_err();
    assert_eq!("FluentTheme.Palettes only supports Light and Dark variants.", error);
    assert_eq!(0, collection.count());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        collection.add(ThemeVariant::default(), ColorPaletteResources::new())
    }));
    assert!(result.is_err());
}

#[test]
fn palette_collection_gives_its_owner_to_the_palettes() {
    let _app = start_application();
    let host = ContentControl::new();
    let collection = ColorPaletteResourcesCollection::new();
    let before = ColorPaletteResources::new();
    collection.add(ThemeVariant::light(), before.clone());

    let provider: Rc<dyn IResourceProvider> = collection.as_resource_provider();
    host.resources().merged_dictionaries().add(provider);
    assert!(collection.owner().is_some());
    assert!(before.owner().is_some());

    let after = ColorPaletteResources::new();
    collection.add(ThemeVariant::dark(), after.clone());
    assert!(after.owner().is_some());

    collection.remove(&ThemeVariant::dark());
    assert!(after.owner().is_none());
}

// --- the theme as a whole -----------------------------------------------------

fn solid_color(brush: Option<Rc<dyn IBrush>>) -> Color {
    brush.expect("a brush").as_solid_color_brush().expect("a solid colour brush").color()
}

#[test]
fn theme_loads_and_is_added_to_the_styles_of_an_application() {
    let _app = start_application();
    let application = Application::current().expect("an application");

    let theme = FluentTheme::new();
    application.styles().add(theme.as_style());

    assert_eq!(1, application.styles().count());
    assert_eq!(1, theme.count());
    assert_eq!(DensityStyle::Normal, theme.density_style());
    // The compact styles are taken out of the resources by the constructor.
    assert!(!theme.resources().contains_key(&key("CompactStyles")));
    // The palette, the system accent colours and the palette collection stay merged dictionaries;
    // the other resources are merged into the dictionary of the theme.
    assert_eq!(3, theme.resources().merged_dictionaries().count());
    assert_eq!(0, theme.palettes().count());
    assert!(theme.resources().contains_key(&key("StringTextFlyoutCopyText")));
    let list_box_theme = application.try_get_resource(&ResourceKey::Type(ferroui_controls::ListBox::TYPE), None);
    assert!(list_box_theme.flatten().is_some());
    assert_eq!(
        Some(Color::from_rgb(0, 120, 215)),
        color_of(application.try_get_resource(&key("SystemAccentColor"), None))
    );
}

#[test]
fn light_and_dark_variants_resolve_their_resources() {
    let _app = start_themed_application();
    let application = Application::current().expect("an application");

    let resource = |name: &str, variant: ThemeVariant| -> Color {
        color_of(application.try_get_resource(&key(name), Some(&variant)))
            .unwrap_or_else(|| panic!("{name} is not a colour of {variant:?}"))
    };
    assert_eq!(Color::parse("#FFFFFFFF").unwrap(), resource("SystemAltHighColor", ThemeVariant::light()));
    assert_eq!(Color::parse("#FF000000").unwrap(), resource("SystemAltHighColor", ThemeVariant::dark()));
    assert_eq!(Color::parse("#FF000000").unwrap(), resource("SystemBaseHighColor", ThemeVariant::light()));
    assert_eq!(Color::parse("#FFFFFFFF").unwrap(), resource("SystemBaseHighColor", ThemeVariant::dark()));

    // A themed control follows the variant of its window.
    let content = ContentControl::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&content)));
    window.show();

    let light = solid_color(window.background());
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    let dark = solid_color(window.background());
    assert_ne!(light, dark);
    window.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert_eq!(light, solid_color(window.background()));
}

#[test]
fn compact_density_style_is_checked_first() {
    let _app = start_application();
    let application = Application::current().expect("an application");
    let theme = FluentTheme::new();
    application.styles().add(theme.as_style());

    let padding = || {
        let value = application.try_get_resource(&key("ButtonPadding"), None);
        from_markup_value::<Thickness>(&value.expect("ButtonPadding is defined")).expect("a thickness")
    };
    let normal = padding();
    assert_ne!(Thickness::new(6.0, 4.0, 6.0, 4.0), normal);

    theme.set_density_style(DensityStyle::Compact);
    assert_eq!(DensityStyle::Compact, theme.get_direct_value(FluentTheme::density_style_property()));
    assert_eq!(Thickness::new(6.0, 4.0, 6.0, 4.0), padding());

    theme.set_density_style(DensityStyle::Normal);
    assert_eq!(normal, padding());
}

#[test]
fn palettes_and_density_style_are_set_from_markup() {
    let _app = start_application();
    let control = load_text(
        "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Styles>
        <FluentTheme DensityStyle='Compact'>
            <FluentTheme.Palettes>
                <ColorPaletteResources x:Key='Light' Accent='Green' RegionColor='White' />
                <ColorPaletteResources x:Key='Dark' Accent='DarkGreen' RegionColor='Black' />
            </FluentTheme.Palettes>
        </FluentTheme>
    </ContentControl.Styles>
</ContentControl>",
    );
    let control = from_markup_value::<Ref<ContentControl>>(&Some(control)).expect("a content control");
    let style = control.styles().get(0);
    let theme = style.as_object().and_then(|object| object.downcast_ref::<FluentTheme>()).expect("the theme").to_ref();

    assert_eq!(DensityStyle::Compact, theme.density_style());
    assert_eq!(2, theme.palettes().count());
    let green = Color::parse("Green").unwrap();
    assert_eq!(green, theme.palettes().get(&ThemeVariant::light()).accent());
    // The palettes of the theme come before its own palette and the system accent.
    assert_eq!(Some(green), color_of(style.try_get_resource(&key("SystemAccentColor"), Some(&ThemeVariant::light()))));
    assert_eq!(
        Some(Color::parse("Black").unwrap()),
        color_of(style.try_get_resource(&key("SystemRegionColor"), Some(&ThemeVariant::dark())))
    );
}
