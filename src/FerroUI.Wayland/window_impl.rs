//! A window of the backend on the UI thread (the port of `WindowImpl.cs`):
//! the state the framework asks for, and the requests to the top-level of the
//! worker.
//!
//! Not built yet (`docs/porting/wayland-platform.md`, stage 2): the text
//! input method, and the file chooser portal of the storage provider with its
//! parent handle from `xdg-foreign` (the managed dialogs answer alone).

use crate::server::persistent::decoration_mode::DecorationMode;
use crate::server::persistent::i_w_surface::IWSurface;
use crate::server::persistent::i_w_surface_event_sink::PlatformInputEventCookie;
use crate::popup_impl::{IPopupParent, PopupImpl};
use crate::server::persistent::i_w_xdg_top_level::{IWXdgShellSurface, IWXdgTopLevel, WXdgShellSurfaceProxy, WXdgTopLevelProxy};
use crate::server::persistent::xdg_configure_batch::{XdgConfigureBatch, XdgToplevelStates};
use crate::server::wayland_dispatch_priority::WaylandDispatchPriority;
use crate::server::wayland_worker_client::WaylandWorkerClient;
use crate::window_impl_base::{get, set, Callback, ISinkOwner, RawEvent, WindowBaseImpl};
use crate::window_impl_sink::WindowImplSink;
use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{IInputRoot, KeyboardDevice, PointerPressedEventArgs, WindowDecorationsElementRole};
use ferroui_base::platform::storage::file_io::BclLauncher;
use ferroui_base::platform::storage::{FallbackStorageProvider, ILauncher, IStorageProvider, StorageProviderFactory};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider, PlatformThemeVariant};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Point, Rect, Size, Thickness};
use ferroui_controls::platform::{
    IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl, IWindowBaseImpl, IWindowIconImpl, IWindowImpl,
    PlatformRequestedDrawnDecoration,
};
use ferroui_controls::{
    AcrylicPlatformCompensationLevels, TopLevel, WindowCloseReason, WindowDecorations, WindowEdge, WindowResizeReason,
    WindowState, WindowTransparencyLevel,
};
use ferroui_dialogs::ManagedStorageProvider;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};
use wayland_protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;

/// The render surfaces of a window, which the thread that renders asks for.
type SharedSurfaces = Arc<Mutex<Vec<Arc<dyn IPlatformRenderSurface>>>>;

pub struct WindowImpl {
    this: Weak<WindowImpl>,
    base: WindowBaseImpl,
    current_sink: RefCell<Option<Rc<WindowImplSink>>>,
    surfaces: SharedSurfaces,
    surface_proxy: RefCell<Option<WXdgTopLevelProxy>>,
    is_activated: Cell<bool>,
    max_auto_size_hint: Cell<Size>,
    window_state: Cell<WindowState>,
    can_resize: Cell<bool>,
    restore_bounds: Cell<Size>,
    shadow_extents: Cell<Thickness>,
    min_size: Cell<Option<Size>>,
    max_size: Cell<Option<Size>>,
    // Sticky-CSD latch on the UI side. Set true the first time the
    // framework asks for partial / no decorations. Never resets,
    // including across reconnects.
    // This is a limitation of V1 of the protocol that's supported in the wild
    csd_sticky: Cell<bool>,
    title: RefCell<Option<String>>,
    storage_provider: RefCell<Option<Rc<FallbackStorageProvider>>>,
    server_reported_mode: Cell<DecorationMode>,
    app_requested_decoration_extension: Cell<bool>,
    resize_stack_depth: Cell<i32>,

    window_state_changed: Callback<dyn Fn(WindowState)>,
    got_input_when_disabled: Callback<dyn Fn()>,
    closing: Callback<dyn Fn(WindowCloseReason) -> bool>,
    extend_client_area_to_decorations_changed: Callback<dyn Fn(bool)>,
    drawn_decorations_request_changed: Callback<dyn Fn()>,
}

/// What a sealed configure asks of a window: its state, whether it is active, and its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ConfigureOutcome {
    pub window_state: WindowState,
    pub is_activated: bool,
    /// The size to restore remembered when the window leaves the normal state.
    pub restore_bounds: Size,
    /// The new client size, when it differs from the current one.
    pub client_size: Option<Size>,
}

/// Works out what a configure changes (the computation of `ApplyConfigureBatch`).
pub(crate) fn evaluate_configure(
    batch: &XdgConfigureBatch,
    old_window_state: WindowState,
    client_size: Size,
    restore_bounds: Size,
    shadow_extents: Thickness,
) -> ConfigureOutcome {
    // Map xdg_toplevel states to the window state of the framework
    let new_window_state = if batch.states.contains(XdgToplevelStates::FULLSCREEN) {
        WindowState::FullScreen
    } else if batch.states.contains(XdgToplevelStates::MAXIMIZED) {
        WindowState::Maximized
    } else {
        WindowState::Normal
    };

    // Save restore bounds when leaving Normal state
    let restore_bounds = if old_window_state == WindowState::Normal && new_window_state != WindowState::Normal {
        client_size
    } else {
        restore_bounds
    };

    // Resize: configure size is window geometry (excluding shadows).
    // Add shadow extents to get full surface size for the UI thread.
    let mut new_client_size = None;
    if batch.size.width > 0 && batch.size.height > 0 {
        let size = Size::new(
            f64::from(batch.size.width) + shadow_extents.left + shadow_extents.right,
            f64::from(batch.size.height) + shadow_extents.top + shadow_extents.bottom,
        );
        if size != client_size {
            new_client_size = Some(size);
        }
    } else if new_window_state == WindowState::Normal
        && old_window_state != WindowState::Normal
        && restore_bounds.width > 0.0
        && restore_bounds.height > 0.0
        && restore_bounds != client_size
    {
        new_client_size = Some(restore_bounds);
    }

    ConfigureOutcome {
        window_state: new_window_state,
        is_activated: batch.states.contains(XdgToplevelStates::ACTIVATED),
        restore_bounds,
        client_size: new_client_size,
    }
}

/// The limits of the framework as the worker takes them: (0,0) and positive infinity mean
/// "unconstrained", which is `None` there.
pub(crate) fn normalize_size_limit(size: Size) -> Option<Size> {
    (size.width > 0.0 && size.height > 0.0 && size.width != f64::INFINITY && size.height != f64::INFINITY).then_some(size)
}

/// The edge of an interactive resize for an edge of the framework.
pub(crate) fn resize_edge_of_window_edge(edge: WindowEdge) -> ResizeEdge {
    match edge {
        WindowEdge::NorthWest => ResizeEdge::TopLeft,
        WindowEdge::North => ResizeEdge::Top,
        WindowEdge::NorthEast => ResizeEdge::TopRight,
        WindowEdge::West => ResizeEdge::Left,
        WindowEdge::East => ResizeEdge::Right,
        WindowEdge::SouthWest => ResizeEdge::BottomLeft,
        WindowEdge::South => ResizeEdge::Bottom,
        WindowEdge::SouthEast => ResizeEdge::BottomRight,
    }
}

/// The edge of an interactive resize for a resize grip of the drawn decorations.
pub(crate) fn resize_edge_of_chrome_role(role: WindowDecorationsElementRole) -> Option<ResizeEdge> {
    Some(match role {
        WindowDecorationsElementRole::ResizeN => ResizeEdge::Top,
        WindowDecorationsElementRole::ResizeS => ResizeEdge::Bottom,
        WindowDecorationsElementRole::ResizeE => ResizeEdge::Right,
        WindowDecorationsElementRole::ResizeW => ResizeEdge::Left,
        WindowDecorationsElementRole::ResizeNE => ResizeEdge::TopRight,
        WindowDecorationsElementRole::ResizeNW => ResizeEdge::TopLeft,
        WindowDecorationsElementRole::ResizeSE => ResizeEdge::BottomRight,
        WindowDecorationsElementRole::ResizeSW => ResizeEdge::BottomLeft,
        _ => return None,
    })
}

/// The cookie of an input event as the worker takes it back.
pub(crate) fn worker_cookie(cookie: Option<&Rc<dyn Any>>) -> Option<PlatformInputEventCookie> {
    cookie.and_then(|cookie| cookie.downcast_ref::<PlatformInputEventCookie>()).cloned()
}

impl WindowImpl {
    /// A window with its top-level on the compositor: the constructor waits for the first
    /// configure of the compositor.
    pub fn new(client: Rc<WaylandWorkerClient>, keyboard: Rc<KeyboardDevice>) -> Rc<Self> {
        let window = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            base: WindowBaseImpl::new(client, keyboard),
            current_sink: RefCell::new(None),
            surfaces: Arc::new(Mutex::new(Vec::new())),
            surface_proxy: RefCell::new(None),
            is_activated: Cell::new(false),
            max_auto_size_hint: Cell::new(Size::default()),
            window_state: Cell::new(WindowState::Normal),
            can_resize: Cell::new(true),
            restore_bounds: Cell::new(Size::default()),
            shadow_extents: Cell::new(Thickness::default()),
            min_size: Cell::new(None),
            max_size: Cell::new(None),
            csd_sticky: Cell::new(false),
            title: RefCell::new(None),
            storage_provider: RefCell::new(None),
            server_reported_mode: Cell::new(DecorationMode::ClientSide),
            app_requested_decoration_extension: Cell::new(false),
            resize_stack_depth: Cell::new(0),
            window_state_changed: RefCell::new(None),
            got_input_when_disabled: RefCell::new(None),
            closing: RefCell::new(None),
            extend_client_area_to_decorations_changed: RefCell::new(None),
            drawn_decorations_request_changed: RefCell::new(None),
        });
        let sink = WindowImplSink::new(&window, false);
        *window.current_sink.borrow_mut() = Some(sink);
        window
    }

    /// The state the window shares with a popup of the backend.
    pub fn base(&self) -> &WindowBaseImpl {
        &self.base
    }

    pub(crate) fn this(&self) -> Weak<WindowImpl> {
        self.this.clone()
    }

    pub(crate) fn surface_proxy(&self) -> Option<WXdgTopLevelProxy> {
        self.surface_proxy.borrow().clone()
    }

    /// Takes the handle of a new surface of the worker (the sink sets `_handle` and
    /// `_surfaceProxy` of its parent).
    pub(crate) fn set_surface(&self, proxy: WXdgTopLevelProxy, render_surfaces: Vec<Arc<dyn IPlatformRenderSurface>>) {
        *self.surfaces.lock().unwrap_or_else(PoisonError::into_inner) = render_surfaces;
        *self.surface_proxy.borrow_mut() = Some(proxy);
    }

    pub(crate) fn shadow_extents_value(&self) -> Thickness {
        self.shadow_extents.get()
    }

    pub(crate) fn title_value(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    pub(crate) fn size_limits(&self) -> (Option<Size>, Option<Size>) {
        (self.min_size.get(), self.max_size.get())
    }

    pub(crate) fn csd_sticky(&self) -> bool {
        self.csd_sticky.get()
    }

    pub(crate) fn raise_closing(&self, reason: WindowCloseReason) -> bool {
        get(&self.closing).is_some_and(|closing| closing(reason))
    }

    pub(crate) fn apply_configure_batch(&self, batch: &XdgConfigureBatch) {
        self.max_auto_size_hint.set(batch.max_size);

        let old_window_state = self.window_state.get();
        let outcome = evaluate_configure(
            batch,
            old_window_state,
            self.base.client_size(),
            self.restore_bounds.get(),
            self.shadow_extents.get(),
        );
        self.restore_bounds.set(outcome.restore_bounds);

        if old_window_state != outcome.window_state {
            self.window_state.set(outcome.window_state);
            if let Some(window_state_changed) = get(&self.window_state_changed) {
                window_state_changed(outcome.window_state);
            }
        }

        // Activation
        let was_activated = self.is_activated.replace(outcome.is_activated);
        if outcome.is_activated && !was_activated {
            if let Some(activated) = get(&self.base.activated) {
                activated();
            }
        } else if !outcome.is_activated && was_activated {
            if let Some(deactivated) = get(&self.base.deactivated) {
                deactivated();
            }
        }

        if let Some(new_client_size) = outcome.client_size {
            self.base.set_client_size(new_client_size);
            if let Some(resized) = get(&self.base.resized) {
                resized(new_client_size, WindowResizeReason::Layout);
            }
        }
    }

    /// The storage providers of the window, in the order of the reference. The first of
    /// the reference, the file chooser of the desktop portal with a parent handle from
    /// `xdg-foreign`, belongs to stage 2: the managed dialogs answer alone until then.
    fn build_storage_factories(&self) -> Vec<StorageProviderFactory> {
        let window = self.this.clone();
        vec![Rc::new(move || {
            // HACK: relies on focus root being TopLevel which currently is true
            let provider: Option<Rc<dyn IStorageProvider>> = window
                .upgrade()
                .and_then(|window| window.base.input_root())
                .and_then(|input_root| input_root.focus_root().cast::<TopLevel>())
                .map(|tl| Rc::new(ManagedStorageProvider::new(Some(&tl), None)) as Rc<dyn IStorageProvider>);
            Box::pin(std::future::ready(provider))
        })]
    }

    /// Refreshes the storage provider's factory chain after a new surface was created.
    pub(crate) fn reset_storage_provider(&self) {
        if let Some(storage_provider) = &*self.storage_provider.borrow() {
            storage_provider.reset(self.build_storage_factories());
        }
    }

    pub(crate) fn apply_decoration_mode(&self, mode: DecorationMode) {
        if self.server_reported_mode.get() == mode {
            return;
        }
        self.server_reported_mode.set(mode);
        self.raise_decoration_change_callbacks();
    }

    fn raise_decoration_change_callbacks(&self) {
        if let Some(changed) = get(&self.drawn_decorations_request_changed) {
            changed();
        }
        if let Some(changed) = get(&self.extend_client_area_to_decorations_changed) {
            changed(self.is_client_area_extended_to_decorations());
        }
    }

    fn size_nearly_equals(x: f64, y: f64) -> bool {
        (x - y).abs() < 0.01
    }

    fn dispose_sink(&self) {
        let current = self.current_sink.borrow_mut().take();
        if let Some(current) = current {
            current.dispose();
        }
    }
}

impl IPopupParent for WindowImpl {
    fn window_base(&self) -> &WindowBaseImpl {
        &self.base
    }

    fn parent_max_auto_size_hint(&self) -> Size {
        self.max_auto_size_hint.get()
    }

    fn shell_surface_proxy(&self) -> Option<WXdgShellSurfaceProxy> {
        self.surface_proxy.borrow().as_ref().map(|proxy| proxy.as_shell_surface().clone())
    }
}

impl ISinkOwner for WindowImpl {
    fn base(&self) -> &WindowBaseImpl {
        &self.base
    }

    fn disconnect_from_surface(&self) {
        let proxy = self.surface_proxy.borrow_mut().take();
        self.surfaces.lock().unwrap_or_else(PoisonError::into_inner).clear();
        if let Some(proxy) = proxy {
            proxy.disconnect();
        }
    }

    fn on_input_while_disabled(&self) {
        if let Some(got_input_when_disabled) = get(&self.got_input_when_disabled) {
            got_input_when_disabled();
        }
    }

    // Title-bar move and edge-resize chrome dispatch is xdg_toplevel-specific.
    fn handle_surface_specific_dispatch(&self, args: &RawEvent) -> bool {
        // TODO: We need to properly check if touch contact is primary, but it's tracked in TouchDevice,
        // so we might want to move that handling to x-plat part of the codebase
        // another point is that we generally need to cancel implicit capture if pointer gesture started
        // with right mouse button
        let (Some(input_root), Some(mouse)) = (self.base.input_root(), args.downcast_ref::<RawPointerEventArgs>()) else {
            return false;
        };
        if !matches!(mouse.type_(), RawPointerEventType::LeftButtonDown | RawPointerEventType::TouchBegin) {
            return false;
        }

        let Some(chrome_role) = input_root.hit_test_chrome_element(mouse.position()) else {
            return false;
        };
        let proxy = self.surface_proxy();
        let cookie = worker_cookie(mouse.platform_input_event_cookie().as_ref());
        if chrome_role == WindowDecorationsElementRole::TitleBar {
            if let Some(proxy) = proxy {
                proxy.move_(cookie, WaylandDispatchPriority::Oob);
            }
            return true;
        }

        if self.can_resize.get() {
            if let Some(edge) = resize_edge_of_chrome_role(chrome_role) {
                if let Some(proxy) = proxy {
                    proxy.resize(cookie, edge, WaylandDispatchPriority::Oob);
                }
                return true;
            }
        }

        false
    }
}

impl IOptionalFeatureProvider for WindowImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IStorageProvider>() {
            let storage_provider: Rc<dyn IStorageProvider> = self
                .storage_provider
                .borrow_mut()
                .get_or_insert_with(|| Rc::new(FallbackStorageProvider::new(self.build_storage_factories())))
                .clone();
            return Some(Rc::new(storage_provider));
        }

        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            let screens = FerroLocator::current().get_service::<dyn IScreenImpl>()?;
            return Some(Rc::new(screens));
        }

        if feature_type == TypeId::of::<dyn ILauncher>() {
            let launcher: Rc<dyn ILauncher> = Rc::new(BclLauncher::new());
            return Some(Rc::new(launcher));
        }

        // The text input method and the clipboard are features of stage 2.
        None
    }
}

impl IDisposable for WindowImpl {
    fn dispose(&self) {
        if self.base.mark_disposed() {
            return;
        }
        self.dispose_sink();
        if let Some(closed) = get(&self.base.closed) {
            closed();
        }
    }
}

impl ITopLevelImpl for WindowImpl {
    fn desktop_scaling(&self) -> f64 {
        1.0
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        None
    }

    fn client_size(&self) -> Size {
        self.base.client_size()
    }

    fn render_scaling(&self) -> f64 {
        self.base.render_scaling()
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        self.surfaces.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn render_surfaces(&self) -> Arc<dyn Fn() -> Vec<Arc<dyn IPlatformRenderSurface>> + Send + Sync> {
        // The surfaces change when the window is hidden and shown again: the thread that
        // renders asks for the current ones.
        let surfaces = self.surfaces.clone();
        Arc::new(move || surfaces.lock().unwrap_or_else(PoisonError::into_inner).clone())
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.base.client().compositor().clone())
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        get(&self.base.input)
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        set(&self.base.input, value);
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        get(&self.base.paint)
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        set(&self.base.paint, value);
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        get(&self.base.resized)
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        set(&self.base.resized, value);
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        get(&self.base.scaling_changed)
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        set(&self.base.scaling_changed, value);
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        get(&self.base.transparency_level_changed)
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        set(&self.base.transparency_level_changed, value);
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        self.base.set_input_root(input_root);
    }

    // Not supported by Wayland.
    fn point_to_client(&self, point: PixelPoint) -> Point {
        Point::new(f64::from(point.x), f64::from(point.y))
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::new(point.x as i32, point.y as i32)
    }

    fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
        let proxy = self.surface_proxy();
        self.base.set_cursor(cursor, proxy.as_ref().map(WXdgTopLevelProxy::as_shell_surface));
    }

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.base.closed)
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.base.closed, value);
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.base.lost_focus)
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.base.lost_focus, value);
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        let parent: Rc<dyn IPopupParent> = self.this.upgrade()?;
        Some(PopupImpl::new(parent))
    }

    fn set_transparency_level_hint(&self, _transparency_levels: &[WindowTransparencyLevel]) {}

    fn transparency_level(&self) -> WindowTransparencyLevel {
        WindowTransparencyLevel::transparent()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        AcrylicPlatformCompensationLevels::default()
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
}

impl IWindowBaseImpl for WindowImpl {
    // TODO: Query client decorations
    fn frame_size(&self) -> Option<Size> {
        None
    }

    /// # Panics
    /// Panics for a window that is disposed (the `ObjectDisposedException` of the reference).
    fn show(&self, _activate: bool, _is_dialog: bool) {
        if self.base.is_disposed() {
            panic!("Cannot access a disposed object. Object name: 'WindowImpl'.");
        }

        if self.current_sink.borrow().is_none() {
            // TODO: Re-apply window state
            if let Some(this) = self.this.upgrade() {
                let sink = WindowImplSink::new(&this, true);
                *self.current_sink.borrow_mut() = Some(sink);
            }
        }

        self.base.client().any_thread_wakeup_render_loop();
    }

    fn hide(&self) {
        self.dispose_sink();
    }

    // Not supported by Wayland.
    fn position(&self) -> PixelPoint {
        PixelPoint::default()
    }

    fn position_changed(&self) -> Option<Rc<dyn Fn(PixelPoint)>> {
        get(&self.base.position_changed)
    }

    fn set_position_changed(&self, value: Option<Rc<dyn Fn(PixelPoint)>>) {
        set(&self.base.position_changed, value);
    }

    fn activate(&self) {}

    fn deactivated(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.base.deactivated)
    }

    fn set_deactivated(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.base.deactivated, value);
    }

    fn activated(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.base.activated)
    }

    fn set_activated(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.base.activated, value);
    }

    fn max_auto_size_hint(&self) -> Size {
        self.max_auto_size_hint.get()
    }

    fn set_topmost(&self, _value: bool) {}
}

impl IWindowImpl for WindowImpl {
    fn window_state(&self) -> WindowState {
        self.window_state.get()
    }

    fn set_window_state(&self, value: WindowState) {
        // Capture current state on the UI thread for the transition logic
        let current_state = self.window_state.get();
        // Don't update the field — the compositor will confirm via configure

        let Some(proxy) = self.surface_proxy() else {
            return;
        };

        match value {
            WindowState::Normal => {
                if current_state == WindowState::Maximized {
                    proxy.unset_maximized();
                } else if current_state == WindowState::FullScreen {
                    proxy.unset_fullscreen();
                }
            }
            WindowState::Maximized => {
                if current_state == WindowState::FullScreen {
                    proxy.unset_fullscreen();
                }
                proxy.set_maximized();
            }
            WindowState::FullScreen => {
                if current_state == WindowState::Maximized {
                    proxy.unset_maximized();
                }
                proxy.set_fullscreen();
            }
            WindowState::Minimized => proxy.set_minimized(),
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
        *self.title.borrow_mut() = title.map(str::to_string);
        if let Some(proxy) = self.surface_proxy() {
            proxy.set_title(title);
        }
    }

    fn set_parent(&self, parent: Option<Rc<dyn IWindowImpl>>) {
        let parent_proxy = parent
            .as_ref()
            .and_then(|parent| parent.as_any().downcast_ref::<WindowImpl>())
            .and_then(WindowImpl::surface_proxy);
        if let Some(proxy) = self.surface_proxy() {
            proxy.set_parent(parent_proxy.as_ref());
        }
    }

    fn set_enabled(&self, enable: bool) {
        self.base.set_is_enabled(enable);
    }

    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.got_input_when_disabled)
    }

    fn set_got_input_when_disabled(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.got_input_when_disabled, value);
    }

    fn set_window_decorations(&self, enabled: WindowDecorations) {
        if self.csd_sticky.get() || enabled == WindowDecorations::Full {
            return;
        }

        // We disable SSD support completely once CSD was requested at least once. Protocol limitation
        self.csd_sticky.set(true);
        if let Some(proxy) = self.surface_proxy() {
            proxy.destroy_decoration();
        }
        self.apply_decoration_mode(DecorationMode::ClientSide);
    }

    fn set_icon(&self, _icon: Option<Rc<dyn IWindowIconImpl>>) {}

    fn show_taskbar_icon(&self, _value: bool) {}

    fn can_resize(&self, value: bool) {
        self.can_resize.set(value);
    }

    fn set_can_minimize(&self, _value: bool) {}

    fn set_can_maximize(&self, _value: bool) {}

    fn closing(&self) -> Option<Rc<dyn Fn(WindowCloseReason) -> bool>> {
        get(&self.closing)
    }

    fn set_closing(&self, value: Option<Rc<dyn Fn(WindowCloseReason) -> bool>>) {
        set(&self.closing, value);
    }

    fn is_client_area_extended_to_decorations(&self) -> bool {
        self.needs_managed_decorations() && self.app_requested_decoration_extension.get()
    }

    fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>> {
        get(&self.extend_client_area_to_decorations_changed)
    }

    fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>) {
        let handler = value.clone();
        set(&self.extend_client_area_to_decorations_changed, value);
        if let Some(handler) = handler {
            handler(true);
        }
    }

    fn needs_managed_decorations(&self) -> bool {
        self.server_reported_mode.get() == DecorationMode::ClientSide
    }

    fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration {
        if self.needs_managed_decorations() {
            PlatformRequestedDrawnDecoration::TITLE_BAR
                | PlatformRequestedDrawnDecoration::BORDER
                | PlatformRequestedDrawnDecoration::RESIZE_GRIPS
                | PlatformRequestedDrawnDecoration::SHADOW
        } else {
            PlatformRequestedDrawnDecoration::NONE
        }
    }

    fn drawn_decorations_request_changed(&self) -> Option<Rc<dyn Fn()>> {
        get(&self.drawn_decorations_request_changed)
    }

    fn set_drawn_decorations_request_changed(&self, value: Option<Rc<dyn Fn()>>) {
        set(&self.drawn_decorations_request_changed, value);
    }

    fn extended_margins(&self) -> Thickness {
        Thickness::default()
    }

    fn off_screen_margin(&self) -> Thickness {
        Thickness::default()
    }

    fn begin_move_drag(&self, e: &PointerPressedEventArgs) {
        let cookie = worker_cookie(e.platform_input_event_cookie());
        e.pointer().capture(None);
        if let Some(proxy) = self.surface_proxy() {
            proxy.move_(cookie, WaylandDispatchPriority::Oob);
        }
    }

    fn begin_resize_drag(&self, edge: WindowEdge, e: &PointerPressedEventArgs) {
        let resize_edge = resize_edge_of_window_edge(edge);
        let cookie = worker_cookie(e.platform_input_event_cookie());
        e.pointer().capture(None);
        if let Some(proxy) = self.surface_proxy() {
            proxy.resize(cookie, resize_edge, WaylandDispatchPriority::Oob);
        }
    }

    fn resize(&self, client_size: Size, reason: WindowResizeReason) {
        // There is some bug on the x-plat side of things that increments the size by 0.000001 or smth and causes
        // a stack overflow
        // So we check for recursion depth and for nearly equal values here.
        if self.resize_stack_depth.get() > 5 {
            return;
        }
        let current = self.base.client_size();
        if Self::size_nearly_equals(client_size.width, current.width) && Self::size_nearly_equals(client_size.height, current.height) {
            return;
        }

        self.resize_stack_depth.set(self.resize_stack_depth.get() + 1);
        // The depth is restored when the callback returns or panics.
        struct Depth<'a>(&'a Cell<i32>);
        impl Drop for Depth<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() - 1);
            }
        }
        let _depth = Depth(&self.resize_stack_depth);

        // Size is just a number, the actual resizing is done by the compositor when submitting a frame
        self.base.set_client_size(client_size);
        if let Some(resized) = get(&self.base.resized) {
            resized(client_size, reason);
        }
    }

    // Not supported by Wayland — surfaces have no screen position.
    fn move_(&self, _point: PixelPoint) {}

    fn set_min_max_size(&self, min_size: Size, max_size: Size) {
        self.min_size.set(normalize_size_limit(min_size));
        self.max_size.set(normalize_size_limit(max_size));
        if let Some(proxy) = self.surface_proxy() {
            proxy.set_min_max_size(self.min_size.get(), self.max_size.get());
        }
    }

    fn set_extend_client_area_to_decorations_hint(&self, extend_into_client_area_hint: bool) {
        self.app_requested_decoration_extension.set(extend_into_client_area_hint);
        self.raise_decoration_change_callbacks();
    }

    fn set_extend_client_area_title_bar_height_hint(&self, _title_bar_height: f64) {}

    fn set_shadow_extents(&self, extents: Thickness) {
        self.shadow_extents.set(extents);
        if let Some(proxy) = self.surface_proxy() {
            proxy.set_shadow_extents(extents);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use ferroui_base::PixelSize;

    fn batch(width: i32, height: i32, states: XdgToplevelStates) -> XdgConfigureBatch {
        XdgConfigureBatch { size: PixelSize::new(width, height), states, ..Default::default() }
    }

    #[test]
    fn a_configure_gives_the_state_the_activation_and_the_size_with_the_shadow() {
        let shadow = Thickness::new(10.0, 10.0, 10.0, 10.0);
        let outcome = evaluate_configure(
            &batch(800, 600, XdgToplevelStates::ACTIVATED),
            WindowState::Normal,
            Size::new(640.0, 480.0),
            Size::default(),
            shadow,
        );
        assert_eq!(outcome.window_state, WindowState::Normal);
        assert!(outcome.is_activated);
        assert_eq!(outcome.client_size, Some(Size::new(820.0, 620.0)));

        // The same size again changes nothing.
        let outcome = evaluate_configure(
            &batch(800, 600, XdgToplevelStates::empty()),
            WindowState::Normal,
            Size::new(820.0, 620.0),
            Size::default(),
            shadow,
        );
        assert_eq!(outcome.client_size, None);
        assert!(!outcome.is_activated);
    }

    #[test]
    fn fullscreen_wins_over_maximized_and_the_normal_size_is_remembered_and_restored() {
        let normal = Size::new(640.0, 480.0);
        let outcome = evaluate_configure(
            &batch(1920, 1080, XdgToplevelStates::MAXIMIZED | XdgToplevelStates::FULLSCREEN),
            WindowState::Normal,
            normal,
            Size::default(),
            Thickness::default(),
        );
        assert_eq!(outcome.window_state, WindowState::FullScreen);
        assert_eq!(outcome.restore_bounds, normal);
        assert_eq!(outcome.client_size, Some(Size::new(1920.0, 1080.0)));

        // Back to normal with a size left to the client: the remembered one.
        let outcome = evaluate_configure(
            &batch(0, 0, XdgToplevelStates::empty()),
            WindowState::FullScreen,
            Size::new(1920.0, 1080.0),
            normal,
            Thickness::default(),
        );
        assert_eq!(outcome.window_state, WindowState::Normal);
        assert_eq!(outcome.client_size, Some(normal));

        // A configure without a size in the normal state keeps the size.
        let outcome =
            evaluate_configure(&batch(0, 0, XdgToplevelStates::empty()), WindowState::Normal, normal, normal, Thickness::default());
        assert_eq!(outcome.client_size, None);
    }

    #[test]
    fn size_limits_without_a_bound_are_no_limits() {
        assert_eq!(normalize_size_limit(Size::new(0.0, 0.0)), None);
        assert_eq!(normalize_size_limit(Size::new(f64::INFINITY, f64::INFINITY)), None);
        assert_eq!(normalize_size_limit(Size::new(100.0, f64::INFINITY)), None);
        assert_eq!(normalize_size_limit(Size::new(100.0, 50.0)), Some(Size::new(100.0, 50.0)));
    }

    #[test]
    fn edges_and_resize_grips_map_to_the_edges_of_the_protocol() {
        assert_eq!(resize_edge_of_window_edge(WindowEdge::NorthWest), ResizeEdge::TopLeft);
        assert_eq!(resize_edge_of_window_edge(WindowEdge::South), ResizeEdge::Bottom);
        assert_eq!(resize_edge_of_window_edge(WindowEdge::East), ResizeEdge::Right);
        assert_eq!(resize_edge_of_chrome_role(WindowDecorationsElementRole::ResizeSE), Some(ResizeEdge::BottomRight));
        assert_eq!(resize_edge_of_chrome_role(WindowDecorationsElementRole::ResizeW), Some(ResizeEdge::Left));
        assert_eq!(resize_edge_of_chrome_role(WindowDecorationsElementRole::TitleBar), None);
    }
}
