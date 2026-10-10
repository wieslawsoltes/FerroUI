//! The events of a surface as the UI thread receives them (the port of
//! `IWSurfaceEventSink.cs`), and the proxies the worker holds
//! (`WSurfaceEventSinkProxy`, `WXdgTopLevelEventSinkProxy`,
//! `WXdgPopupEventSinkProxy`, which the reference generates): every call is
//! posted to the UI thread at the default priority, with its arguments as
//! values.
//!
//! The members of drag and drop (`OnDragEnter`, `OnDragMotion`, `OnDragLeave`,
//! `OnDrop`) belong to a later part of stage 2 of
//! `docs/porting/wayland-platform.md`.

use super::decoration_mode::DecorationMode;
use super::xdg_configure_batch::XdgConfigureBatch;
use super::xdg_popup_configure_batch::XdgPopupConfigureBatch;
use crate::screens::wayland_output_snapshot::WaylandOutputId;
use crate::server::wayland_marshallers::UiThreadRef;
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Vector};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// The opaque cookie of an input event (`object? platformCookie`): made by the worker, carried
/// by the raw event and the pointer event of the framework, and given back to the worker with
/// a request that needs the serial of the event.
pub type PlatformInputEventCookie = Arc<dyn Any + Send + Sync>;

/// UI-thread sink for worker→UI events common to all surface kinds.
pub trait IWSurfaceEventSink {
    fn on_pointer_enter(&self, timestamp: u64, serial: u32, position: Point);
    fn on_pointer_leave(&self, serial: u32);
    fn on_pointer_motion(&self, timestamp: u64, position: Point, modifiers: RawInputModifiers);
    #[allow(clippy::too_many_arguments)]
    fn on_pointer_button(
        &self,
        timestamp: u64,
        serial: u32,
        type_: RawPointerEventType,
        modifiers: RawInputModifiers,
        position: Point,
        platform_cookie: Option<PlatformInputEventCookie>,
    );
    fn on_pointer_axis(&self, timestamp: u64, delta: Vector, modifiers: RawInputModifiers, position: Point);
    fn on_touch_down(&self, timestamp: u64, touch_id: i32, position: Point, platform_cookie: Option<PlatformInputEventCookie>);
    fn on_touch_move(&self, timestamp: u64, touch_id: i32, position: Point);
    fn on_touch_up(&self, timestamp: u64, touch_id: i32, position: Point);
    fn on_touch_cancel(&self, touch_id: i32, position: Point);
    fn on_key_down(&self, timestamp: u64, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<String>);
    fn on_key_up(&self, timestamp: u64, key: Key, modifiers: RawInputModifiers, physical_key: PhysicalKey, key_symbol: Option<String>);
    fn on_keyboard_leave(&self);
    fn on_key_repeat_info(&self, rate: i32, delay: i32);

    /// Compositor's preferred render scale for this surface changed.
    /// Either an integer (wl_surface preferred_buffer_scale / wl_output max)
    /// or fractional (wp_fractional_scale_v1) value.
    fn on_scale_changed(&self, scale: f64);

    /// The set of outputs this surface currently overlaps changed.
    /// Each entry is the opaque identity of a `WaylandOutputSnapshot`; the
    /// list is in enter order, so the last entry is the most recently
    /// entered output (used as "current" by screen-from-toplevel).
    fn on_surface_outputs_changed(&self, output_ids: Vec<WaylandOutputId>);
}

/// UI-thread sink for worker→UI events specific to `xdg_toplevel` surfaces.
pub trait IWXdgTopLevelEventSink: IWSurfaceEventSink {
    fn on_configure(&self, batch: Arc<XdgConfigureBatch>);
    fn on_close(&self);
    fn on_decoration_mode_changed(&self, mode: DecorationMode);
}

/// UI-thread sink for worker→UI events specific to `xdg_popup` surfaces.
/// Marshalled across the worker→UI boundary by `WXdgPopupEventSinkProxy`:
/// calls are posted to the UI dispatcher.
pub trait IWXdgPopupEventSink: IWSurfaceEventSink {
    /// Compositor delivered a new popup configure (x, y, width, height),
    /// sealed by the wrapping xdg_surface.configure(serial).
    fn on_popup_configure(&self, batch: XdgPopupConfigureBatch);

    /// Compositor dismissed the popup (xdg_popup.popup_done). The popup
    /// must be torn down; after this no further events arrive.
    fn on_popup_done(&self);
}

/// The sink of a surface as the worker holds it.
#[derive(Clone)]
pub struct WSurfaceEventSinkProxy {
    target: UiThreadRef<dyn IWSurfaceEventSink>,
}

impl WSurfaceEventSinkProxy {
    /// A proxy of `target`, an object of the calling thread, whose dispatcher `dispatcher` is.
    pub fn new(target: Rc<dyn IWSurfaceEventSink>, dispatcher: Arc<Dispatcher>) -> Self {
        Self { target: UiThreadRef::new(target, dispatcher) }
    }

    /// Whether both proxies are of the same sink.
    pub fn is_same(&self, other: &WSurfaceEventSinkProxy) -> bool {
        self.target.is_same(&other.target)
    }

    pub fn on_pointer_enter(&self, timestamp: u64, serial: u32, position: Point) {
        self.target.post(move |sink| sink.on_pointer_enter(timestamp, serial, position));
    }

    pub fn on_pointer_leave(&self, serial: u32) {
        self.target.post(move |sink| sink.on_pointer_leave(serial));
    }

    pub fn on_pointer_motion(&self, timestamp: u64, position: Point, modifiers: RawInputModifiers) {
        self.target.post(move |sink| sink.on_pointer_motion(timestamp, position, modifiers));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn on_pointer_button(
        &self,
        timestamp: u64,
        serial: u32,
        type_: RawPointerEventType,
        modifiers: RawInputModifiers,
        position: Point,
        platform_cookie: Option<PlatformInputEventCookie>,
    ) {
        self.target
            .post(move |sink| sink.on_pointer_button(timestamp, serial, type_, modifiers, position, platform_cookie));
    }

    pub fn on_pointer_axis(&self, timestamp: u64, delta: Vector, modifiers: RawInputModifiers, position: Point) {
        self.target.post(move |sink| sink.on_pointer_axis(timestamp, delta, modifiers, position));
    }

    pub fn on_touch_down(
        &self,
        timestamp: u64,
        touch_id: i32,
        position: Point,
        platform_cookie: Option<PlatformInputEventCookie>,
    ) {
        self.target.post(move |sink| sink.on_touch_down(timestamp, touch_id, position, platform_cookie));
    }

    pub fn on_touch_move(&self, timestamp: u64, touch_id: i32, position: Point) {
        self.target.post(move |sink| sink.on_touch_move(timestamp, touch_id, position));
    }

    pub fn on_touch_up(&self, timestamp: u64, touch_id: i32, position: Point) {
        self.target.post(move |sink| sink.on_touch_up(timestamp, touch_id, position));
    }

    pub fn on_touch_cancel(&self, touch_id: i32, position: Point) {
        self.target.post(move |sink| sink.on_touch_cancel(touch_id, position));
    }

    pub fn on_key_down(
        &self,
        timestamp: u64,
        key: Key,
        modifiers: RawInputModifiers,
        physical_key: PhysicalKey,
        key_symbol: Option<String>,
    ) {
        self.target.post(move |sink| sink.on_key_down(timestamp, key, modifiers, physical_key, key_symbol));
    }

    pub fn on_key_up(
        &self,
        timestamp: u64,
        key: Key,
        modifiers: RawInputModifiers,
        physical_key: PhysicalKey,
        key_symbol: Option<String>,
    ) {
        self.target.post(move |sink| sink.on_key_up(timestamp, key, modifiers, physical_key, key_symbol));
    }

    pub fn on_keyboard_leave(&self) {
        self.target.post(|sink| sink.on_keyboard_leave());
    }

    pub fn on_key_repeat_info(&self, rate: i32, delay: i32) {
        self.target.post(move |sink| sink.on_key_repeat_info(rate, delay));
    }

    pub fn on_scale_changed(&self, scale: f64) {
        self.target.post(move |sink| sink.on_scale_changed(scale));
    }

    pub fn on_surface_outputs_changed(&self, output_ids: Vec<WaylandOutputId>) {
        self.target.post(move |sink| sink.on_surface_outputs_changed(output_ids));
    }
}

/// The sink of a top-level as the worker holds it.
#[derive(Clone)]
pub struct WXdgTopLevelEventSinkProxy {
    target: UiThreadRef<dyn IWXdgTopLevelEventSink>,
    surface: WSurfaceEventSinkProxy,
}

impl WXdgTopLevelEventSinkProxy {
    /// A proxy of `target`, an object of the calling thread, whose dispatcher `dispatcher` is.
    pub fn new<T: IWXdgTopLevelEventSink + 'static>(target: Rc<T>, dispatcher: Arc<Dispatcher>) -> Self {
        let surface: Rc<dyn IWSurfaceEventSink> = target.clone();
        let top_level: Rc<dyn IWXdgTopLevelEventSink> = target;
        Self {
            target: UiThreadRef::new(top_level, dispatcher.clone()),
            surface: WSurfaceEventSinkProxy::new(surface, dispatcher),
        }
    }

    /// The proxy as the sink of the events every surface has (the base interface).
    pub fn as_surface_sink(&self) -> &WSurfaceEventSinkProxy {
        &self.surface
    }

    pub fn on_configure(&self, batch: Arc<XdgConfigureBatch>) {
        self.target.post(move |sink| sink.on_configure(batch));
    }

    pub fn on_close(&self) {
        self.target.post(|sink| sink.on_close());
    }

    pub fn on_decoration_mode_changed(&self, mode: DecorationMode) {
        self.target.post(move |sink| sink.on_decoration_mode_changed(mode));
    }
}

/// The sink of a popup as the worker holds it.
#[derive(Clone)]
pub struct WXdgPopupEventSinkProxy {
    target: UiThreadRef<dyn IWXdgPopupEventSink>,
    surface: WSurfaceEventSinkProxy,
}

impl WXdgPopupEventSinkProxy {
    /// A proxy of `target`, an object of the calling thread, whose dispatcher `dispatcher` is.
    pub fn new<T: IWXdgPopupEventSink + 'static>(target: Rc<T>, dispatcher: Arc<Dispatcher>) -> Self {
        let surface: Rc<dyn IWSurfaceEventSink> = target.clone();
        let popup: Rc<dyn IWXdgPopupEventSink> = target;
        Self {
            target: UiThreadRef::new(popup, dispatcher.clone()),
            surface: WSurfaceEventSinkProxy::new(surface, dispatcher),
        }
    }

    /// The proxy as the sink of the events every surface has (the base interface).
    pub fn as_surface_sink(&self) -> &WSurfaceEventSinkProxy {
        &self.surface
    }

    pub fn on_popup_configure(&self, batch: XdgPopupConfigureBatch) {
        self.target.post(move |sink| sink.on_popup_configure(batch));
    }

    pub fn on_popup_done(&self) {
        self.target.post(|sink| sink.on_popup_done());
    }
}
