//! Intentionally failing platform-capability reproducer outside the normal suite.
mod platform_settings;
use ferroribbon::theming::RibbonThemeResources;
use ferroui_base::{
    FerroLocator,
    controls::ResourceDictionary,
    media::Color,
    platform::{ColorContrastPreference, IPlatformSettings, PlatformColorValues},
    threading::CancellationToken,
};
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "FerroRibbon.SystemColors.Reproducer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn active_system_window_color_precedes_application_fallback_like_source_ui_settings() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY)).dispatch(|| {
        ferroribbon::register_types();let scope=FerroLocator::enter_scope();let settings=platform_settings::Settings::new();
        settings.set_colors(PlatformColorValues::new().with_contrast_preference(ColorContrastPreference::High));
        FerroLocator::current_mutable().bind::<dyn IPlatformSettings>().to_constant(settings);
        let app=Application::current().unwrap();app.resources().set("SystemColorWindowColor",Some(Rc::new(Color::from_argb(255,1,3,5))));
        let r=RibbonThemeResources::with_shared_resources(ResourceDictionary::new());
        let actual=r.get_brush("RibbonWindowBackgroundBrush",Some("HighContrast")).unwrap().color();
        app.resources().remove(&"SystemColorWindowColor".into());scope.dispose();
        // Whole original source corpus systemPrimary reads Window=(20,30,40)
        // from UISettings while application resources contain (1,3,5).
        // IPlatformSettings exposes contrast/accent values, no system Window/
        // WindowText/Highlight/ButtonFace/etc values to supply that environment.
        assert_eq!(actual,Color::from_argb(255,20,30,40),"Source UISettings primary system colors cannot currently be represented by the selected native platform contract; do not map accent colors to unrelated UIElementColor roles.");
    },CancellationToken::none()).wait().expect("actual source system-color parity assertion");
}
