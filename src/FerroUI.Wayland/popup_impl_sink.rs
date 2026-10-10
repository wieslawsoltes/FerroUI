//! The sink of a popup (the port of `PopupImpl.Sink.cs`): it creates the
//! popup of the worker and receives its events on the UI thread.

use crate::ferro_wayland_exception::FerroWaylandException;
use crate::popup_impl::PopupImpl;
use crate::screens::wayland_output_snapshot::WaylandOutputId;
use crate::server::persistent::i_w_surface::IWSurface;
use crate::server::persistent::i_w_surface_event_sink::{
    IWSurfaceEventSink, IWXdgPopupEventSink, PlatformInputEventCookie, WXdgPopupEventSinkProxy,
};
use crate::server::persistent::i_w_xdg_top_level::{IWXdgPopup, IWXdgShellSurface, WXdgPopupProxy};
use crate::server::persistent::xdg_popup_configure_batch::XdgPopupConfigureBatch;
use crate::window_impl_base::{get, ISinkOwner, Sink};
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Rect, Vector};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub(crate) struct PopupImplSink {
    base: Rc<Sink>,
    parent: Weak<PopupImpl>,
    surface_proxy: RefCell<Option<WXdgPopupProxy>>,
}

impl PopupImplSink {
    /// Creates the popup of the worker for a popup of the UI thread.
    ///
    /// # Panics
    /// Panics when the parent of the popup has no surface (the `InvalidOperationException`
    /// of the reference).
    pub(crate) fn new(parent: &Rc<PopupImpl>) -> Rc<Self> {
        let owner: Weak<dyn ISinkOwner> = parent.this();
        let client = parent.base().client().clone();
        let sink = Rc::new(Self {
            base: Sink::new(owner, &client),
            parent: Rc::downgrade(parent),
            surface_proxy: RefCell::new(None),
        });

        // Worker→UI sink calls (on_pointer_enter, on_popup_configure, etc.)
        // are invoked from the worker thread; wrap ourselves in a UI-
        // thread proxy so they marshal onto the dispatcher.
        let sink_proxy = WXdgPopupEventSinkProxy::new(sink.clone(), Dispatcher::ui_thread());

        // The xdg_popup parent is the direct parent surface (toplevel
        // or another popup). The compositor uses parent-relative
        // coordinates and is free to reposition the popup; chaining
        // through the real visual hierarchy lets it constrain each
        // link's anchor rect against its direct parent.
        let Some(parent_proxy) = parent.parent().shell_surface_proxy() else {
            FerroWaylandException::new("Cannot create popup: parent surface has not been mapped yet.").throw()
        };

        let handle = client.create_popup_handle(sink_proxy, &parent_proxy);
        let surface_proxy = handle.proxy;
        *sink.surface_proxy.borrow_mut() = Some(surface_proxy.clone());
        parent.set_surface(surface_proxy.clone(), handle.render_surfaces);

        // Replay the cached positioner (if Show was called after
        // UpdatePositioner — common ordering of the framework) so the worker
        // can attach the popup as soon as the parent is mapped.
        if let Some(positioner) = parent.last_positioner() {
            surface_proxy.update_positioner(positioner);
        }

        // Re-apply cursor (defaults to Arrow on a fresh worker surface).
        if parent.base().current_cursor().is_some() {
            parent.base().apply_current_cursor(Some(surface_proxy.as_shell_surface()));
        }

        // A fresh worker surface starts hit-test visible.
        if !parent.is_hit_test_visible() {
            surface_proxy.set_hit_test_visible(false);
        }

        sink
    }

    pub(crate) fn dispose(&self) {
        self.base.dispose();
    }

    fn parent(&self) -> Option<Rc<PopupImpl>> {
        self.parent.upgrade()
    }
}

impl IWSurfaceEventSink for PopupImplSink {
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

impl IWXdgPopupEventSink for PopupImplSink {
    fn on_popup_configure(&self, batch: XdgPopupConfigureBatch) {
        // Marshalled to UI thread by the proxy.
        if self.base.is_disposed() {
            return;
        }

        // The compositor's suggested size is intentionally IGNORED
        // for now: per xdg_surface.configure, this is a *suggested*
        // surface change, and the framework's positioner-supplied
        // size is the authoritative one we use for layout. The
        // worker→UI plumbing is kept because we may want to revisit
        // this for compositor-driven shrink-to-fit behaviour.
        let _ = batch.width;
        let _ = batch.height;

        // Ack the configure on next commit.
        if let Some(proxy) = &*self.surface_proxy.borrow() {
            proxy.set_pending_ack_serial(batch.serial);
        }

        // Not in the reference: a popup that was created again in place of a reposition
        // (a compositor without `xdg_popup.reposition`) has no buffer and is shown again by
        // its next frame, which nothing else asks for when only its place changed.
        if batch.recreated {
            if let Some(parent) = self.parent() {
                if let Some(paint) = get(&parent.base().paint) {
                    paint(Rect::from_size(parent.base().client_size()));
                }
            }
        }
    }

    fn on_popup_done(&self) {
        // Compositor dismissed the popup (click outside, parent
        // unmapped, etc.). xdg-shell forbids any further use of the
        // xdg_popup object after popup_done; treat this as a close
        // and let Dispose tear it down.
        if self.base.is_disposed() {
            return;
        }
        if let Some(parent) = self.parent() {
            parent.dispose();
        }
    }
}
