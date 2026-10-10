//! The keyboard of a seat (the port of `WaylandInputDispatcher.Keyboard.cs`).

use super::wayland_input_dispatcher::{find_surface_for_wl_surface, modifiers_from_xkb_mask, with_seat, InputContext};
use crate::server::persistent::w_surface::WSurfaceId;
use crate::server::wayland_worker::WaylandWorkerState;
use crate::xkb_common_keymap::XkbCommonKeymap;
use crate::xkb_compose_state::{XkbComposeState, XkbComposeStatus};
use crate::xkb_compose_table::XkbComposeTable;
use crate::xkb_context::XkbContext;
use crate::xkb_key_transform::{IXkbKeymap, XkbKeyTransform};
use ferroui_base::input::{Key, RawInputModifiers};
use wayland_client::protocol::wl_keyboard::{self, WlKeyboard};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

pub(crate) struct KeyboardHandler {
    keyboard: WlKeyboard,
    // The order of the fields is the order they are dropped in: the states before the keymap
    // and the table, those before the context.
    compose_state: Option<XkbComposeState>,
    keymap: Option<XkbCommonKeymap>,
    compose_table: Option<XkbComposeTable>,
    xkb_context: Option<XkbContext>,
    focused_surface: Option<WSurfaceId>,
    repeat_rate: i32,  // keys/sec (0 = disabled)
    repeat_delay: i32, // ms before first repeat
}

impl KeyboardHandler {
    pub(crate) fn new(wl_seat: &WlSeat, seat_name: u32, queue_handle: &QueueHandle<WaylandWorkerState>) -> Self {
        // Without the library there is no context and no keymap: keys are then translated
        // from their codes alone.
        let xkb_context = XkbContext::new();
        let compose_table = xkb_context.as_ref().and_then(XkbComposeTable::try_create);
        Self {
            keyboard: wl_seat.get_keyboard(queue_handle, seat_name),
            compose_state: None,
            keymap: None,
            compose_table,
            xkb_context,
            focused_surface: None,
            repeat_rate: 0,
            repeat_delay: 0,
        }
    }

    pub(crate) fn dispose(self) {
        self.keyboard.release();
    }

    /// Handles an event. Returns the modifiers of the seat when the event changes them.
    fn on_event(
        &mut self,
        event: wl_keyboard::Event,
        keyboard_modifiers: RawInputModifiers,
        cx: &InputContext<'_>,
    ) -> Option<RawInputModifiers> {
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                self.compose_state = None;
                self.keymap = None;

                if format == WEnum::Value(wl_keyboard::KeymapFormat::XkbV1) {
                    // If keymap creation fails, fall back to evdev-only mapping
                    if let Some(context) = &self.xkb_context {
                        if let Ok(keymap) = XkbCommonKeymap::new(context, fd, size) {
                            self.keymap = Some(keymap);
                            self.compose_state = self.compose_table.as_ref().and_then(XkbComposeState::new);
                        }
                    }
                }
                // Otherwise the descriptor is closed when it is dropped.
                None
            }
            wl_keyboard::Event::Enter { surface, .. } => {
                self.focused_surface = find_surface_for_wl_surface(&surface);
                if let Some(sink) = self.focused_surface.and_then(|surface| cx.sink_of(surface)) {
                    sink.on_key_repeat_info(self.repeat_rate, self.repeat_delay);
                }
                None
            }
            wl_keyboard::Event::Leave { .. } => {
                if let Some(compose_state) = &self.compose_state {
                    compose_state.reset();
                }
                if let Some(sink) = self.focused_surface.take().and_then(|surface| cx.sink_of(surface)) {
                    sink.on_keyboard_leave();
                }
                Some(RawInputModifiers::NONE)
            }
            wl_keyboard::Event::Key { time, key, state, .. } => {
                let focused_surface = self.focused_surface?;
                let sink = cx.sink_of(focused_surface)?;
                let pressed = state == WEnum::Value(wl_keyboard::KeyState::Pressed);

                let physical_key = XkbKeyTransform::physical_key_from_evdev(key);
                let mods = keyboard_modifiers;

                let (mut key_value, mut key_symbol) = match &self.keymap {
                    Some(keymap) => XkbKeyTransform::resolve_key_with_fallback(keymap, key, physical_key),
                    None => (XkbKeyTransform::key_from_evdev(key), None),
                };

                // Process compose sequences on key press only, and only when the
                // focused surface has an active text input client (e.g. a TextBox)
                let has_text_input_client =
                    cx.shell(focused_surface).is_some_and(|shell| shell.surface().has_text_input_client());
                if let (true, true, Some(keymap), Some(compose_state)) =
                    (pressed, has_text_input_client, &self.keymap, &self.compose_state)
                {
                    let (resolved_keysym, _) = keymap.resolve_key(key);
                    match compose_state.feed(resolved_keysym) {
                        // In the middle of a compose sequence — suppress entire event
                        XkbComposeStatus::Composing => return None,
                        // Compose sequence complete — use composed text
                        XkbComposeStatus::Composed => {
                            key_symbol = XkbKeyTransform::filter_key_symbol(compose_state.get_composed_text());
                            let composed_key = XkbKeyTransform::key_from_keysym(compose_state.get_composed_keysym());
                            if composed_key != Key::None {
                                key_value = composed_key;
                            }
                        }
                        // Sequence cancelled — normal processing of current key
                        XkbComposeStatus::Cancelled => {}
                        // Normal processing
                        XkbComposeStatus::Nothing => {}
                    }
                }

                if pressed {
                    sink.on_key_down(u64::from(time), key_value, mods, physical_key, key_symbol);
                } else {
                    sink.on_key_up(u64::from(time), key_value, mods, physical_key, key_symbol);
                }
                None
            }
            wl_keyboard::Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } => {
                if let Some(keymap) = &self.keymap {
                    keymap.update_modifiers(mods_depressed, mods_latched, mods_locked, group);
                }
                let effective = mods_depressed | mods_latched | mods_locked;
                Some(modifiers_from_xkb_mask(effective))
            }
            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                self.repeat_rate = rate;
                self.repeat_delay = delay;
                if let Some(sink) = self.focused_surface.and_then(|surface| cx.sink_of(surface)) {
                    sink.on_key_repeat_info(rate, delay);
                }
                None
            }
            _ => None,
        }
    }
}

impl Dispatch<WlKeyboard, u32> for WaylandWorkerState {
    fn event(
        state: &mut Self,
        _proxy: &WlKeyboard,
        event: wl_keyboard::Event,
        data: &u32,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        with_seat(state, *data, |seat, cx| {
            let keyboard_modifiers = seat.keyboard_modifiers;
            if let Some(handler) = &mut seat.keyboard_handler {
                if let Some(modifiers) = handler.on_event(event, keyboard_modifiers, cx) {
                    seat.keyboard_modifiers = modifiers;
                }
            }
        });
    }
}
