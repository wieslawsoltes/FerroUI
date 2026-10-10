//! Window implementation for the Win32 platform.
//!
//! The first part of the file is the state of a window that is plain data
//! and the decisions that are made from it (the styles of a set of window
//! properties, the command a state is shown with, the placement a resize
//! asks for); it is compiled, and tested, on every host. The window itself
//! is the second part.

use crate::interop::unmanaged_methods::{
    HitTestValues, ShowWindowCommand, WindowPlacementFlags, WindowStyles, RECT, WINDOWPLACEMENT,
};
use ferroui_base::Rect;
use ferroui_controls::{WindowDecorations, WindowEdge, WindowState};

/// The Windows DPI which equates to a render scaling of 1.0.
pub const STANDARD_DPI: f64 = 96.0;

pub(crate) const WINDOW_STATE_MASK: WindowStyles = WindowStyles::WS_MAXIMIZE.union(WindowStyles::WS_MINIMIZE);

/// The properties of a window its styles are made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowProperties {
    pub show_in_taskbar: bool,
    pub is_resizable: bool,
    pub is_minimizable: bool,
    pub is_maximizable: bool,
    pub decorations: WindowDecorations,
    pub is_full_screen: bool,
    pub window_state: WindowState,
}

/// What a window was before it went full screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SavedWindowInfo {
    pub style: WindowStyles,
    pub ex_style: WindowStyles,
    pub window_rect: RECT,
}

impl Default for WindowStyles {
    fn default() -> Self {
        WindowStyles::empty()
    }
}

/// The non-client hit test value that starts resizing a window by an edge.
pub(crate) fn hit_test_from_edge(edge: WindowEdge) -> i32 {
    match edge {
        WindowEdge::East => HitTestValues::HTRIGHT,
        WindowEdge::North => HitTestValues::HTTOP,
        WindowEdge::NorthEast => HitTestValues::HTTOPRIGHT,
        WindowEdge::NorthWest => HitTestValues::HTTOPLEFT,
        WindowEdge::South => HitTestValues::HTBOTTOM,
        WindowEdge::SouthEast => HitTestValues::HTBOTTOMRIGHT,
        WindowEdge::SouthWest => HitTestValues::HTBOTTOMLEFT,
        WindowEdge::West => HitTestValues::HTLEFT,
    }
}

/// The state of a window from the show command of its placement.
pub(crate) fn window_state_from_show_command(show_cmd: i32) -> WindowState {
    match show_cmd {
        ShowWindowCommand::MAXIMIZE => WindowState::Maximized,
        ShowWindowCommand::MINIMIZE => WindowState::Minimized,
        _ => WindowState::Normal,
    }
}

/// The rectangle of a window that is invalidated for a rectangle in device
/// independent pixels: the smallest rectangle of whole device pixels that
/// holds it.
pub(crate) fn invalidate_rect_from(rect: Rect, scaling: f64) -> RECT {
    RECT {
        left: (rect.x * scaling).floor() as i32,
        top: (rect.y * scaling).floor() as i32,
        right: (rect.right() * scaling).ceil() as i32,
        bottom: (rect.bottom() * scaling).ceil() as i32,
    }
}

/// The styles of a window that is not full screen.
///
/// `window_state_styles` are the minimize and maximize bits the window has
/// now, which are kept; `is_visible` is whether the window is visible now.
pub(crate) fn window_styles_from_properties(
    new_properties: &WindowProperties,
    use_redirection_bitmap: bool,
    is_embedded: bool,
    is_visible: bool,
    window_state_styles: WindowStyles,
) -> (WindowStyles, WindowStyles) {
    let mut ex_style = WindowStyles::WS_EX_WINDOWEDGE
        | if use_redirection_bitmap { WindowStyles::empty() } else { WindowStyles::WS_EX_NOREDIRECTIONBITMAP };

    if new_properties.show_in_taskbar {
        ex_style |= WindowStyles::WS_EX_APPWINDOW;
    } else {
        ex_style &= !WindowStyles::WS_EX_APPWINDOW;
    }

    let mut style = WindowStyles::WS_CLIPCHILDREN | WindowStyles::WS_CLIPSIBLINGS;

    if is_embedded {
        style |= WindowStyles::WS_CHILD;
    }

    if is_visible {
        style |= WindowStyles::WS_VISIBLE;
    }

    match new_properties.decorations {
        WindowDecorations::Full => {
            style |= WindowStyles::WS_BORDER | WindowStyles::WS_SYSMENU | WindowStyles::WS_CAPTION;
        }

        WindowDecorations::BorderOnly => {
            style |= WindowStyles::WS_BORDER;
        }

        WindowDecorations::None => {}
    }

    if new_properties.is_minimizable {
        style |= WindowStyles::WS_MINIMIZEBOX;
    }

    if new_properties.is_maximizable
        || (new_properties.window_state == WindowState::Maximized && new_properties.is_resizable)
    {
        style |= WindowStyles::WS_MAXIMIZEBOX;
    }

    if new_properties.decorations != WindowDecorations::None && new_properties.is_resizable {
        style |= WindowStyles::WS_THICKFRAME;
    }

    style &= !WINDOW_STATE_MASK;
    style |= window_state_styles & WINDOW_STATE_MASK;

    (style, ex_style)
}

/// How a window is shown in a state: the command for `ShowWindow`, if one
/// is needed, and whether the window is full screen afterwards.
pub(crate) fn show_window_command(state: WindowState, is_visible: bool, activate: bool) -> (Option<i32>, bool) {
    match state {
        WindowState::Minimized => (Some(ShowWindowCommand::MINIMIZE), false),
        WindowState::Maximized => (Some(ShowWindowCommand::MAXIMIZE), false),

        WindowState::Normal => {
            let command = if is_visible {
                ShowWindowCommand::RESTORE
            } else if activate {
                ShowWindowCommand::NORMAL
            } else {
                ShowWindowCommand::SHOW_NO_ACTIVATE
            };
            (Some(command), false)
        }

        WindowState::FullScreen => (if is_visible { None } else { Some(ShowWindowCommand::RESTORE) }, true),
    }
}

/// The placement a resize to a window size asks for, or `None` when the
/// placement is left alone: the restored size is already the requested
/// one, or the window is minimized and will be restored to its maximized
/// size.
///
/// # Panics
/// Panics for a shown window whose last state is full screen, as the
/// reference throws: the caller returns before for a window that is full
/// screen.
pub(crate) fn placement_for_resize(
    mut window_placement: WINDOWPLACEMENT,
    window_width: i32,
    window_height: i32,
    shown: bool,
    last_window_state: WindowState,
) -> Option<WINDOWPLACEMENT> {
    if window_width == window_placement.normal_position.width() && window_height == window_placement.normal_position.height() {
        return None;
    }

    // If the window is minimized, don't change the restore position, because this.Position is currently
    // out of screen with values similar to -32000,-32000. Windows considers such a position invalid on restore
    // and instead moves the window back to 0,0.
    if window_placement.show_cmd == ShowWindowCommand::SHOW_MINIMIZED {
        // The window is minimized but will be restored to maximized: don't change our normal size,
        // or it will incorrectly be set to the maximized size.
        if (window_placement.flags & WindowPlacementFlags::RESTORE_TO_MAXIMIZED.bits()) != 0 {
            return None;
        }
    }

    window_placement.normal_position.right = window_placement.normal_position.left + window_width;
    window_placement.normal_position.bottom = window_placement.normal_position.top + window_height;

    window_placement.show_cmd = if !shown {
        ShowWindowCommand::HIDE
    } else {
        match last_window_state {
            WindowState::Minimized => ShowWindowCommand::SHOW_MIN_NO_ACTIVE,
            WindowState::Maximized => ShowWindowCommand::SHOW_MAXIMIZED,
            WindowState::Normal => ShowWindowCommand::SHOW_NO_ACTIVATE,
            WindowState::FullScreen => panic!("The method or operation is not implemented."),
        }
    };

    Some(window_placement)
}

#[cfg(windows)]
pub use imp::WindowImpl;
#[cfg(windows)]
pub(crate) use imp::WindowKind;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::cursor_factory::CursorImpl;
    use crate::framebuffer_manager::FramebufferManager;
    use crate::icon_impl::IconImpl;
    use crate::interop::win32_icon::Win32Icon;
    use crate::input::{Imm32InputMethod, Imm32Parent, WindowsInputPane, WindowsKeyboardDevice, WindowsMouseDevice};
    use ferroui_base::input::text_input::ITextInputMethodImpl;
    use ferroui_base::input::raw::{RawKeyEventArgs, RawKeyEventType, RawTextInputEventArgs};
    use ferroui_base::input::{IInputDevice, Key, KeyDeviceType, PhysicalKey, RawInputModifiers};
    use ferroui_controls::platform::IInputPane;
    use crate::interop::unmanaged_methods::*;
    use crate::offscreen_parent_window::OffscreenParentWindow;
    use crate::open_gl::WglGlPlatformSurface;
    use crate::platform_constants::{PlatformConstants, Version};
    use crate::screen_impl::ScreenImpl;
    use crate::win32_gl_manager::{Win32GlManager, Win32PlatformGraphicsKind};
    use crate::win32_platform::Win32Platform;
    use crate::win32_top_level_scene_info::Win32TopLevelSceneInfo;
    use crate::win32_type_extensions::Win32TypeExtensions;
    use crate::wnd_proc_guard;
    use ferroui_base::input::platform::IClipboard;
    use ferroui_base::input::raw::IRawInputEventArgs;
    use crate::i_blur_host::{BlurEffect, ICompositionEffectsSurface};
    use crate::i_windows_surface_factory::{IWindowsSurfaceFactory, WindowsSurface};
    use ferroui_base::input::{IInputRoot, PenDevice, PointerPressedEventArgs, TouchDevice};
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::surfaces::IPlatformRenderSurface;
    use ferroui_opengl::egl::{EglGlPlatformSurface, IEglWindowGlPlatformSurfaceInfo};
    use ferroui_base::platform::{
        ICursorImpl, IOptionalFeatureProvider, IPlatformGraphics, IPlatformSettings, PlatformThemeVariant,
    };
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::rendering::composition::Compositor;
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};
    use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, PixelSize, Point, Size, Thickness};
    use ferroui_controls::platform::{
        CustomWindowStylesCallback, CustomWndProcHookCallback, INativePlatformHandleSurface, IPlatformHandle, IPopupImpl,
        IScreenImpl, ITopLevelImpl, IWin32OptionsTopLevelImpl, IWindowBaseImpl, IWindowIconImpl, IWindowImpl,
        PlatformRequestedDrawnDecoration, WindowCornerPreference,
    };
    use ferroui_controls::primitives::popup_positioning::IPopupPositioner;
    use ferroui_controls::{
        AcrylicPlatformCompensationLevels, Design, WindowCloseReason, WindowResizeReason, WindowTransparencyLevel,
    };
    use std::any::{Any, TypeId};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::rc::{Rc, Weak};
    use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering};
    use std::sync::Arc;

    thread_local! {
        /// The windows of this thread that exist: what roots a window while
        /// its handle is alive.
        static INSTANCES: RefCell<Vec<Rc<WindowImpl>>> = const { RefCell::new(Vec::new()) };
        /// The windows of this thread by handle, for the window procedure.
        static WINDOWS: RefCell<HashMap<isize, Weak<WindowImpl>>> = RefCell::new(HashMap::new());
        /// The window that is being created: the system sends its first
        /// messages before the handle is known here.
        static CREATING: RefCell<Option<Weak<WindowImpl>>> = const { RefCell::new(None) };
    }

    /// The names of the window classes are unique in the process.
    static CLASS_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// What kind of top-level a window is. The reference has a class for
    /// each (`PopupImpl` and `EmbeddedWindowImpl` derive from `WindowImpl`
    /// and override a handful of members); here the members that differ
    /// ask the kind.
    pub(crate) enum WindowKind {
        /// A top-level window.
        Window,
        /// A popup: what its parent is, its positioner and its own state.
        Popup(crate::popup_impl::PopupState),
        /// A child window that a host places in a foreign window.
        Embedded,
    }

    /// The handle of a window as a platform handle and as a render surface.
    ///
    /// It is shared with the thread that renders, so it holds numbers: the
    /// handle, and the scaling of the window as it was last set on the UI
    /// thread.
    pub(crate) struct WindowImplPlatformHandle {
        hwnd: AtomicIsize,
        scaling_bits: AtomicU64,
    }

    impl WindowImplPlatformHandle {
        fn client_pixel_size(&self) -> PixelSize {
            let rect = get_client_rect(self.hwnd.load(Ordering::Acquire));
            PixelSize::new(rect.right, rect.bottom)
        }
    }

    impl IPlatformHandle for WindowImplPlatformHandle {
        fn handle(&self) -> isize {
            self.hwnd.load(Ordering::Acquire)
        }

        fn handle_descriptor(&self) -> Option<&str> {
            Some(PlatformConstants::WINDOW_HANDLE_TYPE)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl IPlatformRenderSurface for WindowImplPlatformHandle {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// What an EGL window surface needs of the window, read by the thread
    /// that renders.
    impl IEglWindowGlPlatformSurfaceInfo for WindowImplPlatformHandle {
        fn handle(&self) -> isize {
            self.hwnd.load(Ordering::Acquire)
        }

        fn size(&self) -> PixelSize {
            let rect = get_client_rect(self.hwnd.load(Ordering::Acquire));

            PixelSize::new((rect.right - rect.left).max(1), (rect.bottom - rect.top).max(1))
        }

        fn scaling(&self) -> f64 {
            f64::from_bits(self.scaling_bits.load(Ordering::Acquire))
        }
    }

    impl INativePlatformHandleSurface for WindowImplPlatformHandle {
        fn size(&self) -> PixelSize {
            self.client_pixel_size()
        }

        fn scaling(&self) -> f64 {
            f64::from_bits(self.scaling_bits.load(Ordering::Acquire))
        }
    }

    /// The handle of a window as the top-level hands it out: the UI thread
    /// side of [`WindowImplPlatformHandle`].
    struct TopLevelHandle(Arc<WindowImplPlatformHandle>);

    impl IPlatformHandle for TopLevelHandle {
        fn handle(&self) -> isize {
            IPlatformHandle::handle(&*self.0)
        }

        fn handle_descriptor(&self) -> Option<&str> {
            Some(PlatformConstants::WINDOW_HANDLE_TYPE)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn equals(&self, other: &dyn IPlatformHandle) -> bool {
            other.as_any().downcast_ref::<TopLevelHandle>().is_some_and(|other| Arc::ptr_eq(&self.0, &other.0))
        }
    }

    type Callback<F> = RefCell<Option<Rc<F>>>;

    /// `WinUiCompositionShared.MinHostBackdropVersion` of the reference:
    /// the first version of the system with the host backdrop brush
    /// attribute of a window.
    const MIN_HOST_BACKDROP_VERSION: Version = Version { major: 10, minor: 0, build: 22000 };

    /// Window implementation for the Win32 platform.
    pub struct WindowImpl {
        this: Weak<WindowImpl>,
        kind: WindowKind,

        saved_window_info: Cell<SavedWindowInfo>,
        is_full_screen_active: Cell<bool>,
        is_client_area_extended: Cell<bool>,
        extended_margins: Cell<Thickness>,
        extend_title_bar_hint: Cell<f64>,
        resize_reason: Cell<WindowResizeReason>,

        mouse_device: Rc<WindowsMouseDevice>,
        framebuffer: RefCell<Option<Arc<FramebufferManager>>>,
        /// The surface the platform graphics render the window through,
        /// when the platform has graphics.
        gl_surface: RefCell<Option<Arc<dyn IPlatformRenderSurface>>>,
        handle: RefCell<Option<Arc<WindowImplPlatformHandle>>>,

        class_name: RefCell<Option<String>>,
        hwnd: Cell<isize>,
        owner: RefCell<Option<Rc<dyn IInputRoot>>>,
        /// The drop target that is registered for the window, from the
        /// moment the window has an input root until it is destroyed.
        drop_target: RefCell<Option<ferroui_microcom::ComPtr<crate::win32_com::IDropTarget>>>,
        window_properties: Cell<WindowProperties>,
        tracking_mouse: Cell<bool>,
        tracking_non_client_mouse: Cell<bool>,
        /// The point of the last mouse move, as the history of the mouse
        /// of the system names it.
        last_wm_mouse_point: Cell<MOUSEMOVEPOINT>,
        touch_device: Rc<TouchDevice>,
        pen_device: Rc<PenDevice>,
        /// Whether the system has the pointer messages (Windows 8 and later).
        wm_pointer_enabled: bool,
        topmost: Cell<bool>,
        scaling: Cell<f64>,
        dpi: Cell<u32>,
        icon_impl: RefCell<Option<Rc<IconImpl>>>,
        /// The icons of the system made for the window, by kind and DPI.
        icon_cache: RefCell<HashMap<(i32, u32), Win32Icon>>,
        storage_provider: RefCell<Option<Rc<crate::win32_storage_provider::Win32StorageProvider>>>,
        native_control_host: RefCell<Option<Rc<crate::win32_native_control_host::Win32NativeControlHost>>>,
        show_window_state: Cell<WindowState>,
        last_window_state: Cell<WindowState>,
        min_size: Cell<Size>,
        max_size: Cell<Size>,
        max_track_size: Cell<POINT>,
        parent: RefCell<Option<Rc<WindowImpl>>>,
        is_close_requested: Cell<bool>,
        shown: Cell<bool>,
        hidden_window_is_parent: Cell<bool>,
        ignore_wm_char: Cell<bool>,
        /// The language of the keyboard layout the input method was last
        /// given for this window.
        langid: Cell<u32>,
        /// The window lost the focus while an input method composed: the
        /// toolkit is told when the composition ends.
        kill_focus_requested: Cell<bool>,
        input_pane: RefCell<Option<Rc<WindowsInputPane>>>,
        /// The first half of a character of two `WM_CHAR` messages.
        pending_high_surrogate: Cell<Option<u16>>,
        /// The surface of the window when it is one of a composition mode
        /// that has the blur effects (`_glSurface as
        /// ICompositionEffectsSurface` in the reference). No surface of
        /// the backend is one before the composition modes of stage 2c.
        composition_effects_surface: RefCell<Option<Arc<dyn ICompositionEffectsSurface>>>,
        /// Releases what the surface of a composition mode holds of the
        /// window (the reference disposes the surface when it is
        /// disposable).
        gl_surface_dispose: RefCell<Option<Arc<dyn Fn() + Send + Sync>>>,
        transparency_level: Cell<WindowTransparencyLevel>,
        default_transparency_level: WindowTransparencyLevel,
        corner_preference: Cell<WindowCornerPreference>,
        current_theme_variant: Cell<PlatformThemeVariant>,
        scene_info: Cell<Win32TopLevelSceneInfo>,
        use_redirection_bitmap: bool,
        screen: Rc<ScreenImpl>,

        activated: Callback<dyn Fn()>,
        closing: Callback<dyn Fn(WindowCloseReason) -> bool>,
        closed: Callback<dyn Fn()>,
        deactivated: Callback<dyn Fn()>,
        input: Callback<dyn Fn(Rc<dyn IRawInputEventArgs>)>,
        paint: Callback<dyn Fn(Rect)>,
        resized: Callback<dyn Fn(Size, WindowResizeReason)>,
        scaling_changed: Callback<dyn Fn(f64)>,
        position_changed: Callback<dyn Fn(PixelPoint)>,
        window_state_changed: Callback<dyn Fn(WindowState)>,
        lost_focus: Callback<dyn Fn()>,
        transparency_level_changed: Callback<dyn Fn(WindowTransparencyLevel)>,
        platform_specific_scene_info_changed: Callback<dyn Fn(Option<Arc<dyn Any + Send + Sync>>)>,
        got_input_when_disabled: Callback<dyn Fn()>,
        extend_client_area_to_decorations_changed: Callback<dyn Fn(bool)>,
        window_styles_callback: RefCell<Option<CustomWindowStylesCallback>>,
        wnd_proc_hook_callback: RefCell<Option<CustomWndProcHookCallback>>,
    }

    fn default_cursor() -> isize {
        thread_local! {
            static DEFAULT_CURSOR: isize = load_cursor(0, Cursor::IDC_ARROW);
        }
        DEFAULT_CURSOR.with(|cursor| *cursor)
    }

    fn call<F: ?Sized>(callback: &Callback<F>) -> Option<Rc<F>> {
        callback.borrow().clone()
    }

    fn set<F: ?Sized>(callback: &Callback<F>, value: Option<Rc<F>>) {
        let old = callback.replace(value);
        drop(old);
    }

    impl WindowImpl {
        /// Creates a top-level window.
        pub fn new() -> Rc<WindowImpl> {
            Self::create(
                WindowKind::Window,
                WindowProperties {
                    show_in_taskbar: false,
                    is_resizable: true,
                    is_minimizable: true,
                    is_maximizable: true,
                    decorations: WindowDecorations::Full,
                    is_full_screen: false,
                    window_state: WindowState::Normal,
                },
            )
        }

        pub(crate) fn create(kind: WindowKind, window_properties: WindowProperties) -> Rc<WindowImpl> {
            // A window is drawn through its redirection surface unless the
            // composition mode that was registered presents through a
            // surface of its own.
            let locator = FerroLocator::current();
            let gl_platform = locator.get_service::<Arc<dyn IPlatformGraphics>>();
            let surface_factory = locator.get_service::<dyn IWindowsSurfaceFactory>();
            let use_redirection_bitmap = match &surface_factory {
                Some(surface_factory) => gl_platform.is_none() || !surface_factory.requires_no_redirection_bitmap(),
                None => true,
            };

            let default_transparency_level = if use_redirection_bitmap {
                WindowTransparencyLevel::none()
            } else {
                WindowTransparencyLevel::transparent()
            };

            let this = Rc::new_cyclic(|this| WindowImpl {
                this: this.clone(),
                kind,
                saved_window_info: Cell::new(SavedWindowInfo::default()),
                is_full_screen_active: Cell::new(false),
                is_client_area_extended: Cell::new(false),
                extended_margins: Cell::new(Thickness::default()),
                extend_title_bar_hint: Cell::new(-1.0),
                resize_reason: Cell::new(WindowResizeReason::Unspecified),
                mouse_device: WindowsMouseDevice::instance(),
                framebuffer: RefCell::new(None),
                gl_surface: RefCell::new(None),
                handle: RefCell::new(None),
                class_name: RefCell::new(None),
                hwnd: Cell::new(0),
                owner: RefCell::new(None),
                drop_target: RefCell::new(None),
                window_properties: Cell::new(window_properties),
                tracking_mouse: Cell::new(false),
                tracking_non_client_mouse: Cell::new(false),
                last_wm_mouse_point: Cell::new(MOUSEMOVEPOINT::default()),
                touch_device: TouchDevice::new(),
                pen_device: PenDevice::new(false),
                wm_pointer_enabled: Win32Platform::windows_version() >= PlatformConstants::WINDOWS8,
                topmost: Cell::new(false),
                scaling: Cell::new(1.0),
                dpi: Cell::new(96),
                icon_impl: RefCell::new(None),
                icon_cache: RefCell::new(HashMap::new()),
                storage_provider: RefCell::new(None),
                native_control_host: RefCell::new(None),
                show_window_state: Cell::new(WindowState::Normal),
                last_window_state: Cell::new(WindowState::Normal),
                min_size: Cell::new(Size::default()),
                max_size: Cell::new(Size::default()),
                max_track_size: Cell::new(POINT::default()),
                parent: RefCell::new(None),
                is_close_requested: Cell::new(false),
                shown: Cell::new(false),
                hidden_window_is_parent: Cell::new(false),
                ignore_wm_char: Cell::new(false),
                langid: Cell::new(0),
                kill_focus_requested: Cell::new(false),
                input_pane: RefCell::new(None),
                pending_high_surrogate: Cell::new(None),
                composition_effects_surface: RefCell::new(None),
                gl_surface_dispose: RefCell::new(None),
                transparency_level: Cell::new(default_transparency_level),
                default_transparency_level,
                corner_preference: Cell::new(WindowCornerPreference::default()),
                current_theme_variant: Cell::new(PlatformThemeVariant::Light),
                scene_info: Cell::new(Win32TopLevelSceneInfo::new(PlatformThemeVariant::Light)),
                use_redirection_bitmap,
                screen: Win32Platform::instance().screen(),
                activated: RefCell::new(None),
                closing: RefCell::new(None),
                closed: RefCell::new(None),
                deactivated: RefCell::new(None),
                input: RefCell::new(None),
                paint: RefCell::new(None),
                resized: RefCell::new(None),
                scaling_changed: RefCell::new(None),
                position_changed: RefCell::new(None),
                window_state_changed: RefCell::new(None),
                lost_focus: RefCell::new(None),
                transparency_level_changed: RefCell::new(None),
                platform_specific_scene_info_changed: RefCell::new(None),
                got_input_when_disabled: RefCell::new(None),
                extend_client_area_to_decorations_changed: RefCell::new(None),
                window_styles_callback: RefCell::new(None),
                wnd_proc_hook_callback: RefCell::new(None),
            });

            this.create_window();
            *this.framebuffer.borrow_mut() = Some(Arc::new(FramebufferManager::new(this.hwnd.get())));

            // The surface of the platform graphics: the one of the surface
            // factory (a composition mode), or the one that fits the
            // platform graphics; the reference tests their type, the
            // graphics manager remembers what it registered.
            if gl_platform.is_some() {
                if let Some(handle) = this.handle.borrow().clone() {
                    if let Some(surface_factory) = &surface_factory {
                        let WindowsSurface { surface, effects, dispose } = surface_factory.create_surface(handle);
                        *this.gl_surface.borrow_mut() = Some(surface);
                        *this.composition_effects_surface.borrow_mut() = effects;
                        *this.gl_surface_dispose.borrow_mut() = dispose;
                    } else if Win32GlManager::platform_graphics_kind() == Some(Win32PlatformGraphicsKind::AngleD3D11) {
                        let gl_surface: Arc<dyn IPlatformRenderSurface> = EglGlPlatformSurface::new(handle);
                        *this.gl_surface.borrow_mut() = Some(gl_surface);
                    } else if Win32GlManager::platform_graphics_kind() == Some(Win32PlatformGraphicsKind::Wgl) {
                        let gl_surface: Arc<dyn IPlatformRenderSurface> = WglGlPlatformSurface::new(handle);
                        *this.gl_surface.borrow_mut() = Some(gl_surface);
                    }
                }
            }

            // (The storage provider and the native control host are
            // created when they are first asked for.)
            *this.input_pane.borrow_mut() = WindowsInputPane::try_create(&this);

            INSTANCES.with(|instances| instances.borrow_mut().push(this.clone()));

            this
        }

        /// The window behind a top-level contract, if it is a window of
        /// this backend.
        pub fn from_top_level(top_level: &dyn ITopLevelImpl) -> Option<&WindowImpl> {
            top_level.as_any().downcast_ref::<WindowImpl>()
        }

        pub(crate) fn this(&self) -> Option<Rc<WindowImpl>> {
            self.this.upgrade()
        }

        pub(crate) fn kind(&self) -> &WindowKind {
            &self.kind
        }

        /// The handle of the window; 0 once the window is destroyed.
        pub fn hwnd(&self) -> isize {
            self.hwnd.get()
        }

        /// The input root of the window.
        ///
        /// # Panics
        /// Panics when the input root has not been set.
        pub(crate) fn owner(&self) -> Rc<dyn IInputRoot> {
            match self.owner.borrow().clone() {
                Some(owner) => owner,
                None => panic!("set_input_root must have been called"),
            }
        }

        #[allow(dead_code)] // Read by the automation of the backend, a later stage.
        pub(crate) fn parent_impl(&self) -> Option<Rc<WindowImpl>> {
            self.parent.borrow().clone()
        }

        pub(crate) fn mouse_device(&self) -> &Rc<WindowsMouseDevice> {
            &self.mouse_device
        }

        pub(crate) fn framebuffer(&self) -> Arc<FramebufferManager> {
            self.framebuffer.borrow().clone().expect("the framebuffer exists once the window is created")
        }

        /// Releases what the surface of a composition mode holds of the
        /// window.
        pub(crate) fn dispose_gl_surface(&self) {
            let dispose = self.gl_surface_dispose.borrow_mut().take();
            if let Some(dispose) = dispose {
                dispose();
            }
        }

        pub(crate) fn screen(&self) -> &Rc<ScreenImpl> {
            &self.screen
        }

        // ------------------------------------------------------------
        // State the window procedure reads and writes
        // ------------------------------------------------------------

        pub(crate) fn window_properties(&self) -> WindowProperties {
            self.window_properties.get()
        }

        pub(crate) fn is_client_area_extended(&self) -> bool {
            self.is_client_area_extended.get()
        }

        pub(crate) fn is_full_screen_active(&self) -> bool {
            self.is_full_screen_active.get()
        }

        pub(crate) fn scaling(&self) -> f64 {
            self.scaling.get()
        }

        fn set_scaling(&self, dpi: u32) {
            self.dpi.set(dpi);
            let scaling = f64::from(dpi) / STANDARD_DPI;
            self.scaling.set(scaling);
            if let Some(handle) = self.handle.borrow().as_ref() {
                handle.scaling_bits.store(scaling.to_bits(), Ordering::Release);
            }
        }

        /// The DPI of the window changed: the scaling follows it. (The
        /// icons of the window are loaded again for the new DPI by the
        /// reference: stage 2.)
        pub(crate) fn set_dpi(&self, dpi: u32) {
            self.set_scaling(dpi);
        }

        pub(crate) fn resize_reason(&self) -> WindowResizeReason {
            self.resize_reason.get()
        }

        /// Sets the reason of the resizes that follow; returns the reason
        /// that was set before, which the caller sets back.
        pub(crate) fn set_resize_reason(&self, reason: WindowResizeReason) -> WindowResizeReason {
            self.resize_reason.replace(reason)
        }

        pub(crate) fn ignore_wm_char(&self) -> bool {
            self.ignore_wm_char.get()
        }

        pub(crate) fn set_ignore_wm_char(&self, value: bool) {
            self.ignore_wm_char.set(value);
        }

        pub(crate) fn kill_focus_requested(&self) -> bool {
            self.kill_focus_requested.get()
        }

        pub(crate) fn set_kill_focus_requested(&self, value: bool) {
            self.kill_focus_requested.set(value);
        }

        /// Gives the input method of the thread this window and the
        /// language of a keyboard layout (`UpdateInputMethod`).
        pub(crate) fn update_input_method(&self, hkl: isize) {
            // note: for non-ime language, also create it so that emoji panel tracks cursor
            let langid = lgid(hkl);

            if langid == self.langid.get() && Imm32InputMethod::current().hwnd() == self.hwnd.get() {
                return;
            }

            self.langid.set(langid);

            let parent: Weak<dyn Imm32Parent> = self.this.clone();
            Imm32InputMethod::current().set_language_and_window(parent, self.hwnd.get(), hkl);
        }

        pub(crate) fn dispose_input_pane(&self) {
            let input_pane = self.input_pane.borrow_mut().take();
            if let Some(input_pane) = input_pane {
                input_pane.dispose();
            }
        }

        /// The text of a `WM_CHAR` message of this window, if the message
        /// completes one.
        pub(crate) fn text_from_char_message(&self, unit: u16) -> Option<String> {
            let mut pending = self.pending_high_surrogate.get();
            let text = crate::window_impl_app_wnd_proc::text_from_char_message(&mut pending, unit);
            self.pending_high_surrogate.set(pending);
            text
        }

        pub(crate) fn tracking_mouse(&self) -> bool {
            self.tracking_mouse.get()
        }

        pub(crate) fn set_tracking_mouse(&self, value: bool) {
            self.tracking_mouse.set(value);
        }

        pub(crate) fn shown(&self) -> bool {
            self.shown.get()
        }

        pub(crate) fn set_shown(&self, value: bool) {
            self.shown.set(value);
        }

        pub(crate) fn last_window_state(&self) -> WindowState {
            self.last_window_state.get()
        }

        pub(crate) fn set_last_window_state(&self, value: WindowState) {
            self.last_window_state.set(value);
        }

        pub(crate) fn set_is_close_requested(&self, value: bool) {
            self.is_close_requested.set(value);
        }

        pub(crate) fn set_max_track_size(&self, value: POINT) {
            self.max_track_size.set(value);
        }

        pub(crate) fn min_size(&self) -> Size {
            self.min_size.get()
        }

        pub(crate) fn max_size(&self) -> Size {
            self.max_size.get()
        }

        pub(crate) fn has_full_decorations(&self) -> bool {
            self.window_properties.get().decorations == WindowDecorations::Full
        }

        pub(crate) fn is_mouse_in_pointer_enabled(&self) -> bool {
            self.wm_pointer_enabled && is_mouse_in_pointer_enabled()
        }

        pub(crate) fn wm_pointer_enabled(&self) -> bool {
            self.wm_pointer_enabled
        }

        pub(crate) fn touch_device(&self) -> &Rc<TouchDevice> {
            &self.touch_device
        }

        pub(crate) fn pen_device(&self) -> &Rc<PenDevice> {
            &self.pen_device
        }

        pub(crate) fn tracking_non_client_mouse(&self) -> bool {
            self.tracking_non_client_mouse.get()
        }

        pub(crate) fn set_tracking_non_client_mouse(&self, value: bool) {
            self.tracking_non_client_mouse.set(value);
        }

        /// Sets the point of the last mouse move and returns the one before.
        pub(crate) fn replace_last_wm_mouse_point(&self, value: MOUSEMOVEPOINT) -> MOUSEMOVEPOINT {
            self.last_wm_mouse_point.replace(value)
        }

        pub(crate) fn extend_title_bar_hint(&self) -> f64 {
            self.extend_title_bar_hint.get()
        }

        pub(crate) fn extended_margins_value(&self) -> Thickness {
            self.extended_margins.get()
        }

        /// The input root of the window, once it has one.
        pub(crate) fn try_owner(&self) -> Option<Rc<dyn IInputRoot>> {
            self.owner.borrow().clone()
        }

        pub(crate) fn should_take_focus_on_click(&self) -> bool {
            !matches!(self.kind, WindowKind::Popup(_))
        }

        pub(crate) fn input_callback(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
            call(&self.input)
        }

        pub(crate) fn invoke_activated(&self) {
            if let Some(callback) = call(&self.activated) {
                callback();
            }
        }

        pub(crate) fn invoke_deactivated(&self) {
            if let Some(callback) = call(&self.deactivated) {
                callback();
            }
        }

        pub(crate) fn invoke_closing(&self, reason: WindowCloseReason) -> Option<bool> {
            call(&self.closing).map(|callback| callback(reason))
        }

        pub(crate) fn invoke_closed(&self) {
            if let Some(callback) = call(&self.closed) {
                callback();
            }
        }

        pub(crate) fn invoke_paint(&self, rect: Rect) {
            if let Some(callback) = call(&self.paint) {
                callback(rect);
            }
        }

        pub(crate) fn invoke_resized(&self, size: Size, reason: WindowResizeReason) {
            if let Some(callback) = call(&self.resized) {
                callback(size, reason);
            }
        }

        pub(crate) fn invoke_scaling_changed(&self, scaling: f64) {
            if let Some(callback) = call(&self.scaling_changed) {
                callback(scaling);
            }
        }

        pub(crate) fn invoke_position_changed(&self, position: PixelPoint) {
            if let Some(callback) = call(&self.position_changed) {
                callback(position);
            }
        }

        pub(crate) fn invoke_window_state_changed(&self, state: WindowState) {
            if let Some(callback) = call(&self.window_state_changed) {
                callback(state);
            }
        }

        pub(crate) fn invoke_lost_focus(&self) {
            if let Some(callback) = call(&self.lost_focus) {
                callback();
            }
        }

        pub(crate) fn invoke_extend_client_area_to_decorations_changed(&self, value: bool) {
            if let Some(callback) = call(&self.extend_client_area_to_decorations_changed) {
                callback(value);
            }
        }

        /// The monitors changed: a popup asks for its largest size again.
        pub(crate) fn on_display_change(&self) {
            if let WindowKind::Popup(popup) = &self.kind {
                popup.reset_max_auto_size();
            }
        }

        // ------------------------------------------------------------
        // The members of the window
        // ------------------------------------------------------------

        /// The thickness of the frame of the window, in device pixels.
        ///
        /// # Panics
        /// Panics when the system cannot compute the frame of the styles
        /// of the window.
        pub(crate) fn border_thickness(&self) -> Thickness {
            let style = self.get_style();
            if style.contains(WindowStyles::WS_BORDER) {
                let ex_style = self.get_extended_style();

                let mut padding = RECT::default();

                if adjust_window_rect_ex(&mut padding, style.bits(), false, ex_style.bits()) {
                    Thickness::new(
                        f64::from(-padding.left),
                        f64::from(-padding.top),
                        f64::from(padding.right),
                        f64::from(padding.bottom),
                    )
                } else {
                    panic!("AdjustWindowRectEx failed with the error code {}", get_last_error());
                }
            } else {
                Thickness::default()
            }
        }

        fn client_size_impl(&self) -> Size {
            let rect = get_client_rect(self.hwnd.get());

            Size::new(f64::from(rect.right), f64::from(rect.bottom)) / self.scaling.get()
        }

        fn frame_size_impl(&self) -> Size {
            let rc_window = get_window_rect(self.hwnd.get());
            Size::new(f64::from(rc_window.width()), f64::from(rc_window.height())) / self.scaling.get()
        }

        pub(crate) fn window_state_impl(&self) -> WindowState {
            if !is_window_visible(self.hwnd.get()) {
                return self.show_window_state.get();
            }

            if self.is_full_screen_active.get() {
                return WindowState::FullScreen;
            }

            let placement = get_window_placement(self.hwnd.get());

            window_state_from_show_command(placement.show_cmd)
        }

        pub(crate) fn set_window_state_impl(&self, value: WindowState) {
            if is_window_visible(self.hwnd.get()) && self.last_window_state.get() != value {
                // If the window is minimized, it shouldn't be activated
                self.show_window(value, value != WindowState::Minimized);
            }

            self.last_window_state.set(value);
            self.show_window_state.set(value);
        }

        fn set_transparency_level(&self, value: WindowTransparencyLevel) {
            if self.transparency_level.get() != value {
                self.transparency_level.set(value);
                if let Some(callback) = call(&self.transparency_level_changed) {
                    callback(value);
                }
            }
        }

        fn is_supported(&self, level: WindowTransparencyLevel) -> bool {
            // None is only supported with redirection bitmap.
            // Note, it's still possible to have non-transparent window with a fallback background brush.
            if level == WindowTransparencyLevel::none() {
                return self.use_redirection_bitmap;
            }

            // Transparent is supported either with DwmEnableBlurBehindWindow (win8+) or with NoRedirectionBitmap.
            if level == WindowTransparencyLevel::transparent() {
                return !self.use_redirection_bitmap || Win32Platform::windows_version() >= PlatformConstants::WINDOWS8;
            }

            let surface = self.composition_effects_surface.borrow();

            if level == WindowTransparencyLevel::blur() {
                return surface.as_ref().is_some_and(|surface| surface.is_blur_supported(BlurEffect::GaussianBlur));
            }

            if level == WindowTransparencyLevel::acrylic_blur() {
                return surface.as_ref().is_some_and(|surface| surface.is_blur_supported(BlurEffect::Acrylic));
            }

            if level == WindowTransparencyLevel::mica() {
                return surface.as_ref().is_some_and(|surface| surface.is_blur_supported(BlurEffect::MicaDark));
            }

            false
        }

        // The composition effects themselves are applied by the render target on the render thread,
        // based on the transparency level and theme variant it receives with the scene info.
        // Here we only adjust the DWM window attributes that have to be set from the UI thread.

        fn set_transparency_transparent(&self) -> bool {
            if self.composition_effects_surface.borrow().is_some() {
                return true;
            }

            self.set_legacy_transparency(true)
        }

        fn set_transparency_acrylic_blur(&self) -> bool {
            self.set_use_host_backdrop_brush(true);
            self.set_legacy_transparency(false);
            true
        }

        fn set_transparency_mica(&self) -> bool {
            self.set_use_host_backdrop_brush(false);
            self.set_legacy_transparency(false);
            true
        }

        fn set_use_host_backdrop_brush(&self, use_host_backdrop_brush: bool) -> bool {
            if Win32Platform::windows_version() < MIN_HOST_BACKDROP_VERSION {
                return false;
            }

            // AcrylicBlur requires window to set DWMWA_USE_HOSTBACKDROPBRUSH flag on Win11+.
            // It's not necessary on older versions and it's not necessary with Mica brush.

            let result = dwm_set_window_attribute(
                self.hwnd.get(),
                DwmWindowAttribute::DWMWA_USE_HOSTBACKDROPBRUSH,
                i32::from(use_host_backdrop_brush),
            );
            result == 0
        }

        fn set_legacy_transparency(&self, enabled: bool) -> bool {
            if Win32Platform::windows_version() < PlatformConstants::WINDOWS8 || !self.use_redirection_bitmap {
                return false;
            }

            // On pre-Win8 this method was blurring a window, which is a different from desired behavior.
            // On win8+ we use this method as a fallback, when WinUI/DComp composition with true transparency isn't available.
            // Note: there is no guarantee that this behavior won't be changed back to true blur in Win12.
            // See https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmenableblurbehindwindow#remarks
            // Also https://github.com/qt/qtbase/blob/fd300f143fd30947bba60a03d614acd2711b635f/src/plugins/platforms/windows/qwindowswindow.cpp#L519
            dwm_enable_blur_behind_window(self.hwnd.get(), enabled)
        }

        pub(crate) fn position_impl(&self) -> PixelPoint {
            let rc = get_window_rect(self.hwnd.get());
            PixelPoint::new(rc.left, rc.top)
        }

        fn set_position(&self, value: PixelPoint) {
            set_window_pos(
                self.hwnd.get(),
                0,
                value.x,
                value.y,
                0,
                0,
                SetWindowPosFlags::SWP_NOSIZE | SetWindowPosFlags::SWP_NOACTIVATE | SetWindowPosFlags::SWP_NOZORDER,
            );

            self.read_monitor_dpi();
        }

        /// Reads the DPI of the monitor the window is on, where the system
        /// reports one for a monitor (Windows 8.1 and later).
        fn read_monitor_dpi(&self) {
            if Win32Platform::windows_version() >= PlatformConstants::WINDOWS8_1 {
                let monitor = monitor_from_window(self.hwnd.get(), MONITOR::MONITOR_DEFAULTTONEAREST);

                if let Some((dpi, _)) = get_dpi_for_monitor(monitor, MONITOR_DPI_TYPE::MDT_EFFECTIVE_DPI) {
                    self.set_scaling(dpi);
                }
            }
        }

        fn resize_impl(&self, value: Size, reason: WindowResizeReason) {
            let requested_client_width = (value.width * self.scaling.get()) as i32;
            let requested_client_height = (value.height * self.scaling.get()) as i32;

            let current_client_rect = get_client_rect(self.hwnd.get());
            if current_client_rect.width() == requested_client_width && current_client_rect.height() == requested_client_height
            {
                // Don't update our window position if the client size is already correct. This leads to Windows updating our
                // "normal position" (i.e. restored bounds) to match our maximised or areo snap size, which is incorrect behaviour.
                // We only want to proceed with this method if the new size is coming from the toolkit.
                return;
            }

            if self.last_window_state.get() == WindowState::FullScreen && self.is_full_screen_active.get() {
                // Fullscreen mode is really a restored window without a frame filling the whole monitor.
                // It doesn't make sense to resize the window in this state, so ignore this request.
                // (If the fullscreen mode isn't yet active, continue normally so that our normal window size gets saved.)
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                    logger.log(None, "Ignoring resize event on fullscreen window.");
                }
                return;
            }

            let window_placement = get_window_placement(self.hwnd.get());

            let requested_client_rect =
                RECT { left: 0, top: 0, right: requested_client_width, bottom: requested_client_height };

            let mut requested_window_rect = self.client_rect_to_window_rect(requested_client_rect, None, None);

            if self.is_client_area_extended.get() {
                // We told Windows we have a border, but since we're actually extending into it,
                // it should be excluded from the final window bounds.
                let style = self.get_style();
                if style.intersects(WindowStyles::WS_BORDER) {
                    let border_only_rect =
                        self.client_rect_to_window_rect(requested_client_rect, Some(WindowStyles::WS_BORDER), None);

                    requested_window_rect.top = border_only_rect.top;

                    // If we're supposed to have a caption, the top border is actually collapsed into it.
                    if style.contains(WindowStyles::WS_CAPTION) {
                        requested_window_rect.top += 1;
                    }
                }
            }

            let window_width = requested_window_rect.width();
            let window_height = requested_window_rect.height();

            let Some(window_placement) = placement_for_resize(
                window_placement,
                window_width,
                window_height,
                self.shown.get(),
                self.last_window_state.get(),
            ) else {
                return;
            };

            let old = self.set_resize_reason(reason);
            set_window_placement(self.hwnd.get(), &window_placement);
            self.set_resize_reason(old);
            wnd_proc_guard::resume_pending();
        }

        /// Destroys the window if it still exists.
        pub(crate) fn dispose_impl(&self) {
            self.dispose_input_pane();
            if self.hwnd.get() != 0 {
                // Detect if we are being closed programmatically - this would mean that WM_CLOSE was not called
                // and we didn't prepare this window for destruction.
                if !self.is_close_requested.get() {
                    self.before_close_cleanup(true);
                }

                destroy_window(self.hwnd.get());
                self.hwnd.set(0);
                wnd_proc_guard::resume_pending();
            }

            self.clear_icon_cache();
        }

        fn set_icon_impl(&self, icon: Option<Rc<dyn IWindowIconImpl>>) {
            let icon = icon.map(|icon| match IconImpl::from_window_icon(&icon) {
                Ok(icon) => icon,
                Err(error) => panic!("The icon of the window could not be read: {error}"),
            });
            let same = match (&*self.icon_impl.borrow(), &icon) {
                (None, None) => true,
                (Some(current), Some(icon)) => Rc::ptr_eq(current, icon),
                _ => false,
            };
            if same {
                return;
            }

            *self.icon_impl.borrow_mut() = icon;
            self.clear_icon_cache();
            self.refresh_icon();
        }

        fn clear_icon_cache(&self) {
            let icons = std::mem::take(&mut *self.icon_cache.borrow_mut());
            for icon in icons.values() {
                icon.dispose();
            }
        }

        /// Whether the window has an icon.
        pub(crate) fn has_icon(&self) -> bool {
            self.icon_impl.borrow().is_some()
        }

        /// The handle of the icon of the window of a kind (`Icons`) for a
        /// DPI; 0 for a window without an icon.
        ///
        /// # Panics
        /// Panics for a kind of icon the system does not have, and when
        /// the icon cannot be made.
        pub(crate) fn load_icon(&self, type_: i32, dpi: u32) -> isize {
            let Some(icon_impl) = self.icon_impl.borrow().clone() else {
                return 0;
            };

            let type_ = if type_ == Icons::ICON_SMALL2 { Icons::ICON_SMALL } else { type_ };

            let icon_key = (type_, dpi);
            if let Some(icon) = self.icon_cache.borrow().get(&icon_key) {
                return icon.handle();
            }
            let scale = f64::from(dpi) / 96.0;
            let icon = match type_ {
                Icons::ICON_SMALL => icon_impl.load_small_icon(scale),
                Icons::ICON_BIG => icon_impl.load_big_icon(scale),
                _ => panic!("the kind of icon {type_} is not implemented"),
            };
            let icon = match icon {
                Ok(icon) => icon,
                Err(error) => panic!("The icon of the window could not be made: {error}"),
            };
            let handle = icon.handle();
            self.icon_cache.borrow_mut().insert(icon_key, icon);
            handle
        }

        pub(crate) fn refresh_icon(&self) {
            let hwnd = self.hwnd.get();
            let dpi = self.dpi.get();
            send_message(hwnd, WindowsMessage::WM_SETICON, Icons::ICON_SMALL as usize, self.load_icon(Icons::ICON_SMALL, dpi));
            send_message(hwnd, WindowsMessage::WM_SETICON, Icons::ICON_BIG as usize, self.load_icon(Icons::ICON_BIG, dpi));

            // This will prompt the taskbar to redraw the icon
            crate::interop::task_bar_list::TaskBarList::set_overlay_icon(hwnd, 0, None);
        }

        /// The DPI of the window.
        pub(crate) fn dpi(&self) -> u32 {
            self.dpi.get()
        }

        /// The window is gone (`WM_DESTROY`): its handle is forgotten and
        /// the window is no longer rooted by the list of windows.
        fn top_level_handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
            let handle = self.handle.borrow().clone()?;
            Some(Rc::new(TopLevelHandle(handle)))
        }

        fn create_drop_target(&self, input_root: Rc<dyn IInputRoot>) {
            use ferroui_base::input::raw::IDragDropDevice;

            if let Some(drag_drop_device) = FerroLocator::current().get_service::<dyn IDragDropDevice>() {
                let odt = crate::ole_drop_target::OleDropTarget::new(self.this.clone(), input_root, drag_drop_device);

                let handle = self.top_level_handle();
                let registered = crate::ole_context::OleContext::current()
                    .is_some_and(|context| context.register_drag_drop(handle.as_deref(), Some(&odt)));
                if registered {
                    *self.drop_target.borrow_mut() = Some(odt);
                }
            }
        }

        /// Removes the drop target of the window, when it has one (the
        /// window is being destroyed).
        pub(crate) fn release_drop_target(&self) {
            let drop_target = self.drop_target.borrow_mut().take();
            if let Some(drop_target) = drop_target {
                let handle = self.top_level_handle();
                if let Some(context) = crate::ole_context::OleContext::current() {
                    context.unregister_drag_drop(handle.as_deref());
                }
                drop(drop_target);
            }
        }

        pub(crate) fn on_destroyed(&self) {
            self.touch_device.dispose();
            let hwnd = self.hwnd.replace(0);
            if let Some(handle) = self.handle.borrow().as_ref() {
                handle.hwnd.store(0, Ordering::Release);
            }
            WINDOWS.with(|windows| windows.borrow_mut().remove(&hwnd));
            // Remove root reference to this class
            let removed = INSTANCES.with(|instances| {
                let mut instances = instances.borrow_mut();
                let index = instances.iter().position(|window| std::ptr::eq(Rc::as_ptr(window), self))?;
                Some(instances.remove(index))
            });
            // The procedure that called this holds a reference of its own.
            drop(removed);
        }

        /// Asks the window to paint a rectangle of its client area.
        pub fn invalidate(&self, rect: Rect) {
            let r = invalidate_rect_from(rect, self.scaling.get());

            invalidate_rect(self.hwnd.get(), &r, false);
        }

        /// Transform a screen pixel point to the point in the client area.
        pub(crate) fn point_to_client_impl(&self, point: PixelPoint) -> Point {
            let p = screen_to_client(self.hwnd.get(), POINT { x: point.x, y: point.y });
            Point::new(f64::from(p.x), f64::from(p.y)) / self.scaling.get()
        }

        fn point_to_screen_impl(&self, point: Point) -> PixelPoint {
            let point = point * self.scaling.get();
            let p = client_to_screen(self.hwnd.get(), POINT { x: point.x as i32, y: point.y as i32 });
            PixelPoint::new(p.x, p.y)
        }

        fn set_parent_impl(&self, parent: Option<Rc<WindowImpl>>) {
            let old = self.parent.replace(parent.clone());
            drop(old);

            let mut parent_hwnd = parent.as_ref().map_or(0, |parent| parent.hwnd.get());

            if parent_hwnd == 0 && !self.window_properties.get().show_in_taskbar {
                parent_hwnd = OffscreenParentWindow::handle();
            }

            self.hidden_window_is_parent.set(parent_hwnd == OffscreenParentWindow::handle());

            set_window_long_ptr(self.hwnd.get(), WindowLongParam::GWL_HWNDPARENT, parent_hwnd);

            // Windows doesn't seem to respect the HWND_TOPMOST flag of a window when showing an owned window for the first time.
            // So we set the HWND_TOPMOST again before the owned window is shown. This only needs to be done once.
            if let Some(parent) = parent {
                parent.ensure_topmost();
            }
        }

        fn ensure_topmost(&self) {
            if self.topmost.get() {
                set_window_pos(
                    self.hwnd.get(),
                    WindowPosZOrder::HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SetWindowPosFlags::SWP_NOMOVE | SetWindowPosFlags::SWP_NOSIZE | SetWindowPosFlags::SWP_NOACTIVATE,
                );
            }
        }

        fn create_window_override(&self, atom: u16) -> isize {
            match &self.kind {
                WindowKind::Window => create_window_ex(
                    if self.use_redirection_bitmap { 0 } else { WindowStyles::WS_EX_NOREDIRECTIONBITMAP.bits() },
                    atom,
                    (WindowStyles::WS_OVERLAPPEDWINDOW | WindowStyles::WS_CLIPCHILDREN).bits(),
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    0,
                ),
                WindowKind::Popup(popup) => popup.create_window(atom),
                WindowKind::Embedded => crate::embedded_window_impl::create_window(atom),
            }
        }

        fn create_window(&self) {
            let class_name = format!(
                "Ferro-{}-{}-{:?}",
                std::process::id(),
                CLASS_COUNTER.fetch_add(1, Ordering::Relaxed),
                std::thread::current().id()
            );

            // Unique DC helps with performance when using Gpu based rendering
            let window_class_style = ClassStyles::CS_OWNDC | ClassStyles::CS_HREDRAW | ClassStyles::CS_VREDRAW;

            let atom = register_class_ex(&class_name, window_class_style.bits(), window_wnd_proc, default_cursor());

            if atom == 0 {
                panic!("The window class could not be registered (error code {})", get_last_error());
            }
            *self.class_name.borrow_mut() = Some(class_name);

            CREATING.with(|creating| *creating.borrow_mut() = Some(self.this.clone()));
            let hwnd = self.create_window_override(atom);
            CREATING.with(|creating| *creating.borrow_mut() = None);
            wnd_proc_guard::resume_pending();

            if hwnd == 0 {
                panic!("The window could not be created (error code {})", get_last_error());
            }
            self.hwnd.set(hwnd);
            WINDOWS.with(|windows| windows.borrow_mut().insert(hwnd, self.this.clone()));

            *self.handle.borrow_mut() = Some(Arc::new(WindowImplPlatformHandle {
                hwnd: AtomicIsize::new(hwnd),
                scaling_bits: AtomicU64::new(self.scaling.get().to_bits()),
            }));

            register_touch_window(hwnd);

            self.read_monitor_dpi();
        }

        fn wnd_proc_message_handler(&self, hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> isize {
            let callback = self.wnd_proc_hook_callback.borrow().clone();
            if let Some(callback) = callback {
                let mut handled = false;
                let ret = callback(hwnd, msg, w_param as isize, l_param, &mut handled);

                if handled {
                    return ret;
                }
            }

            self.wnd_proc(hwnd, msg, w_param, l_param)
        }

        /// The window procedure: a popup answers a few messages itself;
        /// with an extended client area the procedure of the custom
        /// caption is asked first; the procedure of the application
        /// handles the rest.
        fn wnd_proc(&self, hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> isize {
            if let WindowKind::Popup(popup) = &self.kind {
                if let Some(result) = popup.wnd_proc(msg) {
                    return result;
                }
            }

            let mut l_ret = 0;
            let mut call_dwp = true;

            if self.is_client_area_extended.get() {
                l_ret = self.custom_caption_proc(hwnd, msg, w_param, l_param, &mut call_dwp);
            }

            if call_dwp {
                l_ret = self.app_wnd_proc(hwnd, msg, w_param, l_param);
            }

            l_ret
        }

        /// Ported from https://github.com/chromium/chromium/blob/master/ui/views/win/fullscreen_handler.cc
        /// Method must only be called from inside update_window_properties.
        fn set_full_screen(&self, fullscreen: bool) {
            let hwnd = self.hwnd.get();
            if fullscreen {
                let current = self.get_style();
                let current_ex = self.get_extended_style();

                let placement = get_window_placement(hwnd);
                let is_minimized = placement.show_cmd == ShowWindowCommand::SHOW_MINIMIZED;
                let mut window_rect;
                let screen_bounds;

                // When minimized, we can't use GetWindowRect since the window is actually way outside the screen.
                // Instead, fall back to WINDOWPLACEMENT.NormalPosition (which is in working area coordinates).
                if is_minimized {
                    window_rect = placement.normal_position;
                    let screen = self.screen.screen_from_rect(window_rect.to_pixel_rect());
                    if let Some(screen) = &screen {
                        let working_area = screen.working_area();
                        window_rect.offset(working_area.x, working_area.y);
                    }
                    screen_bounds = screen.map(|screen| screen.bounds());
                } else {
                    window_rect = get_window_rect(hwnd);
                    screen_bounds = self
                        .screen
                        .screen_from_hwnd(hwnd, MONITOR::MONITOR_DEFAULTTONEAREST)
                        .map(|screen| (*screen).as_ref().bounds());
                }

                self.saved_window_info.set(SavedWindowInfo { window_rect, style: current, ex_style: current_ex });

                if current.contains(WindowStyles::WS_SYSMENU) {
                    // Create the system menu copy before fullscreen removes WS_SYSMENU.
                    get_system_menu(hwnd, false);
                }

                // Set new window style and size.
                self.set_style(current & !WindowStyles::WS_OVERLAPPEDWINDOW, false);
                self.set_extended_style(
                    current_ex
                        & !(WindowStyles::WS_EX_DLGMODALFRAME
                            | WindowStyles::WS_EX_WINDOWEDGE
                            | WindowStyles::WS_EX_CLIENTEDGE
                            | WindowStyles::WS_EX_STATICEDGE),
                    false,
                );

                // On expand, if we're given a window_rect, grow to it, otherwise do
                // not resize.
                if let Some(screen_bounds) = screen_bounds {
                    self.is_full_screen_active.set(true);

                    if is_minimized {
                        show_window(hwnd, ShowWindowCommand::RESTORE);
                    }

                    set_window_pos(
                        hwnd,
                        0,
                        screen_bounds.x,
                        screen_bounds.y,
                        screen_bounds.width,
                        screen_bounds.height,
                        SetWindowPosFlags::SWP_NOZORDER | SetWindowPosFlags::SWP_NOACTIVATE | SetWindowPosFlags::SWP_FRAMECHANGED,
                    );
                }
            } else {
                // Reset original window style and size.  The multiple window size/moves
                // here are ugly, but if SetWindowPos() doesn't redraw, the taskbar won't be
                // repainted.  Better-looking methods welcome.
                self.is_full_screen_active.set(false);

                let window_states = self.get_window_state_styles();
                let saved = self.saved_window_info.get();
                self.set_style((saved.style & !WINDOW_STATE_MASK) | window_states, false);
                self.set_extended_style(saved.ex_style, false);

                // On restore, resize to the previous saved rect size.
                let new_client_rect = saved.window_rect.to_pixel_rect();

                set_window_pos(
                    hwnd,
                    0,
                    new_client_rect.x,
                    new_client_rect.y,
                    new_client_rect.width,
                    new_client_rect.height,
                    SetWindowPosFlags::SWP_NOZORDER | SetWindowPosFlags::SWP_NOACTIVATE | SetWindowPosFlags::SWP_FRAMECHANGED,
                );

                self.update_window_properties(self.window_properties.get(), true);
            }

            crate::interop::task_bar_list::TaskBarList::mark_fullscreen(self.hwnd.get(), fullscreen);

            self.extend_client_area();
        }

        fn update_extend_margins(&self) -> MARGINS {
            let mut border_thickness = RECT::default();
            let mut border_caption_thickness = RECT::default();
            let style = self.get_style();

            self.adjust_window_rect(&mut border_caption_thickness, style, WindowStyles::empty());
            self.adjust_window_rect(&mut border_thickness, style & !WindowStyles::WS_CAPTION, WindowStyles::empty());

            border_thickness.left *= -1;
            border_thickness.top *= -1;
            border_caption_thickness.left *= -1;
            border_caption_thickness.top *= -1;

            if self.window_properties.get().decorations == WindowDecorations::Full {
                if self.extend_title_bar_hint.get() != -1.0 {
                    border_caption_thickness.top = (self.extend_title_bar_hint.get() * self.scaling.get()) as i32;
                }
            } else {
                border_caption_thickness.top = border_thickness.top;
            }

            //using a default margin of 0 when using WinUiComp removes artefacts when resizing. See issue #8316
            let default_margin = if self.use_redirection_bitmap { 1 } else { 0 };

            let margins = MARGINS {
                cx_left_width: default_margin,
                cx_right_width: default_margin,
                cy_bottom_height: default_margin,
                cy_top_height: default_margin,
            };

            if self.window_state_impl() == WindowState::Maximized {
                self.extended_margins.set(Thickness::new(
                    0.0,
                    f64::from(border_caption_thickness.top - border_thickness.top) / self.scaling.get(),
                    0.0,
                    0.0,
                ));
            } else {
                self.extended_margins.set(Thickness::new(
                    0.0,
                    f64::from(border_caption_thickness.top) / self.scaling.get(),
                    0.0,
                    0.0,
                ));
            }

            margins
        }

        fn update_window_corner_preference(&self) {
            if Win32Platform::windows_version() < Version::with_build(10, 0, 22000) {
                return;
            }

            dwm_set_window_attribute(
                self.hwnd.get(),
                DwmWindowAttribute::DWMWA_WINDOW_CORNER_PREFERENCE,
                self.corner_preference.get() as i32,
            );
        }

        pub(crate) fn extend_client_area(&self) {
            if !self.shown.get() {
                self.invoke_extend_client_area_to_decorations_changed(self.is_client_area_extended.get());
                return;
            }

            if dwm_is_composition_enabled() != Some(true) {
                self.is_client_area_extended.set(false);
                return;
            }
            let rc_window = get_window_rect(self.hwnd.get());

            if self.is_client_area_extended.get()
                && self.window_state_impl() != WindowState::FullScreen
                && self.get_style().contains(WindowStyles::WS_BORDER)
            {
                let margins = self.update_extend_margins();
                dwm_extend_frame_into_client_area(self.hwnd.get(), &margins);

                // Make sure that Windows still paints the non-client area so we get native borders and shadows.
                self.set_nc_rendering_policy(DwmNCRenderingPolicy::DWMNCRP_ENABLED);
            } else {
                let margins = MARGINS::default();
                dwm_extend_frame_into_client_area(self.hwnd.get(), &margins);

                self.extended_margins.set(Thickness::default());

                self.set_nc_rendering_policy(DwmNCRenderingPolicy::DWMNCRP_USEWINDOWSTYLE);
            }

            // Inform the application of the frame change.
            set_window_pos(
                self.hwnd.get(),
                0,
                rc_window.left,
                rc_window.top,
                0,
                0,
                SetWindowPosFlags::SWP_FRAMECHANGED | SetWindowPosFlags::SWP_NOACTIVATE | SetWindowPosFlags::SWP_NOSIZE,
            );

            self.invoke_extend_client_area_to_decorations_changed(self.is_client_area_extended.get());
        }

        fn set_nc_rendering_policy(&self, value: u32) {
            dwm_set_window_attribute(self.hwnd.get(), DwmWindowAttribute::DWMWA_NCRENDERING_POLICY, value as i32);
        }

        pub(crate) fn show_window(&self, state: WindowState, activate: bool) {
            if self.is_client_area_extended.get() {
                self.extend_client_area();
            }

            let mut new_window_properties = self.window_properties.get();

            let (command, is_full_screen) = show_window_command(state, is_window_visible(self.hwnd.get()), activate);
            new_window_properties.is_full_screen = is_full_screen;

            new_window_properties.window_state = state;

            self.update_window_properties(
                new_window_properties,
                new_window_properties.decorations != WindowDecorations::Full,
            );

            if let Some(command) = command {
                show_window(self.hwnd.get(), command);
            }

            if !Design::is_design_mode() && activate {
                set_focus(self.hwnd.get());
                set_foreground_window(self.hwnd.get());
            }
            wnd_proc_guard::resume_pending();
        }

        pub(crate) fn before_close_cleanup(&self, is_disposing: bool) {
            // Based on https://github.com/dotnet/wpf/blob/master/src/Microsoft.DotNet.Wpf/src/PresentationFramework/System/Windows/Window.cs#L4270-L4337
            // We need to enable parent window before destroying child window to prevent OS from activating a random window behind us (or last active window).
            // This is described here: https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enablewindow#remarks
            // We need to verify if parent is still alive (perhaps it got destroyed somehow).
            let parent = self.parent.borrow().clone();
            if let Some(parent) = parent {
                if is_window(parent.hwnd.get()) {
                    let was_active = get_active_window() == self.hwnd.get();

                    // We can only set enabled state if we are not disposing - generally Dispose happens after enabled state has been set.
                    // Ignoring this would cause us to enable a window that might be disabled.
                    if !is_disposing {
                        // Our window closed callback will set enabled state to a correct value after child window gets destroyed.
                        enable_window(parent.hwnd.get(), true);
                    }

                    // We also need to activate our parent window since again OS might try to activate a window behind if it is not set.
                    if was_active {
                        set_active_window(parent.hwnd.get());
                    }
                }
            }
        }

        pub(crate) fn after_close_cleanup(&self) {
            let class_name = self.class_name.borrow_mut().take();
            if let Some(class_name) = class_name {
                unregister_class(&class_name);
            }
        }

        fn get_window_state_styles(&self) -> WindowStyles {
            self.get_style() & WINDOW_STATE_MASK
        }

        pub(crate) fn get_style(&self) -> WindowStyles {
            if self.is_full_screen_active.get() {
                self.saved_window_info.get().style
            } else {
                WindowStyles::from_bits_retain(get_window_long(self.hwnd.get(), WindowLongParam::GWL_STYLE))
            }
        }

        fn get_extended_style(&self) -> WindowStyles {
            if self.is_full_screen_active.get() {
                self.saved_window_info.get().ex_style
            } else {
                WindowStyles::from_bits_retain(get_window_long(self.hwnd.get(), WindowLongParam::GWL_EXSTYLE))
            }
        }

        fn set_style(&self, style: WindowStyles, save: bool) {
            if save {
                let mut saved = self.saved_window_info.get();
                saved.style = style;
                self.saved_window_info.set(saved);
            }

            if !self.is_full_screen_active.get() {
                set_window_long(self.hwnd.get(), WindowLongParam::GWL_STYLE, style.bits());
            }
        }

        fn set_extended_style(&self, style: WindowStyles, save: bool) {
            if save {
                let mut saved = self.saved_window_info.get();
                saved.ex_style = style;
                self.saved_window_info.set(saved);
            }

            if !self.is_full_screen_active.get() {
                set_window_long(self.hwnd.get(), WindowLongParam::GWL_EXSTYLE, style.bits());
            }
        }

        pub(crate) fn update_window_properties(&self, new_properties: WindowProperties, force_changes: bool) {
            let old_properties = self.window_properties.get();
            let hwnd = self.hwnd.get();

            // Calling SetWindowPos will cause events to be sent and we need to respond
            // according to the new values already.
            self.window_properties.set(new_properties);

            if old_properties.is_full_screen == new_properties.is_full_screen {
                if (old_properties.show_in_taskbar != new_properties.show_in_taskbar) || force_changes {
                    if new_properties.show_in_taskbar {
                        if self.hidden_window_is_parent.get() {
                            // Can't enable the taskbar icon by clearing the parent window unless the window
                            // is hidden. Hide the window and show it again with the same activation state
                            // when we've finished. Interestingly it seems to work fine the other way.
                            let shown = is_window_visible(hwnd);
                            let activated = get_active_window() == hwnd;

                            if shown {
                                self.hide_impl();
                            }

                            self.hidden_window_is_parent.set(false);
                            self.set_parent_impl(None);

                            if shown {
                                self.show_impl(activated, false);
                            }
                        }
                    } else {
                        // To hide a non-owned window's taskbar icon we need to parent it to a hidden window.
                        if self.parent.borrow().is_none() {
                            set_window_long_ptr(hwnd, WindowLongParam::GWL_HWNDPARENT, OffscreenParentWindow::handle());
                            self.hidden_window_is_parent.set(true);
                        }
                    }
                }

                let (mut style, mut ex_style) = window_styles_from_properties(
                    &new_properties,
                    self.use_redirection_bitmap,
                    matches!(self.kind, WindowKind::Embedded),
                    is_window_visible(hwnd),
                    self.get_window_state_styles(),
                );

                let mut saved = self.saved_window_info.get();
                saved.style = style;
                saved.ex_style = ex_style;
                self.saved_window_info.set(saved);

                let callback = self.window_styles_callback.borrow().clone();
                if let Some(callback) = callback {
                    let (s, e) = callback(style.bits(), ex_style.bits());

                    style = WindowStyles::from_bits_retain(s);
                    ex_style = WindowStyles::from_bits_retain(e);
                }

                self.set_style(style, true);
                self.set_extended_style(ex_style, true);
            } else {
                self.set_full_screen(new_properties.is_full_screen);
            }

            if !self.is_full_screen_active.get()
                && ((old_properties.decorations != new_properties.decorations) || force_changes)
            {
                let margin = if new_properties.decorations == WindowDecorations::BorderOnly { 1 } else { 0 };

                let margins = MARGINS {
                    cy_bottom_height: margin,
                    cx_right_width: margin,
                    cx_left_width: margin,
                    cy_top_height: margin,
                };

                dwm_extend_frame_into_client_area(hwnd, &margins);

                if self.shown.get() || force_changes {
                    set_window_pos(
                        hwnd,
                        0,
                        0,
                        0,
                        0,
                        0,
                        SetWindowPosFlags::SWP_NOZORDER
                            | SetWindowPosFlags::SWP_NOACTIVATE
                            | SetWindowPosFlags::SWP_NOSIZE
                            | SetWindowPosFlags::SWP_NOMOVE
                            | SetWindowPosFlags::SWP_FRAMECHANGED,
                    );
                }
            }

            // Ensure window state if decorations change
            if self.shown.get() && old_properties.decorations != new_properties.decorations {
                self.show_window(self.window_state_impl(), false);
            }
        }

        fn client_rect_to_window_rect(
            &self,
            mut client_rect: RECT,
            style_override: Option<WindowStyles>,
            extended_style_override: Option<WindowStyles>,
        ) -> RECT {
            let style = style_override.unwrap_or_else(|| self.get_style());
            let extended_style = extended_style_override.unwrap_or_else(|| self.get_extended_style());

            self.adjust_window_rect(&mut client_rect, style, extended_style);

            client_rect
        }

        /// Grows a client rectangle to the window rectangle of the given
        /// styles at the DPI of the window (the window rectangle adjuster
        /// of the reference): with the function of the system that takes a
        /// DPI where it exists, and otherwise with the frame of the primary
        /// screen scaled to the window.
        pub(crate) fn adjust_window_rect(&self, rect: &mut RECT, style: WindowStyles, ex_style: WindowStyles) {
            if Win32Platform::windows_version() >= PlatformConstants::WINDOWS10_1607 {
                let dpi = (self.scaling.get() * STANDARD_DPI) as u32;
                if adjust_window_rect_ex_for_dpi(rect, style.bits(), false, ex_style.bits(), dpi).is_some() {
                    return;
                }
            }

            let primary_scaling =
                self.screen.all_screens().iter().find(|screen| screen.is_primary()).map_or(1.0, |screen| screen.scaling());
            let relative_scaling = self.scaling.get() / primary_scaling;

            adjust_window_rect_ex(rect, style.bits(), false, ex_style.bits());
            rect.top = (f64::from(rect.top) * relative_scaling) as i32;
            rect.right = (f64::from(rect.right) * relative_scaling) as i32;
            rect.left = (f64::from(rect.left) * relative_scaling) as i32;
            rect.bottom = (f64::from(rect.bottom) * relative_scaling) as i32;
        }

        fn hide_impl(&self) {
            show_window(self.hwnd.get(), ShowWindowCommand::HIDE);
            wnd_proc_guard::resume_pending();
        }

        fn show_impl(&self, activate: bool, _is_dialog: bool) {
            if matches!(self.kind, WindowKind::Popup(_)) {
                // Popups are always shown non-activated.
                show_window(self.hwnd.get(), ShowWindowCommand::SHOW_NO_ACTIVATE);
                wnd_proc_guard::resume_pending();
                return;
            }

            // Read in a statement of its own: the borrow of an argument lasts until the call
            // returns, and the call replaces what the cell holds.
            let parent = self.parent.borrow().clone();
            self.set_parent_impl(parent);
            self.show_window(self.show_window_state.get(), activate);
        }

        pub(crate) fn is_our_window_global(hwnd: isize) -> bool {
            if hwnd == 0 {
                return false;
            }

            INSTANCES.with(|instances| instances.borrow().iter().any(|window| window.hwnd.get() == hwnd))
        }

        fn update_properties(&self, change: impl FnOnce(&mut WindowProperties)) {
            let mut new_window_properties = self.window_properties.get();

            change(&mut new_window_properties);

            self.update_window_properties(new_window_properties, false);
            wnd_proc_guard::resume_pending();
        }
    }

    /// The window procedure of every window class of this backend.
    unsafe extern "system" fn window_wnd_proc(
        hwnd: windows_sys::Win32::Foundation::HWND,
        msg: u32,
        w_param: usize,
        l_param: isize,
    ) -> isize {
        let hwnd = hwnd_to_isize(hwnd);
        let handled = wnd_proc_guard::guard(None, || {
            let mut window = WINDOWS.with(|windows| windows.borrow().get(&hwnd).and_then(Weak::upgrade));
            if window.is_none() {
                // The first messages of a window arrive while it is being
                // created, before its handle is known to the window.
                let creating = CREATING.with(|creating| creating.borrow().clone());
                if let Some(creating) = creating {
                    WINDOWS.with(|windows| windows.borrow_mut().insert(hwnd, creating.clone()));
                    window = creating.upgrade();
                }
            }

            let result = window.map(|window| window.wnd_proc_message_handler(hwnd, msg, w_param, l_param));

            if msg == WindowsMessage::WM_NCDESTROY {
                WINDOWS.with(|windows| windows.borrow_mut().remove(&hwnd));
            }

            result
        });

        match handled {
            Some(result) => result,
            None => def_window_proc(hwnd, msg, w_param, l_param),
        }
    }

    impl Imm32Parent for WindowImpl {
        fn desktop_scaling(&self) -> f64 {
            self.scaling.get()
        }

        fn set_ignore_wm_char(&self, value: bool) {
            self.ignore_wm_char.set(value);
        }

        fn raw_text_input(&self, timestamp: u64, text: &str) -> bool {
            let Some(input) = self.input_callback() else {
                return false;
            };
            let device: Rc<dyn IInputDevice> = WindowsKeyboardDevice::instance();
            input(Rc::new(RawTextInputEventArgs::new(device, timestamp, self.owner(), text.to_owned())));
            true
        }

        fn raw_key_press(&self, key: Key, physical_key: PhysicalKey) {
            let Some(input) = self.input_callback() else {
                return;
            };
            // The reference stamps the two events with the ticks of the
            // clock; here they carry the time of the message being
            // processed, like every other event of the window.
            let timestamp = u64::from(get_message_time() as u32);
            for event_type in [RawKeyEventType::KeyDown, RawKeyEventType::KeyUp] {
                let device: Rc<dyn IInputDevice> = WindowsKeyboardDevice::instance();
                input(Rc::new(RawKeyEventArgs::new(
                    device,
                    timestamp,
                    self.owner(),
                    event_type,
                    key,
                    RawInputModifiers::NONE,
                    physical_key,
                    None,
                    KeyDeviceType::Keyboard,
                )));
            }
        }
    }

    impl IOptionalFeatureProvider for WindowImpl {
        /// The optional features of a window. The launcher, whose
        /// implementation arrives with a later stage, is absent.
        fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
            if feature_type == TypeId::of::<dyn ITextInputMethodImpl>() {
                let input_method: Rc<dyn ITextInputMethodImpl> = Imm32InputMethod::current();
                return Some(Rc::new(input_method));
            }

            if feature_type == TypeId::of::<dyn IInputPane>() {
                let input_pane: Rc<dyn IInputPane> = self.input_pane.borrow().clone()?;
                return Some(Rc::new(input_pane));
            }

            if feature_type == TypeId::of::<dyn IScreenImpl>() {
                let screens: Rc<dyn IScreenImpl> = self.screen.clone();
                return Some(Rc::new(screens));
            }

            if feature_type == TypeId::of::<dyn ferroui_base::platform::storage::IStorageProvider>() {
                let storage_provider: Rc<dyn ferroui_base::platform::storage::IStorageProvider> = self
                    .storage_provider
                    .borrow_mut()
                    .get_or_insert_with(|| Rc::new(crate::win32_storage_provider::Win32StorageProvider::new(self.hwnd.get())))
                    .clone();
                return Some(Rc::new(storage_provider));
            }

            if feature_type == TypeId::of::<dyn ferroui_controls::platform::INativeControlHostImpl>() {
                let native_control_host: Rc<dyn ferroui_controls::platform::INativeControlHostImpl> = self
                    .native_control_host
                    .borrow_mut()
                    .get_or_insert_with(|| {
                        crate::win32_native_control_host::Win32NativeControlHost::new(self.this.clone(), !self.use_redirection_bitmap)
                    })
                    .clone();
                return Some(Rc::new(native_control_host));
            }

            if feature_type == TypeId::of::<dyn IClipboard>() {
                let clipboard = FerroLocator::current().get_required_service::<dyn IClipboard>();
                return Some(Rc::new(clipboard));
            }

            None
        }
    }

    impl IDisposable for WindowImpl {
        fn dispose(&self) {
            self.dispose_impl();
        }
    }

    impl ITopLevelImpl for WindowImpl {
        fn desktop_scaling(&self) -> f64 {
            self.scaling.get()
        }

        fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
            let handle = self.handle.borrow().clone()?;
            Some(Rc::new(TopLevelHandle(handle)))
        }

        fn client_size(&self) -> Size {
            self.client_size_impl()
        }

        fn render_scaling(&self) -> f64 {
            self.scaling.get()
        }

        fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
            let mut surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = Vec::new();
            if let Some(handle) = self.handle.borrow().clone() {
                surfaces.push(handle);
            }
            if let Some(gl_surface) = self.gl_surface.borrow().clone() {
                surfaces.push(gl_surface);
            }
            if let Some(framebuffer) = self.framebuffer.borrow().clone() {
                surfaces.push(framebuffer);
            }
            surfaces
        }

        fn compositor(&self) -> Option<Rc<Compositor>> {
            Some(Win32Platform::compositor())
        }

        fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
            call(&self.input)
        }

        fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
            set(&self.input, value);
        }

        fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
            call(&self.paint)
        }

        fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
            set(&self.paint, value);
        }

        fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
            call(&self.resized)
        }

        fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
            set(&self.resized, value);
        }

        fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
            call(&self.scaling_changed)
        }

        fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
            set(&self.scaling_changed, value);
        }

        fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
            call(&self.transparency_level_changed)
        }

        fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
            set(&self.transparency_level_changed, value);
        }

        fn platform_specific_scene_info(&self) -> Option<Arc<dyn Any + Send + Sync>> {
            Some(Arc::new(self.scene_info.get()))
        }

        fn platform_specific_scene_info_changed(&self) -> Option<Rc<dyn Fn(Option<Arc<dyn Any + Send + Sync>>)>> {
            call(&self.platform_specific_scene_info_changed)
        }

        fn set_platform_specific_scene_info_changed(&self, value: Option<Rc<dyn Fn(Option<Arc<dyn Any + Send + Sync>>)>>) {
            set(&self.platform_specific_scene_info_changed, value);
        }

        /// Sets the input root, and registers the window as a drop target
        /// for it.
        fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
            let old = self.owner.replace(Some(input_root.clone()));
            drop(old);
            self.create_drop_target(input_root);
        }

        fn point_to_client(&self, point: PixelPoint) -> Point {
            self.point_to_client_impl(point)
        }

        fn point_to_screen(&self, point: Point) -> PixelPoint {
            self.point_to_screen_impl(point)
        }

        fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
            let h_cursor = cursor
                .as_ref()
                .and_then(|cursor| cursor.as_any().downcast_ref::<CursorImpl>())
                .map_or_else(default_cursor, CursorImpl::handle);
            set_class_long_ptr(self.hwnd.get(), ClassLongIndex::GCLP_HCURSOR, h_cursor);

            crate::interop::unmanaged_methods::set_cursor(h_cursor);
        }

        fn closed(&self) -> Option<Rc<dyn Fn()>> {
            call(&self.closed)
        }

        fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
            set(&self.closed, value);
        }

        fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
            call(&self.lost_focus)
        }

        fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
            set(&self.lost_focus, value);
        }

        fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
            if Win32Platform::use_overlay_popups() {
                None
            } else {
                let parent: Rc<dyn IWindowBaseImpl> = self.this()?;
                let popup: Rc<dyn IPopupImpl> = crate::popup_impl::PopupImpl::new(parent);
                Some(popup)
            }
        }

        fn set_transparency_level_hint(&self, transparency_levels: &[WindowTransparencyLevel]) {
            if transparency_levels.len() == 1 && transparency_levels[0] == WindowTransparencyLevel::none() {
                // Explicitly disable transparency. Ignore the UseRedirectionBitmap property.
                self.set_transparency_level(WindowTransparencyLevel::none());
                return;
            }

            for &level in transparency_levels {
                if !self.is_supported(level) {
                    continue;
                }

                if level == self.transparency_level.get() {
                    return;
                }
                if level == WindowTransparencyLevel::transparent() {
                    if !self.set_transparency_transparent() {
                        continue;
                    }
                } else if level == WindowTransparencyLevel::acrylic_blur() {
                    if !self.set_transparency_acrylic_blur() {
                        continue;
                    }
                } else if level == WindowTransparencyLevel::mica() && !self.set_transparency_mica() {
                    continue;
                }

                self.set_transparency_level(level);
                return;
            }

            // If we get here, we didn't find a supported level. Report the default.
            self.set_transparency_level(self.default_transparency_level);
        }

        fn transparency_level(&self) -> WindowTransparencyLevel {
            self.transparency_level.get()
        }

        fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
            AcrylicPlatformCompensationLevels::new(1.0, 0.8, 0.0)
        }

        fn set_frame_theme_variant(&self, theme_variant: Option<PlatformThemeVariant>) {
            let current = theme_variant.unwrap_or_else(|| {
                FerroLocator::current().get_required_service::<dyn IPlatformSettings>().get_color_values().theme_variant()
            });
            self.current_theme_variant.set(current);
            if self.scene_info.get().theme_variant != current {
                let scene_info = Win32TopLevelSceneInfo::new(current);
                self.scene_info.set(scene_info);
                if let Some(callback) = call(&self.platform_specific_scene_info_changed) {
                    callback(Some(Arc::new(scene_info)));
                }
            }
            if Win32Platform::windows_version().build >= 22000 {
                dwm_set_window_attribute(
                    self.hwnd.get(),
                    DwmWindowAttribute::DWMWA_USE_IMMERSIVE_DARK_MODE,
                    i32::from(current == PlatformThemeVariant::Dark),
                );
                if self.transparency_level.get() == WindowTransparencyLevel::mica() {
                    self.set_transparency_mica();
                }
            }
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_window_base_impl(&self) -> Option<&dyn IWindowBaseImpl> {
            Some(self)
        }

        fn as_window_impl(&self) -> Option<&dyn IWindowImpl> {
            Some(self)
        }

        fn as_popup_impl(&self) -> Option<&dyn IPopupImpl> {
            match self.kind {
                WindowKind::Popup(_) => Some(self),
                _ => None,
            }
        }

        fn as_win32_options_top_level_impl(&self) -> Option<&dyn IWin32OptionsTopLevelImpl> {
            Some(self)
        }
    }

    impl IWindowBaseImpl for WindowImpl {
        fn frame_size(&self) -> Option<Size> {
            Some(self.frame_size_impl())
        }

        fn show(&self, activate: bool, is_dialog: bool) {
            self.show_impl(activate, is_dialog);
        }

        fn hide(&self) {
            self.hide_impl();
        }

        fn position(&self) -> PixelPoint {
            self.position_impl()
        }

        fn position_changed(&self) -> Option<Rc<dyn Fn(PixelPoint)>> {
            call(&self.position_changed)
        }

        fn set_position_changed(&self, value: Option<Rc<dyn Fn(PixelPoint)>>) {
            set(&self.position_changed, value);
        }

        fn activate(&self) {
            set_foreground_window(self.hwnd.get());
        }

        fn deactivated(&self) -> Option<Rc<dyn Fn()>> {
            call(&self.deactivated)
        }

        fn set_deactivated(&self, value: Option<Rc<dyn Fn()>>) {
            set(&self.deactivated, value);
        }

        fn activated(&self) -> Option<Rc<dyn Fn()>> {
            call(&self.activated)
        }

        fn set_activated(&self, value: Option<Rc<dyn Fn()>>) {
            set(&self.activated, value);
        }

        fn max_auto_size_hint(&self) -> Size {
            if let WindowKind::Popup(popup) = &self.kind {
                return popup.max_auto_size_hint(self);
            }

            let max_track_size = self.max_track_size.get();
            Size::new(
                f64::from(max_track_size.x) / self.scaling.get(),
                f64::from(max_track_size.y) / self.scaling.get(),
            )
        }

        fn set_topmost(&self, value: bool) {
            if value == self.topmost.get() {
                return;
            }

            let hwnd_insert_after = if value { WindowPosZOrder::HWND_TOPMOST } else { WindowPosZOrder::HWND_NOTOPMOST };
            set_window_pos(
                self.hwnd.get(),
                hwnd_insert_after,
                0,
                0,
                0,
                0,
                SetWindowPosFlags::SWP_NOMOVE | SetWindowPosFlags::SWP_NOSIZE | SetWindowPosFlags::SWP_NOACTIVATE,
            );

            self.topmost.set(value);
        }
    }

    impl IWindowImpl for WindowImpl {
        fn window_state(&self) -> WindowState {
            self.window_state_impl()
        }

        fn set_window_state(&self, value: WindowState) {
            self.set_window_state_impl(value);
        }

        fn window_state_getter_is_usable(&self) -> bool {
            false
        }

        fn window_state_changed(&self) -> Option<Rc<dyn Fn(WindowState)>> {
            call(&self.window_state_changed)
        }

        fn set_window_state_changed(&self, value: Option<Rc<dyn Fn(WindowState)>>) {
            set(&self.window_state_changed, value);
        }

        fn set_title(&self, title: Option<&str>) {
            set_window_text(self.hwnd.get(), title);
        }

        fn set_parent(&self, parent: Option<Rc<dyn IWindowImpl>>) {
            let parent = parent
                .as_ref()
                .and_then(|parent| parent.as_any().downcast_ref::<WindowImpl>())
                .and_then(WindowImpl::this);
            self.set_parent_impl(parent);
        }

        fn set_enabled(&self, enable: bool) {
            enable_window(self.hwnd.get(), enable);
        }

        fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
            call(&self.got_input_when_disabled)
        }

        fn set_got_input_when_disabled(&self, value: Option<Rc<dyn Fn()>>) {
            set(&self.got_input_when_disabled, value);
        }

        fn set_window_decorations(&self, enabled: WindowDecorations) {
            self.update_properties(|properties| properties.decorations = enabled);
        }

        /// # Panics
        /// Panics for an icon: the icons of the backend (`IconImpl`,
        /// `Win32Icon`) are stage 2, and no icon of this backend can exist
        /// before it.
        fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>) {
            self.set_icon_impl(icon);
        }

        fn show_taskbar_icon(&self, value: bool) {
            self.update_properties(|properties| properties.show_in_taskbar = value);
        }

        fn can_resize(&self, value: bool) {
            self.update_properties(|properties| properties.is_resizable = value);
        }

        fn set_can_minimize(&self, value: bool) {
            self.update_properties(|properties| properties.is_minimizable = value);
        }

        fn set_can_maximize(&self, value: bool) {
            self.update_properties(|properties| properties.is_maximizable = value);
        }

        fn closing(&self) -> Option<Rc<dyn Fn(WindowCloseReason) -> bool>> {
            call(&self.closing)
        }

        fn set_closing(&self, value: Option<Rc<dyn Fn(WindowCloseReason) -> bool>>) {
            set(&self.closing, value);
        }

        fn is_client_area_extended_to_decorations(&self) -> bool {
            self.is_client_area_extended.get()
        }

        fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>> {
            call(&self.extend_client_area_to_decorations_changed)
        }

        fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>) {
            set(&self.extend_client_area_to_decorations_changed, value);
        }

        fn needs_managed_decorations(&self) -> bool {
            self.is_client_area_extended.get()
        }

        fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration {
            if self.is_client_area_extended.get() {
                PlatformRequestedDrawnDecoration::TITLE_BAR
            } else {
                PlatformRequestedDrawnDecoration::NONE
            }
        }

        fn extended_margins(&self) -> Thickness {
            self.extended_margins.get()
        }

        fn off_screen_margin(&self) -> Thickness {
            Thickness::default()
        }

        fn begin_move_drag(&self, e: &PointerPressedEventArgs) {
            e.pointer().capture(None);

            if !e.pointer().is_primary() {
                panic!("BeginMoveDrag Failed");
            }

            let hwnd = self.hwnd.get();
            Dispatcher::ui_thread().post_local(
                move || {
                    // SendMessage's return value is dependent on the message send.  WM_SYSCOMMAND
                    // and WM_LBUTTONUP return value just signify whether the WndProc handled the
                    // message or not, so they are not interesting

                    send_message(hwnd, WindowsMessage::WM_SYSCOMMAND, SC_MOUSEMOVE as usize, 0);
                    send_message(hwnd, WindowsMessage::WM_LBUTTONUP, 0, 0);
                },
                DispatcherPriority::SEND,
            );
        }

        fn begin_resize_drag(&self, edge: WindowEdge, e: &PointerPressedEventArgs) {
            if self.window_properties.get().is_resizable {
                e.pointer().capture(None);
                def_window_proc(self.hwnd.get(), WindowsMessage::WM_NCLBUTTONDOWN, hit_test_from_edge(edge) as usize, 0);
            }
        }

        fn resize(&self, client_size: Size, reason: WindowResizeReason) {
            self.resize_impl(client_size, reason);
        }

        fn move_(&self, point: PixelPoint) {
            self.set_position(point);
        }

        fn set_min_max_size(&self, min_size: Size, max_size: Size) {
            self.min_size.set(min_size);
            self.max_size.set(max_size);
        }

        fn set_extend_client_area_to_decorations_hint(&self, extend_into_client_area_hint: bool) {
            self.is_client_area_extended.set(extend_into_client_area_hint);

            self.extend_client_area();
        }

        fn set_extend_client_area_title_bar_height_hint(&self, title_bar_height: f64) {
            self.extend_title_bar_hint.set(title_bar_height);

            self.extend_client_area();
        }
    }

    impl IWin32OptionsTopLevelImpl for WindowImpl {
        fn window_styles_callback(&self) -> Option<CustomWindowStylesCallback> {
            self.window_styles_callback.borrow().clone()
        }

        fn set_window_styles_callback(&self, value: Option<CustomWindowStylesCallback>) {
            let old = self.window_styles_callback.replace(value);
            drop(old);
        }

        fn wnd_proc_hook_callback(&self) -> Option<CustomWndProcHookCallback> {
            self.wnd_proc_hook_callback.borrow().clone()
        }

        fn set_wnd_proc_hook_callback(&self, value: Option<CustomWndProcHookCallback>) {
            let old = self.wnd_proc_hook_callback.replace(value);
            drop(old);
        }

        fn set_window_corner_preference(&self, preference: WindowCornerPreference) {
            self.corner_preference.set(preference);
            self.update_window_corner_preference();
        }
    }

    impl IPopupImpl for WindowImpl {
        fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>> {
            match &self.kind {
                WindowKind::Popup(popup) => popup.popup_positioner(),
                _ => None,
            }
        }

        fn set_window_manager_add_shadow_hint(&self, enabled: bool) {
            if let WindowKind::Popup(popup) = &self.kind {
                popup.set_window_manager_add_shadow_hint(self.hwnd.get(), enabled);
            }
        }

        fn take_focus(&self) {
            if let WindowKind::Popup(popup) = &self.kind {
                popup.take_focus();
            }
        }

        fn set_hit_test_visible(&self, is_hit_test_visible: bool) {
            if let WindowKind::Popup(popup) = &self.kind {
                popup.set_hit_test_visible(is_hit_test_visible);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn properties() -> WindowProperties {
        WindowProperties {
            show_in_taskbar: true,
            is_resizable: true,
            is_minimizable: true,
            is_maximizable: true,
            decorations: WindowDecorations::Full,
            is_full_screen: false,
            window_state: WindowState::Normal,
        }
    }

    fn styles(properties: &WindowProperties) -> (WindowStyles, WindowStyles) {
        window_styles_from_properties(properties, true, false, false, WindowStyles::empty())
    }

    #[test]
    fn a_window_with_full_decorations_has_the_styles_of_an_overlapped_window() {
        let (style, ex_style) = styles(&properties());
        assert_eq!(
            style,
            WindowStyles::WS_OVERLAPPEDWINDOW | WindowStyles::WS_CLIPCHILDREN | WindowStyles::WS_CLIPSIBLINGS
        );
        assert_eq!(ex_style, WindowStyles::WS_EX_WINDOWEDGE | WindowStyles::WS_EX_APPWINDOW);
    }

    #[test]
    fn the_decorations_choose_the_frame() {
        let mut p = properties();
        p.decorations = WindowDecorations::BorderOnly;
        let (style, _) = styles(&p);
        assert!(style.contains(WindowStyles::WS_BORDER | WindowStyles::WS_THICKFRAME));
        assert!(!style.contains(WindowStyles::WS_CAPTION));
        assert!(!style.contains(WindowStyles::WS_SYSMENU));

        p.decorations = WindowDecorations::None;
        let (style, _) = styles(&p);
        assert!(!style.intersects(WindowStyles::WS_BORDER | WindowStyles::WS_THICKFRAME | WindowStyles::WS_SYSMENU));
        // The boxes are properties of the window, not of its frame.
        assert!(style.contains(WindowStyles::WS_MINIMIZEBOX | WindowStyles::WS_MAXIMIZEBOX));
    }

    #[test]
    fn the_abilities_of_a_window_choose_its_boxes_and_its_sizing_border() {
        let mut p = properties();
        p.is_resizable = false;
        p.is_minimizable = false;
        p.is_maximizable = false;
        let (style, _) = styles(&p);
        assert!(!style.intersects(WindowStyles::WS_THICKFRAME | WindowStyles::WS_MINIMIZEBOX | WindowStyles::WS_MAXIMIZEBOX));
        assert!(style.contains(WindowStyles::WS_CAPTION | WindowStyles::WS_SYSMENU));

        // A maximized window that can be resized keeps the box that
        // restores it.
        p.is_resizable = true;
        p.window_state = WindowState::Maximized;
        let (style, _) = styles(&p);
        assert!(style.contains(WindowStyles::WS_MAXIMIZEBOX));
    }

    #[test]
    fn the_taskbar_button_is_an_extended_style() {
        let mut p = properties();
        p.show_in_taskbar = false;
        let (_, ex_style) = styles(&p);
        assert_eq!(ex_style, WindowStyles::WS_EX_WINDOWEDGE);

        let (_, ex_style) = window_styles_from_properties(&properties(), false, false, false, WindowStyles::empty());
        assert!(ex_style.contains(WindowStyles::WS_EX_NOREDIRECTIONBITMAP));
    }

    #[test]
    fn what_the_window_is_now_is_kept() {
        let (style, _) = window_styles_from_properties(&properties(), true, false, true, WindowStyles::WS_MAXIMIZE);
        assert!(style.contains(WindowStyles::WS_VISIBLE | WindowStyles::WS_MAXIMIZE));
        assert!(!style.contains(WindowStyles::WS_MINIMIZE));

        // Only the state bits of the current style are taken over.
        let (style, _) =
            window_styles_from_properties(&properties(), true, false, false, WindowStyles::WS_MINIMIZE | WindowStyles::WS_DISABLED);
        assert!(style.contains(WindowStyles::WS_MINIMIZE));
        assert!(!style.contains(WindowStyles::WS_DISABLED));

        let (style, _) = window_styles_from_properties(&properties(), true, true, false, WindowStyles::empty());
        assert!(style.contains(WindowStyles::WS_CHILD));
    }

    #[test]
    fn the_command_a_state_is_shown_with() {
        assert_eq!(show_window_command(WindowState::Minimized, false, true), (Some(ShowWindowCommand::MINIMIZE), false));
        assert_eq!(show_window_command(WindowState::Maximized, true, true), (Some(ShowWindowCommand::MAXIMIZE), false));

        assert_eq!(show_window_command(WindowState::Normal, true, false), (Some(ShowWindowCommand::RESTORE), false));
        assert_eq!(show_window_command(WindowState::Normal, false, true), (Some(ShowWindowCommand::NORMAL), false));
        assert_eq!(
            show_window_command(WindowState::Normal, false, false),
            (Some(ShowWindowCommand::SHOW_NO_ACTIVATE), false)
        );

        // A visible window goes full screen by its styles and its
        // position alone; a hidden one is shown restored first.
        assert_eq!(show_window_command(WindowState::FullScreen, true, true), (None, true));
        assert_eq!(show_window_command(WindowState::FullScreen, false, true), (Some(ShowWindowCommand::RESTORE), true));
    }

    #[test]
    fn the_state_of_a_placement() {
        assert_eq!(window_state_from_show_command(ShowWindowCommand::MAXIMIZE), WindowState::Maximized);
        assert_eq!(window_state_from_show_command(ShowWindowCommand::MINIMIZE), WindowState::Minimized);
        assert_eq!(window_state_from_show_command(ShowWindowCommand::NORMAL), WindowState::Normal);
        assert_eq!(window_state_from_show_command(ShowWindowCommand::HIDE), WindowState::Normal);
    }

    fn placement(show_cmd: i32, flags: u32) -> WINDOWPLACEMENT {
        WINDOWPLACEMENT {
            show_cmd,
            flags,
            normal_position: RECT { left: 100, top: 50, right: 900, bottom: 650 },
            ..WINDOWPLACEMENT::default()
        }
    }

    #[test]
    fn a_resize_changes_the_restored_size_and_keeps_the_position() {
        let result = placement_for_resize(placement(ShowWindowCommand::NORMAL, 0), 1000, 700, true, WindowState::Normal)
            .expect("the size changes");
        assert_eq!(result.normal_position, RECT { left: 100, top: 50, right: 1100, bottom: 750 });
        assert_eq!(result.show_cmd, ShowWindowCommand::SHOW_NO_ACTIVATE);

        // A window that was never shown stays hidden.
        let result = placement_for_resize(placement(ShowWindowCommand::NORMAL, 0), 1000, 700, false, WindowState::Maximized)
            .expect("the size changes");
        assert_eq!(result.show_cmd, ShowWindowCommand::HIDE);

        let result = placement_for_resize(placement(ShowWindowCommand::NORMAL, 0), 1000, 700, true, WindowState::Maximized)
            .expect("the size changes");
        assert_eq!(result.show_cmd, ShowWindowCommand::SHOW_MAXIMIZED);
    }

    #[test]
    fn a_resize_to_the_restored_size_changes_nothing() {
        assert_eq!(placement_for_resize(placement(ShowWindowCommand::NORMAL, 0), 800, 600, true, WindowState::Normal), None);
    }

    #[test]
    fn a_minimized_window_that_restores_to_maximized_keeps_its_restored_size() {
        let minimized = placement(ShowWindowCommand::SHOW_MINIMIZED, WindowPlacementFlags::RESTORE_TO_MAXIMIZED.bits());
        assert_eq!(placement_for_resize(minimized, 1000, 700, true, WindowState::Minimized), None);

        let minimized = placement(ShowWindowCommand::SHOW_MINIMIZED, 0);
        let result = placement_for_resize(minimized, 1000, 700, true, WindowState::Minimized).expect("the size changes");
        assert_eq!(result.show_cmd, ShowWindowCommand::SHOW_MIN_NO_ACTIVE);
    }

    #[test]
    fn an_invalidated_rectangle_covers_whole_device_pixels() {
        let rect = invalidate_rect_from(Rect::new(10.2, 20.6, 30.3, 40.1), 1.5);
        assert_eq!(rect, RECT { left: 15, top: 30, right: 61, bottom: 92 });
        let rect = invalidate_rect_from(Rect::new(0.0, 0.0, 100.0, 50.0), 1.0);
        assert_eq!(rect, RECT { left: 0, top: 0, right: 100, bottom: 50 });
    }

    #[test]
    fn the_edges_of_a_resize_drag_are_the_hit_test_values_of_the_frame() {
        assert_eq!(hit_test_from_edge(WindowEdge::West), 10);
        assert_eq!(hit_test_from_edge(WindowEdge::East), 11);
        assert_eq!(hit_test_from_edge(WindowEdge::North), 12);
        assert_eq!(hit_test_from_edge(WindowEdge::NorthWest), 13);
        assert_eq!(hit_test_from_edge(WindowEdge::NorthEast), 14);
        assert_eq!(hit_test_from_edge(WindowEdge::South), 15);
        assert_eq!(hit_test_from_edge(WindowEdge::SouthWest), 16);
        assert_eq!(hit_test_from_edge(WindowEdge::SouthEast), 17);
    }
}
