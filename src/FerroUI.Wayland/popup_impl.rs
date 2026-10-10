//! A popup of the backend on the UI thread (the port of `PopupImpl.cs`).
//!
//! The reference derives the popup and the window from one base class and
//! gives the popup its parent as that base class. Here the parent is the
//! trait [`IPopupParent`], which the window and the popup implement.

use crate::popup_impl_sink::PopupImplSink;
use crate::server::persistent::i_w_surface::IWSurface;
use crate::server::persistent::i_w_xdg_top_level::{IWXdgPopup, WXdgPopupProxy, WXdgShellSurfaceProxy};
use crate::server::persistent::xdg_popup_positioner_params::XdgPopupPositionerParams;
use crate::wayland_conversion_extensions::positioner_parameters_to_wayland;
use crate::window_impl_base::{get, set, ISinkOwner, WindowBaseImpl};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::IInputRoot;
use ferroui_base::platform::storage::file_io::BclLauncher;
use ferroui_base::platform::storage::ILauncher;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider, PlatformThemeVariant};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Point, Rect, Size};
use ferroui_controls::platform::{IPlatformHandle, IPopupImpl, IScreenImpl, ITopLevelImpl, IWindowBaseImpl};
use ferroui_controls::primitives::popup_positioning::{IPopupPositioner, PopupPositionerParameters};
use ferroui_controls::{AcrylicPlatformCompensationLevels, WindowResizeReason, WindowTransparencyLevel};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};

/// The render surfaces of a popup, which the thread that renders asks for.
type SharedSurfaces = Arc<Mutex<Vec<Arc<dyn IPlatformRenderSurface>>>>;

/// What a popup needs of its parent, a window or another popup (the members of
/// `WindowBaseImpl` of the reference the popup reads of `_parent`).
pub(crate) trait IPopupParent {
    /// The state every window of the backend has.
    fn window_base(&self) -> &WindowBaseImpl;

    fn parent_max_auto_size_hint(&self) -> Size;

    /// Returns the worker-side `xdg_surface`-derived proxy backing
    /// this top-level / popup, or `None` when no surface is
    /// currently created.
    fn shell_surface_proxy(&self) -> Option<WXdgShellSurfaceProxy>;
}

/// Wayland xdg_popup window implementation. Created via
/// `WindowImpl::create_popup` (or another `PopupImpl::create_popup`
/// for nested popups) and parented to either a top-level or another popup.
///
/// We deliberately do NOT call `xdg_popup.grab()`. Dismissal is
/// driven by: (1) framework light-dismiss, (2) the compositor's
/// `popup_done` event, (3) focus-leave on the parent toplevel.
///
/// Coordinate convention: the positioner's
/// `update` receives anchor-rect coordinates
/// in the parent's client-area logical pixels (i.e. parent
/// buffer-relative). We forward them straight through to the worker via
/// `positioner_parameters_to_wayland`;
/// the worker handles the buffer→geometry origin shift and clamping.
pub struct PopupImpl {
    this: Weak<PopupImpl>,
    base: WindowBaseImpl,
    parent: Rc<dyn IPopupParent>,
    current_sink: RefCell<Option<Rc<PopupImplSink>>>,
    surfaces: SharedSurfaces,
    surface_proxy: RefCell<Option<WXdgPopupProxy>>,
    last_positioner: Cell<Option<XdgPopupPositionerParams>>,
    is_hit_test_visible: Cell<bool>,
    popup_positioner: RefCell<Option<Rc<dyn IPopupPositioner>>>,
}

impl PopupImpl {
    pub(crate) fn new(parent: Rc<dyn IPopupParent>) -> Rc<Self> {
        let client = parent.window_base().client().clone();
        let keyboard = parent.window_base().keyboard_device();
        let popup = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            base: WindowBaseImpl::new(client, keyboard),
            parent,
            current_sink: RefCell::new(None),
            surfaces: Arc::new(Mutex::new(Vec::new())),
            surface_proxy: RefCell::new(None),
            last_positioner: Cell::new(None),
            is_hit_test_visible: Cell::new(true),
            popup_positioner: RefCell::new(None),
        });
        // Inherit the parent's render scale at construction time so the
        // first layout pass uses something sensible. The compositor will
        // re-issue scale events via on_scale_changed once the popup is
        // mapped — at which point we'll converge on the authoritative value.
        popup.base.set_render_scaling(popup.parent.window_base().render_scaling());

        let positioner: Rc<dyn IPopupPositioner> = Rc::new(WaylandPopupPositioner { owner: popup.this.clone() });
        *popup.popup_positioner.borrow_mut() = Some(positioner);
        popup
    }

    /// The state the popup shares with a window of the backend.
    pub fn base(&self) -> &WindowBaseImpl {
        &self.base
    }

    pub(crate) fn this(&self) -> Weak<PopupImpl> {
        self.this.clone()
    }

    pub(crate) fn parent(&self) -> &Rc<dyn IPopupParent> {
        &self.parent
    }

    pub(crate) fn surface_proxy(&self) -> Option<WXdgPopupProxy> {
        self.surface_proxy.borrow().clone()
    }

    /// Takes the handle of a new surface of the worker (the sink sets `_handle` and
    /// `_surfaceProxy` of its parent).
    pub(crate) fn set_surface(&self, proxy: WXdgPopupProxy, render_surfaces: Vec<Arc<dyn IPlatformRenderSurface>>) {
        *self.surfaces.lock().unwrap_or_else(PoisonError::into_inner) = render_surfaces;
        *self.surface_proxy.borrow_mut() = Some(proxy);
    }

    pub(crate) fn last_positioner(&self) -> Option<XdgPopupPositionerParams> {
        self.last_positioner.get()
    }

    pub(crate) fn is_hit_test_visible(&self) -> bool {
        self.is_hit_test_visible.get()
    }

    /// The framework drives popup positioning through this entry. We translate
    /// the framework's bitfield enums into the protocol's dense enums and
    /// hand the bundle to the worker, which (re-)builds an
    /// `xdg_positioner` on every connect.
    pub(crate) fn update_positioner(&self, parameters: &PopupPositionerParameters) {
        let translated = positioner_parameters_to_wayland(parameters);
        self.last_positioner.set(Some(translated));
        // Adopt the requested popup size as our client-size up-front so
        // the renderer paints at the right dimensions before the
        // compositor's first popup configure event arrives. The setter of the client size
        // only stores + wakes the render loop, so fire `resized`
        // explicitly to drive the layout pass of the framework.
        if translated.size.width > 0.0 && translated.size.height > 0.0 && translated.size != self.base.client_size() {
            self.base.set_client_size(translated.size);
            if let Some(resized) = get(&self.base.resized) {
                resized(translated.size, WindowResizeReason::Layout);
            }
        }
        if let Some(proxy) = self.surface_proxy() {
            proxy.update_positioner(translated);
        }
    }

    fn dispose_sink(&self) {
        let current = self.current_sink.borrow_mut().take();
        if let Some(current) = current {
            current.dispose();
        }
    }
}

impl IPopupParent for PopupImpl {
    fn window_base(&self) -> &WindowBaseImpl {
        &self.base
    }

    fn parent_max_auto_size_hint(&self) -> Size {
        self.parent.parent_max_auto_size_hint()
    }

    fn shell_surface_proxy(&self) -> Option<WXdgShellSurfaceProxy> {
        self.surface_proxy.borrow().as_ref().map(|proxy| proxy.as_shell_surface().clone())
    }
}

impl ISinkOwner for PopupImpl {
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
}

impl IOptionalFeatureProvider for PopupImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IScreenImpl>() {
            let screens = FerroLocator::current().get_service::<dyn IScreenImpl>()?;
            return Some(Rc::new(screens));
        }

        if feature_type == TypeId::of::<dyn ILauncher>() {
            let launcher: Rc<dyn ILauncher> = Rc::new(BclLauncher::new());
            return Some(Rc::new(launcher));
        }

        // The clipboard is a feature of a later part of stage 2.
        None
    }
}

impl IDisposable for PopupImpl {
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

impl ITopLevelImpl for PopupImpl {
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
        // The surfaces change when the popup is hidden and shown again: the thread that
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
        let proxy = self.shell_surface_proxy();
        self.base.set_cursor(cursor, proxy.as_ref());
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

    fn as_popup_impl(&self) -> Option<&dyn IPopupImpl> {
        Some(self)
    }
}

impl IWindowBaseImpl for PopupImpl {
    fn frame_size(&self) -> Option<Size> {
        None
    }

    /// # Panics
    /// Panics for a popup that is disposed (the `ObjectDisposedException` of the reference),
    /// and for a popup whose parent has no surface (its `InvalidOperationException`).
    fn show(&self, _activate: bool, _is_dialog: bool) {
        if self.base.is_disposed() {
            panic!("Cannot access a disposed object. Object name: 'PopupImpl'.");
        }

        if self.current_sink.borrow().is_none() {
            if let Some(this) = self.this.upgrade() {
                let sink = PopupImplSink::new(&this);
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

    /// Inherit the parent's auto-size hint (which on Wayland comes from
    /// `xdg_toplevel.configure_bounds` — i.e. the usable area of
    /// the output the parent is on). xdg_popup is constrained by the
    /// compositor to fit on-screen, so the same bound applies.
    fn max_auto_size_hint(&self) -> Size {
        self.parent.parent_max_auto_size_hint()
    }

    fn set_topmost(&self, _value: bool) {}
}

impl IPopupImpl for PopupImpl {
    fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>> {
        self.popup_positioner.borrow().clone()
    }

    fn set_window_manager_add_shadow_hint(&self, _enabled: bool) {
        // No-op on Wayland — the compositor doesn't draw popup shadows;
        // CSD shadows (if any) are baked into the buffer by the framework.
    }

    fn take_focus(&self) {}

    fn set_hit_test_visible(&self, is_hit_test_visible: bool) {
        self.is_hit_test_visible.set(is_hit_test_visible);
        if let Some(proxy) = self.surface_proxy() {
            proxy.set_hit_test_visible(is_hit_test_visible);
        }
    }
}

/// Bridges the positioner contract of the framework to [`PopupImpl::update_positioner`].
struct WaylandPopupPositioner {
    owner: Weak<PopupImpl>,
}

impl IPopupPositioner for WaylandPopupPositioner {
    fn update(&self, parameters: PopupPositionerParameters) {
        if let Some(owner) = self.owner.upgrade() {
            owner.update_positioner(&parameters);
        }
    }
}
