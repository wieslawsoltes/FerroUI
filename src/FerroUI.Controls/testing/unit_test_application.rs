use super::TestServices;
use crate::platform::IWindowingPlatform;
use crate::{Application, ApplicationImpl, Control};
use ferroui_base::animation::IGlobalClock;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{
    IAccessKeyHandler, IInputManager, IKeyboardDevice, IKeyboardNavigationHandler, IMouseDevice,
};
use ferroui_base::platform::{
    DefaultPlatformSettings, IAssetLoader, ICursorFactory, IPlatformRenderInterface, IPlatformSettings,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{IGlobalStyles, Style};
use ferroui_base::threading::{Dispatcher, FerroSynchronizationContext, UnitTestDispatcherScope};
use ferroui_base::{ferro_class, instantiate, FerroLocator, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// The application of unit tests: it registers the services of a
/// [`TestServices`] instead of those of a real application.
#[repr(C)]
pub struct UnitTestApplication {
    base: Application,
    services: TestServices,
}

ferro_class!(UnitTestApplication: Application);

impl FerroObjectImpl for UnitTestApplication {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        Application::bind_current(this.to_ref().upcast());
        this.register_services();
    }
}

impl ApplicationImpl for UnitTestApplication {
    fn register_services(this: &Self) {
        let services = this.services();
        let locator = FerroLocator::current_mutable();

        bind(&locator, services.asset_loader.clone() as Option<Rc<dyn IAssetLoader>>);
        bind(&locator, services.global_clock.clone() as Option<Rc<dyn IGlobalClock>>);
        locator.bind::<dyn IGlobalStyles>().to_constant(this.as_global_styles());
        bind(&locator, services.input_manager.clone() as Option<Rc<dyn IInputManager>>);
        bind(
            &locator,
            services
                .input_manager
                .as_ref()
                .map(|input_manager| crate::ToolTipService::new(input_manager) as Rc<dyn crate::IToolTipService>),
        );
        bind(&locator, services.keyboard_device.as_ref().and_then(|f| f()) as Option<Rc<dyn IKeyboardDevice>>);
        bind(&locator, services.mouse_device.as_ref().and_then(|f| f()) as Option<Rc<dyn IMouseDevice>>);
        bind_func::<dyn IKeyboardNavigationHandler>(&locator, services.keyboard_navigation.clone());
        bind(&locator, services.render_interface.clone() as Option<Rc<dyn IPlatformRenderInterface>>);
        // The font services of a text test scope around the application stay
        // in place when the services name none.
        if let Some(font_manager_impl) = services.font_manager_impl.clone() {
            locator.bind::<dyn ferroui_base::platform::IFontManagerImpl>().to_constant(font_manager_impl);
        }
        if let Some(text_shaper_impl) = services.text_shaper_impl.clone() {
            locator.bind::<dyn ferroui_base::platform::ITextShaperImpl>().to_constant(text_shaper_impl);
        }
        bind(&locator, services.standard_cursor_factory.clone() as Option<Rc<dyn ICursorFactory>>);
        bind(&locator, services.windowing_platform.clone() as Option<Rc<dyn IWindowingPlatform>>);
        if let Some(platform) = services.platform.clone() {
            locator.bind::<dyn ferroui_base::platform::IRuntimePlatform>().to_constant(platform);
        }
        locator.bind_to_self_singleton::<PlatformHotkeyConfiguration>();
        let platform_settings: Rc<dyn IPlatformSettings> =
            services.platform_settings.clone().unwrap_or_else(|| Rc::new(DefaultPlatformSettings::new()));
        locator.bind::<dyn IPlatformSettings>().to_constant(platform_settings);
        bind_func::<dyn IAccessKeyHandler>(&locator, services.access_key_handler.clone());
        // Top-levels created by tests render through the null renderer (the
        // counterpart of the dummy compositor of the reference test suite).
        super::NullRenderer::register();

        this.initialize_theme_variant();

        let theme = services.theme.as_ref().map(|theme| theme());

        if let Some(theme) = theme {
            match theme.as_object().and_then(|object| object.downcast_ref::<Style>()) {
                Some(styles) => this.styles().add_range(styles.children().snapshot().iter().cloned()),
                None => this.styles().add(theme),
            }
        }
    }
}

/// Registers `service` as the service of type `T`; a missing service is
/// registered too, so that it hides the one of an outer scope.
fn bind<T: ?Sized + 'static>(locator: &FerroLocator, service: Option<Rc<T>>) {
    match service {
        Some(service) => locator.bind::<T>().to_constant(service),
        None => locator.bind::<T>().to_func(|| None),
    };
}

/// Registers `factory` as the function that produces the service of type
/// `T`; without a factory the service is registered as missing.
fn bind_func<T: ?Sized + 'static>(locator: &FerroLocator, factory: Option<Rc<dyn Fn() -> Option<Rc<T>>>>) {
    match factory {
        Some(factory) => locator.bind::<T>().to_func(move || factory()),
        None => locator.bind::<T>().to_func(|| None),
    };
}

impl UnitTestApplication {
    /// Creates the application, makes it the current one and registers
    /// `services`.
    pub fn new(services: TestServices) -> Ref<Self> {
        instantiate(Self::construct(services))
    }

    /// Field initialisation, for the application classes of tests that
    /// derive from this one.
    pub fn construct(services: TestServices) -> Self {
        Self { base: Application::construct(), services }
    }

    /// The current application, when it is a unit test application.
    pub fn current() -> Option<Ref<UnitTestApplication>> {
        Application::current().and_then(|application| application.cast::<UnitTestApplication>())
    }

    /// The services the application was created with.
    pub fn services(&self) -> &TestServices {
        &self.services
    }

    /// Starts a unit test application with `services` in a new service
    /// locator scope, on a dispatcher that belongs to the calling test.
    ///
    /// Disposing (or dropping) the returned scope runs the pending
    /// dispatcher jobs and restores the previous scope.
    pub fn start(services: TestServices) -> UnitTestApplicationScope {
        let dispatcher = Dispatcher::unit_test_scope();
        Control::reset_loaded_queue_for_unit_tests();
        let scope = FerroLocator::enter_scope();
        let old_context = FerroSynchronizationContext::current();
        let _ = UnitTestApplication::new(services);
        Dispatcher::reset_before_unit_tests();
        UnitTestApplicationScope { state: RefCell::new(Some(ScopeState { scope, old_context, dispatcher })) }
    }
}

struct ScopeState {
    scope: Rc<dyn IDisposable>,
    old_context: Option<Arc<FerroSynchronizationContext>>,
    dispatcher: UnitTestDispatcherScope,
}

/// The running unit test application; see [`UnitTestApplication::start`].
pub struct UnitTestApplicationScope {
    state: RefCell<Option<ScopeState>>,
}

impl UnitTestApplicationScope {
    /// Ends the unit test application. Further calls do nothing.
    pub fn dispose(&self) {
        let Some(state) = self.state.borrow_mut().take() else { return };

        // Do not run more user code while a failed test unwinds.
        if !std::thread::panicking() {
            let dispatcher = Dispatcher::ui_thread();
            if dispatcher.check_access() {
                dispatcher.run_jobs(None);
            }

            let tool_tip_service = ferroui_base::LocatorExtensions::get_service::<dyn crate::IToolTipService>(
                &*FerroLocator::current(),
            );
            if let Some(tool_tip_service) = tool_tip_service.as_ref().and_then(|service| service.as_tool_tip_service()) {
                tool_tip_service.dispose();
            }

            Dispatcher::reset_for_unit_tests();
        }

        state.scope.dispose();
        Dispatcher::reset_before_unit_tests();
        FerroSynchronizationContext::set_current(state.old_context);
        Control::reset_loaded_queue_for_unit_tests();
        drop(state.dispatcher);
    }
}

impl IDisposable for UnitTestApplicationScope {
    fn dispose(&self) {
        UnitTestApplicationScope::dispose(self)
    }
}

impl Drop for UnitTestApplicationScope {
    fn drop(&mut self) {
        UnitTestApplicationScope::dispose(self)
    }
}
