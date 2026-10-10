//! The sink of a window (the port of `WindowImpl.Sink.cs`): it creates the
//! top-level of the worker, waits for its first configure, and receives its
//! events on the UI thread.

use crate::screens::wayland_output_snapshot::WaylandOutputId;
use crate::server::persistent::decoration_mode::DecorationMode;
use crate::server::persistent::i_w_surface_event_sink::{
    IWSurfaceEventSink, IWXdgTopLevelEventSink, PlatformInputEventCookie, WXdgTopLevelEventSinkProxy,
};
use crate::server::persistent::i_w_xdg_top_level::{IWXdgShellSurface, IWXdgTopLevel, WXdgTopLevelProxy};
use crate::server::persistent::xdg_configure_batch::XdgConfigureBatch;
use crate::ferro_wayland_exception::FerroWaylandException;
use crate::window_impl::WindowImpl;
use crate::window_impl_base::{ISinkOwner, Sink};
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Size, Vector};
use ferroui_controls::WindowCloseReason;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

pub(crate) struct WindowImplSink {
    base: Rc<Sink>,
    parent: Weak<WindowImpl>,
    initial_batch: RefCell<Option<Arc<XdgConfigureBatch>>>,
    surface_proxy: RefCell<Option<WXdgTopLevelProxy>>,
}

impl WindowImplSink {
    /// Creates the top-level of a window on the worker and applies its first configure.
    ///
    /// # Panics
    /// Panics when the worker does not answer with a configure: it is gone.
    pub(crate) fn new(parent: &Rc<WindowImpl>, second_show: bool) -> Rc<Self> {
        let owner: Weak<dyn ISinkOwner> = parent.this();
        let client = parent.base().client().clone();
        let sink = Rc::new(Self {
            base: Sink::new(owner, &client),
            parent: Rc::downgrade(parent),
            initial_batch: RefCell::new(None),
            surface_proxy: RefCell::new(None),
        });

        let handle = client.create_top_level_handle(WXdgTopLevelEventSinkProxy::new(sink.clone(), Dispatcher::ui_thread()));
        let surface_proxy = handle.proxy;
        *sink.surface_proxy.borrow_mut() = Some(surface_proxy.clone());
        parent.set_surface(surface_proxy.clone(), handle.render_surfaces);

        let initial_batch = match handle.basic_init_completed.map(|completed| completed.recv()) {
            Some(Ok(batch)) => batch,
            _ => FerroWaylandException::new("The Wayland worker did not configure the window: it is gone").throw(),
        };
        *sink.initial_batch.borrow_mut() = Some(initial_batch.clone());
        parent.apply_configure_batch(&initial_batch);

        // If compositor didn't provide a size, auto-size
        if !second_show && !(initial_batch.size.width > 0 && initial_batch.size.height > 0) {
            let max_auto_size_hint = initial_batch.max_size;
            parent.base().set_client_size(Size::new(640f64.min(max_auto_size_hint.width), 480f64.min(max_auto_size_hint.height)));
        }

        // Ack the initial configure
        surface_proxy.set_pending_ack_serial(initial_batch.serial);

        if let Some(mode) = initial_batch.initial_decoration_mode {
            parent.apply_decoration_mode(mode);
        }

        // Re-send stored shadow extents to the new surface
        let shadow_extents = parent.shadow_extents_value();
        if shadow_extents != Default::default() {
            surface_proxy.set_shadow_extents(shadow_extents);
        }

        // Re-send cached title to the new surface
        if let Some(title) = parent.title_value() {
            surface_proxy.set_title(Some(&title));
        }

        // Re-apply cached min/max size constraints after a fresh worker
        // surface is created. None on both sides means set_min_max_size was
        // never called (or both bounds are unconstrained) — nothing to push.
        let (min_size, max_size) = parent.size_limits();
        if min_size.is_some() || max_size.is_some() {
            surface_proxy.set_min_max_size(min_size, max_size);
        }

        // Re-apply cursor (defaults to Arrow on a fresh worker surface).
        if parent.base().current_cursor().is_some() {
            parent.base().apply_current_cursor(Some(surface_proxy.as_shell_surface()));
        }

        // Refresh the storage provider's factory chain — the previous wayland
        // connection is gone, and the new compositor may expose different globals.
        parent.reset_storage_provider();

        sink
    }

    pub(crate) fn dispose(&self) {
        self.base.dispose();
    }

    fn parent(&self) -> Option<Rc<WindowImpl>> {
        self.parent.upgrade()
    }
}

impl IWSurfaceEventSink for WindowImplSink {
    fn on_pointer_enter(&self, timestamp: u64, serial: u32, position: Point) {
        self.base.on_pointer_enter(timestamp, serial, position);
    }

    fn on_pointer_leave(&self, serial: u32) {
        self.base.on_pointer_leave(serial);
    }

    fn on_pointer_motion(&self, timestamp: u64, position: Point, modifiers: RawInputModifiers) {
        self.base.on_pointer_motion(timestamp, position, modifiers);
    }

    fn on_pointer_button(
        &self,
        timestamp: u64,
        serial: u32,
        type_: RawPointerEventType,
        modifiers: RawInputModifiers,
        position: Point,
        platform_cookie: Option<PlatformInputEventCookie>,
    ) {
        self.base.on_pointer_button(timestamp, serial, type_, modifiers, position, platform_cookie);
    }

    fn on_pointer_axis(&self, timestamp: u64, delta: Vector, modifiers: RawInputModifiers, position: Point) {
        self.base.on_pointer_axis(timestamp, delta, modifiers, position);
    }

    fn on_touch_down(&self, timestamp: u64, touch_id: i32, position: Point, platform_cookie: Option<PlatformInputEventCookie>) {
        self.base.on_touch_down(timestamp, touch_id, position, platform_cookie);
    }

    fn on_touch_move(&self, timestamp: u64, touch_id: i32, position: Point) {
        self.base.on_touch_move(timestamp, touch_id, position);
    }

    fn on_touch_up(&self, timestamp: u64, touch_id: i32, position: Point) {
        self.base.on_touch_up(timestamp, touch_id, position);
    }

    fn on_touch_cancel(&self, touch_id: i32, position: Point) {
        self.base.on_touch_cancel(touch_id, position);
    }

    fn on_key_down(&self, timestamp: u64, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<String>) {
        self.base.on_key_down(timestamp, key, modifiers, physical_key, key_symbol);
    }

    fn on_key_up(&self, timestamp: u64, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<String>) {
        self.base.on_key_up(timestamp, key, modifiers, physical_key, key_symbol);
    }

    fn on_keyboard_leave(&self) {
        self.base.on_keyboard_leave();
    }

    fn on_key_repeat_info(&self, rate: i32, delay: i32) {
        self.base.on_key_repeat_info(rate, delay);
    }

    fn on_scale_changed(&self, scale: f64) {
        self.base.on_scale_changed(scale);
    }

    fn on_surface_outputs_changed(&self, output_ids: Vec<WaylandOutputId>) {
        self.base.on_surface_outputs_changed(output_ids);
    }
}

impl IWXdgTopLevelEventSink for WindowImplSink {
    fn on_configure(&self, batch: Arc<XdgConfigureBatch>) {
        // Marshalled to UI thread by the proxy.
        if self.base.is_disposed() {
            return;
        }

        // Skip the initial batch — already processed synchronously in the constructor
        let is_initial = self.initial_batch.borrow().as_ref().is_some_and(|initial| Arc::ptr_eq(initial, &batch));
        if is_initial {
            *self.initial_batch.borrow_mut() = None;
            return;
        }

        let Some(parent) = self.parent() else {
            return;
        };
        parent.apply_configure_batch(&batch);

        // Post the ack serial back to the wayland thread for next commit
        if let Some(proxy) = &*self.surface_proxy.borrow() {
            proxy.set_pending_ack_serial(batch.serial);
        }
    }

    fn on_decoration_mode_changed(&self, mode: DecorationMode) {
        let Some(parent) = self.parent() else {
            return;
        };
        if self.base.is_disposed() || parent.csd_sticky() {
            return;
        }
        parent.apply_decoration_mode(mode);
    }

    fn on_close(&self) {
        // Marshalled to UI thread by the proxy.
        if self.base.is_disposed() {
            return;
        }
        let refused = self.parent().is_some_and(|parent| parent.raise_closing(WindowCloseReason::WindowClosing));
        if !refused {
            self.dispose();
        }
    }
}
