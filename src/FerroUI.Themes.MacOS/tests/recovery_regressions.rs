//! Resource lifetime regressions for PR #340. These are runtime contracts, not
//! claims of native visual equivalence. Run with the complete theme library suite.
use crate::tests::support::start_themed_application;
use crate::MacOSTheme;
use ferroui_base::animation::{TimeSpan, TransitionBase, Transitions};
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::{Application, Border, Canvas, Control, Grid, ToggleSwitch, Window};
use std::rc::Rc;

fn installed_theme() -> Ref<MacOSTheme> {
    let app = Application::current().expect("test application");
    let style = app.styles().get(0);
    style.as_object().unwrap().downcast_ref::<MacOSTheme>().unwrap().to_ref()
}

fn resource<T: Clone + 'static>(theme: &MacOSTheme, key: &str, variant: &ThemeVariant) -> T {
    let value = theme.try_get_resource(&ResourceKey::from(key), Some(variant))
        .unwrap_or_else(|| panic!("missing {key}"));
    from_markup_value::<T>(&value).unwrap_or_else(|| panic!("incorrect resource type for {key}"))
}

fn resource_color(theme: &MacOSTheme, key: &str, variant: &ThemeVariant) -> Color {
    let value = theme.try_get_resource(&ResourceKey::from(key), Some(variant)).expect("defined resource");
    if let Some(color) = from_markup_value::<Color>(&value) { return color; }
    let brush = from_markup_value::<Rc<dyn IBrush>>(&value).unwrap_or_else(|| panic!("{key}: color or brush"));
    brush.as_solid_color_brush().unwrap_or_else(|| panic!("{key}: solid brush")).color()
}

fn duration(transitions: &Transitions) -> TimeSpan {
    assert_eq!(1, transitions.count());
    let transition = transitions.get(0);
    assert_eq!(Canvas::left_property().as_property(), transition.property());
    transition.as_object().and_then(|value| value.cast::<TransitionBase>())
        .expect("a typed transition").duration()
}

#[test]
fn switch_transition_resource_is_resolved_before_a_control_exists() {
    let _app = start_themed_application();
    let theme = installed_theme();
    let first = resource::<Transitions>(&theme, "MacOSSwitchKnobTransitions", &ThemeVariant::light());
    let second = resource::<Transitions>(&theme, "MacOSSwitchKnobTransitions", &ThemeVariant::light());
    assert_eq!(TimeSpan::from_milliseconds(180.0), duration(&first));
    assert!(!first.ptr_eq(&second), "one control must not mutate another's transition collection");
    first.clear();
    assert_eq!(1, second.count());
}

#[test]
fn existing_switch_knob_transitions_follow_reduce_motion_and_restore() {
    let _app = start_themed_application();
    let theme = installed_theme();
    let switch = ToggleSwitch::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&switch)));
    window.show();
    window.update_layout();
    // ToggleSwitch installs KnobTransitions on PART_MovingKnobs in on_loaded.
    // Layout applies the template but does not dispatch the queued Loaded event.
    // Exercise the real lifecycle rather than manually assigning transitions.
    Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
    let moving = switch.get_visual_descendants()
        .find(|v| v.name().as_deref() == Some("PART_MovingKnobs"))
        .and_then(|v| v.cast::<Grid>()).expect("moving knob panel");
    for reduced in [false, true, false, true, false] {
        theme.set_reduce_motion(reduced);
        let expected = if reduced { TimeSpan::ZERO } else { TimeSpan::from_milliseconds(180.0) };
        assert_eq!(expected, duration(&switch.knob_transitions().expect("control transitions")));
        assert_eq!(expected, duration(&moving.transitions().expect("installed transitions")));
    }
    window.close();
}

#[test]
fn progress_period_stays_nonzero_under_every_motion_preference() {
    let _app = start_themed_application();
    let theme = installed_theme();
    for reduced in [false, true, false] {
        theme.set_reduce_motion(reduced);
        for variant in [ThemeVariant::light(), ThemeVariant::dark(), MacOSTheme::high_contrast()] {
            assert_eq!(TimeSpan::from_seconds(1.2),
                resource::<TimeSpan>(&theme, "MacOSProgressPulseDuration", &variant));
            assert_eq!(!reduced, resource::<bool>(&theme, "MacOSAnimationsEnabled", &variant));
        }
    }
}

#[test]
fn named_and_inherited_high_contrast_windows_use_the_system_window_color() {
    let _app = start_themed_application();
    let theme = installed_theme();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&Border::new())));
    window.show();
    for variant in [MacOSTheme::high_contrast(), ThemeVariant::new("DerivedContrast", Some(MacOSTheme::high_contrast()))] {
        window.set_requested_theme_variant(Some(variant.clone()));
        let expected = resource::<Color>(&theme, "SystemColorWindowColor", &variant);
        assert_eq!(expected, resource_color(&theme, "SolidBackgroundFillColorBaseBrush", &variant));
        assert_eq!(expected, resource::<Color>(&theme, "SolidBackgroundFillColorBase", &variant));
        assert_eq!(expected, window.background().unwrap().as_solid_color_brush().unwrap().color());
        assert_eq!(255, resource::<Color>(&theme, "TextFillColorPrimary", &variant).a);
    }
    window.close();
}

#[test]
fn inactive_selection_foreground_and_background_stay_paired_in_every_state() {
    let _app = start_themed_application();
    let theme = installed_theme();
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        for prefix in ["ListViewItem", "TreeViewItem", "TabularSurfaceRow"] {
            let suffix = if prefix == "TabularSurfaceRow" { "Brush" } else { "" };
            let keys: Vec<_> = ["", "PointerOver", "Pressed"].into_iter()
                .map(|state| (format!("{prefix}BackgroundSelected{state}{suffix}"),
                              format!("{prefix}ForegroundSelected{state}{suffix}")))
                .collect();
            let original: Vec<_> = keys.iter().map(|(bg, fg)|
                (resource_color(&theme, bg, &variant), resource_color(&theme, fg, &variant))).collect();
            theme.set_window_active(false);
            let (bg, fg) = &keys[0];
            let neutral = (resource_color(&theme, bg, &variant), resource_color(&theme, fg, &variant));
            assert_ne!(neutral.0, neutral.1);
            for (bg, fg) in &keys {
                assert_eq!(neutral, (resource_color(&theme, bg, &variant), resource_color(&theme, fg, &variant)));
            }
            theme.set_window_active(true);
            for ((bg, fg), expected) in keys.iter().zip(&original) {
                assert_eq!(*expected, (resource_color(&theme, bg, &variant), resource_color(&theme, fg, &variant)));
            }
        }
    }
}

#[test]
fn inactive_primary_button_resources_restore_without_overriding_disabled_state() {
    let _app = start_themed_application();
    let theme = installed_theme();
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        for part in ["Background", "Foreground"] {
            for state in ["", "PointerOver", "Pressed"] {
                let primary = format!("AccentButton{part}{state}");
                let neutral = format!("Button{part}{state}");
                let disabled = format!("AccentButton{part}Disabled");
                let active = resource_color(&theme, &primary, &variant);
                let disabled_before = resource_color(&theme, &disabled, &variant);
                theme.set_window_active(false);
                assert_eq!(resource_color(&theme, &neutral, &variant), resource_color(&theme, &primary, &variant));
                assert_eq!(disabled_before, resource_color(&theme, &disabled, &variant));
                theme.set_window_active(true);
                assert_eq!(active, resource_color(&theme, &primary, &variant));
            }
        }
    }
}
