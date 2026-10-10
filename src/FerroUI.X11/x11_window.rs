//! A window of the X server as a top-level, a window or a popup of the
//! framework (the port of `X11Window.cs`). The keyboard part of the class
//! is in `x11_window_ime.rs`, the modes in `x11_window_modes/`.

use crate::activity_tracking_helper::WindowActivationTrackingHelper;
use crate::raw_event_grouping::{IRawEventGrouperDispatchQueue, RawEventGrouper};
use crate::transparency_helper::TransparencyHelper;
use crate::x11_cursor_factory::CursorImpl;
use crate::x11_enum_extensions::X11EnumExtensions;
use crate::x11_enums::{XEventMask, XModifierMask};
use crate::glx::GlxGlPlatformSurface;
use crate::x11_framebuffer_surface::X11FramebufferSurface;
use crate::x11_icon_loader::X11IconData;
use crate::x11_info::X11Info;
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::{
    ChangeWindowFlags, CreateWindowArgs, EventMask, Gravity, MotifDecorations, MotifFlags, MotifFunctions,
    MotifWmHints, NetWmMoveResize, NotifyDetail, SetWindowValuemask, XEventName, XSizeHintsFlags, XWMHintsFlags,
};
use crate::x11_window_info::X11WindowInfo;
use crate::x11_window_modes::{DefaultTopLevelWindowMode, InputProxyWindowMode, X11WindowMode};
use crate::xi2_manager::IXI2Client;
use crate::xlib::{self, Atom, PropertyMode, XConfigureEvent, XDisplay, XEvent, XSyncValue, XIC, XID};
use ferroui_base::input::platform::{IClipboard, IPlatformClipboardManagerImpl};
use ferroui_base::input::raw::{
    IRawInputEventArgs, RawDragEvent, RawMouseWheelEventArgs, RawPointerEventArgs, RawPointerEventType,
    RawTextInputEventArgs,
};
use ferroui_base::input::{
    IInputDevice, IInputRoot, IKeyboardDevice, MouseDevice, PenDevice, PointerPressedEventArgs, TouchDevice,
    WindowDecorationsElementRole,
};
use ferroui_base::platform::storage::file_io::BclLauncher;
use ferroui_base::platform::storage::{FallbackStorageProvider, ILauncher, IStorageProvider};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider, IPlatformGraphics, PlatformThemeVariant};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, PixelSize, Point, Rect, Size, Thickness, Vector};
use ferroui_controls::platform::{
    INativePlatformHandleSurface, IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl, IWindowBaseImpl,
    IWindowIconImpl, IWindowImpl, IX11OptionsToplevelImplFeature, PlatformAllowedWindowActions, PlatformHandle,
    PlatformRequestedDrawnDecoration, X11NetWmWindowType,
};
use ferroui_controls::primitives::popup_positioning::{
    IPopupPositioner, ManagedPopupPositioner, ManagedPopupPositionerPopupImplHelper,
};
use ferroui_controls::{
    AcrylicPlatformCompensationLevels, TopLevel, WindowCloseReason, WindowDecorations, WindowEdge,
    WindowResizeReason, WindowState, WindowTransparencyLevel,
};
use ferroui_dialogs::ManagedStorageProvider;
use ferroui_opengl::egl::{EglGlPlatformSurface, IEglWindowGlPlatformSurfaceInfo};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::Arc;

pub(crate) type Callback<F> = RefCell<Option<Rc<F>>>;

pub(crate) fn get<F: ?Sized>(callback: &Callback<F>) -> Option<Rc<F>> {
    callback.borrow().clone()
}

pub(crate) fn set<F: ?Sized>(callback: &Callback<F>, value: Option<Rc<F>>) {
    let old = callback.replace(value);
    drop(old);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum XSyncState {
    None,
    WaitConfigure,
    WaitPaint,
}

const MAX_WINDOW_DIMENSION: i32 = 100000;

/// What the thread that renders reads of a window: its scaling and its
/// size (the reference reads the fields of the window, the scaling with
/// interlocked operations).
pub(crate) struct WindowShared {
    scaling: AtomicU64,
    real_width: AtomicI32,
    real_height: AtomicI32,
    render_handle: AtomicU64,
}

impl WindowShared {
    fn scaling(&self) -> f64 {
        f64::from_bits(self.scaling.load(Ordering::SeqCst))
    }

    fn real_size(&self) -> PixelSize {
        PixelSize::new(self.real_width.load(Ordering::SeqCst), self.real_height.load(Ordering::SeqCst))
    }
}

/// What a surface of EGL or GLX reads of the window (`SurfaceInfo`), on the
/// thread that renders: the render window, the size of its parent (which
/// the render window is given on the way) and the scaling.
struct SurfaceInfo {
    shared: Arc<WindowShared>,
    display: XDisplay,
    parent: XID,
    handle: XID,
}

impl IEglWindowGlPlatformSurfaceInfo for SurfaceInfo {
    fn handle(&self) -> isize {
        self.handle as isize
    }

    fn size(&self) -> PixelSize {
        xlib::x_lock_display(self.display);
        // The reference reads the geometry unchecked; a window that is gone has no size.
        let geo = xlib::x_get_geometry(self.display, self.parent).unwrap_or_default();
        xlib::x_resize_window(self.display, self.handle, geo.width as _, geo.height as _);
        xlib::x_unlock_display(self.display);
        PixelSize::new(geo.width, geo.height)
    }

    fn scaling(&self) -> f64 {
        self.shared.scaling()
    }
}

/// The Motif functions and decorations of a window (`UpdateMotifHints`).
pub(crate) fn motif_hints(
    no_decorations: bool,
    can_resize: bool,
    can_minimize: bool,
    can_maximize: bool,
    is_disabled: bool,
) -> MotifWmHints {
    let mut functions = MotifFunctions::MOVE
        | MotifFunctions::CLOSE
        | MotifFunctions::RESIZE
        | MotifFunctions::MINIMIZE
        | MotifFunctions::MAXIMIZE;
    let mut decorations = MotifDecorations::MENU
        | MotifDecorations::TITLE
        | MotifDecorations::BORDER
        | MotifDecorations::MAXIMIZE
        | MotifDecorations::MINIMIZE
        | MotifDecorations::RESIZE_H;

    if no_decorations {
        decorations = MotifDecorations::empty();
    }

    if !can_resize || is_disabled {
        functions &= !MotifFunctions::RESIZE;
        decorations &= !MotifDecorations::RESIZE_H;
    }

    if !can_minimize || is_disabled {
        functions &= !MotifFunctions::MINIMIZE;
        decorations &= !MotifDecorations::MINIMIZE;
    }

    if !can_maximize || is_disabled {
        functions &= !MotifFunctions::MAXIMIZE;
        decorations &= !MotifDecorations::MAXIMIZE;
    }

    MotifWmHints {
        flags: (MotifFlags::DECORATIONS | MotifFlags::FUNCTIONS).bits() as _,
        decorations: decorations.bits() as _,
        functions: functions.bits() as _,
        input_mode: 0,
        status: 0,
    }
}

/// The size hints of a window as plain values (`UpdateSizeHints`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SizeHints {
    pub flags: XSizeHintsFlags,
    pub min: PixelSize,
    /// The maximum size, when it is small enough to be stated.
    pub max: Option<PixelSize>,
}

/// Computes the size hints of a window (`UpdateSizeHints`).
pub(crate) fn size_hints(
    min_max_size: (PixelSize, PixelSize),
    real_size: PixelSize,
    can_resize: bool,
    force_disable_resize: bool,
    pre_resize: Option<PixelSize>,
    use_positioning_flags: bool,
) -> SizeHints {
    let (mut min, mut max) = min_max_size;

    if !can_resize || force_disable_resize {
        match pre_resize {
            Some(pre_resize) => {
                max = pre_resize;
                min = pre_resize;
            }
            None => {
                max = real_size;
                min = real_size;
            }
        }
    } else if let Some(desired) = pre_resize {
        max = PixelSize::new(desired.width.max(max.width), desired.height.max(max.height));
        min = PixelSize::new(desired.width.min(min.width), desired.height.min(min.height));
    }

    let mut flags = XSizeHintsFlags::P_MIN_SIZE | XSizeHintsFlags::P_RESIZE_INC;
    if use_positioning_flags {
        flags |= XSizeHintsFlags::P_POSITION | XSizeHintsFlags::P_SIZE;
    }

    // People might be passing double.MaxValue
    let max = if max.width < 100000 && max.height < 100000 {
        flags |= XSizeHintsFlags::P_MAX_SIZE;
        Some(max)
    } else {
        None
    };

    SizeHints { flags, min, max }
}

/// The minimum and maximum size in pixels of sizes in logical units
/// (`SetMinMaxSize`).
pub(crate) fn min_max_pixel_size(min_size: Size, max_size: Size, render_scaling: f64) -> (PixelSize, PixelSize) {
    let min = PixelSize::new(
        (if min_size.width < 1.0 { 1.0 } else { min_size.width * render_scaling }) as i32,
        (if min_size.height < 1.0 { 1.0 } else { min_size.height * render_scaling }) as i32,
    );

    let max_dim = MAX_WINDOW_DIMENSION as f64;
    let max = PixelSize::new(
        (if max_size.width > max_dim { max_dim } else { (min.width as f64).max(max_size.width * render_scaling) }) as i32,
        (if max_size.height > max_dim { max_dim } else { (min.height as f64).max(max_size.height * render_scaling) })
            as i32,
    );

    (min, max)
}

/// The atoms of the window states the backend reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowStateAtoms {
    pub hidden: Atom,
    pub maximized_horz: Atom,
    pub maximized_vert: Atom,
    pub fullscreen: Atom,
}

/// The state of a window from its `_NET_WM_STATE` (`OnPropertyChange`).
pub(crate) fn window_state_from_net_wm_state(atoms: &[Atom], names: WindowStateAtoms) -> WindowState {
    let mut maximized = 0;
    let (mut has_minimized, mut has_fullscreen) = (false, false);
    for atom in atoms {
        if *atom == names.hidden {
            has_minimized = true;
        }

        if *atom == names.maximized_horz || *atom == names.maximized_vert {
            maximized += 1;
        }

        if *atom == names.fullscreen {
            has_fullscreen = true;
        }
    }

    if has_minimized {
        WindowState::Minimized
    } else if has_fullscreen {
        WindowState::FullScreen
    } else if maximized == 2 {
        WindowState::Maximized
    } else {
        WindowState::Normal
    }
}

/// The atoms of the actions of a window manager the backend reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowActionAtoms {
    pub maximize_vert: Atom,
    pub maximize_horz: Atom,
    pub fullscreen: Atom,
    pub minimize: Atom,
}

/// The actions the window manager allows (`GetAllowedActions`).
pub(crate) fn allowed_actions(net_supported: Option<&[Atom]>, names: WindowActionAtoms) -> PlatformAllowedWindowActions {
    let Some(net_supported) = net_supported else {
        return PlatformAllowedWindowActions::ALL;
    };

    let mut actions = PlatformAllowedWindowActions::NONE;

    if net_supported.contains(&names.maximize_vert) && net_supported.contains(&names.maximize_horz) {
        actions |= PlatformAllowedWindowActions::MAXIMIZE;
    }

    if net_supported.contains(&names.fullscreen) {
        actions |= PlatformAllowedWindowActions::FULLSCREEN;
    }

    if net_supported.contains(&names.minimize) {
        actions |= PlatformAllowedWindowActions::MINIMIZE;
    }

    actions
}

/// The side a move or resize by the window manager starts at for an edge
/// (`BeginResizeDrag`).
pub(crate) fn move_resize_side(edge: WindowEdge) -> NetWmMoveResize {
    match edge {
        WindowEdge::East => NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_RIGHT,
        WindowEdge::North => NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOP,
        WindowEdge::South => NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOM,
        WindowEdge::West => NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_LEFT,
        WindowEdge::NorthEast => NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOPRIGHT,
        WindowEdge::NorthWest => NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOPLEFT,
        WindowEdge::SouthEast => NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOMRIGHT,
        WindowEdge::SouthWest => NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOMLEFT,
    }
}

/// The move or resize a press on a drawn decoration starts
/// (`ScheduleInput`).
pub(crate) fn chrome_move_resize_side(role: WindowDecorationsElementRole, can_resize: bool) -> Option<NetWmMoveResize> {
    match role {
        WindowDecorationsElementRole::TitleBar => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_MOVE),
        WindowDecorationsElementRole::ResizeN if can_resize => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOP),
        WindowDecorationsElementRole::ResizeS if can_resize => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOM),
        WindowDecorationsElementRole::ResizeE if can_resize => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_RIGHT),
        WindowDecorationsElementRole::ResizeW if can_resize => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_LEFT),
        WindowDecorationsElementRole::ResizeNE if can_resize => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOPRIGHT),
        WindowDecorationsElementRole::ResizeNW if can_resize => Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_TOPLEFT),
        WindowDecorationsElementRole::ResizeSE if can_resize => {
            Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOMRIGHT)
        }
        WindowDecorationsElementRole::ResizeSW if can_resize => {
            Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOMLEFT)
        }
        _ => None,
    }
}

/// The pointer event of a core button, pressed or released.
pub(crate) fn core_button_event_type(button: u32, down: bool) -> Option<RawPointerEventType> {
    Some(match (button, down) {
        (1, true) => RawPointerEventType::LeftButtonDown,
        (2, true) => RawPointerEventType::MiddleButtonDown,
        (3, true) => RawPointerEventType::RightButtonDown,
        (8, true) => RawPointerEventType::XButton1Down,
        (9, true) => RawPointerEventType::XButton2Down,
        (1, false) => RawPointerEventType::LeftButtonUp,
        (2, false) => RawPointerEventType::MiddleButtonUp,
        (3, false) => RawPointerEventType::RightButtonUp,
        (8, false) => RawPointerEventType::XButton1Up,
        (9, false) => RawPointerEventType::XButton2Up,
        _ => return None,
    })
}

/// The wheel delta of a core scroll button (4 to 7; the reference maps
/// every other button it reaches here like button 7).
pub(crate) fn core_scroll_delta(button: u32) -> Vector {
    if button == 4 {
        Vector::new(0.0, 1.0)
    } else if button == 5 {
        Vector::new(0.0, -1.0)
    } else if button == 6 {
        Vector::new(1.0, 0.0)
    } else {
        Vector::new(-1.0, 0.0)
    }
}

fn is_handled_leave_enter_detail(detail: i32) -> bool {
    detail == NotifyDetail::NotifyNonlinear as i32
        || detail == NotifyDetail::NotifyNonlinearVirtual as i32
        || detail == NotifyDetail::NotifyVirtual as i32
        || detail == NotifyDetail::NotifyAncestor as i32
}

/// A window of the X11 platform.
pub struct X11Window {
    this: Weak<X11Window>,
    platform: Rc<FerroX11Platform>,
    popup: bool,
    override_redirect: bool,
    pub(crate) x11: Rc<X11Info>,
    configure: Cell<Option<XConfigureEvent>>,
    configure_point: Cell<Option<PixelPoint>>,
    triggered_expose: Cell<bool>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    mouse: Rc<MouseDevice>,
    pen: Rc<PenDevice>,
    touch: Rc<TouchDevice>,
    pub(crate) keyboard: Rc<dyn IKeyboardDevice>,
    storage_provider: RefCell<Option<Rc<dyn IStorageProvider>>>,
    position: Cell<Option<PixelPoint>>,
    real_size: Cell<PixelSize>,
    cleaning_up: Cell<bool>,
    handle: Cell<XID>,
    pub(crate) xic: Cell<XIC>,
    render_handle: Cell<XID>,
    x_sync_counter: Cell<XID>,
    x_sync_value: Cell<XSyncValue>,
    x_sync_state: Cell<XSyncState>,
    mapped: Cell<bool>,
    was_mapped_at_least_once: Cell<bool>,
    shown: Cell<bool>,
    scaling_override: Cell<Option<f64>>,
    disabled: Cell<bool>,
    transparency_helper: RefCell<Option<Rc<TransparencyHelper>>>,
    activation_tracker: RefCell<Option<Rc<WindowActivationTrackingHelper>>>,
    activation_subscription: Cell<u64>,
    transient_parent: RefCell<Option<Weak<X11Window>>>,
    raw_event_grouper: RefCell<Option<Rc<RawEventGrouper>>>,
    use_render_window: bool,
    use_compositor_driven_render_window_resize: bool,
    use_positioning_flags: Cell<bool>,
    mode: Box<dyn X11WindowMode>,
    icon_impl: RefCell<Option<Rc<dyn IWindowIconImpl>>>,
    screens_subscription: Cell<u64>,
    net_supported_subscription: Cell<u64>,

    last_window_state: Cell<WindowState>,
    requested_window_decorations: Cell<WindowDecorations>,
    window_decorations: Cell<WindowDecorations>,
    can_resize: Cell<bool>,
    can_minimize: Cell<bool>,
    can_maximize: Cell<bool>,
    scaled_min_max_size: Cell<(Size, Size)>,
    min_max_size: Cell<(PixelSize, PixelSize)>,
    shared: Arc<WindowShared>,
    extending_client_area_to_decorations: Cell<bool>,
    is_client_area_extended_to_decorations: Cell<bool>,

    surfaces: RefCell<Vec<Arc<dyn IPlatformRenderSurface>>>,
    platform_handle: Rc<dyn IPlatformHandle>,
    popup_positioner: RefCell<Option<Rc<dyn IPopupPositioner>>>,

    input: Callback<dyn Fn(Rc<dyn IRawInputEventArgs>)>,
    paint: Callback<dyn Fn(Rect)>,
    resized: Callback<dyn Fn(Size, WindowResizeReason)>,
    scaling_changed: Callback<dyn Fn(f64)>,
    deactivated: Callback<dyn Fn()>,
    activated: Callback<dyn Fn()>,
    closing: Callback<dyn Fn(WindowCloseReason) -> bool>,
    window_state_changed: Callback<dyn Fn(WindowState)>,
    extend_client_area_to_decorations_changed: Callback<dyn Fn(bool)>,
    closed: Callback<dyn Fn()>,
    position_changed: Callback<dyn Fn(PixelPoint)>,
    lost_focus: Callback<dyn Fn()>,
    allowed_window_actions_changed: Callback<dyn Fn(PlatformAllowedWindowActions)>,
    got_input_when_disabled: Callback<dyn Fn()>,
}

impl X11Window {
    /// A window, or a popup of `popup_parent`.
    pub fn new(
        platform: &Rc<FerroX11Platform>,
        popup_parent: Option<Rc<dyn IWindowImpl>>,
        override_redirect: bool,
    ) -> Rc<X11Window> {
        let mode: Box<dyn X11WindowMode> = if platform.options().enable_input_focus_proxy {
            Box::new(InputProxyWindowMode::default())
        } else {
            Box::new(DefaultTopLevelWindowMode)
        };
        Self::with_mode(platform, popup_parent, mode, override_redirect)
    }

    pub fn with_mode(
        platform: &Rc<FerroX11Platform>,
        popup_parent: Option<Rc<dyn IWindowImpl>>,
        mode: Box<dyn X11WindowMode>,
        override_redirect: bool,
    ) -> Rc<X11Window> {
        let popup = popup_parent.is_some();
        let override_redirect = popup || override_redirect;
        let x11 = platform.info().clone();
        let display = x11.display();

        let glfeature = FerroLocator::current().get_service::<Arc<dyn IPlatformGraphics>>();
        let mut attr = xlib::new_set_window_attributes();

        attr.backing_store = 1;
        attr.bit_gravity = Gravity::NorthWestGravity as i32;
        attr.win_gravity = Gravity::NorthWestGravity as i32;
        let mut value_mask = SetWindowValuemask::BACK_PIXEL
            | SetWindowValuemask::BORDER_PIXEL
            | SetWindowValuemask::BACK_PIXMAP
            | SetWindowValuemask::BACKING_STORE
            | SetWindowValuemask::BIT_GRAVITY
            | SetWindowValuemask::WIN_GRAVITY;

        if override_redirect {
            attr.override_redirect = 1;
            value_mask |= SetWindowValuemask::OVERRIDE_REDIRECT;
        }

        // OpenGL seems to be do weird things to it's current window which breaks resize sometimes
        let use_render_window = glfeature.is_some();

        // The reference tests the registered graphics for their type. The
        // platform graphics of the port are a contract without a way to
        // ask for the type, so the platform remembers which of its own it
        // registered.
        let glx = platform.glx_graphics();
        let egl = platform.egl_graphics();
        let mut use_compositor_driven_render_window_resize = false;
        let mut visual_info = None;
        if let Some(glx) = &glx {
            visual_info = Some(glx.display().visual_info());
            // TODO: We should use this for all backends, but need to actually test the change
            // TODO: We probably need to resize the window from the compositor thread too
            use_compositor_driven_render_window_resize = true;
        } else if let Some(egl) = &egl {
            visual_info = egl.visual_info();
        } else if glfeature.is_none() {
            visual_info = x11.transparent_visual_info();
        }

        let mut visual = std::ptr::null_mut();
        let mut depth = 24;
        if let Some(visual_info) = &visual_info {
            visual = visual_info.visual;
            depth = visual_info.depth;
            attr.colormap = xlib::x_create_colormap(display, x11.root_window(), visual_info, 0);
            value_mask |= SetWindowValuemask::COLOR_MAP;
        }

        let (mut default_width, mut default_height) = (0, 0);
        let mut initial_scaling = 1.0;

        if !popup {
            let mut screens = platform.screens().all_screens();
            screens.sort_by(|a, b| a.scaling().total_cmp(&b.scaling()));
            let monitor = screens.iter().find(|m| m.bounds().contains(PixelPoint::default()));

            if let Some(monitor) = monitor {
                // Emulate Window 7+'s default window size behavior.
                default_width = (monitor.working_area().width as f64 * 0.75) as i32;
                default_height = (monitor.working_area().height as f64 * 0.7) as i32;

                // The default size is in pixels, so initialize the scaling to match the monitor.
                // Otherwise UpdateScaling() would treat the pixel size as DIPs and scale it again.
                initial_scaling = monitor.scaling();
            }
        }

        // check if the calculated size is zero then compensate to hardcoded resolution
        default_width = default_width.max(300);
        default_height = default_height.max(200);

        let handle = xlib::x_create_window(
            display,
            x11.root_window(),
            10,
            10,
            default_width,
            default_height,
            0,
            depth,
            CreateWindowArgs::InputOutput.0,
            visual,
            value_mask.bits() as u32 as _,
            &mut attr,
        );

        let render_handle = if use_render_window {
            let mut render_value_mask = SetWindowValuemask::BORDER_PIXEL
                | SetWindowValuemask::BIT_GRAVITY
                | SetWindowValuemask::WIN_GRAVITY
                | SetWindowValuemask::BACKING_STORE;
            // A window with a non-default visual must be created with a matching colormap, otherwise X11
            // raises BadMatch. This is required by nvidia when a custom visual is selected for the EGL config.
            if visual_info.is_some() {
                render_value_mask |= SetWindowValuemask::COLOR_MAP;
            }

            xlib::x_create_window(
                display,
                handle,
                0,
                0,
                default_width,
                default_height,
                0,
                depth,
                CreateWindowArgs::InputOutput.0,
                visual,
                render_value_mask.bits() as u32 as _,
                &mut attr,
            )
        } else {
            handle
        };

        let shared = Arc::new(WindowShared {
            scaling: AtomicU64::new(initial_scaling.to_bits()),
            real_width: AtomicI32::new(default_width),
            real_height: AtomicI32::new(default_height),
            render_handle: AtomicU64::new(render_handle as u64),
        });

        let window = Rc::new_cyclic(|this| X11Window {
            this: this.clone(),
            platform: platform.clone(),
            popup,
            override_redirect,
            x11: x11.clone(),
            configure: Cell::new(None),
            configure_point: Cell::new(None),
            triggered_expose: Cell::new(false),
            input_root: RefCell::new(None),
            mouse: MouseDevice::primary(),
            pen: PenDevice::new(false),
            touch: TouchDevice::new(),
            keyboard: platform.keyboard_device(),
            storage_provider: RefCell::new(None),
            position: Cell::new(None),
            real_size: Cell::new(PixelSize::new(default_width, default_height)),
            cleaning_up: Cell::new(false),
            handle: Cell::new(handle),
            xic: Cell::new(std::ptr::null_mut()),
            render_handle: Cell::new(render_handle),
            x_sync_counter: Cell::new(0),
            x_sync_value: Cell::new(XSyncValue { hi: 0, lo: 0 }),
            x_sync_state: Cell::new(XSyncState::None),
            mapped: Cell::new(false),
            was_mapped_at_least_once: Cell::new(false),
            shown: Cell::new(false),
            scaling_override: Cell::new(None),
            disabled: Cell::new(false),
            transparency_helper: RefCell::new(None),
            activation_tracker: RefCell::new(None),
            activation_subscription: Cell::new(0),
            transient_parent: RefCell::new(None),
            raw_event_grouper: RefCell::new(None),
            use_render_window,
            use_compositor_driven_render_window_resize,
            use_positioning_flags: Cell::new(false),
            mode,
            icon_impl: RefCell::new(None),
            screens_subscription: Cell::new(0),
            net_supported_subscription: Cell::new(0),
            last_window_state: Cell::new(WindowState::Normal),
            requested_window_decorations: Cell::new(WindowDecorations::Full),
            window_decorations: Cell::new(WindowDecorations::Full),
            can_resize: Cell::new(true),
            can_minimize: Cell::new(true),
            can_maximize: Cell::new(true),
            scaled_min_max_size: Cell::new((Size::new(1.0, 1.0), Size::new(f64::INFINITY, f64::INFINITY))),
            min_max_size: Cell::new((
                PixelSize::new(1, 1),
                PixelSize::new(MAX_WINDOW_DIMENSION, MAX_WINDOW_DIMENSION),
            )),
            shared,
            extending_client_area_to_decorations: Cell::new(false),
            is_client_area_extended_to_decorations: Cell::new(false),
            surfaces: RefCell::new(Vec::new()),
            platform_handle: Rc::new(PlatformHandle::new(handle as isize, Some("XID"))),
            popup_positioner: RefCell::new(None),
            input: RefCell::new(None),
            paint: RefCell::new(None),
            resized: RefCell::new(None),
            scaling_changed: RefCell::new(None),
            deactivated: RefCell::new(None),
            activated: RefCell::new(None),
            closing: RefCell::new(None),
            window_state_changed: RefCell::new(None),
            extend_client_area_to_decorations_changed: RefCell::new(None),
            closed: RefCell::new(None),
            position_changed: RefCell::new(None),
            lost_focus: RefCell::new(None),
            allowed_window_actions_changed: RefCell::new(None),
            got_input_when_disabled: RefCell::new(None),
        });

        window.append_pid(handle);

        window.mode.on_handle_created(&window, handle);

        // The window mode may have set a scaling override (e.g. XEmbed forces a scaling of 1), which takes
        // precedence over the monitor scaling. Keep them in sync, otherwise UpdateScaling() would see a
        // mismatch and trigger a spurious DPI resize.
        if let Some(scaling_override) = window.scaling_override.get() {
            window.set_render_scaling(scaling_override);
        }

        let weak = Rc::downgrade(&window);
        platform.set_window(
            handle,
            X11WindowInfo::new(
                Rc::new({
                    let weak = weak.clone();
                    move |ev: &mut XEvent| {
                        if let Some(window) = weak.upgrade() {
                            window.on_event(ev);
                        }
                    }
                }),
                Some(weak.clone()),
            ),
        );
        let mut ignored_mask =
            XEventMask::SUBSTRUCTURE_REDIRECT_MASK | XEventMask::RESIZE_REDIRECT_MASK | XEventMask::POINTER_MOTION_HINT_MASK;
        if let Some(xi2) = platform.xi2() {
            let client: Weak<dyn IXI2Client> = weak.clone();
            ignored_mask |= xi2.add_window(handle, client);
        }
        let mask = 0xffffff ^ ignored_mask.bits();
        xlib::x_select_input(display, handle, mask as _);
        if !override_redirect {
            let protocols = [x11.atoms().WM_DELETE_WINDOW];
            xlib::x_set_wm_protocols(display, handle, &protocols);
            window.set_net_wm_window_type(X11NetWmWindowType::Normal);

            window.set_wm_class_of(handle, platform.options().wm_class.as_deref());
        }

        let mut surfaces: Vec<Arc<dyn IPlatformRenderSurface>> = vec![Arc::new(X11FramebufferSurface::new(
            x11.deferred_display(),
            render_handle,
            depth,
            platform.options().use_retained_framebuffer.unwrap_or(false),
        ))];

        // XShm needs a 32-bit visual (other depths would require a slow XShmPutImage conversion) and the
        // MIT-SHM extension, probed once by X11Info on the deferred display.
        if platform.options().use_x_shm_framebuffer == Some(true) && depth == 32 && x11.has_x_shm() {
            panic!(
                "X11PlatformOptions::use_x_shm_framebuffer: the shared memory framebuffer (X11ShmFramebufferSurface) \
                 is not built yet (stage 2 of docs/porting/x11-platform.md)"
            );
        }

        if egl.is_some() {
            surfaces.insert(
                0,
                EglGlPlatformSurface::new(Arc::new(SurfaceInfo {
                    shared: window.shared.clone(),
                    display: x11.deferred_display(),
                    parent: handle,
                    handle: render_handle,
                })),
            );
        }
        if glx.is_some() {
            surfaces.insert(
                0,
                GlxGlPlatformSurface::new(Arc::new(SurfaceInfo {
                    shared: window.shared.clone(),
                    display: x11.deferred_display(),
                    parent: handle,
                    handle: render_handle,
                })),
            );
        }

        surfaces.push(Arc::new(SurfacePlatformHandle { shared: window.shared.clone() }));

        *window.surfaces.borrow_mut() = surfaces;
        window.update_effective_system_decorations();
        window.update_motif_hints();
        window.update_size_hints(None, false);

        let dispatch_input = {
            let weak = weak.clone();
            move |args: Rc<dyn IRawInputEventArgs>| {
                if let Some(window) = weak.upgrade() {
                    window.dispatch_input(args);
                }
            }
        };
        let queue: Rc<dyn IRawEventGrouperDispatchQueue> = platform.event_grouper_dispatch_queue().clone();
        *window.raw_event_grouper.borrow_mut() = Some(Rc::new(RawEventGrouper::new(dispatch_input, Some(queue))));

        let transparency_helper = TransparencyHelper::new(&x11, handle, platform.globals());
        transparency_helper.set_transparency_request(&[]);
        *window.transparency_helper.borrow_mut() = Some(transparency_helper);

        let activation_tracker = WindowActivationTrackingHelper::new(platform, &weak, handle);
        window.activation_subscription.set(activation_tracker.activation_changed.subscribe({
            let weak = weak.clone();
            move |active| {
                if let Some(window) = weak.upgrade() {
                    window.handle_activation(active);
                }
            }
        }));
        *window.activation_tracker.borrow_mut() = Some(activation_tracker);

        window.net_supported_subscription.set(platform.globals().net_supported_changed.subscribe({
            let weak = weak.clone();
            move |()| {
                if let Some(window) = weak.upgrade() {
                    window.on_net_supported_changed();
                }
            }
        }));

        window.create_ic();

        xlib::x_flush(display);
        if let Some(popup_parent) = popup_parent {
            let parent: Rc<dyn ITopLevelImpl> = popup_parent;
            let weak = weak.clone();
            let helper = ManagedPopupPositionerPopupImplHelper::new(
                parent,
                Rc::new(move |position, size: Size, scaling| {
                    if let Some(window) = weak.upgrade() {
                        window.move_resize(position, size, scaling);
                    }
                }),
            );
            let positioner: Rc<dyn IPopupPositioner> = Rc::new(ManagedPopupPositioner::new(Rc::new(helper)));
            *window.popup_positioner.borrow_mut() = Some(positioner);
        }
        // Stage 2 of docs/porting/x11-platform.md, in the order of the
        // reference: the exporter of the native menu over D-Bus
        // (`DBusMenuExporter`, with `X11PlatformOptions::use_d_bus_menu`),
        // the native control host (`X11NativeControlHost`) and the input
        // method of the window (`InitializeIme`).

        let mut data = vec![x11.atoms().WM_DELETE_WINDOW, x11.atoms()._NET_WM_SYNC_REQUEST];

        window.mode.append_wm_protocols(&window, &mut data);

        xlib::x_change_property_longs(
            display,
            handle,
            x11.atoms().WM_PROTOCOLS,
            x11.atoms().ATOM,
            PropertyMode::Replace,
            &data,
        );

        if x11.has_x_sync() {
            let counter = xlib::x_sync_create_counter(display, window.x_sync_value.get());
            window.x_sync_counter.set(counter);
            xlib::x_change_property_longs(
                display,
                handle,
                x11.atoms()._NET_WM_SYNC_REQUEST_COUNTER,
                x11.atoms().CARDINAL,
                PropertyMode::Replace,
                &[counter],
            );
        }

        // The storage providers of the reference, in its order: the file
        // chooser of the desktop portal over D-Bus and the GTK dialogs
        // (both stage 2 of docs/porting/x11-platform.md), then the managed
        // dialogs, which is the one provider there is until then.
        let storage_window = weak.clone();
        let storage_provider: Rc<dyn IStorageProvider> = Rc::new(FallbackStorageProvider::new(vec![Rc::new(move || {
            // TODO: This will be incompatible with "root element is not a TopLevel" scenarios,
            // HACK: this relies on focus root being TopLevel which currently is true
            let provider: Option<Rc<dyn IStorageProvider>> = storage_window
                .upgrade()
                .and_then(|window| window.input_root.borrow().clone())
                .and_then(|input_root| input_root.focus_root().cast::<TopLevel>())
                .map(|tl| Rc::new(ManagedStorageProvider::new(Some(&tl), None)) as Rc<dyn IStorageProvider>);
            Box::pin(std::future::ready(provider))
        })]));
        *window.storage_provider.borrow_mut() = Some(storage_provider);

        // Stage 2: the drop target of the window (`X11DropTarget`), when
        // the drag and drop device is registered.

        window.screens_subscription.set(platform.x11_screens().changed_event.subscribe({
            let weak = weak.clone();
            move |()| {
                if let Some(window) = weak.upgrade() {
                    window.on_screens_changed();
                }
            }
        }));

        // The render surface (EGL/GLX) is created on the deferred display connection, which is a separate
        // X11 connection from the one the windows were created on. Force a round-trip so the server has
        // actually created the windows before anything on the other connection tries to use them.
        xlib::x_sync(display, false);

        window
    }

    // ------------------------------------------------------------------
    // What the modes and the other parts of the class read.
    // ------------------------------------------------------------------

    pub(crate) fn platform(&self) -> &Rc<FerroX11Platform> {
        &self.platform
    }

    pub(crate) fn x11(&self) -> &Rc<X11Info> {
        &self.x11
    }

    fn display(&self) -> XDisplay {
        self.x11.display()
    }

    /// The identifier of the window (`_handle`); zero once it is closed.
    pub fn xid(&self) -> XID {
        self.handle.get()
    }

    /// The identifier of the window frames are drawn into.
    pub fn render_xid(&self) -> XID {
        self.render_handle.get()
    }

    pub(crate) fn set_shown(&self, value: bool) {
        self.shown.set(value);
    }

    pub(crate) fn set_was_mapped_at_least_once(&self, value: bool) {
        self.was_mapped_at_least_once.set(value);
    }

    pub(crate) fn override_redirect(&self) -> bool {
        self.override_redirect
    }

    pub(crate) fn position_or_default(&self) -> PixelPoint {
        self.position.get().unwrap_or_default()
    }

    pub(crate) fn scaling(&self) -> f64 {
        self.shared.scaling()
    }

    fn set_render_scaling(&self, value: f64) {
        self.shared.scaling.store(value.to_bits(), Ordering::SeqCst);
    }

    fn set_real_size(&self, value: PixelSize) {
        self.real_size.set(value);
        self.shared.real_width.store(value.width, Ordering::SeqCst);
        self.shared.real_height.store(value.height, Ordering::SeqCst);
    }

    pub(crate) fn input_root_or_none(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    /// The input root of the window.
    ///
    /// # Panics
    /// Panics when `set_input_root` was not called yet.
    pub fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.input_root_or_none().unwrap_or_else(|| panic!("set_input_root must have been called"))
    }

    // ------------------------------------------------------------------

    fn update_motif_hints(&self) {
        if self.override_redirect {
            return;
        }

        let is_disabled = !self.is_enabled();
        let hints = motif_hints(
            self.popup || self.window_decorations.get() == WindowDecorations::None,
            self.can_resize.get(),
            self.can_minimize.get(),
            self.can_maximize.get(),
            is_disabled,
        );

        self.update_size_hints(None, is_disabled);

        let atoms = self.x11.atoms();
        xlib::x_change_property_longs(
            self.display(),
            self.handle.get(),
            atoms._MOTIF_WM_HINTS,
            atoms._MOTIF_WM_HINTS,
            PropertyMode::Replace,
            &hints.to_longs(),
        );
    }

    fn append_pid(&self, window_x_id: XID) {
        // See the issue 17444 of the reference.
        let pid = std::process::id();
        // The type of `_NET_WM_PID` is `CARDINAL` which is 32-bit unsigned integer, see https://specifications.freedesktop.org/wm-spec/1.3/ar01s05.html
        let atoms = self.x11.atoms();
        xlib::x_change_property_longs(
            self.display(),
            window_x_id,
            atoms._NET_WM_PID,
            atoms.CARDINAL,
            PropertyMode::Replace,
            &[pid as _],
        );

        let Some(name) = xlib::get_host_name() else {
            // Fail
            return;
        };

        xlib::x_change_property_bytes(
            self.display(),
            window_x_id,
            atoms.WM_CLIENT_MACHINE,
            atoms.STRING,
            PropertyMode::Replace,
            &name,
        );
    }

    fn update_size_hints(&self, pre_resize: Option<PixelSize>, force_disable_resize: bool) {
        if self.override_redirect {
            return;
        }
        let computed = size_hints(
            self.min_max_size.get(),
            self.real_size.get(),
            self.can_resize.get(),
            force_disable_resize,
            pre_resize,
            self.use_positioning_flags.get(),
        );

        let mut hints = xlib::new_size_hints();
        hints.min_width = computed.min.width;
        hints.min_height = computed.min.height;
        hints.height_inc = 1;
        hints.width_inc = 1;
        if let Some(max) = computed.max {
            hints.max_width = max.width;
            hints.max_height = max.height;
        }

        hints.flags = computed.flags.bits() as _;

        xlib::x_set_wm_normal_hints(self.display(), self.handle.get(), &mut hints);
    }

    fn client_size_value(&self) -> Size {
        let real_size = self.real_size.get();
        Size::new(real_size.width as f64 / self.scaling(), real_size.height as f64 / self.scaling())
    }

    /// The actions the window manager allows (`AllowedWindowActions`).
    fn allowed_actions_value(&self) -> PlatformAllowedWindowActions {
        let atoms = self.x11.atoms();
        allowed_actions(
            self.platform.globals().net_supported().as_deref(),
            WindowActionAtoms {
                maximize_vert: atoms._NET_WM_ACTION_MAXIMIZE_VERT,
                maximize_horz: atoms._NET_WM_ACTION_MAXIMIZE_HORZ,
                fullscreen: atoms._NET_WM_ACTION_FULLSCREEN,
                minimize: atoms._NET_WM_ACTION_MINIMIZE,
            },
        )
    }

    fn on_net_supported_changed(&self) {
        if let Some(changed) = get(&self.allowed_window_actions_changed) {
            changed(self.allowed_actions_value());
        }
    }

    fn on_event(&self, ev: &mut XEvent) {
        let Some(input_root) = self.input_root_or_none() else {
            return;
        };

        if self.mode.on_event(self, ev) {
            return;
        }

        let activation_tracker = self.activation_tracker.borrow().clone();
        if let Some(activation_tracker) = activation_tracker {
            activation_tracker.on_event(ev);
        }

        let event_type = xlib::event_type(ev);
        let display = self.display();
        let atoms = self.x11.atoms();
        if event_type == XEventName::MapNotify as i32 {
            self.mapped.set(true);
            if self.use_render_window {
                xlib::x_map_window(display, self.render_handle.get());
            }
        } else if event_type == XEventName::UnmapNotify as i32 {
            self.mapped.set(false);
        } else if event_type == XEventName::Expose as i32
            || (event_type == XEventName::VisibilityNotify as i32 && xlib::visibility_event(ev).state < 2)
        {
            self.enqueue_paint();
        } else if event_type == XEventName::MotionNotify as i32 {
            self.mouse_event(RawPointerEventType::Move, ev, xlib::motion_event(ev).state);
        } else if event_type == XEventName::LeaveNotify as i32 {
            if is_handled_leave_enter_detail(xlib::crossing_event(ev).detail) {
                self.mouse_event(RawPointerEventType::LeaveWindow, ev, xlib::crossing_event(ev).state);
            }
        } else if event_type == XEventName::EnterNotify as i32 {
            if is_handled_leave_enter_detail(xlib::crossing_event(ev).detail) {
                self.mouse_event(RawPointerEventType::Move, ev, xlib::crossing_event(ev).state);
            }
        } else if event_type == XEventName::PropertyNotify as i32 {
            let property = xlib::property_event(ev);
            self.on_property_change(property.atom, property.state == 0);
        } else if event_type == XEventName::ButtonPress as i32 {
            if self.activate_transient_child_if_needed() {
                return;
            }
            let button_event = *xlib::button_event(ev);
            let button = button_event.button;
            if button < 4 || button == 8 || button == 9 {
                let type_ = core_button_event_type(button, true)
                    .unwrap_or_else(|| panic!("Unexepected RawPointerEventType."));
                self.mouse_event(type_, ev, button_event.state);
            } else {
                let delta = core_scroll_delta(button);
                let args = Rc::new(RawMouseWheelEventArgs::new(
                    self.mouse.clone(),
                    button_event.time as u64,
                    input_root,
                    Point::new(button_event.x as f64, button_event.y as f64),
                    delta,
                    XModifierMask::from_bits_retain(button_event.state as i32).to_raw_input_modifiers(),
                ));
                self.schedule_input_with_event(args, ev);
            }
        } else if event_type == XEventName::ButtonRelease as i32 {
            let button_event = *xlib::button_event(ev);
            let button = button_event.button;
            if button < 4 || button == 8 || button == 9 {
                let type_ = core_button_event_type(button, false)
                    .unwrap_or_else(|| panic!("Unexepected RawPointerEventType."));
                self.mouse_event(type_, ev, button_event.state);
            }
        } else if event_type == XEventName::ConfigureNotify as i32 {
            let configure = *xlib::configure_event(ev);
            if configure.window != self.handle.get() {
                return;
            }
            let need_enqueue = self.configure.get().is_none();
            self.configure.set(Some(configure));
            if configure.override_redirect != 0 || configure.send_event != 0 {
                self.configure_point.set(Some(PixelPoint::new(configure.x, configure.y)));
            } else {
                let (tx, ty, _) =
                    xlib::x_translate_coordinates(display, self.handle.get(), self.x11.root_window(), 0, 0)
                        .unwrap_or_default();
                self.configure_point.set(Some(PixelPoint::new(tx, ty)));
            }
            if need_enqueue {
                let weak = self.this.clone();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(window) = weak.upgrade() {
                            window.apply_configure();
                        }
                    },
                    DispatcherPriority::ASYNC_RENDER_TARGET_RESIZE,
                );
            }

            if self.use_render_window && !self.use_compositor_driven_render_window_resize {
                xlib::x_configure_resize_window(display, self.render_handle.get(), configure.width, configure.height);
            }
            if self.x_sync_state.get() == XSyncState::WaitConfigure {
                self.x_sync_state.set(XSyncState::WaitPaint);
                self.enqueue_paint();
            }
        } else if event_type == XEventName::DestroyNotify as i32
            && xlib::destroy_window_event(ev).window == self.handle.get()
        {
            self.mode.on_destroy_notify(self);
            self.cleanup(true);
        } else if event_type == XEventName::ClientMessage as i32 {
            let message = *xlib::client_message_event(ev);
            let message_type = message.message_type;
            if message_type == atoms.WM_PROTOCOLS {
                let protocol = message.data.get_long(0) as Atom;
                if protocol == atoms.WM_DELETE_WINDOW {
                    if self.is_enabled()
                        && get(&self.closing).map(|closing| closing(WindowCloseReason::WindowClosing)) != Some(true)
                    {
                        self.dispose();
                    }
                } else if protocol == atoms._NET_WM_SYNC_REQUEST {
                    self.x_sync_value.set(XSyncValue {
                        lo: message.data.get_long(2) as u32,
                        hi: message.data.get_long(3) as i32,
                    });
                    self.x_sync_state.set(XSyncState::WaitConfigure);
                }
            }
            // Stage 2 of docs/porting/x11-platform.md: the messages of the
            // drag and drop protocol (`XdndEnter`, `XdndPosition`,
            // `XdndLeave`, `XdndDrop`) go to the drop target of the
            // window, which is not built.
        } else if event_type == XEventName::KeyPress as i32 || event_type == XEventName::KeyRelease as i32 {
            if self.activate_transient_child_if_needed() {
                return;
            }
            self.handle_key_event(ev);
        }
    }

    /// The deferred part of a configure event: the size and the position
    /// the server reported last.
    fn apply_configure(&self) {
        let Some(cev) = self.configure.get() else {
            return;
        };
        let npos = self.configure_point.get().unwrap_or_default();
        self.configure.set(None);
        self.configure_point.set(None);

        let nsize = PixelSize::new(cev.width, cev.height);
        let changed_size = self.real_size.get() != nsize;
        let changed_pos = self.position.get() != Some(npos);
        self.set_real_size(nsize);
        self.position.set(Some(npos));
        let mut updated_size_via_scaling = false;
        if changed_pos {
            if let Some(position_changed) = get(&self.position_changed) {
                position_changed(npos);
            }
            updated_size_via_scaling = self.update_scaling(false);
        }
        self.update_ime_position();

        if changed_size && !updated_size_via_scaling && !self.override_redirect {
            if let Some(resized) = get(&self.resized) {
                resized(self.client_size_value(), WindowResizeReason::Unspecified);
            }
        }
    }

    fn handle_activation(&self, active: bool) {
        if active {
            if self.activate_transient_child_if_needed() {
                return;
            }
            if let Some(activated) = get(&self.activated) {
                activated();
            }
            // Stage 2: the input method of the window is told that the
            // window is active (`_imeControl.SetWindowActive`).
        } else if let Some(deactivated) = get(&self.deactivated) {
            deactivated();
        }
    }

    fn get_frame_extents(&self) -> Option<Thickness> {
        if self.window_decorations.get() != WindowDecorations::Full {
            return Some(Thickness::uniform(0.0));
        }

        let property = xlib::x_get_window_property(
            self.display(),
            self.handle.get(),
            self.x11.atoms()._NET_FRAME_EXTENTS,
            0,
            4,
            false,
            xlib::ANY_PROPERTY_TYPE,
        );

        if property.nitems != 4 {
            // Window hasn't been mapped by the WM yet, so can't get the extents.
            return None;
        }

        frame_extents_from_property(&property.longs())
    }

    fn on_screens_changed(&self) {
        self.update_scaling(false);
    }

    fn update_scaling(&self, skip_resize: bool) -> bool {
        let new_scaling = match self.scaling_override.get() {
            Some(scaling_override) => scaling_override,
            None => {
                let mut screens = self.platform.x11_screens().all_screens();
                screens.sort_by(|a, b| a.scaling().total_cmp(&b.scaling()));
                let position = self.position_or_default();
                screens
                    .iter()
                    .find(|m| m.bounds().contains(position))
                    .map_or(self.scaling(), |monitor| monitor.scaling())
            }
        };

        if self.scaling() != new_scaling {
            let old_scaled_size = self.client_size_value();
            self.set_render_scaling(new_scaling);
            if let Some(scaling_changed) = get(&self.scaling_changed) {
                scaling_changed(self.scaling());
            }
            self.update_ime_position();
            let (min_size, max_size) = self.scaled_min_max_size.get();
            self.set_min_max_size(min_size, max_size);
            if !skip_resize {
                self.resize_core(old_scaled_size, true, WindowResizeReason::DpiChange);
            }
            return true;
        }

        false
    }

    fn on_property_change(&self, property: Atom, has_value: bool) {
        let atoms = self.x11.atoms();
        if property == atoms._NET_FRAME_EXTENTS {
            // Occurs once the window has been mapped, which is the earliest the extents
            // can be retrieved, so invoke event to force update of TopLevel.FrameSize.
            if let Some(resized) = get(&self.resized) {
                resized(self.client_size_value(), WindowResizeReason::Unspecified);
            }
        }

        if property == atoms._NET_WM_STATE {
            let state_atoms = if has_value {
                xlib::x_get_window_property_as_int_ptr_array(self.display(), self.handle.get(), atoms._NET_WM_STATE, 4)
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            let state = window_state_from_net_wm_state(
                &state_atoms,
                WindowStateAtoms {
                    hidden: atoms._NET_WM_STATE_HIDDEN,
                    maximized_horz: atoms._NET_WM_STATE_MAXIMIZED_HORZ,
                    maximized_vert: atoms._NET_WM_STATE_MAXIMIZED_VERT,
                    fullscreen: atoms._NET_WM_STATE_FULLSCREEN,
                },
            );

            if self.last_window_state.get() != state {
                self.last_window_state.set(state);
                if let Some(window_state_changed) = get(&self.window_state_changed) {
                    window_state_changed(state);
                }

                let geometry = xlib::x_get_geometry(self.display(), self.handle.get()).unwrap_or_default();
                let new_size = PixelSize::new(geometry.width, geometry.height);
                if new_size != self.real_size.get() {
                    self.set_real_size(new_size);
                    if let Some(resized) = get(&self.resized) {
                        resized(self.client_size_value(), WindowResizeReason::Unspecified);
                    }
                }
            }

            let activation_tracker = self.activation_tracker.borrow().clone();
            if let Some(activation_tracker) = activation_tracker {
                activation_tracker.on_net_wm_state_changed(&state_atoms);
            }
        }
    }

    fn schedule_input_with_event(&self, args: Rc<dyn IRawInputEventArgs>, xev: &XEvent) {
        self.x11.set_last_activity_timestamp(xlib::button_event(xev).time);
        self.schedule_input(args);
    }

    fn dispatch_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        let Some(input_root) = self.input_root_or_none() else {
            return;
        };

        if self.disabled.get() {
            if let Some(pargs) = args.downcast_ref::<RawPointerEventArgs>() {
                if pargs.type_() == RawPointerEventType::Move {
                    return;
                }
            }
        }

        let Some(input) = get(&self.input) else {
            return;
        };
        input(args.clone());
        if !args.handled() {
            if let Some(text) = crate::x11_window_ime::text_of(&args).filter(|text| !text.is_empty()) {
                input(Rc::new(RawTextInputEventArgs::new(self.keyboard.clone(), args.timestamp(), input_root, text)));
            }
        }
    }

    pub(crate) fn schedule_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        if let Some(mouse) = args.downcast_ref::<RawPointerEventArgs>() {
            mouse.set_position(mouse.position() / self.scaling());

            // Chrome hit-test for drawn decorations
            if self.needs_drawn_decorations() && mouse.type_() == RawPointerEventType::LeftButtonDown {
                if let Some(input_root) = self.input_root_or_none() {
                    if let Some(role) = input_root.hit_test_chrome_element(mouse.position()) {
                        if let Some(move_resize_side) = chrome_move_resize_side(role, self.can_resize.get()) {
                            let pos = xlib::get_cursor_pos(self.display(), self.x11.root_window(), None);
                            xlib::x_ungrab_pointer(self.display(), 0);
                            self.send_net_wm_message(
                                self.x11.atoms()._NET_WM_MOVERESIZE,
                                pos.0 as i64,
                                Some(pos.1 as i64),
                                Some(move_resize_side as i64),
                                Some(1),
                                Some(1),
                            );
                            return;
                        }
                    }
                }
            }
        }
        if let Some(drag) = args.downcast_ref::<RawDragEvent>() {
            drag.set_location(drag.location() / self.scaling());
        }

        let grouper = self.raw_event_grouper.borrow().clone();
        if let Some(grouper) = grouper {
            grouper.handle_event(args);
        }
    }

    fn mouse_event(&self, type_: RawPointerEventType, ev: &XEvent, mods: u32) {
        let Some(input_root) = self.input_root_or_none() else {
            return;
        };
        // As the reference, which reads the time and the position of every
        // pointer event through the layout of a button event (the motion
        // and crossing events have these members at the same places).
        let button_event = xlib::button_event(ev);
        let mev = Rc::new(RawPointerEventArgs::new(
            self.mouse.clone(),
            button_event.time as u64,
            input_root,
            type_,
            Point::new(button_event.x as f64, button_event.y as f64),
            XModifierMask::from_bits_retain(mods as i32).to_raw_input_modifiers(),
        ));
        self.schedule_input_with_event(mev, ev);
    }

    fn enqueue_paint(&self) {
        if !self.triggered_expose.get() {
            self.triggered_expose.set(true);
            let weak = self.this.clone();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(window) = weak.upgrade() {
                        window.triggered_expose.set(false);
                        window.do_paint();
                    }
                },
                DispatcherPriority::UI_THREAD_RENDER,
            );
        }
    }

    fn do_paint(&self) {
        if let Some(paint) = get(&self.paint) {
            paint(Rect::default());
        }
        if self.x_sync_counter.get() != 0 && self.x_sync_state.get() == XSyncState::WaitPaint {
            self.x_sync_state.set(XSyncState::None);
            xlib::x_sync_set_counter(self.display(), self.x_sync_counter.get(), self.x_sync_value.get());
        }
    }

    fn cleanup(&self, from_destroy_notification: bool) {
        // Prevent reentrancy
        if self.cleaning_up.replace(true) {
            return;
        }

        // Remove from AT-SPI tree before closing
        self.platform.untrack_window(self);

        // If we're closing the active window, speculatively hand activation back to its owner so an
        // awaited ShowDialog() sees the owner as active immediately, instead of waiting for the
        // asynchronous activation notification (auto-corrected later if the guess is wrong).
        // Mirroring win32's BeforeCloseCleanup, the owner has to be re-enabled before it's activated:
        // while it's still modally disabled the activation would be swallowed by
        // ActivateTransientChildIfNeeded. The managed layer sets the final enabled state afterwards.
        // Only speculate when there's a root _NET_ACTIVE_WINDOW notification to auto-correct against.
        let parent = self.transient_parent.borrow().as_ref().and_then(Weak::upgrade);
        if let Some(parent) = parent {
            let is_active = self.activation_tracker.borrow().as_ref().is_some_and(|tracker| tracker.is_active());
            if self.handle.get() != 0
                && parent.handle.get() != 0
                && is_active
                && self.platform.active_window_tracker().tracks_root_active_window()
            {
                parent.set_enabled(true);
                parent.set_active_speculatively();
            }
        }

        // Before doing anything else notify the TopLevel that ITopLevelImpl is no longer valid
        if self.handle.get() != 0 {
            if let Some(closed) = get(&self.closed) {
                closed();
            }
        }

        let grouper = self.raw_event_grouper.borrow_mut().take();
        if let Some(grouper) = grouper {
            grouper.dispose();
        }

        let activation_tracker = self.activation_tracker.borrow_mut().take();
        if let Some(activation_tracker) = activation_tracker {
            activation_tracker.activation_changed.unsubscribe(self.activation_subscription.get());
            activation_tracker.dispose();
        }

        let transparency_helper = self.transparency_helper.borrow_mut().take();
        if let Some(transparency_helper) = transparency_helper {
            transparency_helper.dispose();
        }

        if !self.xic.get().is_null() {
            xlib::x_destroy_ic(self.xic.get());
            self.xic.set(std::ptr::null_mut());
        }

        if self.x_sync_counter.get() != 0 {
            xlib::x_sync_destroy_counter(self.display(), self.x_sync_counter.get());
            self.x_sync_counter.set(0);
        }

        if self.handle.get() != 0 {
            let handle = self.handle.replace(0);
            self.platform.remove_window(handle);
            if let Some(xi2) = self.platform.xi2() {
                xi2.on_window_destroyed(handle);
            }
            self.pen.dispose();
            self.touch.dispose();
            if !from_destroy_notification {
                xlib::x_destroy_window(self.display(), handle);
            }
        }

        self.platform.x11_screens().changed_event.unsubscribe(self.screens_subscription.get());
        self.platform.globals().net_supported_changed.unsubscribe(self.net_supported_subscription.get());

        if self.use_render_window && self.render_handle.get() != 0 {
            self.render_handle.set(0);
            self.shared.render_handle.store(0, Ordering::SeqCst);
        }
    }

    pub(crate) fn activate_transient_child_if_needed(&self) -> bool {
        if self.disabled.get() {
            if let Some(got_input_when_disabled) = get(&self.got_input_when_disabled) {
                got_input_when_disabled();
            }
            return true;
        }

        false
    }

    fn set_active_speculatively(&self) {
        let activation_tracker = self.activation_tracker.borrow().clone();
        if let Some(activation_tracker) = activation_tracker {
            activation_tracker.set_active_speculatively();
        }
    }

    fn update_effective_system_decorations(&self) {
        // When extending client area or forcing drawn decorations, always hide WM decorations (we draw our own)
        let effective = if self.use_managed_decorations() {
            WindowDecorations::None
        } else if self.requested_window_decorations.get() == WindowDecorations::Full {
            WindowDecorations::Full
        } else {
            WindowDecorations::None
        };

        if self.window_decorations.get() == effective {
            return;
        }

        self.window_decorations.set(effective);
        self.update_motif_hints();
        self.update_size_hints(None, false);
    }

    fn move_resize(&self, position: PixelPoint, size: Size, scaling: f64) {
        self.move_(position);
        self.scaling_override.set(Some(scaling));
        self.update_scaling(true);
        self.resize_core(size, true, WindowResizeReason::Layout);
    }

    fn to_pixel_size(&self, size: Size) -> PixelSize {
        PixelSize::new((size.width * self.scaling()) as i32, (size.height * self.scaling()) as i32)
    }

    fn resize_core(&self, client_size: Size, force: bool, reason: WindowResizeReason) {
        if !force && client_size == self.client_size_value() {
            return;
        }

        let need_immediate_popup_resize = client_size != self.client_size_value();

        let pixel_size = self.to_pixel_size(client_size);
        self.update_size_hints(Some(pixel_size), false);
        xlib::x_configure_resize_window(self.display(), self.handle.get(), pixel_size.width, pixel_size.height);
        if self.use_render_window && !self.use_compositor_driven_render_window_resize {
            xlib::x_configure_resize_window(self.display(), self.render_handle.get(), pixel_size.width, pixel_size.height);
        }
        xlib::x_flush(self.display());

        if force || !self.was_mapped_at_least_once.get() || (self.override_redirect && need_immediate_popup_resize) {
            self.set_real_size(pixel_size);
            if let Some(resized) = get(&self.resized) {
                resized(self.client_size_value(), reason);
            }
        }
    }

    fn set_position(&self, value: PixelPoint) {
        if !self.use_positioning_flags.get() {
            self.use_positioning_flags.set(true);
            self.update_size_hints(None, false);
        }

        let mut changes = xlib::new_window_changes();
        changes.x = value.x;
        changes.y = value.y;

        xlib::x_configure_window(
            self.display(),
            self.handle.get(),
            (ChangeWindowFlags::CWX | ChangeWindowFlags::CWY).bits() as u32,
            &mut changes,
        );
        xlib::x_flush(self.display());
        if !self.was_mapped_at_least_once.get() {
            self.position.set(Some(value));
            if let Some(position_changed) = get(&self.position_changed) {
                position_changed(value);
            }
            self.update_scaling(false);
        }
    }

    pub(crate) fn send_net_wm_message(
        &self,
        message_type: Atom,
        l0: i64,
        l1: Option<i64>,
        l2: Option<i64>,
        l3: Option<i64>,
        l4: Option<i64>,
    ) {
        let mut xev = xlib::new_event();
        {
            let message = xlib::client_message_event_mut(&mut xev);
            message.type_ = XEventName::ClientMessage as i32;
            message.send_event = 1;
            message.window = self.handle.get();
            message.message_type = message_type;
            message.format = 32;
            message.data.set_long(0, l0 as _);
            message.data.set_long(1, l1.unwrap_or(0) as _);
            message.data.set_long(2, l2.unwrap_or(0) as _);
            message.data.set_long(3, l3.unwrap_or(0) as _);
            message.data.set_long(4, l4.unwrap_or(0) as _);
        }
        xlib::x_send_event(
            self.display(),
            self.x11.root_window(),
            false,
            (EventMask::SUBSTRUCTURE_REDIRECT_MASK | EventMask::SUBSTRUCTURE_NOTIFY_MASK).bits() as _,
            &mut xev,
        );
    }

    fn begin_move_resize(&self, side: NetWmMoveResize, e: &PointerPressedEventArgs) {
        let pos = xlib::get_cursor_pos(self.display(), self.x11.root_window(), None);
        xlib::x_ungrab_pointer(self.display(), 0);
        self.send_net_wm_message(
            self.x11.atoms()._NET_WM_MOVERESIZE,
            pos.0 as i64,
            Some(pos.1 as i64),
            Some(side as i64),
            Some(1),
            Some(1),
        ); // left button

        e.pointer().capture(None);
    }

    /// Sets `WM_CLASS` of a window of this connection (`SetWmClass(handle,
    /// wmClass)`).
    pub(crate) fn set_wm_class_of(&self, handle: XID, wm_class: Option<&str>) {
        // See https://tronche.com/gui/x/icccm/sec-4.html#WM_CLASS
        // We don't actually parse the application's command line, so we only use RESOURCE_NAME and argv[0]
        let app_id = std::env::var("RESOURCE_NAME").ok().unwrap_or_else(process_name);

        let encoded_app_id = encode_ascii(&app_id);
        let encoded_wm_class = encode_ascii(wm_class.unwrap_or(&app_id));

        xlib::x_set_class_hint(self.display(), handle, &encoded_app_id, &encoded_wm_class);
    }

    fn update_wm_hints(&self) {
        let mut hints = xlib::x_get_wm_hints(self.display(), self.handle.get()).unwrap_or_else(xlib::new_wm_hints);

        hints.flags |= XWMHintsFlags::INPUT_HINT.bits() as std::ffi::c_long;
        hints.input = i32::from(!self.disabled.get());

        xlib::x_set_wm_hints(self.display(), self.handle.get(), &mut hints);
    }

    fn use_managed_decorations(&self) -> bool {
        self.extending_client_area_to_decorations.get() || self.platform.options().force_drawn_decorations_internal()
    }

    fn needs_drawn_decorations(&self) -> bool {
        self.use_managed_decorations() || self.requested_window_decorations.get() == WindowDecorations::BorderOnly
    }

    fn change_wm_atoms(&self, enable: bool, atoms: &[Atom]) {
        if atoms.len() != 1 && atoms.len() != 2 {
            panic!("Value does not fall within the expected range.");
        }

        let names = self.x11.atoms();
        if !self.mapped.get() {
            let current =
                xlib::x_get_window_property_as_int_ptr_array(self.display(), self.handle.get(), names._NET_WM_STATE, 4)
                    .unwrap_or_default();
            let new_atoms = changed_state_atoms(&current, enable, atoms);

            xlib::x_change_property_longs(
                self.display(),
                self.handle.get(),
                names._NET_WM_STATE,
                4,
                PropertyMode::Replace,
                &new_atoms,
            );
        }

        self.send_net_wm_message(
            names._NET_WM_STATE,
            i64::from(enable),
            Some(atoms[0] as i64),
            Some(atoms.get(1).copied().unwrap_or(0) as i64),
            Some(atoms.get(2).copied().unwrap_or(0) as i64),
            Some(atoms.get(3).copied().unwrap_or(0) as i64),
        );
    }

    pub fn is_enabled(&self) -> bool {
        !self.disabled.get() && !self.mode.block_input()
    }

    /// Places the input method of the window (`UpdateImePosition`). The
    /// input methods are stage 2 of docs/porting/x11-platform.md; until
    /// then there is none to place.
    fn update_ime_position(&self) {}
}

/// `_NET_WM_STATE` with atoms added or removed (`ChangeWMAtoms` for a
/// window that is not mapped). The reference keeps the atoms in a hash
/// set, whose order is not defined; here the atoms that stay keep their
/// order and new ones follow.
pub(crate) fn changed_state_atoms(current: &[Atom], enable: bool, atoms: &[Atom]) -> Vec<Atom> {
    let mut seen = HashSet::new();
    let mut new_atoms: Vec<Atom> = current.iter().copied().filter(|atom| seen.insert(*atom)).collect();

    for atom in atoms {
        if enable {
            if seen.insert(*atom) {
                new_atoms.push(*atom);
            }
        } else if seen.remove(atom) {
            new_atoms.retain(|known| known != atom);
        }
    }
    new_atoms
}

/// The frame extents of the items of `_NET_FRAME_EXTENTS` (left, right,
/// top, bottom).
pub(crate) fn frame_extents_from_property(data: &[std::ffi::c_ulong]) -> Option<Thickness> {
    if data.len() != 4 {
        return None;
    }
    Some(Thickness::new(data[0] as i32 as f64, data[2] as i32 as f64, data[1] as i32 as f64, data[3] as i32 as f64))
}

/// The name of the process (`Process.GetCurrentProcess().ProcessName`).
fn process_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

/// `Encoding.ASCII.GetBytes`: a character outside ASCII becomes a question
/// mark.
pub(crate) fn encode_ascii(text: &str) -> Vec<u8> {
    text.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).collect()
}

impl IOptionalFeatureProvider for X11Window {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        // Not available yet, each a feature the reference answers here:
        // the exporter of the native menu, the input method and the native
        // control host (stage 2 of docs/porting/x11-platform.md).

        if feature_type == TypeId::of::<dyn IStorageProvider>() {
            let storage_provider = self.storage_provider.borrow().clone()?;
            return Some(Rc::new(storage_provider));
        }

        if feature_type == TypeId::of::<dyn IClipboard>() {
            let clipboard = FerroLocator::current().get_required_service::<dyn IClipboard>();
            return Some(Rc::new(clipboard));
        }

        if feature_type == TypeId::of::<dyn IPlatformClipboardManagerImpl>() {
            let manager = FerroLocator::current().get_required_service::<dyn IPlatformClipboardManagerImpl>();
            return Some(Rc::new(manager));
        }

        if feature_type == TypeId::of::<dyn ILauncher>() {
            let launcher: Rc<dyn ILauncher> = Rc::new(BclLauncher::new());
            return Some(Rc::new(launcher));
        }

        if feature_type == TypeId::of::<dyn IX11OptionsToplevelImplFeature>() {
            let this: Rc<dyn IX11OptionsToplevelImplFeature> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }

        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            return Some(Rc::new(self.platform.screens()));
        }

        None
    }
}

impl IDisposable for X11Window {
    fn dispose(&self) {
        self.cleanup(false);
    }
}

impl ITopLevelImpl for X11Window {
    fn desktop_scaling(&self) -> f64 {
        self.scaling()
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        Some(self.platform_handle.clone())
    }

    fn client_size(&self) -> Size {
        self.client_size_value()
    }

    fn render_scaling(&self) -> f64 {
        self.scaling()
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        self.surfaces.borrow().clone()
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.platform.compositor().clone())
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        get(&self.input)
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        set(&self.input, value);
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        get(&self.paint)
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        set(&self.paint, value);
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        get(&self.resized)
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        set(&self.resized, value);
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        get(&self.scaling_changed)
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        set(&self.scaling_changed, value);
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        self.transparency_helper.borrow().as_ref().and_then(|helper| helper.transparency_level_changed())
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        let helper = self.transparency_helper.borrow().clone();
        if let Some(helper) = helper {
            helper.set_transparency_level_changed(value);
        }
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        let previous = self.input_root.replace(Some(input_root));
        drop(previous);
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        self.mode.point_to_client(self, point)
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        self.mode.point_to_screen(self, point)
    }

    fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
        match cursor {
            None => {
                xlib::x_define_cursor(self.display(), self.handle.get(), self.x11.default_cursor());
            }
            Some(cursor) => {
                if let Some(cursor) = cursor.as_any().downcast_ref::<CursorImpl>() {
                    xlib::x_define_cursor(self.display(), self.handle.get(), cursor.handle());
                }
            }
        }
    }

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.closed)
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.closed, value);
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.lost_focus)
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.lost_focus, value);
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        if self.platform.options().overlay_popups {
            return None;
        }
        let parent: Rc<dyn IWindowImpl> = self.this.upgrade()?;
        Some(X11Window::new(&self.platform, Some(parent), false))
    }

    fn set_transparency_level_hint(&self, transparency_levels: &[WindowTransparencyLevel]) {
        let helper = self.transparency_helper.borrow().clone();
        if let Some(helper) = helper {
            helper.set_transparency_request(transparency_levels);
        }
    }

    fn transparency_level(&self) -> WindowTransparencyLevel {
        self.transparency_helper
            .borrow()
            .as_ref()
            .map_or_else(WindowTransparencyLevel::none, |helper| helper.current_level())
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        AcrylicPlatformCompensationLevels::new(1.0, 0.8, 0.8)
    }

    fn set_frame_theme_variant(&self, _theme_variant: Option<PlatformThemeVariant>) {}

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
        Some(self)
    }
}

impl IWindowBaseImpl for X11Window {
    fn frame_size(&self) -> Option<Size> {
        let extents = self.get_frame_extents()?;
        let real_size = self.real_size.get();

        Some(Size::new(
            (real_size.width as f64 + extents.left + extents.right) / self.scaling(),
            (real_size.height as f64 + extents.top + extents.bottom) / self.scaling(),
        ))
    }

    fn show(&self, activate: bool, is_dialog: bool) {
        self.mode.show(self, activate, is_dialog);

        self.platform.track_window(self);
    }

    fn hide(&self) {
        self.mode.hide(self);
    }

    fn position(&self) -> PixelPoint {
        let Some(position) = self.position.get() else {
            return PixelPoint::default();
        };

        let extents = self.get_frame_extents().unwrap_or_default();

        PixelPoint::new(position.x - extents.left as i32, position.y - extents.top as i32)
    }

    fn position_changed(&self) -> Option<Rc<dyn Fn(PixelPoint)>> {
        get(&self.position_changed)
    }

    fn set_position_changed(&self, value: Option<Rc<dyn Fn(PixelPoint)>>) {
        set(&self.position_changed, value);
    }

    fn activate(&self) {
        self.mode.activate(self);
    }

    fn deactivated(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.deactivated)
    }

    fn set_deactivated(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.deactivated, value);
    }

    fn activated(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.activated)
    }

    fn set_activated(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.activated, value);
    }

    fn max_auto_size_hint(&self) -> Size {
        let mut sizes: Vec<Size> = self
            .platform
            .x11_screens()
            .all_screens()
            .iter()
            .map(|s| s.bounds().size().to_size(s.scaling()))
            .collect();
        // A stable sort, descending, as the ordering of the reference.
        sizes.sort_by(|a, b| (b.width + b.height).total_cmp(&(a.width + a.height)));
        sizes.first().copied().unwrap_or_default()
    }

    fn set_topmost(&self, value: bool) {
        self.change_wm_atoms(value, &[self.x11.atoms()._NET_WM_STATE_ABOVE]);
    }
}

impl IWindowImpl for X11Window {
    fn window_state(&self) -> WindowState {
        self.last_window_state.get()
    }

    fn set_window_state(&self, value: WindowState) {
        let previous_state = self.last_window_state.get();
        if previous_state == value {
            return;
        }

        let atoms = self.x11.atoms();
        if value == WindowState::Minimized {
            xlib::x_iconify_window(self.display(), self.handle.get(), self.x11.default_screen());
            return;
        }

        // When going from Minimized to any other state programatically, we might need to re-map the window.
        // Not doing that will leave the window invisible.
        let needs_remap = self.shown.get() && previous_state == WindowState::Minimized && !self.mapped.get();

        let maximized = [atoms._NET_WM_STATE_MAXIMIZED_VERT, atoms._NET_WM_STATE_MAXIMIZED_HORZ];
        if value == WindowState::Maximized {
            self.change_wm_atoms(false, &[atoms._NET_WM_STATE_HIDDEN]);
            self.change_wm_atoms(false, &[atoms._NET_WM_STATE_FULLSCREEN]);
            self.change_wm_atoms(true, &maximized);
        } else if value == WindowState::FullScreen {
            self.change_wm_atoms(false, &[atoms._NET_WM_STATE_HIDDEN]);
            self.change_wm_atoms(true, &[atoms._NET_WM_STATE_FULLSCREEN]);
            self.change_wm_atoms(false, &maximized);
        } else {
            self.change_wm_atoms(false, &[atoms._NET_WM_STATE_HIDDEN]);
            self.change_wm_atoms(false, &[atoms._NET_WM_STATE_FULLSCREEN]);
            self.change_wm_atoms(false, &maximized);
        }

        if needs_remap {
            xlib::x_map_window(self.display(), self.handle.get());
        }

        if self.shown.get() {
            self.send_net_wm_message(
                atoms._NET_ACTIVE_WINDOW,
                1,
                Some(self.x11.last_activity_timestamp() as i64),
                Some(0),
                None,
                None,
            );
        }
    }

    fn window_state_getter_is_usable(&self) -> bool {
        true
    }

    fn window_state_changed(&self) -> Option<Rc<dyn Fn(WindowState)>> {
        get(&self.window_state_changed)
    }

    fn set_window_state_changed(&self, value: Option<Rc<dyn Fn(WindowState)>>) {
        set(&self.window_state_changed, value);
    }

    fn set_title(&self, title: Option<&str>) {
        let atoms = self.x11.atoms();
        match title.filter(|title| !title.is_empty()) {
            None => {
                xlib::x_delete_property(self.display(), self.handle.get(), atoms._NET_WM_NAME);
                xlib::x_delete_property(self.display(), self.handle.get(), atoms.WM_NAME);
            }
            Some(title) => {
                xlib::x_change_property_bytes(
                    self.display(),
                    self.handle.get(),
                    atoms._NET_WM_NAME,
                    atoms.UTF8_STRING,
                    PropertyMode::Replace,
                    title.as_bytes(),
                );
                xlib::x_store_name(self.display(), self.handle.get(), title);
            }
        }
    }

    fn set_parent(&self, parent: Option<Rc<dyn IWindowImpl>>) {
        let parent_handle = parent.as_ref().and_then(|parent| parent.handle()).map(|handle| handle.handle());
        match (parent, parent_handle) {
            (Some(parent), Some(parent_handle)) if parent_handle != 0 => {
                let transient_parent = parent
                    .as_any()
                    .downcast_ref::<X11Window>()
                    .map(|window| window.this.clone());
                *self.transient_parent.borrow_mut() = transient_parent;
                xlib::x_set_transient_for_hint(self.display(), self.handle.get(), parent_handle as XID);
            }
            _ => {
                *self.transient_parent.borrow_mut() = None;
                xlib::x_delete_property(self.display(), self.handle.get(), self.x11.atoms().WM_TRANSIENT_FOR);
            }
        }
    }

    fn set_enabled(&self, enable: bool) {
        self.disabled.set(!enable);

        self.update_wm_hints();
        self.update_motif_hints();

        if enable {
            // Some window managers ignore Motif hints when switching from disabled to enabled on the first update
            // so setting it again forces the update
            self.update_motif_hints();
        } else {
            // Showing a dialog should result in pointer capture being lost. We don't currently use XGrabPointer on
            // X11 to implement pointer capture, so no we have no OS-level event to hook into. Instead, release the
            // pointer capture when the owner window is disabled. This behavior matches win32, which sends a
            // WM_CANCELMODE message when EnableWindow(hWnd, false) is called from SetEnabled.
            self.mouse.platform_capture_lost();
            self.touch.platform_capture_lost();
        }
    }

    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.got_input_when_disabled)
    }

    fn set_got_input_when_disabled(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.got_input_when_disabled, value);
    }

    fn set_window_decorations(&self, enabled: WindowDecorations) {
        self.requested_window_decorations.set(enabled);
        self.update_effective_system_decorations();
    }

    fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>) {
        let same = match (&*self.icon_impl.borrow(), &icon) {
            (None, None) => true,
            (Some(current), Some(icon)) => std::ptr::addr_eq(Rc::as_ptr(current), Rc::as_ptr(icon)),
            _ => false,
        };
        if same {
            return;
        }

        let previous = self.icon_impl.replace(icon.clone());
        drop(previous);

        let atoms = self.x11.atoms();
        match icon {
            Some(icon) => {
                let data = match X11IconData::from_icon_impl(&icon) {
                    Ok(data) => data,
                    Err(error) => panic!("Unable to read the window icon: {error}"),
                };
                xlib::x_change_property_longs(
                    self.display(),
                    self.handle.get(),
                    atoms._NET_WM_ICON,
                    6,
                    PropertyMode::Replace,
                    data.data(),
                );
            }
            None => {
                xlib::x_delete_property(self.display(), self.handle.get(), atoms._NET_WM_ICON);
            }
        }
    }

    fn show_taskbar_icon(&self, value: bool) {
        self.change_wm_atoms(!value, &[self.x11.atoms()._NET_WM_STATE_SKIP_TASKBAR]);
    }

    fn can_resize(&self, value: bool) {
        self.can_resize.set(value);
        self.update_motif_hints();
        self.update_size_hints(None, false);
    }

    fn set_can_minimize(&self, value: bool) {
        self.can_minimize.set(value);
        self.update_motif_hints();
    }

    fn set_can_maximize(&self, value: bool) {
        self.can_maximize.set(value);
        self.update_motif_hints();
    }

    fn closing(&self) -> Option<Rc<dyn Fn(WindowCloseReason) -> bool>> {
        get(&self.closing)
    }

    fn set_closing(&self, value: Option<Rc<dyn Fn(WindowCloseReason) -> bool>>) {
        set(&self.closing, value);
    }

    fn is_client_area_extended_to_decorations(&self) -> bool {
        self.is_client_area_extended_to_decorations.get()
    }

    fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>> {
        get(&self.extend_client_area_to_decorations_changed)
    }

    fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>) {
        set(&self.extend_client_area_to_decorations_changed, value);
    }

    fn needs_managed_decorations(&self) -> bool {
        self.needs_drawn_decorations()
    }

    fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration {
        if self.needs_drawn_decorations() {
            PlatformRequestedDrawnDecoration::BORDER
                | PlatformRequestedDrawnDecoration::RESIZE_GRIPS
                | PlatformRequestedDrawnDecoration::TITLE_BAR
                | PlatformRequestedDrawnDecoration::SHADOW
        } else {
            PlatformRequestedDrawnDecoration::NONE
        }
    }

    fn extended_margins(&self) -> Thickness {
        Thickness::default()
    }

    fn off_screen_margin(&self) -> Thickness {
        Thickness::default()
    }

    fn begin_move_drag(&self, e: &PointerPressedEventArgs) {
        self.begin_move_resize(NetWmMoveResize::_NET_WM_MOVERESIZE_MOVE, e);
    }

    fn begin_resize_drag(&self, edge: WindowEdge, e: &PointerPressedEventArgs) {
        self.begin_move_resize(move_resize_side(edge), e);
    }

    fn resize(&self, client_size: Size, reason: WindowResizeReason) {
        self.resize_core(client_size, false, reason);
    }

    fn move_(&self, point: PixelPoint) {
        self.set_position(point);
        self.update_scaling(false);
    }

    fn set_min_max_size(&self, min_size: Size, max_size: Size) {
        self.scaled_min_max_size.set((min_size, max_size));
        self.min_max_size.set(min_max_pixel_size(min_size, max_size, self.scaling()));
        self.update_size_hints(None, false);
    }

    fn set_extend_client_area_to_decorations_hint(&self, extend_into_client_area_hint: bool) {
        if !self.platform.options().enable_drawn_decorations_internal() {
            return;
        }

        if self.extending_client_area_to_decorations.get() == extend_into_client_area_hint {
            return;
        }

        self.extending_client_area_to_decorations.set(extend_into_client_area_hint);
        self.update_effective_system_decorations();

        self.is_client_area_extended_to_decorations.set(extend_into_client_area_hint);
        if let Some(changed) = get(&self.extend_client_area_to_decorations_changed) {
            changed(extend_into_client_area_hint);
        }
    }

    fn set_extend_client_area_title_bar_height_hint(&self, _title_bar_height: f64) {}

    fn allowed_window_actions(&self) -> PlatformAllowedWindowActions {
        self.allowed_actions_value()
    }

    fn allowed_window_actions_changed(&self) -> Option<Rc<dyn Fn(PlatformAllowedWindowActions)>> {
        get(&self.allowed_window_actions_changed)
    }

    fn set_allowed_window_actions_changed(&self, value: Option<Rc<dyn Fn(PlatformAllowedWindowActions)>>) {
        set(&self.allowed_window_actions_changed, value);
    }
}

impl IPopupImpl for X11Window {
    fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>> {
        self.popup_positioner.borrow().clone()
    }

    fn set_window_manager_add_shadow_hint(&self, _enabled: bool) {}

    fn take_focus(&self) {
        // TODO: Not yet implemented: need to check if this is required on X11 or not.
    }

    fn set_hit_test_visible(&self, is_hit_test_visible: bool) {
        if !self.x11.has_x_fixes() {
            return;
        }

        // The render window is a child of _handle when a GPU backend is in use. Shaping the
        // parent alone doesn't take the child out of the input hierarchy, so it would keep
        // capturing the pointer.
        let handle = self.handle.get();
        let render_handle = self.render_handle.get();
        if render_handle != handle {
            xlib::x_fixes_set_input_shape(self.display(), &[handle, render_handle], !is_hit_test_visible);
        } else {
            xlib::x_fixes_set_input_shape(self.display(), &[handle], !is_hit_test_visible);
        }

        xlib::x_flush(self.display());
    }
}

impl IXI2Client for X11Window {
    fn is_enabled(&self) -> bool {
        X11Window::is_enabled(self)
    }

    fn input_root(&self) -> Rc<dyn IInputRoot> {
        X11Window::input_root(self)
    }

    fn schedule_xi2_input(&self, args: Rc<dyn IRawInputEventArgs>) {
        if let Some(pargs) = args.downcast_ref::<RawPointerEventArgs>() {
            if matches!(
                pargs.type_(),
                RawPointerEventType::TouchBegin
                    | RawPointerEventType::TouchUpdate
                    | RawPointerEventType::LeftButtonDown
                    | RawPointerEventType::RightButtonDown
                    | RawPointerEventType::MiddleButtonDown
                    | RawPointerEventType::NonClientLeftButtonDown
            ) && self.activate_transient_child_if_needed()
            {
                return;
            }
            if pargs.type_() == RawPointerEventType::TouchEnd && self.activate_transient_child_if_needed() {
                pargs.set_type(RawPointerEventType::TouchCancel);
            }
        }

        self.schedule_input(args);
    }

    fn mouse_device(&self) -> Rc<dyn IInputDevice> {
        self.mouse.clone()
    }

    fn pen_device(&self) -> Rc<dyn IInputDevice> {
        self.pen.clone()
    }

    fn touch_device(&self) -> Rc<dyn IInputDevice> {
        self.touch.clone()
    }
}

impl IX11OptionsToplevelImplFeature for X11Window {
    fn set_net_wm_window_type(&self, type_: X11NetWmWindowType) {
        if self.handle.get() == 0 {
            return;
        }

        let atoms = self.x11.atoms();
        let atom = match type_ {
            X11NetWmWindowType::Dialog => atoms._NET_WM_WINDOW_TYPE_DIALOG,
            X11NetWmWindowType::Utility => atoms._NET_WM_WINDOW_TYPE_UTILITY,
            X11NetWmWindowType::Toolbar => atoms._NET_WM_WINDOW_TYPE_TOOLBAR,
            X11NetWmWindowType::Splash => atoms._NET_WM_WINDOW_TYPE_SPLASH,
            X11NetWmWindowType::Dock => atoms._NET_WM_WINDOW_TYPE_DOCK,
            X11NetWmWindowType::Desktop => atoms._NET_WM_WINDOW_TYPE_DESKTOP,
            _ => atoms._NET_WM_WINDOW_TYPE_NORMAL,
        };

        xlib::x_change_property_longs(
            self.display(),
            self.handle.get(),
            atoms._NET_WM_WINDOW_TYPE,
            atoms.ATOM,
            PropertyMode::Replace,
            &[atom],
        );
    }

    fn set_wm_class(&self, class_name: Option<&str>) {
        if self.handle.get() == 0 {
            return;
        }
        let default_class = self.platform.options().wm_class.clone();
        self.set_wm_class_of(self.handle.get(), class_name.or(default_class.as_deref()));
    }
}

/// The window frames are drawn into, as a native surface
/// (`SurfacePlatformHandle`).
pub struct SurfacePlatformHandle {
    shared: Arc<WindowShared>,
}

impl SurfacePlatformHandle {
    fn client_size(&self) -> Size {
        let real_size = self.shared.real_size();
        Size::new(real_size.width as f64 / self.shared.scaling(), real_size.height as f64 / self.shared.scaling())
    }
}

impl IPlatformHandle for SurfacePlatformHandle {
    fn handle(&self) -> isize {
        self.shared.render_handle.load(Ordering::SeqCst) as isize
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some("XID")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IPlatformRenderSurface for SurfacePlatformHandle {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl INativePlatformHandleSurface for SurfacePlatformHandle {
    fn size(&self) -> PixelSize {
        // `ToPixelSize(ClientSize)` of the window.
        let client_size = self.client_size();
        PixelSize::new(
            (client_size.width * self.shared.scaling()) as i32,
            (client_size.height * self.shared.scaling()) as i32,
        )
    }

    fn scaling(&self) -> f64 {
        self.shared.scaling()
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn decoded(hints: MotifWmHints) -> (MotifFunctions, MotifDecorations) {
        (
            MotifFunctions::from_bits_retain(hints.functions as i32),
            MotifDecorations::from_bits_retain(hints.decorations as i32),
        )
    }

    #[test]
    fn a_plain_window_has_every_function_and_decoration() {
        let hints = motif_hints(false, true, true, true, false);
        assert_eq!(hints.flags as i32, (MotifFlags::DECORATIONS | MotifFlags::FUNCTIONS).bits());
        let (functions, decorations) = decoded(hints);
        assert_eq!(
            functions,
            MotifFunctions::MOVE
                | MotifFunctions::CLOSE
                | MotifFunctions::RESIZE
                | MotifFunctions::MINIMIZE
                | MotifFunctions::MAXIMIZE
        );
        assert_eq!(
            decorations,
            MotifDecorations::MENU
                | MotifDecorations::TITLE
                | MotifDecorations::BORDER
                | MotifDecorations::MAXIMIZE
                | MotifDecorations::MINIMIZE
                | MotifDecorations::RESIZE_H
        );
        // The property is five items of format 32.
        assert_eq!(hints.to_longs(), [3, functions.bits() as _, decorations.bits() as _, 0, 0]);
    }

    #[test]
    fn the_hints_follow_what_the_window_may_do() {
        let (functions, decorations) = decoded(motif_hints(false, false, true, true, false));
        assert!(!functions.contains(MotifFunctions::RESIZE) && !decorations.contains(MotifDecorations::RESIZE_H));
        assert!(functions.contains(MotifFunctions::MAXIMIZE | MotifFunctions::MINIMIZE));

        let (functions, decorations) = decoded(motif_hints(false, true, false, false, false));
        assert!(!functions.intersects(MotifFunctions::MINIMIZE | MotifFunctions::MAXIMIZE));
        assert!(!decorations.intersects(MotifDecorations::MINIMIZE | MotifDecorations::MAXIMIZE));
        assert!(functions.contains(MotifFunctions::RESIZE));

        // A disabled window (the owner of a dialog) can only be moved and closed.
        let (functions, decorations) = decoded(motif_hints(false, true, true, true, true));
        assert_eq!(functions, MotifFunctions::MOVE | MotifFunctions::CLOSE);
        assert_eq!(decorations, MotifDecorations::MENU | MotifDecorations::TITLE | MotifDecorations::BORDER);

        // Without decorations the functions stay.
        let (functions, decorations) = decoded(motif_hints(true, true, true, true, false));
        assert!(decorations.is_empty());
        assert!(functions.contains(MotifFunctions::RESIZE | MotifFunctions::CLOSE));
    }

    const LIMITS: (PixelSize, PixelSize) =
        (PixelSize::new(1, 1), PixelSize::new(MAX_WINDOW_DIMENSION, MAX_WINDOW_DIMENSION));

    #[test]
    fn the_default_size_hints_state_a_minimum_only() {
        let hints = size_hints(LIMITS, PixelSize::new(640, 480), true, false, None, false);
        assert_eq!(hints.flags, XSizeHintsFlags::P_MIN_SIZE | XSizeHintsFlags::P_RESIZE_INC);
        assert_eq!(hints.min, PixelSize::new(1, 1));
        // The default maximum is too large to be a hint.
        assert_eq!(hints.max, None);
    }

    #[test]
    fn a_window_that_cannot_be_resized_is_pinned_to_its_size() {
        let hints = size_hints(LIMITS, PixelSize::new(640, 480), false, false, None, true);
        assert_eq!(hints.min, PixelSize::new(640, 480));
        assert_eq!(hints.max, Some(PixelSize::new(640, 480)));
        assert!(hints.flags.contains(XSizeHintsFlags::P_MAX_SIZE | XSizeHintsFlags::P_POSITION | XSizeHintsFlags::P_SIZE));

        // A resize by the application first moves the pin, so that the window manager lets it through.
        let hints = size_hints(LIMITS, PixelSize::new(640, 480), false, false, Some(PixelSize::new(800, 600)), false);
        assert_eq!((hints.min, hints.max), (PixelSize::new(800, 600), Some(PixelSize::new(800, 600))));

        // A disabled window is pinned although it can be resized.
        let hints = size_hints(LIMITS, PixelSize::new(300, 200), true, true, None, false);
        assert_eq!((hints.min, hints.max), (PixelSize::new(300, 200), Some(PixelSize::new(300, 200))));
    }

    #[test]
    fn a_resize_beyond_the_limits_widens_them() {
        let limits = (PixelSize::new(200, 100), PixelSize::new(800, 600));
        let hints = size_hints(limits, PixelSize::new(400, 300), true, false, Some(PixelSize::new(1000, 50)), false);
        assert_eq!(hints.min, PixelSize::new(200, 50));
        assert_eq!(hints.max, Some(PixelSize::new(1000, 600)));
        let hints = size_hints(limits, PixelSize::new(400, 300), true, false, None, false);
        assert_eq!((hints.min, hints.max), (limits.0, Some(limits.1)));
    }

    #[test]
    fn the_limits_in_pixels_follow_the_scaling() {
        let unlimited = Size::new(f64::INFINITY, f64::INFINITY);
        assert_eq!(
            min_max_pixel_size(Size::new(1.0, 1.0), unlimited, 2.0),
            (PixelSize::new(2, 2), PixelSize::new(MAX_WINDOW_DIMENSION, MAX_WINDOW_DIMENSION))
        );
        assert_eq!(
            min_max_pixel_size(Size::new(0.0, 100.5), Size::new(400.0, 300.0), 1.5),
            (PixelSize::new(1, 150), PixelSize::new(600, 450))
        );
        // A maximum below the minimum is raised to it.
        assert_eq!(
            min_max_pixel_size(Size::new(200.0, 200.0), Size::new(100.0, 100.0), 1.0),
            (PixelSize::new(200, 200), PixelSize::new(200, 200))
        );
    }

    const STATES: WindowStateAtoms = WindowStateAtoms { hidden: 10, maximized_horz: 11, maximized_vert: 12, fullscreen: 13 };

    #[test]
    fn the_window_state_is_read_from_the_state_atoms() {
        let state = |atoms: &[Atom]| window_state_from_net_wm_state(atoms, STATES);
        assert_eq!(state(&[]), WindowState::Normal);
        assert_eq!(state(&[99]), WindowState::Normal);
        assert_eq!(state(&[11, 12]), WindowState::Maximized);
        // Maximized in one direction only (a tiled window) is not maximized.
        assert_eq!(state(&[11]), WindowState::Normal);
        assert_eq!(state(&[12]), WindowState::Normal);
        assert_eq!(state(&[13]), WindowState::FullScreen);
        assert_eq!(state(&[11, 12, 13]), WindowState::FullScreen);
        // A minimized window keeps its other states; minimized wins.
        assert_eq!(state(&[10, 11, 12]), WindowState::Minimized);
        assert_eq!(state(&[13, 10]), WindowState::Minimized);
    }

    const ACTIONS: WindowActionAtoms = WindowActionAtoms { maximize_vert: 20, maximize_horz: 21, fullscreen: 22, minimize: 23 };

    #[test]
    fn the_allowed_actions_are_those_the_window_manager_supports() {
        assert_eq!(allowed_actions(None, ACTIONS), PlatformAllowedWindowActions::ALL);
        assert_eq!(allowed_actions(Some(&[]), ACTIONS), PlatformAllowedWindowActions::NONE);
        assert_eq!(allowed_actions(Some(&[20]), ACTIONS), PlatformAllowedWindowActions::NONE);
        assert_eq!(allowed_actions(Some(&[20, 21]), ACTIONS), PlatformAllowedWindowActions::MAXIMIZE);
        assert_eq!(
            allowed_actions(Some(&[22, 23]), ACTIONS),
            PlatformAllowedWindowActions::FULLSCREEN | PlatformAllowedWindowActions::MINIMIZE
        );
        assert_eq!(allowed_actions(Some(&[20, 21, 22, 23]), ACTIONS), PlatformAllowedWindowActions::ALL);
    }

    #[test]
    fn state_atoms_are_added_and_removed_as_a_set() {
        assert_eq!(changed_state_atoms(&[], true, &[5, 6]), vec![5, 6]);
        assert_eq!(changed_state_atoms(&[5, 7], true, &[5, 6]), vec![5, 7, 6]);
        assert_eq!(changed_state_atoms(&[5, 7, 6], false, &[5, 6]), vec![7]);
        assert_eq!(changed_state_atoms(&[7], false, &[5]), vec![7]);
        // A property with a repeated atom is cleaned.
        assert_eq!(changed_state_atoms(&[7, 7, 8], true, &[8]), vec![7, 8]);
    }

    #[test]
    fn the_frame_extents_are_left_right_top_bottom() {
        assert_eq!(frame_extents_from_property(&[1, 2, 30, 4]), Some(Thickness::new(1.0, 30.0, 2.0, 4.0)));
        assert_eq!(frame_extents_from_property(&[1, 2, 3]), None);
        assert_eq!(frame_extents_from_property(&[]), None);
    }

    #[test]
    fn the_edges_map_to_the_sides_of_the_move_resize_protocol() {
        assert_eq!(move_resize_side(WindowEdge::NorthWest) as i32, 0);
        assert_eq!(move_resize_side(WindowEdge::North) as i32, 1);
        assert_eq!(move_resize_side(WindowEdge::NorthEast) as i32, 2);
        assert_eq!(move_resize_side(WindowEdge::East) as i32, 3);
        assert_eq!(move_resize_side(WindowEdge::SouthEast) as i32, 4);
        assert_eq!(move_resize_side(WindowEdge::South) as i32, 5);
        assert_eq!(move_resize_side(WindowEdge::SouthWest) as i32, 6);
        assert_eq!(move_resize_side(WindowEdge::West) as i32, 7);
        assert_eq!(NetWmMoveResize::_NET_WM_MOVERESIZE_MOVE as i32, 8);
    }

    #[test]
    fn drawn_decorations_start_moves_and_resizes() {
        assert_eq!(
            chrome_move_resize_side(WindowDecorationsElementRole::TitleBar, false),
            Some(NetWmMoveResize::_NET_WM_MOVERESIZE_MOVE)
        );
        assert_eq!(
            chrome_move_resize_side(WindowDecorationsElementRole::ResizeSE, true),
            Some(NetWmMoveResize::_NET_WM_MOVERESIZE_SIZE_BOTTOMRIGHT)
        );
        assert_eq!(chrome_move_resize_side(WindowDecorationsElementRole::ResizeSE, false), None);
        assert_eq!(chrome_move_resize_side(WindowDecorationsElementRole::User, true), None);
        assert_eq!(chrome_move_resize_side(WindowDecorationsElementRole::None, true), None);
    }

    #[test]
    fn core_buttons_are_pointer_buttons_or_wheel_steps() {
        assert_eq!(core_button_event_type(1, true), Some(RawPointerEventType::LeftButtonDown));
        assert_eq!(core_button_event_type(2, false), Some(RawPointerEventType::MiddleButtonUp));
        assert_eq!(core_button_event_type(3, true), Some(RawPointerEventType::RightButtonDown));
        assert_eq!(core_button_event_type(8, false), Some(RawPointerEventType::XButton1Up));
        assert_eq!(core_button_event_type(9, true), Some(RawPointerEventType::XButton2Down));
        assert_eq!(core_button_event_type(4, true), None);
        assert_eq!(core_scroll_delta(4), Vector::new(0.0, 1.0));
        assert_eq!(core_scroll_delta(5), Vector::new(0.0, -1.0));
        assert_eq!(core_scroll_delta(6), Vector::new(1.0, 0.0));
        assert_eq!(core_scroll_delta(7), Vector::new(-1.0, 0.0));
    }

    #[test]
    fn the_class_hint_is_ascii() {
        assert_eq!(encode_ascii("control-catalog"), b"control-catalog");
        assert_eq!(encode_ascii("Zażółć"), b"Za????");
    }

    #[test]
    fn enter_and_leave_are_handled_for_the_window_itself() {
        assert!(is_handled_leave_enter_detail(NotifyDetail::NotifyAncestor as i32));
        assert!(is_handled_leave_enter_detail(NotifyDetail::NotifyVirtual as i32));
        assert!(is_handled_leave_enter_detail(NotifyDetail::NotifyNonlinear as i32));
        assert!(is_handled_leave_enter_detail(NotifyDetail::NotifyNonlinearVirtual as i32));
        // The pointer moved to a child window: the window still has it.
        assert!(!is_handled_leave_enter_detail(NotifyDetail::NotifyInferior as i32));
    }
}
