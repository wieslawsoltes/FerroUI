//! The cases of the reference theme variant tests about top-levels and the
//! theme variant scope. The cases about the application alone are in
//! `application_tests.rs`, whose platform settings double is used here.

use crate::application_tests::TestPlatformSettings;
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Application, Border, Control, ThemeVariantScope, Window};
use ferroui_base::platform::PlatformThemeVariant;
use ferroui_base::styling::{IThemeVariantHost, ThemeVariant};
use ferroui_base::{FerroLocator, Ref};
use std::rc::Rc;

fn start_application(platform_settings: &Rc<TestPlatformSettings>) -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::styled_window().with_platform_settings(platform_settings.clone()))
}

fn bind_application_as_theme_variant_host() {
    FerroLocator::current_mutable()
        .bind::<dyn IThemeVariantHost>()
        .to_constant(Application::current().unwrap().as_theme_variant_host());
}

fn light() -> Option<ThemeVariant> {
    Some(ThemeVariant::light())
}

fn dark() -> Option<ThemeVariant> {
    Some(ThemeVariant::dark())
}

fn scoped_window(scope: &Ref<ThemeVariantScope>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(scope.clone())));
    window.show();
    window
}

#[test]
fn top_level_actual_theme_variant_is_initialized_from_application() {
    let _app = start_application(&TestPlatformSettings::new(PlatformThemeVariant::Light));

    bind_application_as_theme_variant_host();
    Application::current().unwrap().set_requested_theme_variant(dark());

    let window = Window::new();

    assert_eq!(dark(), window.actual_theme_variant());
}

#[test]
fn top_level_actual_theme_variant_is_initialized_from_platform_settings_without_theme_variant_host() {
    let _app = start_application(&TestPlatformSettings::new(PlatformThemeVariant::Dark));

    let window = Window::new();

    assert_eq!(dark(), window.actual_theme_variant());
}

#[test]
fn top_level_actual_theme_variant_follows_application() {
    let platform_settings = TestPlatformSettings::new(PlatformThemeVariant::Light);
    let _app = start_application(&platform_settings);
    let application = Application::current().unwrap();

    bind_application_as_theme_variant_host();

    let window = Window::new();
    assert_eq!(light(), window.actual_theme_variant());

    application.set_requested_theme_variant(dark());
    assert_eq!(dark(), window.actual_theme_variant());

    application.set_requested_theme_variant(Some(ThemeVariant::default()));
    assert_eq!(light(), window.actual_theme_variant());

    platform_settings.set_theme_variant(PlatformThemeVariant::Dark);
    assert_eq!(dark(), window.actual_theme_variant());
}

#[test]
fn top_level_requested_theme_variant_overrides_application_and_is_sent_to_platform_impl() {
    let _app = start_application(&TestPlatformSettings::new(PlatformThemeVariant::Dark));

    bind_application_as_theme_variant_host();

    let window = Window::new();
    window.set_requested_theme_variant(light());

    assert_eq!(dark(), Application::current().unwrap().actual_theme_variant());
    assert_eq!(light(), window.actual_theme_variant());
}

#[test]
fn top_level_actual_theme_variant_is_inherited_by_children() {
    let _app = start_application(&TestPlatformSettings::new(PlatformThemeVariant::Dark));

    bind_application_as_theme_variant_host();

    let child = Border::new();
    let scope = ThemeVariantScope::new();
    scope.set_child(child.clone());
    let window = scoped_window(&scope);

    assert_eq!(dark(), scope.actual_theme_variant());
    assert_eq!(dark(), child.actual_theme_variant());

    scope.set_requested_theme_variant(light());

    assert_eq!(dark(), window.actual_theme_variant());
    assert_eq!(light(), scope.actual_theme_variant());
    assert_eq!(light(), child.actual_theme_variant());
}

#[test]
fn theme_variant_scope_requesting_default_reinherits_from_parent() {
    let _app = start_application(&TestPlatformSettings::new(PlatformThemeVariant::Light));

    bind_application_as_theme_variant_host();
    Application::current().unwrap().set_requested_theme_variant(dark());

    let child = Border::new();
    let scope = ThemeVariantScope::new();
    scope.set_child(child.clone());
    let _window = scoped_window(&scope);

    scope.set_requested_theme_variant(light());
    assert_eq!(light(), scope.actual_theme_variant());
    assert_eq!(light(), child.actual_theme_variant());

    scope.set_requested_theme_variant(Some(ThemeVariant::default()));
    assert_eq!(dark(), scope.actual_theme_variant());
    assert_eq!(dark(), child.actual_theme_variant());
}
