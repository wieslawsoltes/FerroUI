//! The source side of the protocol: a drag that starts in a window of the
//! application (the port of `X11DragSource.cs`).
//!
//! The reference has one nested class, `Handler`, which is the event hook
//! of the dispatcher for the time of a drag and calls Xlib and the
//! platform directly. Here the handler has the protocol (which target is
//! under the pointer, what is sent to it and when, what its answers mean)
//! and reaches the connection and the platform through
//! [`IDragSourceHost`], so that the protocol can run against a host that
//! is not a server (the tests); [`XlibDragSourceHost`] is the host on the
//! connection of the platform.

use crate::dispatching::x11_event_dispatcher::IEventHook;
use crate::selections::data_format_helper;
use crate::selections::drag_drop::drag_drop_data_provider::DragDropDataProvider;
use crate::selections::drag_drop::drag_drop_timeout_manager::DragDropTimeoutManager;
use crate::selections::drag_drop::i_xdnd_window::IXdndWindow;
use crate::selections::drag_drop::x11_drop_target::send_xdnd_message;
use crate::selections::drag_drop::xdnd_action_helper::XdndActions;
use crate::selections::drag_drop::xdnd_constants::{MIN_XDND_VERSION, XDND_VERSION};
use crate::selections::selection_helper::{self, TaskCompletionSource};
use crate::x11_atoms::X11Atoms;
use crate::x11_cursor_factory::X11CursorFactory;
use crate::x11_enum_extensions::X11EnumExtensions;
use crate::x11_enums::XModifierMask;
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::{EventMask, XEventName};
use crate::xlib::{self, Atom, PropertyMode, Time, XEvent, XIEventData, XKeyEvent, XID};
use ferroui_base::input::platform::IPlatformDragSource;
use ferroui_base::input::raw::{RawDragEvent, RawDragEventType};
use ferroui_base::input::{
    DragDropEffects, IDataTransfer, KeyModifiers, LocalBoxFuture, PointerPressedEventArgs, RawInputModifiers,
    StandardCursorType,
};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelPoint, Visual};
use ferroui_controls::TopLevel;
use std::cell::{Cell, OnceCell, RefCell};
use std::ffi::{c_int, c_long, c_ulong};
use std::rc::{Rc, Weak};

/// Implementation of the platform drag source for X11 using XDND.
/// Specs: <https://www.freedesktop.org/wiki/Specifications/XDND/>
pub struct X11DragSource {
    platform: Weak<FerroX11Platform>,
}

impl X11DragSource {
    pub fn new(platform: &Rc<FerroX11Platform>) -> Self {
        Self { platform: Rc::downgrade(platform) }
    }
}

impl IPlatformDragSource for X11DragSource {
    /// # Panics
    /// Panics when the source of the trigger event is not in a window of
    /// this platform ("Invalid drag source"), and when called from a
    /// thread other than the UI thread.
    fn do_drag_drop_async(
        &self,
        trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        Dispatcher::ui_thread().verify_access();

        let platform = self.platform.upgrade();
        let source = trigger_event.source().and_then(|source| source.cast::<Visual>());
        let window = TopLevel::get_top_level(source.as_deref())
            .and_then(|top_level| top_level.try_get_platform_handle())
            .zip(platform.as_ref())
            .and_then(|(handle, platform)| platform.get_window(handle.handle() as XID))
            .and_then(|info| info.window());
        let (Some(window), Some(platform)) = (window, platform) else {
            panic!("Invalid drag source (Parameter 'triggerEvent')");
        };

        trigger_event.pointer().capture(None);

        let host = XlibDragSourceHost::new(&platform, IXdndWindow::handle(&*window), data_transfer, allowed_effects);
        let handler = Handler::new(
            host.clone(),
            IXdndWindow::handle(&*window),
            allowed_effects,
            trigger_event.key_modifiers(),
            XdndSourceAtoms::new(platform.info().atoms()),
            platform.xi2().map(|_| platform.info().x_input_opcode()),
        );
        host.attach(&handler);

        // Disposes the handler when the drag ends, and when the future is
        // dropped before that (the `using` of the reference).
        struct DisposeHandler(Rc<Handler>);
        impl Drop for DisposeHandler {
            fn drop(&mut self) {
                self.0.dispose();
            }
        }
        let guard = DisposeHandler(handler);

        Box::pin(async move {
            let completion = guard.0.completion();
            completion.await
        })
    }
}

/// The atoms the source side uses.
#[derive(Clone, Copy, Debug)]
pub(crate) struct XdndSourceAtoms {
    pub enter: Atom,
    pub position: Atom,
    pub status: Atom,
    pub leave: Atom,
    pub drop: Atom,
    pub finished: Atom,
    pub aware: Atom,
    pub proxy: Atom,
    pub atom: Atom,
    pub window: Atom,
    pub actions: XdndActions,
}

impl XdndSourceAtoms {
    fn new(atoms: &X11Atoms) -> Self {
        Self {
            enter: atoms.XdndEnter,
            position: atoms.XdndPosition,
            status: atoms.XdndStatus,
            leave: atoms.XdndLeave,
            drop: atoms.XdndDrop,
            finished: atoms.XdndFinished,
            aware: atoms.XdndAware,
            proxy: atoms.XdndProxy,
            atom: atoms.ATOM,
            window: atoms.WINDOW,
            actions: XdndActions::new(atoms),
        }
    }
}

/// What the handler of a drag does on the connection and in the platform.
pub(crate) trait IDragSourceHost {
    /// Whether a window is a top-level of this application.
    fn is_in_process_window(&self, window: XID) -> bool;

    /// The first item of a property of a window, read with a type.
    fn get_window_property(&self, window: XID, property: Atom, type_: Atom) -> Option<c_ulong>;

    fn root_window(&self) -> XID;

    /// The child of `window` that contains a point of the root window (0
    /// for none); `None` when the server cannot tell.
    fn child_at(&self, window: XID, root_position: PixelPoint) -> Option<XID>;

    /// The format atoms of the data of the drag.
    fn format_atoms(&self) -> Vec<Atom>;

    /// Makes the source window the owner of the selection of the protocol
    /// and lists the formats on it (`XdndTypeList`).
    fn publish(&self, format_atoms: &[Atom]);

    /// Sends a message of the protocol to a window; the first item of the
    /// data is the source window.
    fn send_xdnd_message(&self, message_type: Atom, message_window: XID, data: [c_long; 5]);

    /// Raises a drag event in a window of this application and gives the
    /// effects its handlers chose.
    fn process_raw_drag_event(
        &self,
        target_window: XID,
        event_type: RawDragEventType,
        root_position: PixelPoint,
        modifiers: RawInputModifiers,
        effective_allowed_effects: DragDropEffects,
    ) -> DragDropEffects;

    fn ungrab_pointer(&self);

    /// Shows the cursor of the effects while the pointer is grabbed.
    fn set_grab_cursor(&self, effects: DragDropEffects);

    fn restart_timeout(&self);

    fn stop_timeout(&self);

    /// Whether sending data to the target restarts the timeout.
    fn set_activity_restarts_timeout(&self, value: bool);

    /// Answers the request of the target for the data.
    fn on_selection_request(&self, evt: &XEvent);

    /// The key symbol of a key event, without modifiers applied.
    fn lookup_keysym(&self, key: &XKeyEvent) -> c_ulong;

    /// The name of an atom, for the log.
    fn atom_name(&self, atom: Atom) -> Option<String>;

    /// Removes the event hook and releases the selection, the data and the
    /// timeout.
    fn dispose(&self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct XdndTargetInfo {
    pub version: u8,
    pub target_window: XID,
    pub message_window: XID,
    /// Whether the target is a window of this application
    /// (`InProcessWindow`), which gets its drag events without the
    /// protocol.
    pub in_process: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PositionRequest {
    position: PixelPoint,
    timestamp: Time,
    action: Atom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DropRequest {
    timestamp: Time,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TargetState {
    target: Option<XdndTargetInfo>,
    allows_drop: bool,
    /// XdndPosition sent, but no XdndStatus received yet
    is_waiting_for_status: bool,
    /// Position not sent via XdndPosition yet
    pending_position: Option<PositionRequest>,
    /// Drop not sent via XdndDrop yet
    pending_drop: Option<DropRequest>,
}

impl TargetState {
    fn new(target: Option<XdndTargetInfo>) -> Self {
        Self { target, ..Default::default() }
    }
}

const XK_SHIFT_L: c_ulong = 0xffe1;
const XK_SHIFT_R: c_ulong = 0xffe2;
const XK_CONTROL_L: c_ulong = 0xffe3;
const XK_CONTROL_R: c_ulong = 0xffe4;
const XK_ALT_L: c_ulong = 0xffe9;
const XK_ALT_R: c_ulong = 0xffea;

/// The effects the modifier keys leave of all effects
/// (`GetAllowedEffectsFromKeyModifiers`).
pub(crate) fn get_allowed_effects_from_key_modifiers(key_modifiers: KeyModifiers) -> DragDropEffects {
    if key_modifiers.contains(KeyModifiers::CONTROL) {
        return DragDropEffects::COPY;
    }
    if key_modifiers.contains(KeyModifiers::SHIFT) {
        return DragDropEffects::MOVE;
    }
    if key_modifiers.contains(KeyModifiers::ALT) {
        return DragDropEffects::LINK;
    }
    DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK
}

/// The item of an `XdndPosition` message that carries the point of the
/// root window.
pub(crate) fn position_to_message(root_position: PixelPoint) -> c_long {
    ((root_position.x as c_long) << 16) | (root_position.y as c_long)
}

/// The state of one drag (`Handler`): the event hook of the dispatcher
/// while the drag lasts.
pub(crate) struct Handler {
    host: Rc<dyn IDragSourceHost>,
    source_window: XID,
    allowed_effects: DragDropEffects,
    atoms: XdndSourceAtoms,
    /// The opcode of the X Input extension when the platform takes
    /// pointer input from it.
    xi_opcode: Option<c_int>,
    log: bool,
    completion_source: Rc<TaskCompletionSource<DragDropEffects>>,
    format_atoms: RefCell<Vec<Atom>>,
    pointer_grabbed: Cell<bool>,
    current_effects: Cell<DragDropEffects>,
    target_state: Cell<TargetState>,
    disposed: Cell<bool>,
}

impl Handler {
    pub(crate) fn new(
        host: Rc<dyn IDragSourceHost>,
        source_window: XID,
        allowed_effects: DragDropEffects,
        initial_key_modifiers: KeyModifiers,
        atoms: XdndSourceAtoms,
        xi_opcode: Option<c_int>,
    ) -> Rc<Self> {
        let handler = Rc::new(Self {
            host,
            source_window,
            allowed_effects,
            atoms,
            xi_opcode,
            log: Logger::try_get(LogEventLevel::Verbose, LogArea::X11_PLATFORM).is_some(),
            completion_source: TaskCompletionSource::new(),
            format_atoms: RefCell::new(Vec::new()),
            pointer_grabbed: Cell::new(false),
            current_effects: Cell::new(DragDropEffects::NONE),
            target_state: Cell::new(TargetState::default()),
            disposed: Cell::new(false),
        });

        if !handler.host.is_in_process_window(source_window) {
            handler.complete(DragDropEffects::NONE);
            return handler;
        }

        // Assume we have an implicit grab here.
        handler.pointer_grabbed.set(true);
        handler.update_current_effects(handler.get_effective_allowed_effects(initial_key_modifiers));

        // The event hook is installed by whoever created the host
        // (`XlibDragSourceHost::attach`).

        let format_atoms = handler.host.format_atoms();

        // Publish our formats.
        handler.host.publish(&format_atoms);
        *handler.format_atoms.borrow_mut() = format_atoms;

        handler
    }

    /// The future of the effect the drag ends with (`Completion`).
    pub(crate) fn completion(&self) -> impl std::future::Future<Output = DragDropEffects> {
        self.completion_source.task()
    }

    pub(crate) fn is_completed(&self) -> bool {
        self.completion_source.is_completed()
    }

    fn log(&self, message: impl FnOnce() -> String) {
        if self.log {
            if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, LogArea::X11_PLATFORM) {
                logger.log(None, &message());
            }
        }
    }

    fn state(&self) -> TargetState {
        self.target_state.get()
    }

    fn update_state(&self, update: impl FnOnce(&mut TargetState)) {
        let mut state = self.target_state.get();
        update(&mut state);
        self.target_state.set(state);
    }

    fn try_handle_source_window_event(&self, evt: &XEvent) -> bool {
        let event_type = xlib::event_type(evt);
        if event_type == XEventName::ClientMessage as i32 {
            let message = xlib::client_message_event(evt);
            let data: [c_long; 5] = std::array::from_fn(|index| message.data.get_long(index));
            if message.message_type == self.atoms.status {
                self.on_xdnd_status(data);
                return true;
            }
            if message.message_type == self.atoms.finished {
                self.on_xdnd_finished(data);
                return true;
            }
            false
        } else if event_type == XEventName::SelectionRequest as i32 {
            self.host.on_selection_request(evt);
            true
        } else {
            false
        }
    }

    fn try_handle_global_pointer_event(&self, evt: &XEvent) -> bool {
        let event_type = xlib::event_type(evt);
        if event_type == XEventName::MotionNotify as i32 {
            let motion = xlib::motion_event(evt);
            self.on_pointer_moved(
                PixelPoint::new(motion.x_root, motion.y_root),
                motion.time,
                XModifierMask::from_bits_retain(motion.state as i32).to_raw_input_modifiers(),
            );
            true
        } else if event_type == XEventName::ButtonPress as i32 {
            true
        } else if event_type == XEventName::ButtonRelease as i32 {
            let button = xlib::button_event(evt);
            self.on_pointer_released(
                PixelPoint::new(button.x_root, button.y_root),
                button.time,
                XModifierMask::from_bits_retain(button.state as i32).to_raw_input_modifiers(),
            );
            true
        } else if event_type == XEventName::KeyPress as i32 || event_type == XEventName::KeyRelease as i32 {
            self.on_key(xlib::key_event(evt));
            true
        } else if event_type == XEventName::GenericEvent as i32 {
            let cookie = xlib::generic_event_cookie(evt);
            if self.xi_opcode != Some(cookie.extension) {
                return false;
            }

            let Some(XIEventData::Device(device_event)) = xlib::copy_xi_event(cookie) else {
                return false;
            };
            let modifiers =
                || XModifierMask::from_bits_retain(device_event.mods_effective).to_raw_input_modifiers();
            let root_position = || PixelPoint::new(device_event.root_x as i32, device_event.root_y as i32);
            match device_event.evtype {
                x11_dl::xinput2::XI_Motion | x11_dl::xinput2::XI_TouchUpdate => {
                    self.on_pointer_moved(root_position(), device_event.time, modifiers());
                    true
                }
                x11_dl::xinput2::XI_ButtonPress | x11_dl::xinput2::XI_TouchBegin => true,
                x11_dl::xinput2::XI_ButtonRelease | x11_dl::xinput2::XI_TouchEnd => {
                    self.on_pointer_released(root_position(), device_event.time, modifiers());
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub(crate) fn on_pointer_moved(&self, root_position: PixelPoint, timestamp: Time, modifiers: RawInputModifiers) {
        // Drop pending, ignore any queued position change.
        if self.state().pending_drop.is_some() {
            return;
        }

        let target = self.find_xdnd_target(root_position);

        // Handle new target.
        if self.state().target != target {
            self.log(|| {
                format!(
                    "Pointer moved from window {:?} to window {:?}.",
                    self.state().target.map(|target| target.target_window),
                    target.map(|target| target.target_window)
                )
            });

            if let Some(last_target) = self.state().target {
                if last_target.in_process {
                    self.process_raw_drag_event(
                        last_target.target_window,
                        RawDragEventType::DragLeave,
                        root_position,
                        modifiers,
                    );
                } else {
                    self.send_xdnd_leave(&last_target);
                }
            }

            self.target_state.set(TargetState::new(target));

            let effective_allowed_effects = self.get_effective_allowed_effects(modifiers.to_key_modifiers());
            self.update_current_effects(effective_allowed_effects);

            if let Some(new_target) = target {
                if new_target.in_process {
                    let effects = self.process_raw_drag_event(
                        new_target.target_window,
                        RawDragEventType::DragEnter,
                        root_position,
                        modifiers,
                    );
                    self.update_state(|state| state.allows_drop = effects != DragDropEffects::NONE);
                    self.update_current_effects(effects);
                } else {
                    self.send_xdnd_enter(&new_target);
                }
            }
        }

        // Update current target.
        if let Some(current_target) = target {
            self.update_target_position(&current_target, root_position, modifiers, timestamp);
        }
    }

    fn update_target_position(
        &self,
        target: &XdndTargetInfo,
        root_position: PixelPoint,
        modifiers: RawInputModifiers,
        timestamp: Time,
    ) {
        if target.in_process {
            let effects =
                self.process_raw_drag_event(target.target_window, RawDragEventType::DragOver, root_position, modifiers);
            self.update_state(|state| state.allows_drop = effects != DragDropEffects::NONE);
            self.update_current_effects(effects);
        } else {
            let effective_allowed_effects = self.get_effective_allowed_effects(modifiers.to_key_modifiers());
            let action = self.atoms.actions.effects_to_action(effective_allowed_effects);
            let position_request = PositionRequest { position: root_position, timestamp, action };

            if self.state().is_waiting_for_status {
                // We already sent a position previously and are waiting for a response. Don't flood.
                self.update_state(|state| state.pending_position = Some(position_request));

                self.log(|| {
                    format!(
                        "Pointer moved to point {root_position:?} on window {} while waiting for XdndStatus. \
                         XdndPosition will be sent later.",
                        target.target_window
                    )
                });
            } else {
                self.send_position_request(position_request, target);
            }
        }
    }

    fn on_key(&self, key: &XKeyEvent) {
        let state = self.state();
        let Some(target) = state.target else {
            return;
        };
        if state.pending_drop.is_some() {
            return;
        }

        let key_sym = self.host.lookup_keysym(key);

        // If Shift/Ctrl/Alt are pressed/released, effective effects might change: update the target.
        let interesting_modifiers = match key_sym {
            XK_SHIFT_L | XK_SHIFT_R => XModifierMask::SHIFT_MASK,
            XK_CONTROL_L | XK_CONTROL_R => XModifierMask::CONTROL_MASK,
            XK_ALT_L | XK_ALT_R => XModifierMask::MOD1_MASK,
            _ => XModifierMask::empty(),
        };

        if interesting_modifiers.is_empty() {
            return;
        }

        let mut modifiers = XModifierMask::from_bits_retain(key.state as i32);
        if key.type_ == XEventName::KeyPress as i32 {
            modifiers |= interesting_modifiers;
        } else if key.type_ == XEventName::KeyRelease as i32 {
            modifiers &= !interesting_modifiers;
        }

        let position = PixelPoint::new(key.x_root, key.y_root);
        self.update_target_position(&target, position, modifiers.to_raw_input_modifiers(), key.time);
    }

    pub(crate) fn on_pointer_released(&self, root_position: PixelPoint, timestamp: Time, modifiers: RawInputModifiers) {
        self.ungrab_pointer();

        let state = self.state();
        let Some(target) = state.target.filter(|_| state.pending_drop.is_none()) else {
            self.complete(DragDropEffects::NONE);
            return;
        };

        if target.in_process {
            if state.allows_drop {
                let effects =
                    self.process_raw_drag_event(target.target_window, RawDragEventType::Drop, root_position, modifiers);
                self.update_current_effects(effects);
            } else {
                self.process_raw_drag_event(
                    target.target_window,
                    RawDragEventType::DragLeave,
                    root_position,
                    modifiers,
                );
                self.update_current_effects(DragDropEffects::NONE);
            }

            self.target_state.set(TargetState::default());
            self.complete(self.current_effects.get());
        } else {
            let drop_request = DropRequest { timestamp };

            if state.is_waiting_for_status {
                // We're still waiting for a XdndStatus response for the last position, defer the drop.
                self.update_state(|state| state.pending_drop = Some(drop_request));

                self.log(|| {
                    format!(
                        "Pointer released on window {} while waiting for XdndStatus. XdndDrop will be sent later.",
                        target.target_window
                    )
                });
            } else if state.allows_drop {
                self.send_drop_request(drop_request, &target);
            } else {
                self.target_state.set(TargetState::default());
                self.update_current_effects(DragDropEffects::NONE);
                self.send_xdnd_leave(&target);
                self.complete(DragDropEffects::NONE);
            }
        }
    }

    pub(crate) fn on_xdnd_status(&self, data: [c_long; 5]) {
        let Some(target) = self.state().target.filter(|target| data[0] as XID == target.target_window) else {
            return;
        };

        self.host.stop_timeout();

        let accepted = (data[1] & 1) == 1;
        let action = if accepted { data[4] as Atom } else { 0 };
        let effects = self.atoms.actions.action_to_effects(action);

        self.update_state(|state| {
            state.is_waiting_for_status = false;
            state.allows_drop = action != 0;
        });

        self.update_current_effects(effects & self.allowed_effects);

        self.log(|| {
            format!(
                "Received XdndStatus with action {:?} for window {}.",
                self.host.atom_name(action),
                target.target_window
            )
        });

        // If we have any pending XdndPosition or XdndDrop to send, now is the time.
        let state = self.state();
        if let Some(position) = state.pending_position {
            self.update_state(|state| state.pending_position = None);
            self.send_position_request(position, &target);
        } else if let Some(drop) = state.pending_drop {
            self.send_drop_request(drop, &target);
        }
    }

    pub(crate) fn on_xdnd_finished(&self, data: [c_long; 5]) {
        let Some(target) = self.state().target.filter(|target| data[0] as XID == target.target_window) else {
            return;
        };

        self.log(|| format!("Received XdndFinished for window {}.", target.target_window));

        self.target_state.set(TargetState::default());
        self.host.set_activity_restarts_timeout(false);
        self.host.stop_timeout();

        if target.version >= 5 {
            let accepted = (data[1] & 1) == 1;
            let action = if accepted { data[2] as Atom } else { 0 };
            let effects = self.atoms.actions.action_to_effects(action);
            self.update_current_effects(effects & self.allowed_effects);
        }

        self.complete(self.current_effects.get());
    }

    pub(crate) fn find_xdnd_target(&self, root_position: PixelPoint) -> Option<XdndTargetInfo> {
        let mut current_window = self.host.root_window();

        while current_window != 0 {
            if let Some(info) = self.try_get_xdnd_target_info(current_window) {
                return Some(info);
            }

            current_window = self.host.child_at(current_window, root_position)?;
        }

        None
    }

    fn try_get_xdnd_target_info(&self, window: XID) -> Option<XdndTargetInfo> {
        // Special case our own windows: we don't need to go through X for them.
        if self.host.is_in_process_window(window) {
            return Some(XdndTargetInfo {
                version: XDND_VERSION,
                target_window: window,
                message_window: window,
                in_process: true,
            });
        }

        let proxy_window = self.get_xdnd_proxy_window(window);
        let message_window = if proxy_window != 0 { proxy_window } else { window };
        let version = self.get_xdnd_version(message_window);
        (version != 0).then_some(XdndTargetInfo { version, target_window: window, message_window, in_process: false })
    }

    fn get_xdnd_proxy_window(&self, window: XID) -> XID {
        // Spec: If this window property exists, it must be of type XA_WINDOW and must contain the ID of the proxy window
        // that should be checked for XdndAware and that should receive all the client messages, etc.
        let proxy_window = self.host.get_window_property(window, self.atoms.proxy, self.atoms.window).unwrap_or(0);
        if proxy_window == 0 {
            return 0;
        }

        // Spec: The proxy window must have the XdndProxy property set to point to itself.
        let proxy_on_proxy =
            self.host.get_window_property(proxy_window, self.atoms.proxy, self.atoms.window).unwrap_or(0);
        if proxy_on_proxy != proxy_window {
            return 0;
        }

        proxy_window
    }

    fn get_xdnd_version(&self, window: XID) -> u8 {
        let mut version = self.host.get_window_property(window, self.atoms.aware, self.atoms.atom).unwrap_or(0);
        if version < c_ulong::from(MIN_XDND_VERSION) || version > c_ulong::from(u8::MAX) {
            version = 0;
        }

        version as u8
    }

    fn send_xdnd_enter(&self, target: &XdndTargetInfo) {
        let version = target.version.min(XDND_VERSION);
        let format_atoms = self.format_atoms.borrow();
        let has_more_formats = format_atoms.len() > 3;

        self.log(|| format!("Sending XdndEnter with version {version} to window {}.", target.target_window));

        let format = |index: usize| format_atoms.get(index).copied().unwrap_or(0) as c_long;
        self.send_xdnd_message(
            self.atoms.enter,
            target.message_window,
            (c_long::from(version) << 24) | c_long::from(has_more_formats),
            format(0),
            format(1),
            format(2),
        );
    }

    fn send_xdnd_position(&self, target: &XdndTargetInfo, root_position: PixelPoint, timestamp: Time, action: Atom) {
        self.log(|| {
            format!(
                "Sending XdndPosition with action {:?} at point {root_position:?} to window {}.",
                self.host.atom_name(action),
                target.target_window
            )
        });

        self.send_xdnd_message(
            self.atoms.position,
            target.message_window,
            0,
            position_to_message(root_position),
            timestamp as c_long,
            action as c_long,
        );
    }

    fn send_xdnd_leave(&self, target: &XdndTargetInfo) {
        self.log(|| format!("Sending XdndLeave to window {}.", target.target_window));

        self.send_xdnd_message(self.atoms.leave, target.message_window, 0, 0, 0, 0);
    }

    fn send_xdnd_drop(&self, target: &XdndTargetInfo, timestamp: Time) {
        self.log(|| format!("Sending XdndDrop to window {}.", target.target_window));

        self.send_xdnd_message(self.atoms.drop, target.message_window, 0, timestamp as c_long, 0, 0);
    }

    fn send_xdnd_message(
        &self,
        message_type: Atom,
        message_window: XID,
        ptr2: c_long,
        ptr3: c_long,
        ptr4: c_long,
        ptr5: c_long,
    ) {
        self.host.send_xdnd_message(
            message_type,
            message_window,
            [self.source_window as c_long, ptr2, ptr3, ptr4, ptr5],
        );
    }

    fn send_position_request(&self, request: PositionRequest, target: &XdndTargetInfo) {
        self.update_state(|state| state.is_waiting_for_status = true);
        self.host.restart_timeout();
        self.send_xdnd_position(target, request.position, request.timestamp, request.action);
    }

    fn send_drop_request(&self, drop_request: DropRequest, target: &XdndTargetInfo) {
        self.host.restart_timeout();
        self.host.set_activity_restarts_timeout(true);
        self.send_xdnd_drop(target, drop_request.timestamp);
    }

    fn process_raw_drag_event(
        &self,
        target_window: XID,
        event_type: RawDragEventType,
        root_position: PixelPoint,
        modifiers: RawInputModifiers,
    ) -> DragDropEffects {
        let effective_allowed_effects = self.get_effective_allowed_effects(modifiers.to_key_modifiers());

        self.host.process_raw_drag_event(target_window, event_type, root_position, modifiers, effective_allowed_effects)
            & self.allowed_effects
    }

    fn ungrab_pointer(&self) {
        self.pointer_grabbed.set(false);
        self.host.ungrab_pointer();
    }

    fn update_current_effects(&self, effects: DragDropEffects) {
        if self.current_effects.get() == effects {
            return;
        }

        self.current_effects.set(effects);

        if self.pointer_grabbed.get() {
            self.host.set_grab_cursor(effects);
        }
    }

    fn get_effective_allowed_effects(&self, key_modifiers: KeyModifiers) -> DragDropEffects {
        self.allowed_effects & get_allowed_effects_from_key_modifiers(key_modifiers)
    }

    fn complete(&self, result_effects: DragDropEffects) {
        self.completion_source.try_set_result(result_effects);
    }

    pub(crate) fn on_timeout(&self) {
        self.log(|| {
            if self.state().is_waiting_for_status {
                "Timeout waiting for XdndStatus.".to_string()
            } else {
                "Timeout waiting for XdndFinished.".to_string()
            }
        });

        self.complete(DragDropEffects::NONE);
    }

    pub(crate) fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        if self.pointer_grabbed.get() {
            self.ungrab_pointer();
        }

        self.target_state.set(TargetState::default());
        self.current_effects.set(DragDropEffects::NONE);

        self.host.dispose();
    }
}

impl IEventHook for Handler {
    fn try_handle_event(&self, evt: &XEvent) -> bool {
        if xlib::event_window(evt) == self.source_window && self.try_handle_source_window_event(evt) {
            return true;
        }

        if self.pointer_grabbed.get() && self.try_handle_global_pointer_event(evt) {
            return true;
        }

        false
    }
}

/// The host of a drag on the connection of the platform.
pub(crate) struct XlibDragSourceHost {
    weak_self: Weak<XlibDragSourceHost>,
    platform: Rc<FerroX11Platform>,
    source_window: XID,
    data_transfer: Rc<dyn IDataTransfer>,
    cursor_factory: Option<Rc<X11CursorFactory>>,
    data_provider: DragDropDataProvider,
    timeout_manager: OnceCell<DragDropTimeoutManager>,
    handler: RefCell<Weak<Handler>>,
    hook: RefCell<Option<Rc<dyn IEventHook>>>,
}

impl XlibDragSourceHost {
    pub(crate) fn new(
        platform: &Rc<FerroX11Platform>,
        source_window: XID,
        data_transfer: Rc<dyn IDataTransfer>,
        _allowed_effects: DragDropEffects,
    ) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            platform: platform.clone(),
            source_window,
            data_provider: DragDropDataProvider::new(platform, data_transfer.clone().to_asynchronous()),
            data_transfer,
            cursor_factory: platform.cursor_factory(),
            timeout_manager: OnceCell::new(),
            handler: RefCell::new(Weak::new()),
            hook: RefCell::new(None),
        })
    }

    /// Gives the host its handler and, unless the drag is over already,
    /// installs the handler as the event hook of the dispatcher.
    pub(crate) fn attach(&self, handler: &Rc<Handler>) {
        *self.handler.borrow_mut() = Rc::downgrade(handler);
        if handler.is_completed() {
            return;
        }

        // Install our global event hook.
        let dispatcher_impl = self.platform.dispatcher_impl();
        let event_dispatcher = dispatcher_impl.event_dispatcher();
        debug_assert!(event_dispatcher.event_hook().is_none());
        let hook: Rc<dyn IEventHook> = handler.clone();
        event_dispatcher.set_event_hook(Some(hook.clone()));
        *self.hook.borrow_mut() = Some(hook);
    }

    fn timeout_manager(&self) -> &DragDropTimeoutManager {
        self.timeout_manager.get_or_init(|| {
            let handler = self.handler.borrow().clone();
            DragDropTimeoutManager::new(selection_helper::TIMEOUT, move || {
                if let Some(handler) = handler.upgrade() {
                    handler.on_timeout();
                }
            })
        })
    }

    fn get_cursor(&self, effects: DragDropEffects) -> XID {
        if let Some(cursor_factory) = &self.cursor_factory {
            if effects.contains(DragDropEffects::COPY) {
                return cursor_factory.get_cursor_handle(StandardCursorType::DragCopy);
            }
            if effects.contains(DragDropEffects::MOVE) {
                return cursor_factory.get_cursor_handle(StandardCursorType::DragMove);
            }
            if effects.contains(DragDropEffects::LINK) {
                return cursor_factory.get_cursor_handle(StandardCursorType::DragLink);
            }
            return cursor_factory.drag_no_drop_cursor_handle();
        }

        self.platform.info().default_cursor()
    }
}

impl IDragSourceHost for XlibDragSourceHost {
    fn is_in_process_window(&self, window: XID) -> bool {
        self.platform.get_window(window).is_some_and(|info| info.window().is_some())
    }

    fn get_window_property(&self, window: XID, property: Atom, type_: Atom) -> Option<c_ulong> {
        xlib::x_get_window_property_as_int_ptr(self.platform.display(), window, property, type_)
    }

    fn root_window(&self) -> XID {
        self.platform.info().root_window()
    }

    fn child_at(&self, window: XID, root_position: PixelPoint) -> Option<XID> {
        xlib::x_translate_coordinates(
            self.platform.display(),
            self.platform.info().root_window(),
            window,
            root_position.x,
            root_position.y,
        )
        .map(|(_, _, child)| child)
    }

    fn format_atoms(&self) -> Vec<Atom> {
        data_format_helper::formats_to_atoms(&self.data_transfer.formats(), self.platform.info().atoms())
    }

    fn publish(&self, format_atoms: &[Atom]) {
        let atoms = self.platform.info().atoms();

        self.data_provider.set_owner(self.source_window);

        xlib::x_change_property_longs(
            self.platform.display(),
            self.source_window,
            atoms.XdndTypeList,
            atoms.ATOM,
            PropertyMode::Replace,
            format_atoms,
        );
    }

    fn send_xdnd_message(&self, message_type: Atom, message_window: XID, data: [c_long; 5]) {
        send_xdnd_message(self.platform.display(), message_window, message_type, data);
    }

    fn process_raw_drag_event(
        &self,
        target_window: XID,
        event_type: RawDragEventType,
        root_position: PixelPoint,
        modifiers: RawInputModifiers,
        effective_allowed_effects: DragDropEffects,
    ) -> DragDropEffects {
        let Some(target_window) = self.platform.get_window(target_window).and_then(|info| info.window()) else {
            return DragDropEffects::NONE;
        };
        let Some(drag_drop_device) = target_window.drag_drop_device() else {
            return DragDropEffects::NONE;
        };
        let Some(input_root) = IXdndWindow::input_root(&*target_window) else {
            return DragDropEffects::NONE;
        };

        let local_position = IXdndWindow::point_to_client(&*target_window, root_position);

        let drag_event = RawDragEvent::new(
            drag_drop_device.clone(),
            event_type,
            input_root,
            local_position,
            self.data_transfer.clone(),
            effective_allowed_effects,
            modifiers,
        );

        drag_drop_device.process_raw_event(&drag_event);

        drag_event.effects()
    }

    fn ungrab_pointer(&self) {
        xlib::x_ungrab_pointer(self.platform.display(), 0);
        xlib::x_flush(self.platform.display());
    }

    fn set_grab_cursor(&self, effects: DragDropEffects) {
        xlib::x_change_active_pointer_grab(
            self.platform.display(),
            (EventMask::BUTTON_PRESS_MASK | EventMask::BUTTON_RELEASE_MASK | EventMask::POINTER_MOTION_MASK).bits()
                as _,
            self.get_cursor(effects),
            0,
        );
    }

    fn restart_timeout(&self) {
        self.timeout_manager().restart();
    }

    fn stop_timeout(&self) {
        if let Some(timeout_manager) = self.timeout_manager.get() {
            timeout_manager.stop();
        }
    }

    fn set_activity_restarts_timeout(&self, value: bool) {
        if !value {
            self.data_provider.set_activity(None);
            return;
        }

        let host = self.weak_self.clone();
        let restart: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(host) = host.upgrade() {
                host.timeout_manager().restart();
            }
        });
        self.data_provider.set_activity(Some(restart));
    }

    fn on_selection_request(&self, evt: &XEvent) {
        self.data_provider.on_selection_request(evt);
    }

    fn lookup_keysym(&self, key: &XKeyEvent) -> c_ulong {
        xlib::x_lookup_keysym(key, 0)
    }

    fn atom_name(&self, atom: Atom) -> Option<String> {
        self.platform.info().atoms().get_atom_name(atom)
    }

    fn dispose(&self) {
        let dispatcher_impl = self.platform.dispatcher_impl();
        let event_dispatcher = dispatcher_impl.event_dispatcher();
        if let Some(hook) = self.hook.borrow_mut().take() {
            if event_dispatcher.event_hook().is_some_and(|current| Rc::ptr_eq(&current, &hook)) {
                event_dispatcher.set_event_hook(None);
            }
        }

        if let Some(timeout_manager) = self.timeout_manager.get() {
            timeout_manager.dispose();
        }

        if self.data_provider.get_owner() == self.source_window {
            self.data_provider.set_owner(0);
        }

        self.data_provider.dispose();
    }
}

#[cfg(test)]
mod tests;
