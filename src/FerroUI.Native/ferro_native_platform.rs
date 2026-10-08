//! The macOS windowing platform: creates the native factory, registers the
//! platform services and creates the top-levels.

use crate::clipboard_impl::ClipboardImpl;
use crate::cursor::CursorFactory;
use crate::dispatcher_impl::DispatcherImpl;
use crate::embeddable_top_level_impl::EmbeddableTopLevelImpl;
use crate::extensions::as_com_bool;
use crate::ferro_native_application_platform::FerroNativeApplicationPlatform;
use crate::ferro_native_menu_exporter::FerroNativeMenuExporter;
use crate::ferro_native_drag_source::{free_data_transfer_handle, FerroNativeDragSource};
use crate::ferro_native_platform_extensions::{
    FerroNativePlatformOptions, FerroNativeRenderingMode, MacOSPlatformOptions,
};
use crate::ferro_native_render_timer::FerroNativeRenderTimer;
use crate::frn_dispatcher::FrnDispatcher;
use crate::frn_string::to_c_string;
use crate::helpers::ComResultExt;
use crate::icon_loader::IconLoader;
use crate::interop::*;
use crate::mac_os_activatable_lifetime::MacOSActivatableLifetime;
use crate::mac_os_mounted_volume_info_provider::MacOSMountedVolumeInfoProvider;
use crate::mac_os_native_menu_commands::MacOSNativeMenuCommands;
use crate::metal::MetalPlatformGraphics;
use crate::native_platform_settings::NativePlatformSettings;
use crate::screen_impl::ScreenImpl;
use crate::storage_provider_api::StorageProviderApi;
use crate::tray_icon_impl::TrayIconImpl;
use crate::window_impl::WindowImpl;
use ferroui_base::input::platform::{
    Clipboard, IClipboard, IClipboardImpl, IPlatformDragSource, KeyGestureFormatInfo, PlatformHotkeyConfiguration,
};
use ferroui_base::input::{IKeyboardDevice, Key, KeyGesture, KeyModifiers, KeyboardDevice};
use ferroui_base::platform::{ICursorFactory, IPlatformGraphics, IPlatformGraphicsContext, IPlatformSettings};
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::{IRenderLoop, RenderLoop, ThreadProxyRenderTimer};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::application_lifetimes::IActivatableLifetime;
use ferroui_controls::Application;
use ferroui_controls::platform::{
    IMountedVolumeInfoProvider, INativeApplicationCommands, IPlatformIconLoader, IPlatformLifetimeEventsImpl,
    IScreenImpl, IStorageProviderFactory, ITopLevelImpl, ITrayIconImpl, IWindowImpl, IWindowingPlatform,
};
use ferroui_microcom::ComPtr;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CString};
use std::rc::Rc;
use std::sync::Arc;

thread_local! {
    static KEYBOARD_DEVICE: Rc<KeyboardDevice> = KeyboardDevice::new();
    static COMPOSITOR: RefCell<Option<Rc<Compositor>>> = const { RefCell::new(None) };
}

extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

const RTLD_NOW: c_int = 0x2;

/// Frees the handles native code holds on to: the data transfers of drag
/// operations started by this backend. (Callback objects are reference
/// counted COM objects, never handles.)
struct GCHandleDeallocator;

impl IFrnGCHandleDeallocatorCallbackImpl for GCHandleDeallocator {
    fn free_gc_handle(&self, handle: *mut c_void) {
        // SAFETY: the only handles this backend gives to native code are
        // the data transfer handles of the drag source.
        crate::callback_base::guard((), || unsafe { free_data_transfer_handle(handle) })
    }
}

/// The macOS windowing platform.
pub struct FerroNativePlatform {
    factory: RefCell<Option<ComPtr<IFerroNativeFactory>>>,
    options: RefCell<Option<FerroNativePlatformOptions>>,
    platform_graphics: RefCell<Option<Rc<dyn IPlatformGraphics>>>,
    is_disposed: Cell<bool>,
    /// The exporters of the application menu and of the dock menu.
    application_menu_exporter: RefCell<Option<Rc<FerroNativeMenuExporter>>>,
    application_dock_menu_exporter: RefCell<Option<Rc<FerroNativeMenuExporter>>>,
}

impl FerroNativePlatform {
    /// The keyboard device of the platform.
    pub fn keyboard_device() -> Rc<KeyboardDevice> {
        KEYBOARD_DEVICE.with(Rc::clone)
    }

    /// The compositor of the platform.
    ///
    /// # Panics
    /// Panics when the platform is not initialized on this thread.
    pub fn compositor() -> Rc<Compositor> {
        COMPOSITOR.with(|compositor| compositor.borrow().clone()).expect("the platform is initialized")
    }

    /// Initializes the platform over an existing native factory.
    pub fn initialize_with_factory(
        factory: ComPtr<IFerroNativeFactory>,
        options: FerroNativePlatformOptions,
    ) -> Rc<FerroNativePlatform> {
        FerroLocator::current_mutable().bind_to_self(Rc::new(factory.clone()));

        let result = Rc::new(FerroNativePlatform {
            factory: RefCell::new(Some(factory)),
            options: RefCell::new(None),
            platform_graphics: RefCell::new(None),
            is_disposed: Cell::new(false),
            application_menu_exporter: RefCell::new(None),
            application_dock_menu_exporter: RefCell::new(None),
        });

        result.do_initialize(options);

        result
    }

    /// Initializes the platform: creates the native factory (from the
    /// library named by the options, or the one linked into the
    /// application) and registers the platform services in the service
    /// locator. Must be called on the main thread.
    ///
    /// # Panics
    /// Panics when the native library named by the options cannot be
    /// loaded or has no factory export, and when the factory cannot be
    /// created.
    pub fn initialize(options: FerroNativePlatformOptions) -> Rc<FerroNativePlatform> {
        let factory = match &options.ferro_native_library_path {
            Some(path) => {
                let path = CString::new(path.as_str()).expect("the library path has no NUL");
                // SAFETY: plain dynamic loading; the export, when present,
                // has the signature of the factory entry point.
                unsafe {
                    let lib = dlopen(path.as_ptr(), RTLD_NOW);
                    if lib.is_null() {
                        panic!("Unable to load the native library {path:?}");
                    }
                    let proc = dlsym(lib, c"CreateFerroNative".as_ptr());
                    if proc.is_null() {
                        panic!("Unable to get \"CreateFerroNative\" export from the native library");
                    }
                    let d: unsafe extern "C" fn() -> *mut IFerroNativeFactory = std::mem::transmute(proc);
                    ComPtr::from_raw(d())
                }
            }
            None => create_ferro_native(),
        };

        Self::initialize_with_factory(factory.expect("the native factory"), options)
    }

    #[track_caller]
    fn factory(&self) -> ComPtr<IFerroNativeFactory> {
        match self.factory.borrow().clone() {
            Some(factory) => factory,
            None => panic!("Cannot access a disposed object: the native factory"),
        }
    }

    /// Exports the application menu: the menu attached to the application
    /// or, without one, a default menu; the standard items (Services, Hide,
    /// Quit, ...) are appended unless the options disable them.
    pub fn setup_application_menu_exporter(&self) {
        let exporter = FerroNativeMenuExporter::for_application(&self.factory());
        *self.application_menu_exporter.borrow_mut() = Some(exporter);
    }

    /// Exports the dock menu attached to the application, now and whenever
    /// it changes.
    pub fn setup_application_dock_menu_exporter(&self) {
        let exporter = FerroNativeMenuExporter::for_dock(&self.factory());
        *self.application_dock_menu_exporter.borrow_mut() = Some(exporter);
    }

    /// Sets the application title shown by the system (menu bar, dock) to
    /// the name of the current application, unless it is empty or white
    /// space.
    ///
    /// # Panics
    /// Panics when there is no current application.
    pub fn setup_application_name(&self) {
        let application = Application::current().expect("the current application");
        self.setup_application_name_with(application.name().as_deref());
    }

    /// [`setup_application_name`](Self::setup_application_name) for a host
    /// without an application object.
    pub fn setup_application_name_with(&self, name: Option<&str>) {
        if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
            if let Some(mac_options) = self.factory().get_mac_options() {
                mac_options.set_application_title(Some(&to_c_string(name))).check();
            }
        }
    }

    fn do_initialize(self: &Rc<Self>, options: FerroNativePlatformOptions) {
        *self.options.borrow_mut() = Some(options.clone());
        let factory = self.factory();
        let locator = FerroLocator::current_mutable();

        let application_platform = FerroNativeApplicationPlatform::new(Rc::downgrade(self));

        let mac_opts = FerroLocator::current().get_service::<MacOSPlatformOptions>();
        let mac_opts = mac_opts.as_deref().cloned().unwrap_or_default();

        if let Some(mac_options) = factory.get_mac_options() {
            mac_options.set_disable_app_delegate(as_com_bool(mac_opts.disable_ferro_app_delegate)).check();
        }

        let deallocator = IFrnGCHandleDeallocatorCallback::from_impl(GCHandleDeallocator);
        let application_events = IFrnApplicationEvents::from_impl(application_platform.clone());
        let dispatcher = IFrnDispatcher::from_impl(FrnDispatcher);
        factory.initialize(Some(&deallocator), Some(&application_events), Some(&dispatcher)).check();

        if let Some(mac_options) = factory.get_mac_options() {
            mac_options.set_show_in_dock(as_com_bool(mac_opts.show_in_dock)).check();
            mac_options.set_disable_set_process_name(as_com_bool(mac_opts.disable_set_process_name)).check();
        }

        let clipboard_impl: Rc<dyn IClipboardImpl> =
            Rc::new(ClipboardImpl::new(factory.create_clipboard().check().expect("the native clipboard")));
        let clipboard: Rc<dyn IClipboard> = Clipboard::new(clipboard_impl.clone());

        Dispatcher::initialize_ui_thread_dispatcher(DispatcherImpl::new(
            factory.create_platform_threading_interface().check().expect("the native threading interface"),
        ));

        let cursor_factory: Rc<dyn ICursorFactory> =
            Rc::new(CursorFactory::new(factory.create_cursor_factory().check().expect("the native cursor factory")));
        let screens: Rc<dyn IScreenImpl> = {
            let factory = factory.clone();
            ScreenImpl::new(move |events| factory.create_screens(Some(events)).check())
        };
        let keyboard_device: Rc<dyn IKeyboardDevice> = Self::keyboard_device();
        let platform_settings: Rc<dyn IPlatformSettings> =
            NativePlatformSettings::new(factory.create_platform_settings().check().expect("the native settings"));
        let windowing_platform: Rc<dyn IWindowingPlatform> = self.clone();
        let render_timer = Arc::new(FerroNativeRenderTimer::new(
            factory.create_platform_render_timer().check().expect("the native render timer"),
        ));
        let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(ThreadProxyRenderTimer::new(render_timer));
        let lifetime_events: Rc<dyn IPlatformLifetimeEventsImpl> = application_platform;
        let application_commands: Rc<dyn INativeApplicationCommands> = Rc::new(MacOSNativeMenuCommands::new(
            factory.create_application_commands().check().expect("the native application commands"),
        ));

        let activatable_lifetime = Rc::new(MacOSActivatableLifetime::new());

        locator
            .bind::<dyn ICursorFactory>()
            .to_constant(cursor_factory)
            .bind::<dyn IScreenImpl>()
            .to_constant(screens)
            .bind::<dyn IPlatformIconLoader>()
            .to_singleton::<IconLoader>(|loader| loader)
            .bind::<dyn IKeyboardDevice>()
            .to_constant(keyboard_device)
            .bind::<dyn IPlatformSettings>()
            .to_constant(platform_settings)
            .bind::<dyn IWindowingPlatform>()
            .to_constant(windowing_platform)
            .bind::<dyn IClipboardImpl>()
            .to_constant(clipboard_impl)
            .bind::<dyn IClipboard>()
            .to_constant(clipboard)
            .bind::<Arc<dyn IRenderLoop>>()
            .to_constant(Rc::new(render_loop))
            .bind::<dyn IMountedVolumeInfoProvider>()
            .to_constant(Rc::new(MacOSMountedVolumeInfoProvider))
            .bind::<dyn IPlatformDragSource>()
            .to_constant(Rc::new(FerroNativeDragSource::new()))
            .bind::<dyn IPlatformLifetimeEventsImpl>()
            .to_constant(lifetime_events)
            .bind::<dyn INativeApplicationCommands>()
            .to_constant(application_commands)
            .bind::<dyn IActivatableLifetime>()
            .to_constant(activatable_lifetime.clone())
            // With its concrete type, for the application events of the
            // native side: they only go to the lifetime of this backend.
            .bind_to_self(activatable_lifetime);

        let storage_provider_api = StorageProviderApi::new(
            factory.create_storage_provider().check().expect("the native storage provider"),
            options.app_sandbox_enabled,
        );
        let storage_provider_factory: Rc<dyn IStorageProviderFactory> = storage_provider_api.clone();
        locator
            .bind::<dyn IStorageProviderFactory>()
            .to_constant(storage_provider_factory)
            // With its concrete type, for the receivers of files of this
            // backend (file activation, the clipboard): they ask for the
            // storage API of this backend only.
            .bind_to_self(storage_provider_api);

        let mut hotkeys =
            PlatformHotkeyConfiguration::with_modifiers(KeyModifiers::META, KeyModifiers::SHIFT, KeyModifiers::ALT);
        let command_modifiers = hotkeys.command_modifiers;
        let selection_modifiers = hotkeys.selection_modifiers;
        hotkeys.move_cursor_to_the_start_of_line.push(KeyGesture::new(Key::Left, command_modifiers));
        hotkeys
            .move_cursor_to_the_start_of_line_with_selection
            .push(KeyGesture::new(Key::Left, command_modifiers | selection_modifiers));
        hotkeys.move_cursor_to_the_end_of_line.push(KeyGesture::new(Key::Right, command_modifiers));
        hotkeys
            .move_cursor_to_the_end_of_line_with_selection
            .push(KeyGesture::new(Key::Right, command_modifiers | selection_modifiers));

        locator.bind_to_self(Rc::new(hotkeys));

        locator.bind_to_self(Rc::new(KeyGestureFormatInfo::new(
            Some(HashMap::from([
                (Key::Back, "⌫".to_string()),
                (Key::Down, "↓".to_string()),
                (Key::End, "↘".to_string()),
                (Key::Escape, "⎋".to_string()),
                (Key::Home, "↖".to_string()),
                (Key::Left, "←".to_string()),
                (Key::Return, "↩".to_string()),
                (Key::PageDown, "⇟".to_string()),
                (Key::PageUp, "⇞".to_string()),
                (Key::Right, "→".to_string()),
                (Key::Space, "␣".to_string()),
                (Key::Tab, "⇥".to_string()),
                (Key::Up, "↑".to_string()),
            ])),
            "⌘",
            "⌃",
            "⌥",
            "⇧",
        )));

        for mode in &options.rendering_mode {
            match mode {
                FerroNativeRenderingMode::OpenGl => {
                    // OpenGL platform graphics are not ported yet: the mode
                    // is skipped like a mode that failed to initialize.
                }
                FerroNativeRenderingMode::Metal => {
                    let Ok(metal) = MetalPlatformGraphics::new(&factory) else {
                        continue;
                    };
                    let Ok(context) = metal.try_create_context() else {
                        continue;
                    };
                    context.dispose();
                    *self.platform_graphics.borrow_mut() = Some(Rc::new(metal));
                    break;
                }
                FerroNativeRenderingMode::Software => break,
            }
        }

        let platform_graphics = self.platform_graphics.borrow().clone();
        if let Some(platform_graphics) = &platform_graphics {
            locator.bind::<dyn IPlatformGraphics>().to_constant(platform_graphics.clone());
        }

        let compositor = Compositor::new(platform_graphics, true);
        COMPOSITOR.with(|slot| *slot.borrow_mut() = Some(compositor.clone()));
        locator.bind_to_self(compositor);
    }

    /// Releases the native factory. A host calls this before the process
    /// exits; the native side calls it when the system terminates the
    /// application.
    pub fn dispose(&self) {
        if self.is_disposed.replace(true) {
            return;
        }

        let exporter = self.application_menu_exporter.borrow_mut().take();
        drop(exporter);
        let exporter = self.application_dock_menu_exporter.borrow_mut().take();
        drop(exporter);
        let factory = self.factory.borrow_mut().take();
        drop(factory);
    }

    fn options(&self) -> FerroNativePlatformOptions {
        self.options.borrow().clone().unwrap_or_default()
    }
}

impl IWindowingPlatform for FerroNativePlatform {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        WindowImpl::new(self.factory(), self.options())
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        EmbeddableTopLevelImpl::new(self.factory())
    }

    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("The method or operation is not implemented.");
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        Some(Rc::new(TrayIconImpl::new(&self.factory())))
    }

    fn get_windows_z_order(&self, windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]) {
        for (window, z_order) in windows.iter().zip(z_order.iter_mut()) {
            *z_order = window
                .as_any()
                .downcast_ref::<WindowImpl>()
                .and_then(WindowImpl::z_order)
                .map_or(0, |z_order| z_order as i64);
        }
    }
}
