use crate::application_lifetimes::IApplicationLifetime;
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Application, ApplicationImpl, ApplicationImplExt, IGlobalDataTemplates};
use ferroui_base::animation::IGlobalClock;
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{IAccessKeyHandler, IInputManager, IKeyboardNavigationHandler};
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::PointerType;
use ferroui_base::platform::{DefaultPlatformSettings, IPlatformSettings, PlatformColorValues, PlatformThemeVariant};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Size;
use std::time::Duration;
use ferroui_base::styling::{IGlobalStyles, IStyle, IThemeVariantHost, Style, ThemeVariant};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn can_bind_to_data_context() {
    let _app = UnitTestApplication::start(TestServices::default());
    let application = Application::current().unwrap();

    application.set_data_context(Some(Rc::new("Test".to_string())));

    application.bind_binding(Application::name_property().as_property(), &ferroui_base::data::ReflectionBinding::new("."));

    assert_eq!(Some("Test"), application.name().as_deref());
}

#[test]
fn application_is_a_data_context_provider() {
    use ferroui_base::IDataContextProvider;

    let application = Application::new();
    assert!(<dyn IDataContextProvider>::is_implemented_by(&application));
    assert!(<dyn IDataContextProvider>::is_implemented_by(&crate::Border::new()));
    assert!(!<dyn IDataContextProvider>::is_implemented_by(&ferroui_base::input::KeyBinding::new()));

    let provider = application.as_data_context_provider();
    provider.set_data_context(Some(Rc::new(5_i32)));
    let value = application.data_context().unwrap();
    assert_eq!((*value).as_any().downcast_ref::<i32>(), Some(&5));
    assert!(provider.data_context().is_some());
}

#[test]
#[should_panic(expected = "Value cannot be null. (Parameter 'main_window')")]
fn throws_argument_null_exception_on_run_if_main_window_is_null() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    Application::current().unwrap().run_window(None::<ferroui_base::Ref<crate::Window>>);
}

#[test]
fn raises_resources_changed_when_event_handler_added_after_resources_has_been_accessed() {
    // Test for #1765.
    let _app = UnitTestApplication::start(TestServices::default());

    let application = Application::current().unwrap();
    let resources = application.resources();
    let raised = Rc::new(Cell::new(false));

    let r = raised.clone();
    application.resources_changed(move |_| r.set(true));
    resources.add_value("foo", "bar".to_string());

    assert!(raised.get());
}

// --- theme variant (ThemeVariantTests) -------------------------------------

/// Platform settings whose theme variant is set by the test.
pub(crate) struct TestPlatformSettings {
    base: DefaultPlatformSettings,
    theme_variant: Cell<PlatformThemeVariant>,
}

impl TestPlatformSettings {
    pub(crate) fn new(theme_variant: PlatformThemeVariant) -> Rc<Self> {
        Rc::new(Self { base: DefaultPlatformSettings::new(), theme_variant: Cell::new(theme_variant) })
    }

    pub(crate) fn set_theme_variant(&self, value: PlatformThemeVariant) {
        if self.theme_variant.get() == value {
            return;
        }

        self.theme_variant.set(value);
        self.base.on_color_values_changed(self.get_color_values());
    }
}

impl IPlatformSettings for TestPlatformSettings {
    fn get_tap_size(&self, type_: PointerType) -> Size {
        self.base.get_tap_size(type_)
    }

    fn get_double_tap_size(&self, type_: PointerType) -> Size {
        self.base.get_double_tap_size(type_)
    }

    fn get_double_tap_time(&self, type_: PointerType) -> Duration {
        self.base.get_double_tap_time(type_)
    }

    fn hold_wait_duration(&self) -> Duration {
        self.base.hold_wait_duration()
    }

    fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
        self.base.hotkey_configuration()
    }

    fn preferred_application_language(&self) -> String {
        self.base.preferred_application_language()
    }

    fn get_color_values(&self) -> PlatformColorValues {
        PlatformColorValues::new().with_theme_variant(self.theme_variant.get())
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
}

fn start_application(platform_settings: &Rc<TestPlatformSettings>) -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::default().with_platform_settings(platform_settings.clone()))
}

fn theme_variant_of(platform_theme_variant: PlatformThemeVariant) -> ThemeVariant {
    match platform_theme_variant {
        PlatformThemeVariant::Light => ThemeVariant::light(),
        PlatformThemeVariant::Dark => ThemeVariant::dark(),
    }
}

#[test]
fn application_actual_theme_variant_falls_back_to_light_without_platform_settings() {
    let application = Application::new();
    application.initialize_theme_variant();

    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::light()));
}

#[test]
fn application_actual_theme_variant_is_initialized_from_platform_settings() {
    for platform_theme_variant in [PlatformThemeVariant::Light, PlatformThemeVariant::Dark] {
        let _app = start_application(&TestPlatformSettings::new(platform_theme_variant));

        assert_eq!(
            Application::current().unwrap().actual_theme_variant(),
            Some(theme_variant_of(platform_theme_variant))
        );
    }
}

#[test]
fn application_actual_theme_variant_follows_platform_settings_changes() {
    let platform_settings = TestPlatformSettings::new(PlatformThemeVariant::Light);
    let _app = start_application(&platform_settings);
    let application = Application::current().unwrap();

    let raised = Rc::new(Cell::new(0));
    let r = raised.clone();
    application.actual_theme_variant_changed(move || r.set(r.get() + 1));

    platform_settings.set_theme_variant(PlatformThemeVariant::Dark);

    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(raised.get(), 1);
}

#[test]
fn application_requested_theme_variant_overrides_platform_settings() {
    let platform_settings = TestPlatformSettings::new(PlatformThemeVariant::Light);
    let _app = start_application(&platform_settings);
    let application = Application::current().unwrap();

    application.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::dark()));

    platform_settings.set_theme_variant(PlatformThemeVariant::Light);
    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::dark()));
}

#[test]
fn application_actual_theme_variant_reverts_to_platform_settings_when_requesting_default() {
    let platform_settings = TestPlatformSettings::new(PlatformThemeVariant::Dark);
    let _app = start_application(&platform_settings);
    let application = Application::current().unwrap();

    application.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::light()));

    application.set_requested_theme_variant(Some(ThemeVariant::default()));
    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::dark()));
}

#[test]
fn application_actual_theme_variant_reverts_to_platform_settings_when_requesting_null() {
    let platform_settings = TestPlatformSettings::new(PlatformThemeVariant::Dark);
    let _app = start_application(&platform_settings);
    let application = Application::current().unwrap();

    application.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::light()));

    application.set_requested_theme_variant(None);
    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::dark()));
}

#[test]
fn application_custom_theme_variant_is_used_as_is() {
    let custom = ThemeVariant::new("Custom", Some(ThemeVariant::dark()));
    let _app = start_application(&TestPlatformSettings::new(PlatformThemeVariant::Light));
    let application = Application::current().unwrap();

    application.set_requested_theme_variant(Some(custom.clone()));

    assert_eq!(application.actual_theme_variant(), Some(custom));
}

// --- tests of this port ----------------------------------------------------

#[test]
fn actual_theme_variant_changed_is_raised_on_the_application_and_its_host() {
    let _app = UnitTestApplication::start(TestServices::default());
    let application = Application::current().unwrap();
    let raised = Rc::new(Cell::new(0));
    let host_raised = Rc::new(Cell::new(0));

    let r = raised.clone();
    let subscription = application.actual_theme_variant_changed(move || r.set(r.get() + 1));
    let r = host_raised.clone();
    let host = application.as_theme_variant_host();
    host.actual_theme_variant_changed(Rc::new(move || r.set(r.get() + 1)));

    application.set_requested_theme_variant(Some(ThemeVariant::dark()));

    assert_eq!(raised.get(), 1);
    assert_eq!(host_raised.get(), 1);
    assert_eq!(host.actual_theme_variant(), Some(ThemeVariant::dark()));

    subscription.dispose();
    application.set_requested_theme_variant(Some(ThemeVariant::light()));

    assert_eq!(raised.get(), 1);
    assert_eq!(host_raised.get(), 2);
}

#[test]
fn switching_back_to_the_default_theme_variant_raises_actual_theme_variant_changed_once() {
    let _app = UnitTestApplication::start(TestServices::default());
    let application = Application::current().unwrap();
    application.set_requested_theme_variant(Some(ThemeVariant::dark()));

    let raised = Rc::new(Cell::new(0));
    let r = raised.clone();
    let _subscription = application.actual_theme_variant_changed(move || r.set(r.get() + 1));

    // The application is a theme variant root: the default variant resolves
    // to the variant of the platform (light) in one step.
    application.set_requested_theme_variant(Some(ThemeVariant::default()));

    assert_eq!(raised.get(), 1);
    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::light()));
}

#[test]
fn current_is_the_application_of_the_scope() {
    assert!(Application::current().is_none());

    {
        let _app = UnitTestApplication::start(TestServices::default());
        let application = Application::current().unwrap();

        assert!(application.is::<UnitTestApplication>());
        assert!(UnitTestApplication::current().is_some());
        assert_eq!(application.name().as_deref(), Some("FerroUI Application"));
    }

    assert!(Application::current().is_none());
}

#[test]
fn name_property_reads_and_writes_the_name() {
    let application = Application::new();
    let changes = Rc::new(Cell::new(0));

    let c = changes.clone();
    let id = Application::name_property().id();
    application.property_changed(move |e| {
        if e.property().id() == id {
            c.set(c.get() + 1);
        }
    });
    application.set_name(Some("Test".to_string()));

    assert_eq!(application.name().as_deref(), Some("Test"));
    assert_eq!(application.get_direct_value(Application::name_property()).as_deref(), Some("Test"));
    assert_eq!(changes.get(), 1);
}

#[test]
fn data_context_can_be_set() {
    let application = Application::new();
    assert!(application.data_context().is_none());

    application.set_data_context(Some(Rc::new("Test".to_string())));

    let value = application.data_context().unwrap();
    assert_eq!((*value).as_any().downcast_ref::<String>().map(String::as_str), Some("Test"));
}

#[test]
fn resources_are_found_in_the_dictionary_and_then_in_the_styles() {
    let application = Application::new();
    let key = ResourceKey::from("foo");
    assert!(!application.has_resources());
    assert!(application.try_get_resource(&key, None).is_none());

    application.styles().resources().add_value("foo", 1_i32);
    assert!(application.has_resources());
    assert!(application.try_get_resource(&key, None).is_some());

    application.resources().add_value("foo", 2_i32);
    let value = application.try_get_resource(&key, None).unwrap().unwrap();
    assert_eq!((*value).as_any().downcast_ref::<i32>(), Some(&2));

    // The interface handle sees the same resources.
    assert!(application.as_resource_host().try_get_resource(&key, None).is_some());
}

#[test]
fn replacing_resources_moves_the_owner() {
    let application = Application::new();
    let old = application.resources();
    assert!(old.owner().is_some());

    let new = ferroui_base::controls::ResourceDictionary::new();
    application.set_resources(new.clone());

    assert!(old.owner().is_none());
    assert!(new.owner().is_some());
    assert!(application.resources().ptr_eq(&new));
}

#[test]
fn global_styles_added_and_removed_are_raised() {
    let application = Application::new();
    let global = application.as_global_styles();
    let added = Rc::new(Cell::new(0));
    let removed = Rc::new(Cell::new(0));

    let a = added.clone();
    let subscription = global.global_styles_added(Rc::new(move |styles: &[Rc<dyn IStyle>]| a.set(a.get() + styles.len())));
    let r = removed.clone();
    global.global_styles_removed(Rc::new(move |styles: &[Rc<dyn IStyle>]| r.set(r.get() + styles.len())));

    assert!(!application.is_styles_initialized());
    assert!(!global.is_styles_initialized());

    let style = Style::new();
    application.styles().add(style.clone());

    assert!(global.is_styles_initialized());
    assert!(global.styling_parent().is_none());
    assert_eq!(added.get(), 1);
    assert_eq!(removed.get(), 0);

    application.styles().remove(style.clone());
    assert_eq!(removed.get(), 1);

    subscription.dispose();
    application.styles().add(style);
    assert_eq!(added.get(), 1);
}

#[test]
fn data_templates_are_created_lazily() {
    let application = Application::new();
    let host = application.as_global_data_templates();

    assert!(!application.is_data_templates_initialized());
    assert!(!host.is_data_templates_initialized());

    assert_eq!(application.data_templates().count(), 0);

    assert!(application.is_data_templates_initialized());
    assert!(host.is_data_templates_initialized());
    assert!(host.data_templates() == application.data_templates());
}

struct TestLifetime;

impl IApplicationLifetime for TestLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn register_services_binds_the_application_services() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    let application = Application::new();
    application.set_application_lifetime(Some(Rc::new(TestLifetime)));

    application.register_services();

    let locator = FerroLocator::current();
    assert!(locator.get_service::<dyn IAccessKeyHandler>().is_some());
    assert!(locator.get_service::<dyn IKeyboardNavigationHandler>().is_some());
    assert!(locator.get_service::<dyn ferroui_base::input::raw::IDragDropDevice>().is_some());
    assert!(locator.get_service::<dyn IInputManager>().is_some());
    assert!(locator.get_service::<dyn IGlobalClock>().is_some());
    assert!(locator.get_service::<dyn IGlobalDataTemplates>().is_some());
    assert!(application.input_manager().is_some());
    assert_eq!(application.actual_theme_variant(), Some(ThemeVariant::light()));

    // Every interface handle of the application is the same object.
    let global_styles = locator.get_service::<dyn IGlobalStyles>().unwrap();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&global_styles), Rc::as_ptr(&application.as_global_styles())));
    let theme_variant_host = locator.get_service::<dyn IThemeVariantHost>().unwrap();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&theme_variant_host), Rc::as_ptr(&application.as_resource_host())));

    // Access key handlers and keyboard navigation handlers are transient.
    let a = locator.get_service::<dyn IAccessKeyHandler>().unwrap();
    let b = locator.get_service::<dyn IAccessKeyHandler>().unwrap();
    assert!(!Rc::ptr_eq(&a, &b));

    assert!(application.application_lifetime().is_some());
    scope.dispose();
}

#[test]
#[should_panic(expected = "It's not possible to change ApplicationLifetime after Application was initialized.")]
fn application_lifetime_cannot_be_changed_after_the_services_are_registered() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let application = Application::new();
    application.register_services();

    application.set_application_lifetime(Some(Rc::new(TestLifetime)));
}

#[test]
fn try_get_feature_returns_the_supported_features_only() {
    let _app = UnitTestApplication::start(TestServices::default());
    let application = Application::current().unwrap();

    assert!(application.platform_settings().is_some());
    assert!(application.try_get::<dyn IPlatformSettings>().is_some());
    assert!(application.as_optional_feature_provider().try_get::<dyn IPlatformSettings>().is_some());
    // Registered with the locator, but not a feature of the application.
    assert!(FerroLocator::current().get_service::<dyn IGlobalStyles>().is_some());
    assert!(application.try_get_feature(TypeId::of::<dyn IGlobalStyles>()).is_none());
}

#[test]
fn unit_test_application_registers_the_test_services() {
    let outer = FerroLocator::current_mutable();
    {
        let app = UnitTestApplication::start(TestServices::real_focus());
        let locator = FerroLocator::current();

        assert!(locator.get_service::<dyn IInputManager>().is_some());
        assert!(locator.get_service::<dyn IKeyboardNavigationHandler>().is_some());
        assert!(locator.get_service::<dyn ferroui_base::input::IKeyboardDevice>().is_some());
        assert!(locator.get_service::<dyn ferroui_base::platform::IAssetLoader>().is_some());
        assert!(locator.get_service::<dyn IAccessKeyHandler>().is_none());
        assert!(locator.get_service::<dyn IGlobalClock>().is_none());
        assert!(UnitTestApplication::current().unwrap().services().input_manager.is_some());

        app.dispose();
        assert!(Application::current().is_none());
        // Disposing twice (and dropping afterwards) does nothing.
        app.dispose();
    }
    assert!(Rc::ptr_eq(&outer, &FerroLocator::current_mutable()));
}

#[test]
fn unit_test_application_adds_the_theme_to_the_styles() {
    let theme = || -> Rc<dyn IStyle> {
        let theme = Style::new();
        theme.children().add(Style::new());
        theme.children().add(Style::new());
        theme.into()
    };
    let _app = UnitTestApplication::start(TestServices::default().with_theme(theme));

    // The children of a style theme are added, not the theme itself.
    assert_eq!(Application::current().unwrap().styles().count(), 2);
}

#[test]
fn unit_test_application_runs_pending_jobs_when_disposed() {
    let ran = Rc::new(Cell::new(false));
    {
        let _app = UnitTestApplication::start(TestServices::default());
        let r = ran.clone();
        Dispatcher::ui_thread().post_local(move || r.set(true), ferroui_base::threading::DispatcherPriority::DEFAULT);
        assert!(!ran.get());
    }
    assert!(ran.get());
}

/// A user-defined application class overrides the virtual members.
#[repr(C)]
struct CountingApplication {
    base: Application,
    initialized: Cell<u32>,
}

ferroui_base::ferro_class!(CountingApplication: Application);
ferroui_base::ferro_impl_classes!(CountingApplication: ferroui_base::FerroObjectImpl);

impl ApplicationImpl for CountingApplication {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.initialized.set(this.initialized.get() + 1);
    }
}

#[test]
fn virtual_members_can_be_overridden() {
    let application =
        ferroui_base::instantiate(CountingApplication { base: Application::construct(), initialized: Cell::new(0) });
    let base: ferroui_base::Ref<Application> = application.clone().upcast();

    base.initialize();
    base.on_framework_initialization_completed();

    assert_eq!(application.initialized.get(), 1);
}
