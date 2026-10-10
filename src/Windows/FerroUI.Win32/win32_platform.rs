//! The Windows windowing platform: creates the message window of the UI
//! thread, registers the platform services and creates the top-levels.

use crate::clipboard_impl::ClipboardImpl;
use crate::cursor_factory::CursorFactory;
use crate::embedded_window_impl::EmbeddedWindowImpl;
use crate::icon_impl::IconImpl;
use crate::input::WindowsKeyboardDevice;
use crate::ole_context::OleContext;
use crate::interop::unmanaged_methods::*;
use crate::platform_constants::Version;
use crate::screen_impl::ScreenImpl;
use crate::win32_dispatcher_impl::{Win32DispatcherImpl, SIGNAL_L, SIGNAL_W};
use crate::win32_gl_manager::Win32GlManager;
use crate::win32_platform_options::{Win32DpiAwareness, Win32PlatformOptions};
use crate::win32_platform_settings::{setting_change, SettingChange, Win32PlatformSettings};
use crate::win_screen::WinScreen;
use crate::window_impl::WindowImpl;
use crate::wnd_proc_guard;
use ferroui_base::input::platform::{Clipboard, IClipboard, IClipboardImpl, KeyGestureFormatInfo, PlatformHotkeyConfiguration};
use ferroui_base::input::{IKeyboardDevice, Key, KeyGesture, KeyModifiers};
use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::platform::{ICursorFactory, IPlatformGraphics, IPlatformSettings, SharedBitmapImpl};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop, SleepLoopRenderTimer, UiThreadRenderTimer};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::application_lifetimes::ShutdownRequestedEventArgs;
use ferroui_controls::platform::{
    IPlatformIconLoader, IPlatformLifetimeEventsImpl, IScreenImpl, ITopLevelImpl, ITrayIconImpl, IWindowIconImpl,
    IWindowImpl, IWindowingPlatform, ScreensBaseImplExt,
};
use ferroui_controls::AppBuilder;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io;
use std::rc::{Rc, Weak};
use std::sync::{Arc, OnceLock};

/// The identifier of the timer of the dispatcher on the message window.
pub(crate) const TIMERID_DISPATCHER: usize = 1;
const DEFAULT_FRAMES_PER_SECOND: i32 = 60;

/// `SPI_SETWORKAREA`: the work area of a monitor changed.
const SPI_SETWORKAREA: usize = 0x002F;

thread_local! {
    static INSTANCE: RefCell<Option<Rc<Win32Platform>>> = const { RefCell::new(None) };
    static OPTIONS: RefCell<Option<Win32PlatformOptions>> = const { RefCell::new(None) };
    static COMPOSITOR: RefCell<Option<Rc<Compositor>>> = const { RefCell::new(None) };
}

/// Selects the Windows backend for an application.
pub trait Win32ApplicationExtensions {
    /// Uses the standard runtime platform and the Windows windowing
    /// subsystem. The backend is initialized with the
    /// [`Win32PlatformOptions`] registered with the builder, or the default
    /// options.
    fn use_win32(&self) -> AppBuilder;
}

impl Win32ApplicationExtensions for AppBuilder {
    fn use_win32(&self) -> AppBuilder {
        self.use_standard_runtime_platform_subsystem().use_windowing_subsystem(
            || {
                let options = FerroLocator::current().get_service::<Win32PlatformOptions>();
                Win32Platform::initialize(options.as_deref().cloned().unwrap_or_default());
            },
            "Win32",
        )
    }
}

/// The Windows windowing platform.
pub struct Win32Platform {
    weak_self: Weak<Win32Platform>,
    hwnd: Cell<isize>,
    dispatcher: Rc<Win32DispatcherImpl>,
    screen: RefCell<Option<Rc<ScreenImpl>>>,
    /// The platform settings the platform registered, which it tells about
    /// the changes of the settings of the system. The reference asks the
    /// services for the settings and tells them when they are of this
    /// type.
    win32_platform_settings: RefCell<Option<Rc<Win32PlatformSettings>>>,
    /// The render timer, when it is the timer of the sleep loop, whose rate
    /// follows the displays.
    sleep_loop_render_timer: RefCell<Option<Arc<SleepLoopRenderTimer>>>,
    shutdown_requested: HandlerList<dyn Fn(&ShutdownRequestedEventArgs)>,
}

impl Win32Platform {
    fn new() -> Rc<Win32Platform> {
        let hwnd = Self::create_message_window();
        Rc::new_cyclic(|weak_self| Win32Platform {
            weak_self: weak_self.clone(),
            hwnd: Cell::new(hwnd),
            dispatcher: Rc::new(Win32DispatcherImpl::new(hwnd)),
            screen: RefCell::new(None),
            win32_platform_settings: RefCell::new(None),
            sleep_loop_render_timer: RefCell::new(None),
            shutdown_requested: HandlerList::new(),
        })
    }

    /// The platform of the calling thread, which becomes the UI thread: the
    /// message window is created on first use.
    pub(crate) fn instance() -> Rc<Win32Platform> {
        if let Some(instance) = INSTANCE.with(|instance| instance.borrow().clone()) {
            return instance;
        }
        let instance = Win32Platform::new();
        INSTANCE.with(|slot| *slot.borrow_mut() = Some(instance.clone()));
        instance
    }

    pub(crate) fn try_instance() -> Option<Rc<Win32Platform>> {
        INSTANCE.try_with(|instance| instance.try_borrow().ok().and_then(|instance| instance.clone())).ok().flatten()
    }

    #[allow(dead_code)]
    pub(crate) fn platform_settings(&self) -> Rc<dyn IPlatformSettings> {
        FerroLocator::current().get_required_service::<dyn IPlatformSettings>()
    }

    /// The screens of the platform.
    ///
    /// # Panics
    /// Panics when the platform has not been initialized.
    pub(crate) fn screen(&self) -> Rc<ScreenImpl> {
        match self.screen.borrow().clone() {
            Some(screen) => screen,
            None => panic!("Win32Platform hasn't been initialized"),
        }
    }

    /// The handle of the message window.
    pub(crate) fn handle(&self) -> isize {
        self.hwnd.get()
    }

    /// The handle of the message window of the platform of the calling
    /// thread: the window the system tells about the changes of its
    /// settings.
    pub fn message_window() -> isize {
        Self::instance().handle()
    }

    /// Gets the actual version of Windows: what `RtlGetVersion` reports.
    pub fn windows_version() -> Version {
        static WINDOWS_VERSION: OnceLock<Version> = OnceLock::new();
        *WINDOWS_VERSION.get_or_init(|| {
            let (major, minor, build) = rtl_get_version();
            Version::with_build(major, minor, build)
        })
    }

    pub(crate) fn use_overlay_popups() -> bool {
        Self::options().overlay_popups
    }

    /// The options the platform was initialized with.
    ///
    /// # Panics
    /// Panics when the platform has not been initialized.
    pub fn options() -> Win32PlatformOptions {
        match OPTIONS.with(|options| options.borrow().clone()) {
            Some(options) => options,
            None => panic!("Win32Platform hasn't been initialized"),
        }
    }

    /// The compositor of the platform.
    ///
    /// # Panics
    /// Panics when the platform has not been initialized.
    pub(crate) fn compositor() -> Rc<Compositor> {
        match COMPOSITOR.with(|compositor| compositor.borrow().clone()) {
            Some(compositor) => compositor,
            None => panic!("Win32Platform hasn't been initialized"),
        }
    }

    /// Initializes the platform with the default options.
    pub fn initialize_default() {
        Self::initialize(Win32PlatformOptions::default());
    }

    /// Initializes the platform: the calling thread becomes the UI thread,
    /// and the platform services are registered in the service locator.
    ///
    /// # Panics
    /// Panics when a custom graphics context is set with a composition
    /// mode that excludes the redirection surface.
    pub fn initialize(options: Win32PlatformOptions) {
        OPTIONS.with(|slot| *slot.borrow_mut() = Some(options.clone()));

        Self::set_dpi_awareness();

        let instance = Self::instance();
        let dispatcher: Rc<Win32DispatcherImpl> = instance.dispatcher.clone();
        Dispatcher::initialize_ui_thread_dispatcher(dispatcher);

        let render_timer: Arc<dyn IRenderTimer> = if options.should_render_on_ui_thread {
            Arc::new(UiThreadRenderTimer::new(DEFAULT_FRAMES_PER_SECOND))
        } else {
            let timer = Arc::new(SleepLoopRenderTimer::new(DEFAULT_FRAMES_PER_SECOND));
            *instance.sleep_loop_render_timer.borrow_mut() = Some(timer.clone());
            timer
        };
        let clipboard_impl: Rc<dyn IClipboardImpl> = Rc::new(ClipboardImpl::new());
        let clipboard: Rc<dyn IClipboard> = Clipboard::new(clipboard_impl.clone());

        let screens = ScreenImpl::new();
        *instance.screen.borrow_mut() = Some(screens.clone());
        let screens: Rc<dyn IScreenImpl> = screens;

        let cursor_factory: Rc<dyn ICursorFactory> = CursorFactory::instance();
        let keyboard_device: Rc<dyn IKeyboardDevice> = WindowsKeyboardDevice::instance();
        let win32_platform_settings = Win32PlatformSettings::new();
        *instance.win32_platform_settings.borrow_mut() = Some(win32_platform_settings.clone());
        let platform_settings: Rc<dyn IPlatformSettings> = win32_platform_settings;
        let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(render_timer);
        let windowing_platform: Rc<dyn IWindowingPlatform> = instance.clone();
        let icon_loader: Rc<dyn IPlatformIconLoader> = instance.clone();
        let lifetime_events: Rc<dyn IPlatformLifetimeEventsImpl> = instance.clone();

        let mut hotkeys = PlatformHotkeyConfiguration::new(KeyModifiers::CONTROL);
        // Add Shift+F10
        hotkeys.open_context_menu.push(KeyGesture::new(Key::F10, KeyModifiers::SHIFT));

        let locator = FerroLocator::current_mutable();
        locator
            .bind::<dyn IClipboardImpl>()
            .to_constant(clipboard_impl)
            .bind::<dyn IClipboard>()
            .to_constant(clipboard)
            .bind::<dyn ICursorFactory>()
            .to_constant(cursor_factory)
            .bind::<dyn IKeyboardDevice>()
            .to_constant(keyboard_device)
            .bind::<dyn IPlatformSettings>()
            .to_constant(platform_settings)
            .bind::<dyn IScreenImpl>()
            .to_constant(screens)
            .bind::<Arc<dyn IRenderLoop>>()
            .to_constant(Rc::new(render_loop))
            .bind::<dyn IWindowingPlatform>()
            .to_constant(windowing_platform)
            .bind_to_self(Rc::new(hotkeys))
            .bind_to_self(Rc::new(KeyGestureFormatInfo::new(Some(HashMap::new()), "Win", "Ctrl", "Alt", "Shift")))
            .bind::<dyn IPlatformIconLoader>()
            .to_constant(icon_loader)
            .bind::<dyn IPlatformLifetimeEventsImpl>()
            .to_constant(lifetime_events);
        // The helper of the non-pumping wait and the provider of the
        // mounted volumes are bound here by the reference: the base
        // library has no contract for the first, and the second arrives
        // with the storage provider of stage 2.

        let platform_graphics: Option<Arc<dyn IPlatformGraphics>> = match &options.custom_platform_graphics {
            Some(custom_platform_graphics) => {
                if !options.custom_platform_graphics_is_compatible() {
                    panic!(
                        "Win32PlatformOptions.custom_platform_graphics is only compatible with \
                         Win32CompositionMode::RedirectionSurface"
                    );
                }

                Some(custom_platform_graphics.clone())
            }
            // The manager registers what it chooses with the services.
            None => Win32GlManager::initialize(&options),
        };

        if let Some(custom_platform_graphics) = &options.custom_platform_graphics {
            locator.bind::<Arc<dyn IPlatformGraphics>>().to_constant(Rc::new(custom_platform_graphics.clone()));
        }

        if OleContext::current().is_some() {
            let drag_source: Rc<dyn ferroui_base::input::platform::IPlatformDragSource> =
                Rc::new(crate::drag_source::DragSource);
            locator.bind::<dyn ferroui_base::input::platform::IPlatformDragSource>().to_constant(drag_source);
        }

        Self::update_timer_fps();

        let compositor = Compositor::new(platform_graphics, false);
        COMPOSITOR.with(|slot| *slot.borrow_mut() = Some(compositor.clone()));
        locator.bind_to_self(compositor);
    }

    /// The window procedure of the message window.
    fn wnd_proc(&self, hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> isize {
        if msg == WindowsMessage::WM_DISPATCH_WORK_ITEM && w_param == SIGNAL_W && l_param == SIGNAL_L {
            self.dispatcher.dispatch_work_item();
        }

        if msg == WindowsMessage::WM_QUERYENDSESSION && !self.shutdown_requested.is_empty() {
            // https://learn.microsoft.com/en-us/windows/win32/shutdown/wm-queryendsession
            // > LPARAM lParam   // logoff option
            // >
            // > This parameter can be one or more of the following values. If this parameter is 0, the system is shutting down or restarting (it is not possible to determine which event is occurring).
            // >
            // > - ENDSESSION_CLOSEAPP 0x00000001 The application is using a file that must be replaced, the system is being serviced, or system resources are exhausted. For more information, see Guidelines for Applications.
            // > - ENDSESSION_CRITICAL 0x40000000 The application is forced to shut down.
            // > - ENDSESSION_LOGOFF 0x80000000 The user is logging off.
            let e = ShutdownRequestedEventArgs::with_is_os_shutdown(l_param == 0);

            let handlers = self.shutdown_requested.snapshot();
            for (_, handler) in handlers.iter() {
                handler(&e);
            }

            if e.cancel() {
                return 0;
            }
        }

        if msg == WindowsMessage::WM_SETTINGCHANGE {
            let win32_platform_settings = self.win32_platform_settings.borrow().clone();
            if let Some(win32_platform_settings) = win32_platform_settings {
                // SAFETY: the second parameter of this message names the
                // setting that changed, or is null.
                let changed_setting = unsafe { read_setting_name(l_param) };
                match setting_change(changed_setting.as_deref()) {
                    Some(SettingChange::ColorValues) => win32_platform_settings.on_color_values_changed(),
                    Some(SettingChange::Language) => win32_platform_settings.on_language_changed(),
                    None => {}
                }
            }

            // Notify WorkingArea changed to Screens
            if w_param == SPI_SETWORKAREA {
                if let Some(screen) = self.screen.borrow().clone() {
                    screen.on_changed();
                }
            }
        }

        if msg == WindowsMessage::WM_TIMER && w_param == TIMERID_DISPATCHER {
            self.dispatcher.fire_timer();
        }

        crate::tray_icon_impl::TrayIconImpl::proc_wnd(hwnd, msg, w_param, l_param);

        def_window_proc(hwnd, msg, w_param, l_param)
    }

    /// Sets the rate of the render timer of the sleep loop to the highest
    /// refresh rate of the screens, and at least 60 frames a second.
    pub(crate) fn update_timer_fps() {
        let Some(instance) = Self::try_instance() else {
            return;
        };
        let Some(screen) = instance.screen.borrow().clone() else {
            return;
        };
        let max_display_frequency =
            screen.all_platform_screens().iter().map(|screen: &Rc<WinScreen>| screen.frequency()).max().unwrap_or(0).max(60);
        let timer = instance.sleep_loop_render_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.set_desired_fps(max_display_frequency);
        }
    }

    fn create_message_window() -> isize {
        let class_name = format!("FerroMessageWindow {}-{:?}", std::process::id(), std::thread::current().id());

        let atom = register_class_ex(&class_name, 0, message_wnd_proc, 0);

        if atom == 0 {
            panic!("The class of the message window could not be registered (error code {})", get_last_error());
        }

        let hwnd = create_window_ex(0, atom, 0, 0, 0, 0, 0, 0);
        wnd_proc_guard::resume_pending();

        if hwnd == 0 {
            panic!("The message window could not be created (error code {})", get_last_error());
        }

        crate::tray_icon_impl::TrayIconImpl::change_window_message_filter(hwnd);

        hwnd
    }

    fn set_dpi_awareness() {
        // Ideally we'd set DPI awareness in the manifest but this doesn't work for
        // apps that are run by a console loader. Instead we have to do it in code,
        // but there are various ways to do this depending on the OS version.
        let dpi_awareness = Self::options().dpi_awareness;

        // `None` from a setter: this version of the system does not have
        // the function, and the next way is tried.
        let done = match dpi_awareness {
            Win32DpiAwareness::Unaware => set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_UNAWARE),
            Win32DpiAwareness::SystemDpiAware => set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE),
            Win32DpiAwareness::PerMonitorDpiAware => {
                match set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) {
                    Some(false) => set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE),
                    other => other,
                }
            }
        };
        if done == Some(true) {
            return;
        }

        let awareness = match dpi_awareness {
            Win32DpiAwareness::Unaware => PROCESS_DPI_AWARENESS::PROCESS_DPI_UNAWARE,
            Win32DpiAwareness::SystemDpiAware => PROCESS_DPI_AWARENESS::PROCESS_SYSTEM_DPI_AWARE,
            Win32DpiAwareness::PerMonitorDpiAware => PROCESS_DPI_AWARENESS::PROCESS_PER_MONITOR_DPI_AWARE,
        };

        if set_process_dpi_awareness(awareness).is_some() {
            return;
        }

        if dpi_awareness != Win32DpiAwareness::Unaware {
            set_process_dpi_aware();
        }
    }
}

unsafe extern "system" fn message_wnd_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    let hwnd = hwnd_to_isize(hwnd);
    let handled = wnd_proc_guard::guard(None, || {
        // While the platform is being created (the first messages of the
        // message window) there is no instance yet.
        Win32Platform::try_instance().map(|platform| platform.wnd_proc(hwnd, msg, w_param, l_param))
    });

    match handled {
        Some(result) => result,
        None => def_window_proc(hwnd, msg, w_param, l_param),
    }
}

impl IPlatformLifetimeEventsImpl for Win32Platform {
    fn shutdown_requested(&self, handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.shutdown_requested.add(handler);
        let this = self.weak_self.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.shutdown_requested.remove(token);
            }
        })
    }
}

impl IWindowingPlatform for Win32Platform {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        WindowImpl::new()
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        self.create_embeddable_window()
    }

    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        use ferroui_controls::platform::IWindowBaseImpl;

        let embedded = EmbeddedWindowImpl::new();
        embedded.show(false, false);
        embedded
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        Some(crate::tray_icon_impl::TrayIconImpl::new())
    }

    fn get_windows_z_order(&self, windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]) {
        let mut handles_to_index: HashMap<isize, usize> = HashMap::with_capacity(windows.len());
        let mut output_array = vec![0i64; windows.len()];

        for (i, window) in windows.iter().enumerate() {
            if let Some(platform_impl) = window.as_any().downcast_ref::<WindowImpl>() {
                handles_to_index.insert(platform_impl.hwnd(), i);
            }
        }

        let mut next_z_order = 0i64;
        enum_windows(&mut |hwnd| {
            if let Some(&index) = handles_to_index.get(&hwnd) {
                // We negate the z-order so that the topmost window has the highest number.
                output_array[index] = -next_z_order;
                next_z_order += 1;
            }
            (next_z_order as usize) < windows.len()
        });

        for (target, value) in z_order.iter_mut().zip(output_array) {
            *target = value;
        }
    }
}

impl IPlatformIconLoader for Win32Platform {
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        let mut stream = std::fs::File::open(file_name)?;
        Ok(IconImpl::new(&mut stream)?)
    }

    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(IconImpl::new(stream)?)
    }

    /// # Panics
    /// Panics when the bitmap cannot be encoded or the icon cannot be made
    /// from it.
    fn load_icon_from_bitmap(&self, bitmap: Arc<SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        let mut memory_stream = Vec::new();
        let icon = bitmap
            .save(&mut memory_stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
            .and_then(|()| IconImpl::new(&mut io::Cursor::new(memory_stream)));
        match icon {
            Ok(icon) => icon,
            Err(error) => panic!("The icon could not be made from the bitmap: {error}"),
        }
    }
}
