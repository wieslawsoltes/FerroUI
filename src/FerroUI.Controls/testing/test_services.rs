use crate::platform::{IPlatformIconLoader, IWindowImpl, IWindowingPlatform};
use ferroui_base::animation::IGlobalClock;
use ferroui_base::input::{
    IAccessKeyHandler, IInputManager, IKeyboardDevice, IKeyboardNavigationHandler, IMouseDevice, InputManager,
    KeyboardDevice, KeyboardNavigationHandler,
};
use ferroui_base::platform::{
    IAssetLoader, ICursorFactory, IPlatformRenderInterface, IPlatformSettings, StandardAssetLoader,
};
use ferroui_base::styling::IStyle;
use std::rc::Rc;

/// The services a [`UnitTestApplication`](super::UnitTestApplication)
/// registers.
///
/// Every service is optional; create a set with the fields a test needs
/// and `..Default::default()`, or start from one of the presets and replace
/// services with the `with_*` methods:
///
/// ```ignore
/// let services = TestServices { input_manager: Some(Rc::new(InputManager::new())), ..Default::default() };
/// let services = TestServices::real_focus().with_platform_settings(settings);
/// ```
#[derive(Clone, Default)]
pub struct TestServices {
    pub asset_loader: Option<Rc<dyn IAssetLoader>>,
    pub input_manager: Option<Rc<dyn IInputManager>>,
    pub global_clock: Option<Rc<dyn IGlobalClock>>,
    pub access_key_handler: Option<Rc<dyn Fn() -> Option<Rc<dyn IAccessKeyHandler>>>>,
    pub keyboard_device: Option<Rc<dyn Fn() -> Option<Rc<dyn IKeyboardDevice>>>>,
    pub keyboard_navigation: Option<Rc<dyn Fn() -> Option<Rc<dyn IKeyboardNavigationHandler>>>>,
    pub mouse_device: Option<Rc<dyn Fn() -> Option<Rc<dyn IMouseDevice>>>>,
    pub render_interface: Option<Rc<dyn IPlatformRenderInterface>>,
    /// The font backend; when none is given the locator keeps the one it has
    /// (the one of a text test scope around the application, if any).
    pub font_manager_impl: Option<Rc<dyn ferroui_base::platform::IFontManagerImpl>>,
    /// The text shaper; when none is given the locator keeps the one it has.
    pub text_shaper_impl: Option<Rc<dyn ferroui_base::platform::ITextShaperImpl>>,
    pub platform_settings: Option<Rc<dyn IPlatformSettings>>,
    pub standard_cursor_factory: Option<Rc<dyn ICursorFactory>>,
    pub theme: Option<Rc<dyn Fn() -> Rc<dyn IStyle>>>,
    pub window_impl: Option<Rc<dyn IWindowImpl>>,
    pub windowing_platform: Option<Rc<dyn IWindowingPlatform>>,
    /// The runtime platform; when none is given the locator keeps the one
    /// it has (none in a unit test).
    pub platform: Option<Rc<dyn ferroui_base::platform::IRuntimePlatform>>,
    /// The loader of the icons of windows and tray icons (the test services
    /// of the reference test suite have none: its tests load no icon); when
    /// none is given the locator keeps the one it has (none in a unit test).
    /// [`TestIconLoader`](super::TestIconLoader) is one.
    pub icon_loader: Option<Rc<dyn IPlatformIconLoader>>,
}

impl TestServices {
    /// Creates a set without services.
    pub fn new() -> Self {
        Self::default()
    }

    /// The services of tests that draw through the platform render
    /// interface.
    pub fn mock_platform_render_interface() -> Self {
        Self { asset_loader: Some(Rc::new(StandardAssetLoader::new(None))), ..Self::default() }
    }

    /// The services of tests that need a runtime platform: a mocked one,
    /// which reports the default platform information.
    pub fn mock_platform_wrapper() -> Self {
        Self { platform: Some(Rc::new(MockRuntimePlatform)), ..Self::default() }
    }

    /// The services of tests that only need assets.
    pub fn mock_threading_interface() -> Self {
        Self { asset_loader: Some(Rc::new(StandardAssetLoader::new(None))), ..Self::default() }
    }

    /// The services of tests that move the keyboard focus: a real keyboard
    /// device, keyboard navigation handler and input manager.
    pub fn real_focus() -> Self {
        Self {
            keyboard_device: Some(Rc::new(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))),
            keyboard_navigation: Some(Rc::new(|| {
                Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>)
            })),
            input_manager: Some(Rc::new(InputManager::new())),
            asset_loader: Some(Rc::new(StandardAssetLoader::new(None))),
            ..Self::default()
        }
    }

    /// The services of tests that lay out text.
    pub fn text_services() -> Self {
        Self { asset_loader: Some(Rc::new(StandardAssetLoader::new(None))), ..Self::default() }
    }

    /// Replaces the asset loader.
    pub fn with_asset_loader(mut self, value: Rc<dyn IAssetLoader>) -> Self {
        self.asset_loader = Some(value);
        self
    }

    /// Replaces the input manager.
    pub fn with_input_manager(mut self, value: Rc<dyn IInputManager>) -> Self {
        self.input_manager = Some(value);
        self
    }

    /// Replaces the global clock.
    pub fn with_global_clock(mut self, value: Rc<dyn IGlobalClock>) -> Self {
        self.global_clock = Some(value);
        self
    }

    /// Replaces the factory of the access key handler.
    pub fn with_access_key_handler(mut self, value: impl Fn() -> Option<Rc<dyn IAccessKeyHandler>> + 'static) -> Self {
        self.access_key_handler = Some(Rc::new(value));
        self
    }

    /// Replaces the factory of the keyboard device.
    pub fn with_keyboard_device(mut self, value: impl Fn() -> Option<Rc<dyn IKeyboardDevice>> + 'static) -> Self {
        self.keyboard_device = Some(Rc::new(value));
        self
    }

    /// Replaces the factory of the keyboard navigation handler.
    pub fn with_keyboard_navigation(
        mut self,
        value: impl Fn() -> Option<Rc<dyn IKeyboardNavigationHandler>> + 'static,
    ) -> Self {
        self.keyboard_navigation = Some(Rc::new(value));
        self
    }

    /// Replaces the factory of the mouse device.
    pub fn with_mouse_device(mut self, value: impl Fn() -> Option<Rc<dyn IMouseDevice>> + 'static) -> Self {
        self.mouse_device = Some(Rc::new(value));
        self
    }

    /// Replaces the platform render interface.
    pub fn with_render_interface(mut self, value: Rc<dyn IPlatformRenderInterface>) -> Self {
        self.render_interface = Some(value);
        self
    }

    /// Replaces the font backend.
    pub fn with_font_manager_impl(mut self, value: Rc<dyn ferroui_base::platform::IFontManagerImpl>) -> Self {
        self.font_manager_impl = Some(value);
        self
    }

    /// Replaces the text shaper.
    pub fn with_text_shaper_impl(mut self, value: Rc<dyn ferroui_base::platform::ITextShaperImpl>) -> Self {
        self.text_shaper_impl = Some(value);
        self
    }

    /// Replaces the platform settings.
    pub fn with_platform_settings(mut self, value: Rc<dyn IPlatformSettings>) -> Self {
        self.platform_settings = Some(value);
        self
    }

    /// Replaces the cursor factory.
    pub fn with_standard_cursor_factory(mut self, value: Rc<dyn ICursorFactory>) -> Self {
        self.standard_cursor_factory = Some(value);
        self
    }

    /// Replaces the factory of the theme.
    pub fn with_theme(mut self, value: impl Fn() -> Rc<dyn IStyle> + 'static) -> Self {
        self.theme = Some(Rc::new(value));
        self
    }

    /// Replaces the window implementation.
    pub fn with_window_impl(mut self, value: Rc<dyn IWindowImpl>) -> Self {
        self.window_impl = Some(value);
        self
    }

    /// Replaces the runtime platform.
    pub fn with_platform(mut self, value: Rc<dyn ferroui_base::platform::IRuntimePlatform>) -> Self {
        self.platform = Some(value);
        self
    }

    /// Replaces the windowing platform.
    pub fn with_windowing_platform(mut self, value: Rc<dyn IWindowingPlatform>) -> Self {
        self.windowing_platform = Some(value);
        self
    }

    /// Replaces the icon loader.
    pub fn with_icon_loader(mut self, value: Rc<dyn IPlatformIconLoader>) -> Self {
        self.icon_loader = Some(value);
        self
    }
}

/// A runtime platform that reports the default platform information (the
/// counterpart of a mocked `IRuntimePlatform`).
pub struct MockRuntimePlatform;

impl ferroui_base::platform::IRuntimePlatform for MockRuntimePlatform {
    fn get_runtime_info(&self) -> ferroui_base::platform::RuntimePlatformInfo {
        ferroui_base::platform::RuntimePlatformInfo::default()
    }
}
