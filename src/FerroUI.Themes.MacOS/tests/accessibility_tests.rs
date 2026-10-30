//! Live accessibility and appearance behavior. These tests verify updates on existing
//! resource consumers, not just the presence of resource names in the XAML documents.

use super::support::*;
use crate::{ControlSize, MacOSTheme};
use ferroui_base::animation::{IterationCount, PageSlide, SlideAxis, TimeSpan, TransitionBase};
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{platform::PlatformHotkeyConfiguration, PointerType};
use ferroui_base::media::{AcrylicBackgroundSource, Color, ExperimentalAcrylicMaterial};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::platform::{ColorContrastPreference, DefaultPlatformSettings, IPlatformSettings, PlatformColorValues};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{CornerRadius, Ref, Size};
use ferroui_controls::{Application, Border, Button, Control, ExperimentalAcrylicBorder, NavigationPage, ProgressBar, Window};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use std::rc::Rc;
use std::cell::Cell;
use std::time::Duration;

struct MutablePlatformSettings {
    base: DefaultPlatformSettings,
    animations: Cell<bool>,
    transparency: Cell<bool>,
    colors: Cell<PlatformColorValues>,
    accessibility_subscribers: Rc<Cell<usize>>,
}

impl MutablePlatformSettings {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            base: DefaultPlatformSettings::new(),
            animations: Cell::new(true),
            transparency: Cell::new(true),
            colors: Cell::new(PlatformColorValues::new().with_accent_color1(Color::from_rgb(0, 122, 255))),
            accessibility_subscribers: Rc::new(Cell::new(0)),
        })
    }
}

impl IPlatformSettings for MutablePlatformSettings {
    fn get_tap_size(&self, pointer: PointerType) -> Size { self.base.get_tap_size(pointer) }
    fn get_double_tap_size(&self, pointer: PointerType) -> Size { self.base.get_double_tap_size(pointer) }
    fn get_double_tap_time(&self, pointer: PointerType) -> Duration { self.base.get_double_tap_time(pointer) }
    fn hold_wait_duration(&self) -> Duration { self.base.hold_wait_duration() }
    fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> { self.base.hotkey_configuration() }
    fn preferred_application_language(&self) -> String { self.base.preferred_application_language() }
    fn animations_enabled(&self) -> bool { self.animations.get() }
    fn transparency_enabled(&self) -> bool { self.transparency.get() }
    fn get_color_values(&self) -> PlatformColorValues { self.colors.get() }
    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }
    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
    fn accessibility_settings_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let inner = self.base.accessibility_settings_changed(handler);
        let subscribers = self.accessibility_subscribers.clone();
        subscribers.set(subscribers.get() + 1);
        Disposable::create(move || {
            inner.dispose();
            subscribers.set(subscribers.get() - 1);
        })
    }
}

fn resource<T: Clone + 'static>(theme: &MacOSTheme, name: &str, variant: &ThemeVariant) -> T {
    let value = theme.try_get_resource(&ResourceKey::from(name), Some(variant))
        .unwrap_or_else(|| panic!("missing {name}"));
    from_markup_value::<T>(&value).unwrap_or_else(|| panic!("{name} has an unexpected resource type"))
}

fn installed_theme() -> Ref<MacOSTheme> {
    let theme = MacOSTheme::new();
    Application::current().expect("a test application").styles().add(theme.as_style());
    theme
}

#[test]
fn reduce_motion_updates_existing_transition_durations_and_can_be_restored() {
    let _app = start_application();
    let theme = installed_theme();
    let value = load_text(
        "<Border xmlns='https://github.com/ferroui'>
            <Border.Transitions>
                <Transitions>
                    <DoubleTransition Property='Opacity' Duration='{DynamicResource MacOSMotionStandardDuration}' />
                </Transitions>
            </Border.Transitions>
        </Border>",
    );
    let border = from_markup_value::<Ref<Border>>(&Some(value)).expect("a border");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&border)));
    window.show();
    let transition = border.transitions().expect("the transition collection").get(0)
        .as_object().and_then(|object| object.cast::<TransitionBase>()).expect("the transition object");
    let original = transition.duration();
    assert!(original > TimeSpan::ZERO);
    assert!(resource::<IterationCount>(&theme, "MacOSProgressPulseIterations", &ThemeVariant::light()).is_infinite());
    theme.set_reduce_motion(true);
    assert_eq!(TimeSpan::ZERO, transition.duration(), "the already-attached transition is updated");
    for key in ["MacOSMotionFastDuration", "MacOSMotionStandardDuration", "MacOSMotionSlowDuration"] {
        assert_eq!(TimeSpan::ZERO, resource::<TimeSpan>(&theme, key, &ThemeVariant::light()));
    }
    assert_eq!(IterationCount::new(0), resource::<IterationCount>(&theme, "MacOSProgressPulseIterations", &ThemeVariant::light()),
        "indeterminate progress must stop repeating under reduced motion");
    theme.set_reduce_motion(false);
    assert_eq!(original, transition.duration());
    window.close();
}

#[test]
fn reduce_motion_replaces_navigation_transitions_on_an_existing_page() {
    let _app = start_application();
    let theme = installed_theme();
    let navigation = NavigationPage::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&navigation)));
    window.show();
    let original = navigation.page_transition().expect("the themed navigation transition");
    let normal_duration = original.as_page_slide().expect("a page slide").duration();
    assert!(normal_duration > TimeSpan::ZERO);

    for reduced in [true, false] {
        theme.set_reduce_motion(reduced);
        let expected_duration = if reduced { TimeSpan::ZERO } else { normal_duration };
        for (key, transition, axis) in [
            ("MacOSNavigationTransition", navigation.page_transition(), SlideAxis::Horizontal),
            ("MacOSModalTransition", navigation.modal_transition(), SlideAxis::Vertical),
        ] {
            let transition = transition.expect("the transition remains assigned");
            let slide = transition.as_page_slide().expect("a page slide");
            assert_eq!(expected_duration, slide.duration(), "{key} updates on the already-attached page");
            assert_eq!(axis, slide.orientation(), "the transition preserves its navigation direction");
            let resource = resource::<Rc<PageSlide>>(&theme, key, &ThemeVariant::light());
            assert_eq!(expected_duration, resource.duration());
            assert_eq!(axis, resource.orientation());
        }
        assert!(!Rc::ptr_eq(&original, &navigation.page_transition().unwrap()),
            "a new transition is assigned because PageSlide.Duration is not a bindable property");
    }
    window.close();
}

#[test]
fn reduce_transparency_replaces_the_material_on_an_existing_glass_surface() {
    let _app = start_application();
    let theme = installed_theme();
    let value = load_text(
        "<ExperimentalAcrylicBorder xmlns='https://github.com/ferroui'
             Material='{DynamicResource MacOSRegularGlassMaterial}' />",
    );
    let border = from_markup_value::<Ref<ExperimentalAcrylicBorder>>(&Some(value)).expect("a glass border");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&border)));
    window.show();
    assert_eq!(AcrylicBackgroundSource::Digger, border.material().unwrap().background_source());
    theme.set_reduce_transparency(true);
    assert_eq!(AcrylicBackgroundSource::None, border.material().unwrap().background_source());
    theme.set_reduce_transparency(false);
    assert_eq!(AcrylicBackgroundSource::Digger, border.material().unwrap().background_source());
    window.close();
}

#[test]
fn inactive_windows_suppress_backdrop_vibrancy_for_all_glass_roles() {
    let _app = start_application();
    let theme = installed_theme();
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        for key in ["MacOSRegularGlassMaterial", "MacOSSidebarGlassMaterial", "MacOSMenuGlassMaterial",
                    "MacOSToolbarGlassMaterial", "MacOSClearGlassMaterial"] {
            theme.set_window_active(false);
            let material = resource::<Ref<ExperimentalAcrylicMaterial>>(&theme, key, &variant);
            assert_eq!(AcrylicBackgroundSource::None, material.background_source(), "{key}");
            theme.set_window_active(true);
            let material = resource::<Ref<ExperimentalAcrylicMaterial>>(&theme, key, &variant);
            assert_eq!(AcrylicBackgroundSource::Digger, material.background_source(), "{key}");
        }
    }
}

#[test]
fn an_accent_override_updates_existing_consumers_and_restores_the_platform_accent() {
    let _app = start_application();
    let theme = installed_theme();
    let value = load_text("<Border xmlns='https://github.com/ferroui' Background='{DynamicResource MacOSAccentBrush}' />");
    let border = from_markup_value::<Ref<Border>>(&Some(value)).expect("a border");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&border)));
    window.show();
    let color = || border.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()));
    let initial = color().expect("the platform accent brush");
    let purple = Color::from_rgb(175, 82, 222);
    theme.set_accent_color(Some(purple));
    assert_eq!(Some(purple), color());
    theme.set_accent_color(None);
    assert_eq!(Some(initial), color());
    window.close();
}

#[test]
fn increased_contrast_strengthens_existing_glass_borders_and_composes_with_transparency() {
    let _app = start_application();
    let theme = installed_theme();
    let value = load_text("<Border xmlns='https://github.com/ferroui' BorderBrush='{DynamicResource MacOSGlassBorderBrush}'
        Background='{DynamicResource TextFillColorPrimaryBrush}' />");
    let border = from_markup_value::<Ref<Border>>(&Some(value)).expect("a border");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&border)));
    window.show();
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        window.set_requested_theme_variant(Some(variant.clone()));
        let border_color = || border.border_brush().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color())).unwrap();
        let label_color = || border.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color())).unwrap();
        let regular = border_color();
        let regular_label = label_color();
        assert!(regular.a < 255);
        theme.set_increase_contrast(true);
        assert_eq!(255, border_color().a, "the attached border must become opaque");
        assert_eq!(255, label_color().a, "semantic brushes must also consume the live contrast color");
        assert_eq!(255, resource::<Color>(&theme, "TextFillColorPrimary", &variant).a);
        assert_eq!(255, resource::<Color>(&theme, "ControlStrokeColorDefault", &variant).a);
        assert_eq!(AcrylicBackgroundSource::None,
            resource::<Ref<ExperimentalAcrylicMaterial>>(&theme, "MacOSRegularGlassMaterial", &variant).background_source());

        // Removing one accessibility preference must preserve the independently selected one.
        theme.set_reduce_transparency(true);
        theme.set_increase_contrast(false);
        assert_eq!(regular, border_color());
        assert_eq!(regular_label, label_color());
        assert_eq!(AcrylicBackgroundSource::None,
            resource::<Ref<ExperimentalAcrylicMaterial>>(&theme, "MacOSRegularGlassMaterial", &variant).background_source());
        theme.set_reduce_transparency(false);
        assert_eq!(AcrylicBackgroundSource::Digger,
            resource::<Ref<ExperimentalAcrylicMaterial>>(&theme, "MacOSRegularGlassMaterial", &variant).background_source());
    }
    window.close();
}

#[test]
fn control_sizes_update_existing_buttons_and_large_sizes_use_capsules() {
    let _app = start_application();
    let theme = installed_theme();
    let button = Button::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&button)));
    window.show();
    let mut heights = Vec::new();
    for size in [ControlSize::Mini, ControlSize::Small, ControlSize::Regular, ControlSize::Large, ControlSize::ExtraLarge] {
        theme.set_control_size(size);
        window.update_layout();
        let height = resource::<f64>(&theme, "MacOSControlHeight", &ThemeVariant::light());
        heights.push(height);
        assert_eq!(height, button.min_height(), "the existing button follows {size:?}");
        if matches!(size, ControlSize::Large | ControlSize::ExtraLarge) {
            let radius = resource::<CornerRadius>(&theme, "MacOSControlCornerRadius", &ThemeVariant::light());
            assert!(radius.top_left >= height / 2.0, "large controls use capsules");
        }
    }
    assert!(heights.windows(2).all(|pair| pair[0] < pair[1]), "control sizes are strictly ordered");
    theme.set_control_size(ControlSize::Regular);
    assert_eq!(heights[2], button.min_height());
    window.close();
}

#[test]
fn accessibility_and_appearance_properties_can_be_loaded_from_xaml() {
    let _app = start_application();
    let loaded = load_text(
        "<MacOSTheme xmlns='https://github.com/ferroui'
             ReduceMotion='True' ReduceTransparency='True' IncreaseContrast='True'
             WindowActive='False' ControlSize='Large' AccentColor='#AF52DE' />",
    );
    let theme = from_markup_value::<Ref<MacOSTheme>>(&Some(loaded)).expect("the theme");
    assert!(theme.reduce_motion());
    assert!(theme.reduce_transparency());
    assert!(theme.increase_contrast());
    assert!(!theme.window_active());
    assert_eq!(ControlSize::Large, theme.control_size());
    assert_eq!(Some(Color::from_rgb(175, 82, 222)), theme.accent_color());
}

#[test]
fn indeterminate_progress_stops_and_resumes_when_reduced_motion_changes() {
    crate::register_types();
    let clock = Rc::new(MockGlobalClock::new());
    let _app = UnitTestApplication::start(TestServices::styled_window()
        .with_global_clock(clock.clone()).with_theme(|| MacOSTheme::new().as_style()));
    let app = Application::current().unwrap();
    let theme_style = app.styles().get(0);
    let theme = theme_style.as_object().unwrap().downcast_ref::<MacOSTheme>().unwrap().to_ref();
    let progress = ProgressBar::new();
    progress.set_is_indeterminate(true);
    progress.set_width(200.0);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&progress)));
    window.show();
    window.update_layout();
    let indicator = progress.get_visual_descendants()
        .find(|visual| visual.name().as_deref() == Some("MacOSIndeterminateFill"))
        .and_then(|visual| visual.cast::<Border>()).expect("the indeterminate indicator");
    let static_indicator = progress.get_visual_descendants()
        .find(|visual| visual.name().as_deref() == Some("MacOSIndeterminateStaticFill"))
        .and_then(|visual| visual.cast::<Border>()).expect("the reduced-motion indicator");
    clock.pulse(0.0);
    clock.pulse(0.3);
    let moving_a = indicator.opacity();
    clock.pulse(0.6);
    let moving_b = indicator.opacity();
    assert!((moving_a - moving_b).abs() > 0.01, "indeterminate progress visibly animates");
    assert!(clock.has_observers(), "the active progress animation subscribes to frames");

    theme.set_reduce_motion(true);
    clock.pulse(0.9);
    let stopped = indicator.opacity();
    clock.pulse(1.5);
    assert_eq!(stopped, indicator.opacity(), "reduced motion freezes the existing animation");
    assert!(!indicator.is_effectively_visible(), "the animated surface is hidden");
    assert!(static_indicator.is_effectively_visible() && static_indicator.opacity() > 0.0,
        "a static progress indication remains visible with reduced motion");
    assert!(!clock.has_observers(), "reduced motion releases the repeating frame subscription");

    theme.set_reduce_motion(false);
    assert!(indicator.is_effectively_visible());
    assert!(!static_indicator.is_effectively_visible());
    clock.pulse(1.8);
    clock.pulse(2.1);
    let resumed_a = indicator.opacity();
    clock.pulse(2.4);
    assert!((resumed_a - indicator.opacity()).abs() > 0.01, "motion resumes without changing progress state");
    window.close();
}

#[test]
fn platform_accessibility_events_update_existing_resources_and_unsubscribe_with_the_theme() {
    crate::register_types();
    let settings = MutablePlatformSettings::new();
    let mut services = TestServices::styled_window().with_platform_settings(settings.clone());
    services.theme = None;
    let _app = UnitTestApplication::start(services);
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
    let theme = MacOSTheme::new();
    let style = theme.as_style();
    let app = Application::current().unwrap();
    app.styles().add(style.clone());
    assert_eq!(1, settings.accessibility_subscribers.get());
    let value = load_text("<ExperimentalAcrylicBorder xmlns='https://github.com/ferroui'
        Material='{DynamicResource MacOSRegularGlassMaterial}' />");
    let glass = from_markup_value::<Ref<ExperimentalAcrylicBorder>>(&Some(value)).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&glass)));
    window.show();
    assert_eq!(AcrylicBackgroundSource::Digger, glass.material().unwrap().background_source());

    // Motion/transparency notifications are independent of the system accent/color event.
    settings.animations.set(false);
    settings.transparency.set(false);
    settings.base.on_accessibility_settings_changed();
    assert!(theme.effective_reduce_motion() && theme.effective_reduce_transparency());
    assert!(!theme.reduce_motion() && !theme.reduce_transparency(), "system policy does not overwrite app preferences");
    assert_eq!(TimeSpan::ZERO, resource::<TimeSpan>(&theme, "MacOSMotionStandardDuration", &ThemeVariant::light()));
    assert_eq!(AcrylicBackgroundSource::None, glass.material().unwrap().background_source());

    settings.colors.set(settings.colors.get().with_contrast_preference(ColorContrastPreference::High));
    settings.base.on_color_values_changed(settings.colors.get());
    assert!(theme.effective_increase_contrast());
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        assert_eq!(255, resource::<Color>(&theme, "TextFillColorPrimary", &variant).a);
    }

    settings.animations.set(true);
    settings.transparency.set(true);
    settings.colors.set(settings.colors.get().with_contrast_preference(ColorContrastPreference::NoPreference));
    settings.base.on_accessibility_settings_changed();
    settings.base.on_color_values_changed(settings.colors.get());
    assert!(!theme.effective_reduce_motion() && !theme.effective_reduce_transparency() && !theme.effective_increase_contrast());
    assert_eq!(AcrylicBackgroundSource::Digger, glass.material().unwrap().background_source());

    // An explicit application accessibility preference cannot be canceled by a system change.
    theme.set_reduce_motion(true);
    settings.base.on_accessibility_settings_changed();
    assert!(theme.effective_reduce_motion());
    app.styles().remove(style.clone());
    assert_eq!(0, settings.accessibility_subscribers.get(), "detaching disposes the platform subscription");
    app.styles().add(style);
    assert_eq!(1, settings.accessibility_subscribers.get(), "reattachment subscribes exactly once");
    window.close();
}

#[test]
fn platform_accessibility_preferences_apply_when_the_theme_is_first_attached() {
    crate::register_types();
    let settings = MutablePlatformSettings::new();
    settings.animations.set(false);
    settings.transparency.set(false);
    settings.colors.set(settings.colors.get().with_contrast_preference(ColorContrastPreference::High));
    let mut services = TestServices::styled_window().with_platform_settings(settings.clone());
    services.theme = None;
    let _app = UnitTestApplication::start(services);
    let theme = installed_theme();
    assert!(theme.effective_reduce_motion());
    assert!(theme.effective_reduce_transparency());
    assert!(theme.effective_increase_contrast());
    for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
        assert_eq!(TimeSpan::ZERO, resource::<TimeSpan>(&theme, "MacOSMotionFastDuration", &variant));
        let material = resource::<Ref<ExperimentalAcrylicMaterial>>(&theme, "MacOSRegularGlassMaterial", &variant);
        assert_eq!(AcrylicBackgroundSource::None, material.background_source());
        assert_eq!(255, resource::<Color>(&theme, "ControlStrokeColorDefault", &variant).a);
    }
}
