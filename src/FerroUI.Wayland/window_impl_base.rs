//! What the windows of the backend share on the UI thread (the port of
//! `WindowImplBase.cs`).
//!
//! The reference has an abstract class the window (and the popup) derive
//! from, with an abstract nested class `Sink` their sinks derive from. Here
//! the window holds a [`WindowBaseImpl`], its sink holds a [`Sink`], and what
//! the derived classes override is the trait [`ISinkOwner`].
//!
//! The drag and drop part of the sink (`WindowImplBase.DragDrop.cs`) belongs
//! to stage 2 of `docs/porting/wayland-platform.md`.

use crate::screens::wayland_output_snapshot::WaylandOutputId;
use crate::server::persistent::i_w_surface::IWSurface;
use crate::server::persistent::i_w_xdg_top_level::WXdgShellSurfaceProxy;
use crate::server::wayland_worker_client::WaylandWorkerClient;
use crate::wayland_cursor_factory::WaylandCursorImpl;
use crate::window_impl_base_keyboard::KeyRepeat;
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawTextInputEventArgs};
use ferroui_base::input::{IInputDevice, IInputRoot, KeyboardDevice, MouseDevice, TouchDevice};
use ferroui_base::platform::ICursorImpl;
use ferroui_base::{PixelPoint, Rect, Size};
use ferroui_controls::{WindowResizeReason, WindowTransparencyLevel};
use ferroui_x11::raw_event_grouping::RawEventGrouper;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) type Callback<F> = RefCell<Option<Rc<F>>>;

pub(crate) fn get<F: ?Sized>(callback: &Callback<F>) -> Option<Rc<F>> {
    callback.borrow().clone()
}

pub(crate) fn set<F: ?Sized>(callback: &Callback<F>, value: Option<Rc<F>>) {
    let old = callback.replace(value);
    drop(old);
}

/// A raw input event.
pub(crate) type RawEvent = Rc<dyn IRawInputEventArgs>;

/// The state every window of the backend has.
pub struct WindowBaseImpl {
    client: Rc<WaylandWorkerClient>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    mouse: Rc<MouseDevice>,
    touch: Rc<TouchDevice>,
    keyboard: Rc<KeyboardDevice>,
    render_scaling: Cell<f64>,
    current_cursor: RefCell<Option<Rc<dyn ICursorImpl>>>,
    is_enabled: Cell<bool>,
    is_disposed: Cell<bool>,
    current_output_ids: RefCell<Vec<WaylandOutputId>>,
    client_size: Cell<Size>,

    pub(crate) closed: Callback<dyn Fn()>,
    pub(crate) lost_focus: Callback<dyn Fn()>,
    pub(crate) position_changed: Callback<dyn Fn(PixelPoint)>,
    pub(crate) activated: Callback<dyn Fn()>,
    pub(crate) deactivated: Callback<dyn Fn()>,
    pub(crate) input: Callback<dyn Fn(RawEvent)>,
    pub(crate) paint: Callback<dyn Fn(Rect)>,
    pub(crate) resized: Callback<dyn Fn(Size, WindowResizeReason)>,
    pub(crate) scaling_changed: Callback<dyn Fn(f64)>,
    pub(crate) transparency_level_changed: Callback<dyn Fn(WindowTransparencyLevel)>,
}

impl WindowBaseImpl {
    pub(crate) fn new(client: Rc<WaylandWorkerClient>, keyboard: Rc<KeyboardDevice>) -> Self {
        Self {
            client,
            input_root: RefCell::new(None),
            mouse: MouseDevice::primary(),
            touch: TouchDevice::new(),
            keyboard,
            render_scaling: Cell::new(1.0),
            current_cursor: RefCell::new(None),
            is_enabled: Cell::new(true),
            is_disposed: Cell::new(false),
            current_output_ids: RefCell::new(Vec::new()),
            client_size: Cell::new(Size::default()),
            closed: RefCell::new(None),
            lost_focus: RefCell::new(None),
            position_changed: RefCell::new(None),
            activated: RefCell::new(None),
            deactivated: RefCell::new(None),
            input: RefCell::new(None),
            paint: RefCell::new(None),
            resized: RefCell::new(None),
            scaling_changed: RefCell::new(None),
            transparency_level_changed: RefCell::new(None),
        }
    }

    pub(crate) fn client(&self) -> &Rc<WaylandWorkerClient> {
        &self.client
    }

    pub(crate) fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    pub(crate) fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        *self.input_root.borrow_mut() = Some(input_root);
    }

    pub(crate) fn mouse(&self) -> Rc<dyn IInputDevice> {
        self.mouse.clone()
    }

    pub(crate) fn touch(&self) -> Rc<dyn IInputDevice> {
        self.touch.clone()
    }

    pub(crate) fn keyboard(&self) -> Rc<dyn IInputDevice> {
        self.keyboard.clone()
    }

    pub fn render_scaling(&self) -> f64 {
        self.render_scaling.get()
    }

    /// Sets the scaling without telling anyone (the setter of `RenderScaling`, which a
    /// popup uses to start with the scaling of its parent).
    pub(crate) fn set_render_scaling(&self, value: f64) {
        self.render_scaling.set(value);
    }

    /// The keyboard device as its type: a popup is made with the device of its parent.
    pub(crate) fn keyboard_device(&self) -> Rc<KeyboardDevice> {
        self.keyboard.clone()
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.is_enabled.get()
    }

    pub(crate) fn set_is_enabled(&self, value: bool) {
        self.is_enabled.set(value);
    }

    pub(crate) fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    /// Marks the window as disposed. Returns whether it was disposed already.
    pub(crate) fn mark_disposed(&self) -> bool {
        self.is_disposed.replace(true)
    }

    /// The outputs the surface of the window is on, in the order it entered them.
    pub fn current_output_ids(&self) -> Vec<WaylandOutputId> {
        self.current_output_ids.borrow().clone()
    }

    pub fn client_size(&self) -> Size {
        self.client_size.get()
    }

    pub(crate) fn set_client_size(&self, value: Size) {
        self.client_size.set(value);
        self.client.any_thread_wakeup_render_loop();
    }

    /// The cursor the window asked for, when it is one of this backend.
    pub(crate) fn current_cursor(&self) -> Option<Rc<dyn ICursorImpl>> {
        self.current_cursor.borrow().clone()
    }

    pub(crate) fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>, proxy: Option<&WXdgShellSurfaceProxy>) {
        // `cursor as WaylandCursorImpl`: a cursor of another backend is no cursor.
        let cursor = cursor.filter(|cursor| cursor.as_any().is::<WaylandCursorImpl>());
        *self.current_cursor.borrow_mut() = cursor;
        self.apply_current_cursor(proxy);
    }

    pub(crate) fn apply_current_cursor(&self, proxy: Option<&WXdgShellSurfaceProxy>) {
        let Some(proxy) = proxy else {
            return;
        };
        let cursor = self.current_cursor.borrow().clone();
        let cursor = cursor.as_ref().and_then(|cursor| cursor.as_any().downcast_ref::<WaylandCursorImpl>());
        proxy.set_cursor(cursor.map(WaylandCursorImpl::cursor));
    }

    pub(crate) fn update_scaling(&self, scale: f64) {
        if (scale - self.render_scaling.get()).abs() < 1e-6 {
            return;
        }
        self.render_scaling.set(scale);
        if let Some(scaling_changed) = get(&self.scaling_changed) {
            scaling_changed(scale);
        }
        self.client.any_thread_wakeup_render_loop();
    }

    pub(crate) fn raise_input(&self, args: RawEvent) {
        if let Some(input) = get(&self.input) {
            input(args);
        }
    }
}

/// What a window is to its sink: the members of `WindowBaseImpl` and of `Sink` the derived
/// classes of the reference override.
pub(crate) trait ISinkOwner {
    fn base(&self) -> &WindowBaseImpl;

    /// Disconnects the window from the surface of the worker.
    fn disconnect_from_surface(&self);

    /// Handles an input event that belongs to the kind of surface (the chrome of a
    /// top-level). Returns whether the event is consumed.
    fn handle_surface_specific_dispatch(&self, _args: &RawEvent) -> bool {
        false
    }

    fn on_input_while_disabled(&self) {}
}

/// The part of a sink every kind of surface has: the queue of raw input and the key repeat.
pub(crate) struct Sink {
    parent: Weak<dyn ISinkOwner>,
    is_disposed: Cell<bool>,
    raw_event_grouper: RawEventGrouper,
    pub(crate) key_repeat: KeyRepeat,
}

impl Sink {
    pub(crate) fn new(parent: Weak<dyn ISinkOwner>, client: &WaylandWorkerClient) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Sink>| {
            let dispatch = this.clone();
            Self {
                parent,
                is_disposed: Cell::new(false),
                raw_event_grouper: RawEventGrouper::new(
                    move |args| {
                        if let Some(this) = dispatch.upgrade() {
                            this.dispatch_input(args);
                        }
                    },
                    Some(client.input_dispatch_queue().clone()),
                ),
                key_repeat: KeyRepeat::new(this.clone()),
            }
        })
    }

    pub(crate) fn parent(&self) -> Option<Rc<dyn ISinkOwner>> {
        self.parent.upgrade()
    }

    pub(crate) fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    pub(crate) fn dispose(&self) {
        if self.is_disposed.replace(true) {
            return;
        }
        self.key_repeat.stop();
        if let Some(parent) = self.parent() {
            parent.disconnect_from_surface();
        }
    }

    pub(crate) fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.parent().and_then(|parent| parent.base().input_root())
    }

    pub(crate) fn schedule_input(&self, args: RawEvent) {
        self.raw_event_grouper.handle_event(args);
    }

    fn dispatch_input(&self, args: RawEvent) {
        let Some(parent) = self.parent() else {
            return;
        };
        let Some(input_root) = parent.base().input_root() else {
            return;
        };
        if self.is_disposed.get() {
            return;
        }

        if !parent.base().is_enabled() {
            parent.on_input_while_disabled();
            return;
        }

        if self.key_repeat.handle_keyboard_dispatch(&args) {
            return;
        }

        if parent.handle_surface_specific_dispatch(&args) {
            return;
        }

        parent.base().raise_input(args.clone());

        if args.handled() {
            return;
        }
        if let Some(key_args) = args.downcast_ref::<RawKeyEventArgs>() {
            if key_args.type_() == RawKeyEventType::KeyDown {
                if let Some(text) = key_args.key_symbol() {
                    parent.base().raise_input(Rc::new(RawTextInputEventArgs::new(
                        parent.base().keyboard(),
                        args.timestamp(),
                        input_root,
                        text,
                    )));
                }
            }
        }
    }

    // Configure/close are surface-role-specific (xdg_toplevel vs xdg_popup
    // carry different payloads). The sinks of the surfaces implement the matching
    // worker→UI interface.

    pub(crate) fn on_scale_changed(&self, scale: f64) {
        if self.is_disposed.get() {
            return;
        }
        if let Some(parent) = self.parent() {
            parent.base().update_scaling(scale);
        }
    }

    pub(crate) fn on_surface_outputs_changed(&self, output_ids: Vec<WaylandOutputId>) {
        if self.is_disposed.get() {
            return;
        }
        if let Some(parent) = self.parent() {
            *parent.base().current_output_ids.borrow_mut() = output_ids;
        }
    }
}
