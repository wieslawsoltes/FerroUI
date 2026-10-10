//! The X11 windowing platform (the port of `X11Platform.cs`): opens the
//! connections, registers the platform services and creates the windows.

use crate::dispatching::{GlibDispatcherImpl, IX11PlatformDispatcher, X11PlatformThreading};
use crate::glx::GlxPlatformGraphics;
use crate::raw_event_grouping::ManualRawEventGrouperDispatchQueue;
use crate::screens::X11Screens;
use crate::selections::clipboard::X11ClipboardImpl;
use crate::x11_active_window_tracker::X11ActiveWindowTracker;
use crate::selections::drag_drop::X11DragSource;
use crate::x11_cursor_factory::X11CursorFactory;
use ferroui_base::input::platform::IPlatformDragSource;
use crate::x11_egl_helper::X11EglPlatformGraphics;
use crate::x11_exception::X11Exception;
use crate::x11_globals::X11Globals;
use crate::x11_icon_loader::X11IconLoader;
use crate::x11_info::X11Info;
use crate::x11_window::X11Window;
use crate::x11_window_info::X11WindowInfo;
use crate::x_error::XError;
use crate::x_resources::XResources;
use crate::xi2_manager::XI2Manager;
use crate::xlib::{self, XDisplay, XID};
use ferroui_base::input::platform::{
    Clipboard, IClipboard, IClipboardImpl, IPlatformClipboardManagerImpl, KeyGestureFormatInfo,
    PlatformClipboardManager, PlatformHotkeyConfiguration,
};
use ferroui_base::input::{IKeyboardDevice, KeyModifiers, KeyboardDevice};
use ferroui_base::platform::{ICursorFactory, IPlatformGraphics, IPlatformSettings};
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop, SleepLoopRenderTimer, UiThreadRenderTimer};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::platform::IMountedVolumeInfoProvider;
use ferroui_freedesktop::dbus_ime::X11DBusImeHelper;
use crate::x11_icon_loader::X11IconData;
use crate::x_embed_tray_icon_impl::XEmbedTrayIconImpl;
use ferroui_controls::platform::IWindowIconImpl;
use ferroui_freedesktop::{DBusPlatformSettings, DBusTrayIconImpl, LinuxMountedVolumeInfoProvider};
use ferroui_controls::platform::{
    IPlatformIconLoader, IScreenImpl, ITopLevelImpl, ITrayIconImpl, IWindowImpl, IWindowingPlatform,
};
use ferroui_controls::AppBuilder;
use ferroui_opengl::{GlProfileType, GlVersion};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// The environment variable that chooses the input method module of an
/// application of this framework, before the ones of the other toolkits.
pub const IM_MODULE_VARIABLE: &str = ferroui_freedesktop::dbus_ime::IM_MODULE_VARIABLE;
/// The environment variable that turns the session management off (`0`).
pub const USE_SESSION_MANAGEMENT_VARIABLE: &str = "FERROUI_X11_USE_SESSION_MANAGEMENT";

/// The X11 windowing platform.
pub struct FerroX11Platform {
    this: Weak<FerroX11Platform>,
    keyboard_device: OnceCell<Rc<KeyboardDevice>>,
    windows: RefCell<HashMap<XID, X11WindowInfo>>,
    xi2: RefCell<Option<Rc<XI2Manager>>>,
    info: OnceCell<Rc<X11Info>>,
    x11_screens: OnceCell<Rc<X11Screens>>,
    compositor: OnceCell<Rc<Compositor>>,
    options: OnceCell<Rc<X11PlatformOptions>>,
    orphaned_window: Cell<XID>,
    globals: OnceCell<Rc<X11Globals>>,
    active_window_tracker: OnceCell<Rc<X11ActiveWindowTracker>>,
    resources: OnceCell<Rc<XResources>>,
    event_grouper_dispatch_queue: Rc<ManualRawEventGrouperDispatchQueue>,
    dispatcher_impl: OnceCell<Rc<dyn IX11PlatformDispatcher>>,
    deferred_display: Cell<Option<XDisplay>>,
    display: Cell<Option<XDisplay>>,
    glx_graphics: OnceCell<Arc<GlxPlatformGraphics>>,
    egl_graphics: OnceCell<Arc<X11EglPlatformGraphics>>,
    cursor_factory: OnceCell<Rc<X11CursorFactory>>,
}

fn initialized<T>(cell: &OnceCell<T>) -> &T {
    cell.get().expect("the X11 platform is initialized")
}

impl FerroX11Platform {
    pub const DEFAULT_FPS: i32 = 60;

    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            keyboard_device: OnceCell::new(),
            windows: RefCell::new(HashMap::new()),
            xi2: RefCell::new(None),
            info: OnceCell::new(),
            x11_screens: OnceCell::new(),
            compositor: OnceCell::new(),
            options: OnceCell::new(),
            orphaned_window: Cell::new(0),
            globals: OnceCell::new(),
            active_window_tracker: OnceCell::new(),
            resources: OnceCell::new(),
            event_grouper_dispatch_queue: Rc::new(ManualRawEventGrouperDispatchQueue::new()),
            dispatcher_impl: OnceCell::new(),
            deferred_display: Cell::new(None),
            display: Cell::new(None),
            glx_graphics: OnceCell::new(),
            egl_graphics: OnceCell::new(),
            cursor_factory: OnceCell::new(),
        })
    }

    /// The keyboard device of the platform, created when first asked for.
    pub fn keyboard_device(&self) -> Rc<KeyboardDevice> {
        self.keyboard_device.get_or_init(KeyboardDevice::new).clone()
    }

    /// The handler of the events of a window of the connection
    /// (`Windows[xid]`).
    pub fn get_window(&self, xid: XID) -> Option<X11WindowInfo> {
        self.windows.borrow().get(&xid).cloned()
    }

    /// Registers the handler of the events of a window
    /// (`Windows[xid] = info`).
    pub fn set_window(&self, xid: XID, info: X11WindowInfo) {
        let previous = self.windows.borrow_mut().insert(xid, info);
        drop(previous);
    }

    /// Forgets a window (`Windows.Remove(xid)`).
    pub fn remove_window(&self, xid: XID) {
        let previous = self.windows.borrow_mut().remove(&xid);
        drop(previous);
    }

    /// The pointer input of the X Input extension, when the server has it.
    pub fn xi2(&self) -> Option<Rc<XI2Manager>> {
        self.xi2.borrow().clone()
    }

    pub fn info(&self) -> &Rc<X11Info> {
        initialized(&self.info)
    }

    pub fn x11_screens(&self) -> &Rc<X11Screens> {
        initialized(&self.x11_screens)
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        initialized(&self.compositor)
    }

    pub fn screens(&self) -> Rc<dyn IScreenImpl> {
        self.x11_screens().clone()
    }

    pub fn options(&self) -> &Rc<X11PlatformOptions> {
        initialized(&self.options)
    }

    /// A window that is never mapped, the parent of windows that have
    /// none for the moment.
    pub fn orphaned_window(&self) -> XID {
        self.orphaned_window.get()
    }

    pub fn globals(&self) -> &Rc<X11Globals> {
        initialized(&self.globals)
    }

    pub fn active_window_tracker(&self) -> &Rc<X11ActiveWindowTracker> {
        initialized(&self.active_window_tracker)
    }

    pub fn resources(&self) -> &Rc<XResources> {
        initialized(&self.resources)
    }

    pub fn event_grouper_dispatch_queue(&self) -> &Rc<ManualRawEventGrouperDispatchQueue> {
        &self.event_grouper_dispatch_queue
    }

    pub fn dispatcher_impl(&self) -> Rc<dyn IX11PlatformDispatcher> {
        initialized(&self.dispatcher_impl).clone()
    }

    /// The connection the thread that renders draws through.
    pub fn deferred_display(&self) -> XDisplay {
        self.deferred_display.get().expect("the X11 platform is initialized")
    }

    /// The connection of the UI thread.
    pub fn display(&self) -> XDisplay {
        self.display.get().expect("the X11 platform is initialized")
    }

    /// The platform graphics of GLX, when they are the ones the platform
    /// registered (`glfeature as GlxPlatformGraphics` of the reference).
    /// The cursor factory of the platform, as its own type (the cast of
    /// the registered service in the reference): the drag source takes
    /// the cursors of a drag from it.
    pub fn cursor_factory(&self) -> Option<Rc<X11CursorFactory>> {
        self.cursor_factory.get().cloned()
    }

    pub fn glx_graphics(&self) -> Option<Arc<GlxPlatformGraphics>> {
        self.glx_graphics.get().cloned()
    }

    /// The platform graphics of EGL, when they are the ones the platform
    /// registered (`glfeature as EglPlatformGraphics` of the reference).
    pub fn egl_graphics(&self) -> Option<Arc<X11EglPlatformGraphics>> {
        self.egl_graphics.get().cloned()
    }

    /// Opens the connections and registers the services of the platform.
    ///
    /// # Panics
    /// Panics when the X libraries cannot be loaded, when no server can be
    /// reached (`XOpenDisplay failed`), and when the rendering modes of
    /// the options are empty or none of them applies.
    pub fn initialize(self: &Rc<Self>, options: X11PlatformOptions) {
        let options = Rc::new(options);
        let _ = self.options.set(options.clone());

        let mut use_xim = false;
        if Self::enable_ime(
            &options,
            std::env::var(IM_MODULE_VARIABLE).ok().as_deref(),
            std::env::var("LANG").ok().as_deref(),
        ) {
            // Attempt to configure DBus-based input method and check if we can fall back to XIM
            if !X11DBusImeHelper::detect_and_register()
                && Self::should_use_xim(
                    std::env::var(IM_MODULE_VARIABLE).ok().as_deref(),
                    std::env::var("GTK_IM_MODULE").ok().as_deref(),
                    std::env::var("QT_IM_MODULE").ok().as_deref(),
                    std::env::var("XMODIFIERS").ok().as_deref(),
                )
            {
                use_xim = true;
            }
        }

        xlib::x_init_threads();
        let Some(display) = xlib::x_open_display() else {
            X11Exception::new("XOpenDisplay failed").throw();
        };
        self.display.set(Some(display));
        let Some(deferred_display) = xlib::x_open_display() else {
            X11Exception::new("XOpenDisplay failed").throw();
        };
        self.deferred_display.set(Some(deferred_display));

        self.orphaned_window.set(xlib::x_create_simple_window(
            display,
            xlib::x_default_root_window(display),
            0,
            0,
            1,
            1,
            0,
            0,
            0,
        ));
        XError::init();

        let info = Rc::new(X11Info::new(display, deferred_display, use_xim));
        let _ = self.info.set(info.clone());
        let _ = self.globals.set(X11Globals::new(self));
        let _ = self.active_window_tracker.set(X11ActiveWindowTracker::new(self));
        let _ = self.resources.set(XResources::new(self));

        let loop_timer =
            (!options.should_render_on_ui_thread).then(|| Arc::new(SleepLoopRenderTimer::new(Self::DEFAULT_FPS)));
        let timer: Arc<dyn IRenderTimer> = match &loop_timer {
            Some(loop_timer) => loop_timer.clone(),
            None => Arc::new(UiThreadRenderTimer::new(Self::DEFAULT_FPS)),
        };

        let clipboard_impl = X11ClipboardImpl::new(self, info.atoms().CLIPBOARD);
        let clipboard = Clipboard::new(clipboard_impl.clone());
        let primary_selection = Clipboard::new(X11ClipboardImpl::new(self, info.atoms().PRIMARY));
        let clipboard_manager = Rc::new(PlatformClipboardManager::new(
            Some(clipboard.clone() as Rc<dyn IClipboard>),
            Some(primary_selection as Rc<dyn IClipboard>),
        ));

        let locator = FerroLocator::current_mutable();
        let windowing_platform: Rc<dyn IWindowingPlatform> = self.clone();
        locator.bind_to_self(self.clone()).bind::<dyn IWindowingPlatform>().to_constant(windowing_platform);
        let dispatcher_impl: Rc<dyn IX11PlatformDispatcher> = if options.use_g_lib_main_loop {
            GlibDispatcherImpl::new(self)
        } else {
            X11PlatformThreading::new(self)
        };
        let _ = self.dispatcher_impl.set(dispatcher_impl.clone());
        Dispatcher::initialize_ui_thread_dispatcher(dispatcher_impl);

        let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(timer);
        let weak = self.this.clone();
        let x11_cursor_factory = Rc::new(X11CursorFactory::new(display));
        let _ = self.cursor_factory.set(x11_cursor_factory.clone());
        let cursor_factory: Rc<dyn ICursorFactory> = x11_cursor_factory;
        let drag_source: Rc<dyn IPlatformDragSource> = Rc::new(X11DragSource::new(self));
        let clipboard_impl: Rc<dyn IClipboardImpl> = clipboard_impl;
        let clipboard: Rc<dyn IClipboard> = clipboard;
        let clipboard_manager: Rc<dyn IPlatformClipboardManagerImpl> = clipboard_manager;
        // Stage 2 of docs/porting/x11-platform.md: the settings of the
        // desktop portal (`DBusPlatformSettings`, the colour scheme and
        // the accent colour) are a service of the FreeDesktop crate; until
        // it is built the defaults of the framework answer.
        let platform_settings: Rc<dyn IPlatformSettings> = DBusPlatformSettings::new();
        let mounted_volumes: Rc<dyn IMountedVolumeInfoProvider> = Rc::new(LinuxMountedVolumeInfoProvider::new());
        let icon_loader: Rc<dyn IPlatformIconLoader> = Rc::new(X11IconLoader);
        locator
            .bind::<Arc<dyn IRenderLoop>>()
            .to_constant(Rc::new(render_loop))
            .bind_to_self(Rc::new(PlatformHotkeyConfiguration::new(KeyModifiers::CONTROL)))
            .bind_to_self(Rc::new(KeyGestureFormatInfo::new(Some(HashMap::new()), "Super", "Ctrl", "Alt", "Shift")))
            .bind::<dyn IKeyboardDevice>()
            .to_func(move || weak.upgrade().map(|platform| platform.keyboard_device() as Rc<dyn IKeyboardDevice>))
            .bind::<dyn ICursorFactory>()
            .to_constant(cursor_factory)
            .bind::<dyn IClipboardImpl>()
            .to_constant(clipboard_impl)
            .bind::<dyn IClipboard>()
            .to_constant(clipboard)
            .bind::<dyn IPlatformClipboardManagerImpl>()
            .to_constant(clipboard_manager)
            .bind::<dyn IPlatformSettings>()
            .to_constant(platform_settings)
            .bind::<dyn IPlatformIconLoader>()
            .to_constant(icon_loader)
            .bind::<dyn IPlatformDragSource>()
            .to_constant(drag_source)
            .bind::<dyn IMountedVolumeInfoProvider>()
            .to_constant(mounted_volumes);
        // Not bound yet, each with the stage of docs/porting/x11-platform.md
        // that builds it: the
        // mounted volumes (`LinuxMountedVolumeInfoProvider`, with the
        // FreeDesktop crate) and the lifetime events of the session
        // manager (`X11PlatformLifetimeEvents`, stage 2).

        let x11_screens = X11Screens::new(self);
        let _ = self.x11_screens.set(x11_screens.clone());

        if let Some(loop_timer) = loop_timer {
            let screens = Rc::downgrade(&x11_screens);
            let timer = loop_timer.clone();
            x11_screens.changed_event.subscribe(move |()| {
                if let Some(screens) = screens.upgrade() {
                    timer.set_desired_fps(screens.max_refresh_rate());
                }
            });
            loop_timer.set_desired_fps(x11_screens.max_refresh_rate());
        }

        if info.x_input_version().is_some() {
            *self.xi2.borrow_mut() = XI2Manager::try_create(self);
        }

        let graphics = Self::initialize_graphics(&options, &mut |rendering_mode| match rendering_mode {
            X11RenderingMode::Glx => {
                let glx = Arc::new(GlxPlatformGraphics::try_create(&info, &options.gl_profiles)?);
                let _ = self.glx_graphics.set(glx.clone());
                Some(glx as Arc<dyn IPlatformGraphics>)
            }
            X11RenderingMode::Egl => {
                let egl = Arc::new(X11EglPlatformGraphics::try_create(&info, &options.gl_profiles)?);
                let _ = self.egl_graphics.set(egl.clone());
                Some(egl as Arc<dyn IPlatformGraphics>)
            }
            // Stage 2a of docs/porting/x11-platform.md, its last part: the platform graphics of
            // Vulkan (`VulkanSupport.TryInitialize`) wait for the Vulkan project of the port and
            // the Vulkan GPU of the Skia backend. Until then the mode is passed over like one
            // that failed to initialize.
            X11RenderingMode::Vulkan => None,
            X11RenderingMode::Software => None,
        });
        if let Some(graphics) = &graphics {
            locator.bind::<Arc<dyn IPlatformGraphics>>().to_constant(Rc::new(graphics.clone()));
        }

        let compositor = Compositor::new(graphics, false);
        let _ = self.compositor.set(compositor.clone());
        locator.bind_to_self(compositor);

        // Stage 3 of docs/porting/x11-platform.md: the accessibility
        // bridge (`X11AtSpiAccessibility`) is not built.
    }

    /// `TrackWindow`: tells the accessibility bridge about a window that
    /// is shown. The bridge is stage 3 of docs/porting/x11-platform.md;
    /// until then there is nothing to tell.
    pub(crate) fn track_window(&self, _window: &X11Window) {}

    /// `UntrackWindow`: see [`track_window`](Self::track_window).
    pub(crate) fn untrack_window(&self, _window: &X11Window) {}

    /// Whether input methods are wanted (`EnableIme`).
    pub(crate) fn enable_ime(options: &X11PlatformOptions, im_module: Option<&str>, lang: Option<&str>) -> bool {
        // Disable if explicitly asked by user
        if im_module == Some("none") {
            return false;
        }

        // Use value from options when specified
        if let Some(enable_ime) = options.enable_ime {
            return enable_ime;
        }

        // Automatically enable for CJK locales
        lang.is_some_and(|lang| {
            lang.contains("zh") || lang.contains("ja") || lang.contains("vi") || lang.contains("ko")
        })
    }

    /// Whether the input method of the server is to be used
    /// (`ShouldUseXim`), from the module variables of this framework, of
    /// GTK and of Qt, and `XMODIFIERS`.
    pub(crate) fn should_use_xim(
        im_module: Option<&str>,
        gtk_im_module: Option<&str>,
        qt_im_module: Option<&str>,
        modifiers: Option<&str>,
    ) -> bool {
        // Priority: the module of this framework > GTK_IM_MODULE >= QT_IM_MODULE
        let mut ime_override = im_module.filter(|value| !value.is_empty());
        if ime_override.is_none() {
            ime_override = gtk_im_module.filter(|value| !value.is_empty());
        }
        if ime_override.is_none() {
            ime_override = qt_im_module.filter(|value| !value.is_empty());
        }

        // Check if we are forbidden from using IME
        if ime_override == Some("none") {
            return false;
        }

        // Check if XIM is configured
        if modifiers.is_some_and(|modifiers| modifiers.contains("@im=")) {
            // If XIM is explicitly requested, or no IME override is configured
            if ime_override == Some("xim") || ime_override.is_none() {
                return true;
            }
        }

        false
    }

    /// Walks the rendering modes of the options (`InitializeGraphics`):
    /// software ends the walk without platform graphics, and each other
    /// mode is tried with `try_create`, the first that gives graphics
    /// ending it.
    fn initialize_graphics(
        opts: &X11PlatformOptions,
        try_create: &mut dyn FnMut(X11RenderingMode) -> Option<Arc<dyn IPlatformGraphics>>,
    ) -> Option<Arc<dyn IPlatformGraphics>> {
        if opts.rendering_mode.is_empty() {
            panic!("X11PlatformOptions.RenderingMode must not be empty or null");
        }

        for rendering_mode in &opts.rendering_mode {
            if *rendering_mode == X11RenderingMode::Software {
                return None;
            }

            if let Some(graphics) = try_create(*rendering_mode) {
                return Some(graphics);
            }
        }

        let modes: Vec<String> = opts.rendering_mode.iter().map(|mode| format!("{mode:?}")).collect();
        panic!(
            "X11PlatformOptions.RenderingMode has a value of \"{}\", but no options were applied.",
            modes.join(", ")
        );
    }
}

/// The data of an icon of this platform as the tray icon takes it
/// (`X11IconConverter`): the width, the height and the pixels; empty for
/// no icon and for an icon the platform cannot read.
fn x11_icon_converter(icon: Option<&Rc<dyn IWindowIconImpl>>) -> Vec<u32> {
    let Some(x11icon) = icon.and_then(|icon| X11IconData::from_icon_impl(icon).ok()) else {
        return Vec::new();
    };

    // An item of the property is a C long that holds 32 bits.
    x11icon.data().iter().map(|x| *x as u32).collect()
}

impl IWindowingPlatform for FerroX11Platform {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        let this = self.this.upgrade().expect("the platform is alive");
        X11Window::new(&this, None, false)
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        self.create_embeddable_window()
    }

    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("Specified method is not supported.");
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        let dbus_tray_icon = DBusTrayIconImpl::new();

        if !dbus_tray_icon.is_active() {
            return Some(Rc::new(XEmbedTrayIconImpl::new()));
        }

        dbus_tray_icon.set_icon_converter_delegate(Some(Rc::new(x11_icon_converter)));

        Some(dbus_tray_icon)
    }

    fn get_windows_z_order(&self, windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]) {
        let info = self.info();
        // a mapping of parent windows to their children, sorted by z-order (bottom to top)
        let mut windows_children: HashMap<XID, Vec<XID>> = HashMap::new();

        let mut index_in_windows_span: HashMap<XID, usize> = HashMap::new();
        let handles: Vec<Option<XID>> = windows
            .iter()
            .map(|window| window.as_any().downcast_ref::<X11Window>().map(X11Window::xid))
            .collect();
        for (i, handle) in handles.iter().enumerate() {
            if let Some(handle) = handle {
                index_in_windows_span.insert(*handle, i);
            }
        }

        for handle in handles.iter().flatten() {
            let mut node = *handle;
            while node != 0 {
                if windows_children.contains_key(&node) {
                    break;
                }

                let Some((_, parent, children)) = xlib::x_query_tree(info.display(), node) else {
                    break;
                };

                // As the reference, which records the children of a window
                // only when it has some.
                if !children.is_empty() {
                    windows_children.insert(node, children);
                }

                node = parent;
            }
        }

        assign_z_order(info.root_window(), &windows_children, &index_in_windows_span, z_order);
    }
}

/// Walks the tree of windows from the root, bottom children first, and
/// numbers the windows it meets; the windows of `index_in_windows_span`
/// get their number in `z_order`.
pub(crate) fn assign_z_order(
    root: XID,
    windows_children: &HashMap<XID, Vec<XID>>,
    index_in_windows_span: &HashMap<XID, usize>,
    output_z_order: &mut [i64],
) {
    let mut stack = vec![root];
    let mut z_order = 0;

    while let Some(current_window) = stack.pop() {
        let Some(children) = windows_children.get(&current_window) else {
            continue;
        };

        if let Some(index) = index_in_windows_span.get(&current_window) {
            output_z_order[*index] = z_order;
        }

        z_order += 1;

        // Children are returned bottom to top, so we need to push them in reverse order
        // In order to traverse bottom children first
        for child in children.iter().rev() {
            stack.push(*child);
        }
    }
}

/// Represents the rendering mode for platform graphics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum X11RenderingMode {
    /// The application is rendered into a framebuffer.
    Software = 1,

    /// Enables Glx rendering.
    Glx = 2,

    /// Enables native Linux EGL rendering.
    Egl = 3,

    /// Enables Vulkan rendering
    Vulkan = 4,
}

/// Platform-specific options which apply to Linux.
#[derive(Clone)]
pub struct X11PlatformOptions {
    /// Gets or sets the rendering modes with fallbacks.
    /// The first element in the array has the highest priority.
    /// The default value is: [`X11RenderingMode::Glx`], [`X11RenderingMode::Software`].
    ///
    /// If application should work on as wide range of devices as possible, at least add
    /// [`X11RenderingMode::Software`] as a fallback value.
    ///
    /// The platform fails to initialize when no value is matched.
    pub rendering_mode: Vec<X11RenderingMode>,

    /// Embeds popups to the window when set to true. The default value is false.
    pub overlay_popups: bool,

    /// Enables global menu support on Linux desktop environments where it's supported (e. g. XFCE and MATE with
    /// plugin, KDE, etc). The default value is true.
    pub use_d_bus_menu: bool,

    /// Enables DBus file picker instead of GTK.
    /// The default value is true.
    pub use_d_bus_file_picker: bool,

    /// Determines whether to use IME.
    /// IME would be enabled by default if the current user input language is one of the following: Mandarin,
    /// Japanese, Vietnamese or Korean.
    ///
    /// Input method editor is a component that enables users to generate characters not natively available
    /// on their input devices by using sequences of characters or mouse operations that are natively available on
    /// their input devices.
    pub enable_ime: Option<bool>,

    /// Determines whether to use Input Focus Proxy.
    /// The default value is false.
    pub enable_input_focus_proxy: bool,

    /// Determines whether to enable support for the
    /// X Session Management Protocol.
    ///
    /// X Session Management Protocol is a standard implemented on most
    /// Linux systems that uses Xorg. This enables apps to control how they
    /// can control and/or cancel the pending shutdown requested by the user.
    pub enable_session_management: bool,

    /// Render directly on the UI thread instead of using a dedicated render thread.
    /// This can be usable if your device don't have multiple cores to begin with.
    /// This setting is false by default.
    pub should_render_on_ui_thread: bool,

    pub gl_profiles: Vec<GlVersion>,

    pub glx_renderer_blacklist: Vec<String>,

    pub wm_class: Option<String>,

    /// Enables multitouch support. The default value is true.
    ///
    /// Multitouch allows a surface (a touchpad or touchscreen) to recognize the presence of more than one point
    /// of contact with the surface at the same time.
    pub enable_multi_touch: Option<bool>,

    /// Retain window framebuffer contents if using CPU rendering mode.
    /// This will keep an offscreen bitmap for each window with contents of the previous frame
    /// While improving performance by saving a blit, it will increase memory consumption
    /// if you have many windows
    pub use_retained_framebuffer: Option<bool>,

    /// Enables the MIT-SHM extension for CPU rendering mode, which uses shared memory
    /// to transfer the framebuffer contents to the X server instead of sending pixels
    /// over the connection socket.
    /// Only used when set to true and the extension is supported by the server.
    /// The default value is null.
    pub use_x_shm_framebuffer: Option<bool>,

    /// If this option is set to true, GMainLoop and GSource based dispatcher implementation will be used instead
    /// of epoll-based one.
    /// Use this if you need to use GLib-based libraries on the main thread
    pub use_g_lib_main_loop: bool,

    /// Enables client-side drawn window decorations on X11.
    /// When true and ExtendClientAreaToDecorationsHint is set on a window,
    /// the framework will draw its own decorations (titlebar, borders, resize grips)
    /// instead of using the X11 window manager decorations.
    ///
    /// Experimental, used mostly for testing.
    pub enable_drawn_decorations: Option<bool>,

    /// Forces client-side drawn window decorations on X11 for all windows,
    /// even when the app has not opted in via ExtendClientAreaToDecorationsHint.
    /// In this mode, Window.ClientSize reflects the usable content area
    /// (platform client size minus decoration margins) and the app is unaware
    /// of the decorations.
    /// Implies EnableDrawnDecorations = true.
    ///
    /// Experimental, used mostly for testing.
    pub force_drawn_decorations: bool,

    /// If the framework is in control of a run loop, we propagate exceptions by stopping the run loop frame
    /// and rethrowing an exception. However, if there is no run loop frame controlled by the framework,
    /// there is no way to report such exceptions, since allowing those to escape native->managed call boundary
    /// will likely brick GLib machinery since it's not aware of managed Exceptions
    /// This property allows to inspect such exceptions before they will be ignored
    pub external_g_lib_main_loop_exception_logger: Option<Rc<dyn Fn(&(dyn std::any::Any + Send))>>,
}

impl X11PlatformOptions {
    pub fn new() -> Self {
        Self {
            rendering_mode: vec![X11RenderingMode::Glx, X11RenderingMode::Software],
            overlay_popups: false,
            use_d_bus_menu: true,
            use_d_bus_file_picker: true,
            enable_ime: Some(true),
            enable_input_focus_proxy: false,
            enable_session_management: std::env::var(USE_SESSION_MANAGEMENT_VARIABLE).ok().as_deref() != Some("0"),
            should_render_on_ui_thread: false,
            gl_profiles: vec![
                GlVersion::new(GlProfileType::OpenGL, 4, 0),
                GlVersion::new(GlProfileType::OpenGL, 3, 2),
                GlVersion::new(GlProfileType::OpenGL, 3, 0),
                GlVersion::new(GlProfileType::OpenGLES, 3, 2),
                GlVersion::new(GlProfileType::OpenGLES, 3, 0),
                GlVersion::new(GlProfileType::OpenGLES, 2, 0),
            ],
            glx_renderer_blacklist: vec![
                // llvmpipe is a software GL rasterizer. If it's returned by glGetString,
                // that usually means that something in the system is horribly misconfigured
                // and sometimes attempts to use GLX might cause a segfault
                "llvmpipe".to_string(),
                // SVGA3D is a driver for VMWare virtual GPU
                // There were reports of various glitches like parts of the UI not being rendered
                // Given that VMs are mostly used by testing, we've decided to blacklist that driver
                // for now
                "SVGA3D".to_string(),
            ],
            // The reference takes the name of the entry assembly; the
            // counterpart of a native program is the name of its
            // executable.
            wm_class: std::env::current_exe()
                .ok()
                .and_then(|path| path.file_stem().map(|name| name.to_string_lossy().into_owned())),
            enable_multi_touch: Some(true),
            use_retained_framebuffer: None,
            use_x_shm_framebuffer: None,
            use_g_lib_main_loop: false,
            enable_drawn_decorations: None,
            force_drawn_decorations: false,
            external_g_lib_main_loop_exception_logger: None,
        }
    }

    pub(crate) fn enable_drawn_decorations_internal(&self) -> bool {
        self.enable_drawn_decorations == Some(true) || self.force_drawn_decorations_internal()
    }

    pub(crate) fn force_drawn_decorations_internal(&self) -> bool {
        self.force_drawn_decorations
    }
}

impl Default for X11PlatformOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Selects the X11 platform for an application.
pub trait FerroX11PlatformExtensions {
    /// Uses the X11 windowing platform, with the [`X11PlatformOptions`]
    /// registered with the builder or the default ones.
    fn use_x11(&self) -> AppBuilder;
}

impl FerroX11PlatformExtensions for AppBuilder {
    fn use_x11(&self) -> AppBuilder {
        self.use_standard_runtime_platform_subsystem().use_windowing_subsystem(
            || {
                let options = FerroLocator::current()
                    .get_service::<X11PlatformOptions>()
                    .map(|options| (*options).clone())
                    .unwrap_or_default();
                FerroX11Platform::new().initialize(options);
            },
            "X11",
        )
    }
}

/// Initializes the X11 platform without an application builder.
pub fn initialize_x11_platform(options: Option<X11PlatformOptions>) -> Rc<FerroX11Platform> {
    let platform = FerroX11Platform::new();
    platform.initialize(options.unwrap_or_default());
    platform
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_default_options_are_those_of_the_reference() {
        let options = X11PlatformOptions::new();
        assert_eq!(options.rendering_mode, vec![X11RenderingMode::Glx, X11RenderingMode::Software]);
        assert!(!options.overlay_popups);
        assert!(options.use_d_bus_menu);
        assert!(options.use_d_bus_file_picker);
        assert_eq!(options.enable_ime, Some(true));
        assert!(!options.enable_input_focus_proxy);
        assert!(!options.should_render_on_ui_thread);
        assert_eq!(options.gl_profiles.len(), 6);
        assert_eq!(options.glx_renderer_blacklist, vec!["llvmpipe".to_string(), "SVGA3D".to_string()]);
        assert_eq!(options.enable_multi_touch, Some(true));
        assert_eq!(options.use_retained_framebuffer, None);
        assert_eq!(options.use_x_shm_framebuffer, None);
        assert!(!options.use_g_lib_main_loop);
        assert!(!options.enable_drawn_decorations_internal());
        assert!(options.wm_class.is_some());
    }

    #[test]
    fn forcing_drawn_decorations_enables_them() {
        let mut options = X11PlatformOptions::new();
        options.force_drawn_decorations = true;
        assert!(options.enable_drawn_decorations_internal());
        let mut options = X11PlatformOptions::new();
        options.enable_drawn_decorations = Some(true);
        assert!(options.enable_drawn_decorations_internal());
        assert!(!options.force_drawn_decorations_internal());
    }

    #[test]
    fn input_methods_follow_the_module_variable_the_options_and_the_locale() {
        let mut options = X11PlatformOptions::new();
        assert!(FerroX11Platform::enable_ime(&options, None, None));
        assert!(!FerroX11Platform::enable_ime(&options, Some("none"), Some("ja_JP.UTF-8")));
        options.enable_ime = Some(false);
        assert!(!FerroX11Platform::enable_ime(&options, None, Some("ja_JP.UTF-8")));
        options.enable_ime = None;
        assert!(!FerroX11Platform::enable_ime(&options, None, None));
        assert!(!FerroX11Platform::enable_ime(&options, None, Some("en_US.UTF-8")));
        for lang in ["zh_CN.UTF-8", "ja_JP.UTF-8", "vi_VN", "ko_KR.UTF-8"] {
            assert!(FerroX11Platform::enable_ime(&options, None, Some(lang)), "{lang}");
        }
    }

    #[test]
    fn the_server_input_method_needs_modifiers_and_no_other_module() {
        let xim = FerroX11Platform::should_use_xim;
        assert!(xim(None, None, None, Some("@im=fcitx")));
        assert!(xim(Some("xim"), None, None, Some("@im=fcitx")));
        assert!(xim(Some(""), Some("xim"), None, Some("@im=ibus")));
        assert!(!xim(None, None, None, None));
        assert!(!xim(None, None, None, Some("")));
        assert!(!xim(Some("none"), Some("xim"), None, Some("@im=fcitx")));
        assert!(!xim(None, Some("ibus"), None, Some("@im=ibus")));
        assert!(!xim(None, None, Some("fcitx"), Some("@im=fcitx")));
        // The module of the framework wins over the ones of the toolkits.
        assert!(xim(Some("xim"), Some("ibus"), Some("fcitx"), Some("@im=x")));
    }

    #[test]
    fn windows_are_numbered_from_the_bottom_of_the_tree() {
        // root(1) -> [10, 20]; 10 -> [11]; 20 -> [21, 22]; the leaves have children too,
        // so that they are numbered (a window without children is passed over).
        let mut children = HashMap::new();
        children.insert(1, vec![10, 20]);
        children.insert(10, vec![11]);
        children.insert(20, vec![21, 22]);
        children.insert(11, vec![110]);
        children.insert(22, vec![220]);
        let index: HashMap<XID, usize> = HashMap::from([(11, 0), (22, 1), (10, 2), (21, 3)]);
        let mut z_order = [-1i64; 4];
        assign_z_order(1, &children, &index, &mut z_order);
        // Order of the walk: 1 (0), 10 (1), 11 (2), 20 (3), 22 (4); 21 has no children.
        assert_eq!(z_order, [2, 4, 1, -1]);
    }

    struct MockGraphics;

    impl IPlatformGraphics for MockGraphics {
        fn uses_shared_context(&self) -> bool {
            false
        }

        fn create_context(&self) -> Rc<dyn ferroui_base::platform::IPlatformGraphicsContext> {
            unreachable!()
        }

        fn get_shared_context(&self) -> Rc<dyn ferroui_base::platform::IPlatformGraphicsContext> {
            unreachable!()
        }
    }

    /// Walks the modes with graphics available for `available`, and returns whether graphics
    /// were chosen and the modes that were tried.
    fn walk(options: &X11PlatformOptions, available: &[X11RenderingMode]) -> (bool, Vec<X11RenderingMode>) {
        let mut tried = Vec::new();
        let graphics = FerroX11Platform::initialize_graphics(options, &mut |mode| {
            tried.push(mode);
            available.contains(&mode).then(|| Arc::new(MockGraphics) as Arc<dyn IPlatformGraphics>)
        });
        (graphics.is_some(), tried)
    }

    #[test]
    fn the_software_mode_has_no_platform_graphics() {
        let mut options = X11PlatformOptions::new();
        // The default list: GLX fails to initialize, software follows.
        assert_eq!(walk(&options, &[]), (false, vec![X11RenderingMode::Glx]));
        options.rendering_mode = vec![X11RenderingMode::Egl, X11RenderingMode::Vulkan, X11RenderingMode::Software];
        assert_eq!(walk(&options, &[]), (false, vec![X11RenderingMode::Egl, X11RenderingMode::Vulkan]));
        // Software is never asked for graphics, and nothing after it is tried.
        options.rendering_mode = vec![X11RenderingMode::Software, X11RenderingMode::Glx];
        assert_eq!(walk(&options, &[X11RenderingMode::Glx]), (false, vec![]));
    }

    #[test]
    fn the_first_mode_that_initializes_gives_the_graphics() {
        let mut options = X11PlatformOptions::new();
        assert_eq!(walk(&options, &[X11RenderingMode::Glx]), (true, vec![X11RenderingMode::Glx]));
        options.rendering_mode = vec![X11RenderingMode::Glx, X11RenderingMode::Egl, X11RenderingMode::Software];
        assert_eq!(
            walk(&options, &[X11RenderingMode::Egl]),
            (true, vec![X11RenderingMode::Glx, X11RenderingMode::Egl])
        );
        assert_eq!(walk(&options, &[X11RenderingMode::Glx, X11RenderingMode::Egl]), (true, vec![X11RenderingMode::Glx]));
    }

    #[test]
    #[should_panic(expected = "but no options were applied")]
    fn modes_that_do_not_apply_are_an_error() {
        let mut options = X11PlatformOptions::new();
        options.rendering_mode = vec![X11RenderingMode::Glx];
        walk(&options, &[]);
    }

    #[test]
    #[should_panic(expected = "must not be empty")]
    fn empty_rendering_modes_are_refused() {
        let mut options = X11PlatformOptions::new();
        options.rendering_mode.clear();
        walk(&options, &[]);
    }
}
