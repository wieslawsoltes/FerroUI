use crate::{MaterialColorRole, MaterialColorScheme, MaterialMotionRole, MaterialMotionScheme, MaterialStatusColor, MaterialTheme};
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::TimeSpan;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{ControlTheme, ThemeVariant};
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, CheckBox, ComboBox, GroupBox, ListBoxItem, ProgressBar, RadioButton, Slider, TabControl, TextBox, ToggleSwitch};
use std::rc::Rc;

#[test]
fn pinned_color_roles_match_androidx_and_are_complete() {
    assert_eq!(MaterialColorRole::ALL.len(), 48);
    assert_eq!(MaterialColorScheme::light().color(MaterialColorRole::Primary), Color::from_uint32(0xff6750a4));
    assert_eq!(MaterialColorScheme::dark().color(MaterialColorRole::Surface), Color::from_uint32(0xff141218));
    for scheme in [MaterialColorScheme::light(), MaterialColorScheme::dark()] {
        for role in MaterialColorRole::ALL {
            assert_eq!(scheme.color(role).a, 255, "{} must be an opaque role", role.name());
        }
    }
}

#[test]
fn compiled_theme_resolves_every_role_in_both_variants() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let theme = MaterialTheme::new();
    for (variant, expected) in [(ThemeVariant::light(), MaterialColorScheme::expressive_light()), (ThemeVariant::dark(), MaterialColorScheme::dark())] {
        for role in MaterialColorRole::ALL {
            let key = ResourceKey::from(format!("Material{}Color", role.name()));
            let color = theme.try_get_resource(&key, Some(&variant)).flatten();
            assert_eq!(from_markup_value::<Color>(&color), Some(expected.color(role)), "{}", role.name());
            let brush = theme.try_get_resource(&ResourceKey::from(format!("Material{}Brush", role.name())), Some(&variant)).flatten();
            assert!(from_markup_value::<Rc<dyn IBrush>>(&brush).is_some(), "{} brush", role.name());
        }
    }
}

#[test]
fn replacing_palette_updates_role_and_compatibility_brushes_together() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let theme = MaterialTheme::new();
    let custom = MaterialColorScheme::light().with_color(MaterialColorRole::Primary, Color::from_rgb(14, 78, 44));
    theme.set_color_scheme(ThemeVariant::light(), &custom);
    let color = theme.try_get_resource(&ResourceKey::from("MaterialPrimaryColor"), Some(&ThemeVariant::light())).flatten();
    assert_eq!(from_markup_value::<Color>(&color), Some(Color::from_rgb(14, 78, 44)));
    let dark = theme.try_get_resource(&ResourceKey::from("MaterialPrimaryColor"), Some(&ThemeVariant::dark())).flatten();
    assert_eq!(from_markup_value::<Color>(&dark), Some(MaterialColorScheme::dark().color(MaterialColorRole::Primary)));
    let legacy = theme.try_get_resource(&ResourceKey::from("AccentButtonBackground"), Some(&ThemeVariant::light())).flatten();
    let legacy = from_markup_value::<Rc<dyn IBrush>>(&legacy).expect("compatibility brush");
    assert_eq!(legacy.as_solid_color_brush().unwrap().color(), Color::from_rgb(14, 78, 44));
}

#[test]
fn existing_button_updates_when_its_complete_scheme_is_replaced() {
    use ferroui_controls::{Application, Control, Window};
    crate::register_types();
    let mut services = TestServices::styled_window()
        .with_global_clock(Rc::new(super::support::MockGlobalClock::new()));
    services.theme = None;
    let _app = UnitTestApplication::start(services);
    let theme = MaterialTheme::new();
    theme.set_reduced_motion(true);
    Application::current().unwrap().styles().add(theme.as_style());
    let button = Button::new();
    button.set_content(Some(Rc::new("Save".to_string())));
    let window = Window::new();
    window.set_requested_theme_variant(Some(ThemeVariant::light()));
    window.set_content(Some(Control::boxed(&button)));
    window.show();
    window.update_layout();
    let before = button.background().unwrap().as_solid_color_brush().unwrap().color();
    assert_eq!(before, MaterialColorScheme::light().color(MaterialColorRole::Primary));
    let primary = Color::from_rgb(14, 78, 44);
    let foreground = Color::from_rgb(250, 255, 250);
    theme.set_color_scheme(ThemeVariant::light(), &MaterialColorScheme::light()
        .with_color(MaterialColorRole::Primary, primary)
        .with_color(MaterialColorRole::OnPrimary, foreground));
    assert_eq!(button.background().unwrap().as_solid_color_brush().unwrap().color(), primary);
    assert_eq!(button.foreground().unwrap().as_solid_color_brush().unwrap().color(), foreground);
}

#[test]
fn reduced_motion_replaces_the_resources_consumed_by_transitions() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let theme = MaterialTheme::new();
    theme.set_reduced_motion(true);
    for role in MaterialMotionRole::ALL {
        let duration = theme.try_get_resource(&ResourceKey::from(format!("MaterialMotion{}Duration", role.resource_name())), None).flatten();
        assert_eq!(from_markup_value::<TimeSpan>(&duration), Some(TimeSpan::ZERO));
        let easing = theme.try_get_resource(&ResourceKey::from(format!("MaterialMotion{}Easing", role.resource_name())), None).flatten();
        let easing = from_markup_value::<Easing>(&easing).expect("actual transition easing");
        assert_eq!(easing.ease(0.25), 1.0);
    }
    theme.set_reduced_motion(false);
    theme.set_motion_scheme(MaterialMotionScheme::Standard);
    let duration = theme.try_get_resource(&ResourceKey::from("MaterialMotionFastSpatialDuration"), None).flatten();
    assert_eq!(from_markup_value::<TimeSpan>(&duration), Some(MaterialMotionScheme::Standard.duration(MaterialMotionRole::FastSpatial, false)));
}

#[test]
fn default_controls_and_material_variants_resolve_compiled_templates() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let theme = MaterialTheme::new();
    for type_ in [Button::TYPE, CheckBox::TYPE, RadioButton::TYPE, ToggleSwitch::TYPE, Slider::TYPE, TextBox::TYPE, ComboBox::TYPE, ListBoxItem::TYPE, TabControl::TYPE, GroupBox::TYPE, ProgressBar::TYPE] {
        let value = theme.try_get_resource(&ResourceKey::Type(type_), None).flatten();
        assert!(from_markup_value::<Ref<ControlTheme>>(&value).is_some(), "{}", type_.name());
    }
    for key in ["MaterialFilledButton", "MaterialTonalButton", "MaterialOutlinedButton", "MaterialTextButton", "MaterialElevatedButton", "MaterialFilledCard", "MaterialElevatedCard", "MaterialOutlinedCard", "MaterialDialog", "MaterialAssistChip", "MaterialFilterChip", "MaterialSearchBar", "MaterialNavigationBar", "MaterialNavigationRail"] {
        let value = theme.try_get_resource(&ResourceKey::from(key), None).flatten();
        assert!(from_markup_value::<Ref<ControlTheme>>(&value).is_some(), "{key}");
    }
}

/// The severities of InfoBar and InfoBadge have fills of their own in both variants: Error the error
/// roles, Success and Warning the status colors the theme adds (Material 3 has no success or
/// warning roles), Informational the primary role; and the glyph of each severity is drawn in a
/// color other than that of the circle behind it.
#[test]
fn the_severities_have_distinct_fills_and_visible_glyphs() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let theme = MaterialTheme::new();
    let brush = |key: &str, variant: &ThemeVariant| {
        let value = theme.try_get_resource(&ResourceKey::from(key), Some(variant)).flatten();
        from_markup_value::<Rc<dyn IBrush>>(&value)
            .and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
            .unwrap_or_else(|| panic!("the brush {key}"))
    };
    for (variant, scheme) in [(ThemeVariant::light(), MaterialColorScheme::expressive_light()), (ThemeVariant::dark(), MaterialColorScheme::dark())] {
        let severities = ["Informational", "Success", "Warning", "Error"];
        let backgrounds: Vec<Color> = severities.iter().map(|s| brush(&format!("InfoBar{s}SeverityBackgroundBrush"), &variant)).collect();
        let circles: Vec<Color> = severities.iter().map(|s| brush(&format!("InfoBar{s}SeverityIconBackground"), &variant)).collect();
        for i in 0..severities.len() {
            for j in i + 1..severities.len() {
                assert_ne!(backgrounds[i], backgrounds[j], "{} and {} in {variant:?}", severities[i], severities[j]);
                assert_ne!(circles[i], circles[j], "{} and {} in {variant:?}", severities[i], severities[j]);
            }
            let glyph = brush(&format!("InfoBar{}SeverityIconForeground", severities[i]), &variant);
            assert_ne!(glyph, circles[i], "the glyph of {} in {variant:?}", severities[i]);
        }
        assert_eq!(backgrounds[3], scheme.color(MaterialColorRole::ErrorContainer));
        assert_eq!(circles[1], scheme.status_color(MaterialStatusColor::Success));
        assert_eq!(circles[2], scheme.status_color(MaterialStatusColor::Warning));
        let badges: Vec<Color> = ["Attention", "Success", "Caution", "Critical"]
            .iter()
            .map(|s| brush(&format!("SystemFillColor{s}Brush"), &variant))
            .chain([brush("SystemFillColorSolidNeutralBrush", &variant)])
            .collect();
        for i in 0..badges.len() {
            for j in i + 1..badges.len() {
                assert_ne!(badges[i], badges[j], "the InfoBadge fills {i} and {j} in {variant:?}");
            }
        }
    }
    assert_ne!(
        MaterialColorScheme::light().status_color(MaterialStatusColor::SuccessContainer),
        MaterialColorScheme::dark().status_color(MaterialStatusColor::SuccessContainer)
    );
}
