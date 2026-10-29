//! Regression contracts for theme resources and instantiated controls.
//! These checks do not establish native AppKit optical or animation fidelity.
use super::{shadow_margin, DECORATIVE_SHADOW_KEYS};
use crate::tests::support::start_themed_application;
use crate::MacOSTheme;
use ferroui_base::animation::{TimeSpan, TransitionBase, Transitions};
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::BoxShadows;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Ref, Thickness, Visual};
use ferroui_controls::{Application, Border, Control, ToolTip, ToggleSwitch, Window};

fn installed_theme() -> Ref<MacOSTheme> {
    let style = Application::current().expect("test application").styles().get(0);
    style.as_object().unwrap().downcast_ref::<MacOSTheme>().unwrap().to_ref()
}

fn resource<T: Clone + 'static>(theme: &MacOSTheme, key: &str, variant: &ThemeVariant) -> T {
    let value = theme.try_get_resource(&ResourceKey::from(key), Some(variant))
        .unwrap_or_else(|| panic!("missing {key}"));
    from_markup_value::<T>(&value).unwrap_or_else(|| panic!("incorrect type: {key}"))
}

fn opacity_duration(transitions: &Transitions) -> TimeSpan {
    assert_eq!(1, transitions.count());
    let transition = transitions.get(0);
    assert_eq!(Visual::opacity_property().as_property(), transition.property());
    transition.as_object().and_then(|value| value.cast::<TransitionBase>())
        .expect("typed transition").duration()
}

fn named_border(control: &Control, name: &str) -> Ref<Border> {
    control.get_visual_descendants()
        .filter(|v| v.name().as_deref() == Some(name))
        .find_map(|v| v.cast::<Border>()).unwrap_or_else(|| panic!("missing {name}"))
}

#[test]
fn tooltip_transition_resource_is_eager_and_independently_owned() {
    let _app = start_themed_application();
    let theme = installed_theme();
    let first = resource::<Transitions>(&theme, "MacOSToolTipTransitions", &ThemeVariant::light());
    let second = resource::<Transitions>(&theme, "MacOSToolTipTransitions", &ThemeVariant::light());
    assert_eq!(TimeSpan::from_milliseconds(100.0), opacity_duration(&first));
    assert!(!first.ptr_eq(&second));
    first.clear();
    assert_eq!(TimeSpan::from_milliseconds(100.0), opacity_duration(&second));
}

#[test]
fn existing_tooltip_transitions_follow_reduce_motion_and_restore() {
    let _app = start_themed_application();
    let theme = installed_theme();
    let tip = ToolTip::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&tip)));
    window.show();
    window.update_layout();
    for reduced in [false, true, false] {
        theme.set_reduce_motion(reduced);
        let expected = if reduced { TimeSpan::ZERO } else { TimeSpan::from_milliseconds(100.0) };
        assert_eq!(expected, opacity_duration(&tip.transitions().expect("installed transition")));
    }
    window.close();
}

#[test]
fn shadow_insets_follow_renderer_bounds_including_spread_and_offsets() {
    for (text, expected) in [
        ("none", Thickness::uniform(0.0)),
        ("0 8 24 0 #33000000, 0 1 3 0 #26000000", Thickness::new(24.0, 16.0, 24.0, 32.0)),
        ("0 12 36 0 #40000000, 0 1 4 0 #33000000", Thickness::new(36.0, 24.0, 36.0, 48.0)),
        ("0 2 6 0 #26000000", Thickness::new(6.0, 4.0, 6.0, 8.0)),
        ("-3 2 5 2 #33000000", Thickness::new(10.0, 5.0, 4.0, 9.0)),
        ("1.25 -2.5 3.25 0.5 #33000000", Thickness::new(3.0, 7.0, 5.0, 2.0)),
        ("inset 10 20 40 5 #33000000", Thickness::uniform(0.0)),
    ] {
        assert_eq!(expected, shadow_margin(&BoxShadows::parse(text).unwrap()), "{text}");
    }
}

#[test]
fn named_and_inherited_contrast_remove_each_declared_decorative_shadow() {
    let _app = start_themed_application();
    let theme = installed_theme();
    for variant in [MacOSTheme::high_contrast(), ThemeVariant::new("ReviewContrast", Some(MacOSTheme::high_contrast()))] {
        for key in DECORATIVE_SHADOW_KEYS {
            assert_eq!(0, resource::<BoxShadows>(&theme, key, &variant).count(), "{key}");
        }
        for key in ["MacOSMenuShadowMargin", "MacOSWindowShadowMargin", "MacOSToolTipShadowMargin"] {
            assert_eq!(Thickness::uniform(0.0), resource::<Thickness>(&theme, key, &variant));
        }
    }
}

#[test]
fn increased_contrast_updates_and_restores_shadows_and_their_insets() {
    let _app = start_themed_application();
    let theme = installed_theme();
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        let original: Vec<BoxShadows> = DECORATIVE_SHADOW_KEYS.iter()
            .map(|key| resource::<BoxShadows>(&theme, key, &variant)).collect();
        assert!(original.iter().all(|shadow| shadow.count() > 0));
        let margin = resource::<Thickness>(&theme, "MacOSMenuShadowMargin", &variant);
        assert_eq!(Thickness::new(24.0, 16.0, 24.0, 32.0), margin);
        theme.set_increase_contrast(true);
        for key in DECORATIVE_SHADOW_KEYS {
            assert_eq!(0, resource::<BoxShadows>(&theme, key, &variant).count(), "{key}");
        }
        assert_eq!(Thickness::uniform(0.0), resource::<Thickness>(&theme, "MacOSMenuShadowMargin", &variant));
        theme.set_increase_contrast(false);
        for (key, expected) in DECORATIVE_SHADOW_KEYS.iter().zip(original) {
            assert_eq!(expected, resource::<BoxShadows>(&theme, key, &variant), "{key}");
        }
        assert_eq!(margin, resource::<Thickness>(&theme, "MacOSMenuShadowMargin", &variant));
    }
}

#[test]
fn tooltip_open_state_does_not_override_the_macos_shadow_recipe() {
    let _app = start_themed_application();
    let theme = installed_theme();
    let tip = ToolTip::new();
    let window = Window::new();
    window.set_requested_theme_variant(Some(ThemeVariant::light()));
    window.set_content(Some(Control::boxed(&tip)));
    window.show();
    window.update_layout();
    let wrapper = named_border(&tip, "ThemeShadowWrapper");
    let expected = resource::<BoxShadows>(&theme, "MacOSToolTipShadow", &ThemeVariant::light());
    assert_eq!(expected, wrapper.box_shadow());
    tip.pseudo_classes().set(":open", true);
    window.update_layout();
    assert_eq!(expected, wrapper.box_shadow());
    assert_eq!(Thickness::new(6.0, 4.0, 6.0, 8.0), tip.margin());
    theme.set_increase_contrast(true);
    assert_eq!(0, wrapper.box_shadow().count());
    assert_eq!(Thickness::uniform(0.0), tip.margin());
    theme.set_increase_contrast(false);
    assert_eq!(expected, wrapper.box_shadow());
    tip.pseudo_classes().set(":open", false);
    window.close();
}

#[test]
fn instantiated_switch_knob_shadow_follows_contrast_without_retemplating() {
    let _app = start_themed_application();
    let theme = installed_theme();
    let switch = ToggleSwitch::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&switch)));
    window.show();
    window.update_layout();
    let knob = named_border(&switch, "SwitchKnob");
    let original = knob.box_shadow();
    assert!(original.count() > 0);
    theme.set_increase_contrast(true);
    assert_eq!(0, knob.box_shadow().count());
    theme.set_increase_contrast(false);
    assert_eq!(original, knob.box_shadow());
    window.close();
}
