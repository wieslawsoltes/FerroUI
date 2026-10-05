//! Tests of this port: the reference has no test class for the builder (it
//! is exercised by the desktop lifetime tests, which wait for `Window`).

use crate::application_lifetimes::{IApplicationLifetime, ISetupApplicationLifetime};
use crate::{AppBuilder, Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use ferroui_base::input::IInputManager;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::platform::IAssetLoader;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, LocatorExtensions, Ref,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn log(entry: &'static str) {
    LOG.with(|log| log.borrow_mut().push(entry));
}

fn take_log() -> Vec<&'static str> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

#[repr(C)]
struct TestApp {
    base: Application,
}

ferro_class!(TestApp: Application);
ferro_impl_classes!(TestApp: FerroObjectImpl);

impl ApplicationImpl for TestApp {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        log("initialize");
    }

    fn register_services(this: &Self) {
        log("register_services");
        Self::parent_register_services(this);
    }

    fn on_framework_initialization_completed(this: &Self) {
        Self::parent_on_framework_initialization_completed(this);
        log("on_framework_initialization_completed");
    }
}

impl NewApplication for TestApp {
    fn new_application() -> Ref<Self> {
        log("new");
        instantiate(Self { base: Application::construct() })
    }
}

struct TestLifetime;

impl IApplicationLifetime for TestLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_setup_application_lifetime(&self) -> Option<&dyn ISetupApplicationLifetime> {
        Some(self)
    }
}

impl ISetupApplicationLifetime for TestLifetime {
    fn before_app_init(&self) {
        log("before_app_init");
    }

    fn after_app_init(&self) {
        log("after_app_init");
    }
}

/// Isolates the dispatcher, the service locator and the setup flag.
struct Scope {
    locator: Rc<dyn IDisposable>,
    _dispatcher: UnitTestDispatcherScope,
}

fn scope() -> Scope {
    let dispatcher = Dispatcher::unit_test_scope();
    AppBuilder::reset_setup_for_unit_tests();
    take_log();
    Scope { locator: FerroLocator::enter_scope(), _dispatcher: dispatcher }
}

impl Drop for Scope {
    fn drop(&mut self) {
        self.locator.dispose();
        AppBuilder::reset_setup_for_unit_tests();
    }
}

fn configured() -> AppBuilder {
    AppBuilder::configure::<TestApp>()
        .use_standard_runtime_platform_subsystem()
        .use_windowing_subsystem(|| log("windowing"), "TestWindowing")
        .use_rendering_subsystem(|| log("rendering"), "TestRendering")
        .use_text_shaping_subsystem(|| log("text shaping"), "TestTextShaping")
}

#[test]
fn configure_records_the_application_type_and_the_subsystems() {
    let _scope = scope();
    let builder = configured();

    assert!(std::ptr::eq(builder.application_type().unwrap(), TestApp::TYPE));
    assert!(builder.instance().is_none());
    assert_eq!(builder.runtime_platform_services_name().as_deref(), Some("StandardRuntimePlatform"));
    assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("TestWindowing"));
    assert_eq!(builder.rendering_subsystem_name().as_deref(), Some("TestRendering"));
    assert_eq!(builder.text_shaping_subsystem_name().as_deref(), Some("TestTextShaping"));
    assert!(builder.runtime_platform_services_initializer().is_some());
    assert!(builder.windowing_subsystem_initializer().is_some());
    assert!(builder.rendering_subsystem_initializer().is_some());
    assert!(builder.text_shaping_subsystem_initializer().is_some());
    assert!(take_log().is_empty());
}

#[test]
fn setup_with_lifetime_runs_the_steps_in_order_and_returns() {
    let _scope = scope();
    let lifetime: Rc<dyn IApplicationLifetime> = Rc::new(TestLifetime);

    let builder = configured()
        .use_runtime_platform_subsystem(|| log("runtime platform"), "TestRuntime")
        .with::<String>(Rc::new("options".to_string()))
        .after_platform_services_setup(|builder| {
            assert!(builder.instance().is_none());
            log("after_platform_services_setup");
        })
        .after_application_setup(|builder| {
            assert!(builder.instance().is_some());
            log("after_application_setup");
        })
        .after_setup(|_| log("after_setup 1"))
        .after_setup(|_| log("after_setup 2"))
        .setup_with_lifetime(lifetime.clone());

    assert_eq!(
        take_log(),
        [
            "before_app_init",
            "runtime platform",
            "text shaping",
            "rendering",
            "windowing",
            "after_platform_services_setup",
            "new",
            "register_services",
            "initialize",
            "after_application_setup",
            "after_setup 1",
            "after_setup 2",
            "on_framework_initialization_completed",
            "after_app_init",
        ]
    );

    let instance = builder.instance().unwrap();
    assert!(instance.is::<TestApp>());
    assert!(Application::current().unwrap().ptr_eq(&instance));
    assert!(Rc::ptr_eq(&instance.application_lifetime().unwrap(), &lifetime));
    assert_eq!(*FerroLocator::current().get_required_service::<String>(), "options");
    assert!(FerroLocator::current().get_service::<dyn IInputManager>().is_some());
}

#[test]
fn setup_without_starting_uses_no_lifetime_and_the_standard_runtime_platform() {
    let _scope = scope();

    let builder = configured().setup_without_starting();

    let instance = builder.instance().unwrap();
    assert!(instance.application_lifetime().is_none());
    assert!(FerroLocator::current().get_service::<dyn IAssetLoader>().is_some());

    // The combined callbacks are exposed.
    builder.after_setup_callback()(&builder);
    builder.after_platform_services_setup_callback()(&builder);
}

#[test]
fn start_sets_up_and_calls_main_with_the_application_and_the_arguments() {
    let _scope = scope();
    let args = ["foo".to_string(), "bar".to_string()];
    let mut called = false;

    let builder = AppBuilder::configure_with(|| {
        log("factory");
        Application::new()
    })
    .use_standard_runtime_platform_subsystem()
    .use_windowing_subsystem(|| {}, "")
    .use_rendering_subsystem(|| {}, "")
    .use_text_shaping_subsystem(|| {}, "");

    assert!(std::ptr::eq(builder.application_type().unwrap(), Application::TYPE));

    builder.start(
        |application, main_args| {
            assert!(Application::current().unwrap().ptr_eq(application));
            assert_eq!(main_args, args);
            called = true;
        },
        &args,
    );

    assert!(called);
    assert_eq!(take_log(), ["factory"]);
}

#[test]
fn with_func_registers_a_service_function() {
    let _scope = scope();

    configured().with_func::<String>(|| Rc::new("made".to_string())).setup_without_starting();

    let a = FerroLocator::current().get_required_service::<String>();
    let b = FerroLocator::current().get_required_service::<String>();
    assert_eq!(*a, "made");
    assert!(!Rc::ptr_eq(&a, &b));
}

#[test]
fn configure_fonts_runs_the_action_with_the_current_font_manager_after_setup() {
    use ferroui_base::media::fonts::testing::TestFontManagerImpl;
    use ferroui_base::media::FontManager;
    use ferroui_base::platform::IFontManagerImpl;

    let _scope = scope();
    let font_manager_impl = TestFontManagerImpl::headless();
    let configured_with: Rc<RefCell<Vec<Rc<FontManager>>>> = Rc::new(RefCell::new(Vec::new()));

    let builder = configured()
        .with_func::<dyn IFontManagerImpl>(move || font_manager_impl.clone())
        .configure_fonts({
            let configured_with = configured_with.clone();
            move |font_manager| configured_with.borrow_mut().push(font_manager.clone())
        });
    assert!(configured_with.borrow().is_empty());

    builder.setup_without_starting();

    assert_eq!(1, configured_with.borrow().len());
    assert!(Rc::ptr_eq(&configured_with.borrow()[0], &FontManager::current()));
}

#[test]
#[should_panic(expected = "Setup was already called on one of AppBuilder instances")]
fn setup_can_only_be_called_once() {
    let _scope = scope();
    configured().setup_without_starting();
    configured().setup_without_starting();
}

#[test]
#[should_panic(expected = "No runtime platform services configured.")]
fn setup_requires_runtime_platform_services() {
    let _scope = scope();
    AppBuilder::configure::<TestApp>().setup_without_starting();
}

#[test]
#[should_panic(expected = "No windowing system configured.")]
fn setup_requires_a_windowing_subsystem() {
    let _scope = scope();
    AppBuilder::configure::<TestApp>().use_standard_runtime_platform_subsystem().setup_without_starting();
}

#[test]
#[should_panic(expected = "No rendering system configured.")]
fn setup_requires_a_rendering_subsystem() {
    let _scope = scope();
    AppBuilder::configure::<TestApp>()
        .use_standard_runtime_platform_subsystem()
        .use_windowing_subsystem(|| {}, "")
        .setup_without_starting();
}

#[test]
#[should_panic(expected = "No text shaping system configured.")]
fn setup_requires_a_text_shaping_subsystem() {
    let _scope = scope();
    AppBuilder::configure::<TestApp>()
        .use_standard_runtime_platform_subsystem()
        .use_windowing_subsystem(|| {}, "")
        .use_rendering_subsystem(|| {}, "")
        .setup_without_starting();
}

#[test]
fn setup_unsafe_does_not_check_the_subsystems() {
    let _scope = scope();
    let builder = AppBuilder::configure::<TestApp>();

    builder.setup_unsafe();
    builder.setup_unsafe();

    assert!(builder.instance().is_some());
}

#[test]
fn log_to_delegate_installs_a_sink() {
    use ferroui_base::logging::Logger;

    let previous = Logger::sink();
    let builder = AppBuilder::configure::<TestApp>();

    builder.log_to_delegate(|_| {}, LogEventLevel::Warning, &[]);
    assert!(Logger::sink().is_some());

    builder.log_to_text_writer(Vec::<u8>::new(), LogEventLevel::Warning, &["Layout"]);
    assert!(Logger::sink().is_some());

    Logger::set_sink(previous);
}

// --- SystemFontAppBuilderExtension (not from upstream: there are no upstream tests) ---

#[test]
fn with_system_font_source_adds_the_fonts_of_the_source_to_the_system_fonts() {
    use ferroui_base::media::fonts::testing::{TestFontBuilder, TestFontManagerImpl};
    use ferroui_base::media::{FontManager, FontStretch, FontStyle, FontWeight};
    use ferroui_base::platform::storage::file_io::StorageProviderHelpers;
    use ferroui_base::platform::IFontManagerImpl;

    /// A directory that is removed when the value is dropped.
    struct TempDirectory(std::path::PathBuf);

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    let _scope = scope();
    let directory =
        TempDirectory(std::env::temp_dir().join(format!("ferroui-app-builder-tests-{}-fonts", std::process::id())));
    let _ = std::fs::remove_dir_all(&directory.0);
    std::fs::create_dir_all(&directory.0).unwrap();
    let font_file = directory.0.join("Extra Sans.ttf");
    std::fs::write(&font_file, TestFontBuilder::new("Extra Sans").build_bytes()).unwrap();
    let font_source = StorageProviderHelpers::uri_from_file_path(font_file.to_str().unwrap(), false);

    let font_manager_impl = TestFontManagerImpl::headless();
    let builder = configured()
        .with_func::<dyn IFontManagerImpl>(move || font_manager_impl.clone())
        .with_system_font_source(font_source);

    builder.setup_without_starting();

    let system_fonts = FontManager::current().system_fonts();
    let glyph_typeface =
        system_fonts.try_get_glyph_typeface("Extra Sans", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal);
    assert!(glyph_typeface.is_some());
    assert!(system_fonts
        .try_get_glyph_typeface("Missing Sans", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .is_none());
}
