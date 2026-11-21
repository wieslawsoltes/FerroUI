//! Independent application startup, using the framework's real platform settings.
use ferroui_base::{
    FerroObjectImpl, Rect, Ref, Size, StaticType, TypeInfo, data::core::ValueTypes, ferro_class,
    ferro_impl_classes, instantiate, metadata::MarkupAssembly, threading::Dispatcher,
    utilities::CultureInfo,
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, Control, NewApplication, Window,
    automation::peers::ControlAutomationPeer,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use ferroui_system::threading::CancellationToken;

#[repr(C)]
struct StartupApp {
    base: Application,
}
ferro_class!(StartupApp:Application);
ferro_impl_classes!(StartupApp:FerroObjectImpl);
impl NewApplication for StartupApp {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for StartupApp {
    fn initialize(_: &Self) {
        // Same composition order as the native sample, before any ribbon type.
        let culture = ferroribbon::localization::initialize_ui_culture_from_platform().unwrap();
        assert_eq!(culture.name(), "fr-CA");
        ferroribbon::register_types();
    }
}
fn build() -> AppBuilder {
    AppBuilder::configure::<StartupApp>().use_headless(FerroHeadlessPlatformOptions::default())
}

#[test]
fn platform_ui_language_initializes_compiled_and_runtime_downstream_ribbons() {
    const CASE: &str = "startup_culture_tests::platform_ui_language_initializes_compiled_and_runtime_downstream_ribbons";
    if std::env::var("FERRORIBBON_STARTUP_CONSUMER").as_deref() != Ok(CASE) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CASE, "--nocapture"])
            .env("FERRORIBBON_STARTUP_CONSUMER", CASE)
            .env("LC_ALL", "fr_CA.UTF-8")
            .output()
            .unwrap();
        assert!(
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("running 1 test\n"),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                use crate::ribbon_consumer::ConsumerRibbon;
                assert_eq!(
                    ferroui_base::utilities::current_ui_locale().to_string(),
                    "fr-CA"
                );
                // The UI-language bridge must not substitute a numeric region.
                assert_eq!(CultureInfo::current_formatting().name(), "");
                assert_eq!(
                    ferroribbon_core::localization::RibbonStrings::current()
                        .file()
                        .unwrap()
                        .to_utf8()
                        .unwrap(),
                    "Fichier"
                );
                TypeInfo::register_namespaces(&[(
                    "ferroribbon_markup_consumer::ribbon_consumer",
                    "FerroRibbon.Consumer",
                )]);
                TypeInfo::register_all(&[<ConsumerRibbon as StaticType>::TYPE]);
                MarkupAssembly::register(&crate::CONSUMER_ASSEMBLY);
                let _theme = crate::tests::NativeThemeScope::new();
                for runtime in [false, true] {
                    let value = if runtime {
                        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                            include_str!("LocalizedRibbon.xaml"),
                            Some(&crate::CONSUMER_ASSEMBLY),
                        )
                        .unwrap()
                    } else {
                        crate::compiled_xaml::try_load(
                            None,
                            "ferres://ferroribbon_markup_consumer/LocalizedRibbon.xaml",
                        )
                        .unwrap()
                        .unwrap()
                    };
                    let root = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<ConsumerRibbon>()
                        .unwrap();
                    root.set_is_application_button_visible(true);
                    let window = Window::new();
                    window.set_content(Some(Control::boxed(root.clone())));
                    window.show();
                    window.measure(Size::new(1000., 300.));
                    window.arrange(Rect::new(0., 0., 1000., 300.));
                    Dispatcher::ui_thread().run_jobs(None);
                    assert_eq!(
                        ControlAutomationPeer::create_peer_for_element(
                            &root.application_button().unwrap()
                        )
                        .get_name(),
                        "Fichier"
                    );
                    window.close();
                    window.set_content(None);
                    Dispatcher::ui_thread().run_jobs(None);
                }
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
