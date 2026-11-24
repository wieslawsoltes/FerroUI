//! The keyboard of a seat (the port of `WaylandInputDispatcher.Keyboard.cs`).

use super::wayland_input_dispatcher::{find_surface_for_wl_surface, modifiers_from_xkb_mask, with_seat, InputContext};
use crate::server::persistent::w_surface::WSurfaceId;
use crate::server::wayland_worker::WaylandWorkerState;
use crate::xkb_common_keymap::XkbCommonKeymap;
use crate::xkb_compose_state::{XkbComposeState, XkbComposeStatus};
use crate::xkb_compose_table::XkbComposeTable;
use crate::xkb_context::XkbContext;
use crate::xkb_key_transform::{IXkbKeymap, XkbKeyTransform};
use crate::server::persistent::wayland_key_info::WaylandKeyInfo;
use ferroui_base::input::{Key, KnownPlatformKeyInfoFormats, PlatformKeyInfo, RawInputModifiers};
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
    /// The effective XKB modifier mask (`_effectiveMods`, 12.2.0).
    effective_mods: u32,
    /// The evdev code of the key that is held down (`_heldKey`, 12.2.0).
    held_key: Option<u32>,
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
            effective_mods: 0,
            held_key: None,
        }
    }

    pub(crate) fn dispose(self) {
        self.keyboard.release();
    }

    /// Handles an event. Returns the modifiers of the seat when the event changes them.
    /// Resolves an evdev key code against the current xkb state (`ResolveKey`, 12.2.0). With
    /// `compose`, the key also runs through the compose state (the caller passes it only for a
    /// press on a surface with an active text input client). `None` while a compose sequence is
    /// in progress: the event is suppressed.
    fn resolve_key(&self, key: u32, press: bool, compose: bool) -> Option<WaylandKeyInfo> {
        let physical_key = XkbKeyTransform::physical_key_from_evdev(key);
        let mut platform_key_info = PlatformKeyInfo::new(KnownPlatformKeyInfoFormats::WAYLAND)
            .with_scan_code(key + 8)
            .with_modifiers(u64::from(self.effective_mods));
        let Some(keymap) = &self.keymap else {
            return Some(WaylandKeyInfo {
                key: XkbKeyTransform::key_from_evdev(key),
                physical_key,
                key_symbol: None,
                key_text: None,
                alternate_key_text: None,
                consumed_modifiers: None,
                platform_key_info,
            });
        };

        let resolved = XkbKeyTransform::resolve_key_with_text(keymap, key, physical_key);
        let (mut key_value, mut key_symbol, mut text) = (resolved.key, resolved.key_symbol, resolved.text);
        if let (true, Some(compose_state)) = (compose, &self.compose_state) {
            match compose_state.feed(resolved.keysym) {
                XkbComposeStatus::Composing => return None,
                // Compose sequence complete — use composed text
                XkbComposeStatus::Composed => {
                    text = compose_state.get_composed_text();
                    key_symbol = XkbKeyTransform::filter_key_symbol(text.clone());
                    let composed_key = XkbKeyTransform::key_from_keysym(compose_state.get_composed_keysym());
                    if composed_key != Key::None {
                        key_value = composed_key;
                    }
                }
                // Sequence cancelled or nothing to compose: normal processing of the key.
                XkbComposeStatus::Cancelled | XkbComposeStatus::Nothing => {}
            }
        }

        if resolved.keysym != 0 {
            platform_key_info = platform_key_info.with_virtual_key(resolved.keysym);
        }
        Some(WaylandKeyInfo {
            key: key_value,
            physical_key,
            key_symbol,
            key_text: text.filter(|_| press),
            alternate_key_text: if press { XkbCommonKeymap::keysym_to_text(resolved.keysym) } else { None },
            // Only pressed modifiers count; the GTK mode also reports consumed modifiers that are
            // not active.
            consumed_modifiers: if press {
                keymap.consumed_modifiers(key).map(|mask| modifiers_from_xkb_mask(mask & self.effective_mods))
            } else {
                None
            },
            platform_key_info,
        })
    }

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
                self.effective_mods = 0;
                self.held_key = None;
                if let Some(sink) = self.focused_surface.take().and_then(|surface| cx.sink_of(surface)) {
                    sink.on_keyboard_leave();
                }
                Some(RawInputModifiers::NONE)
            }
            wl_keyboard::Event::Key { time, key, state, .. } => {
                let focused_surface = self.focused_surface?;
                let sink = cx.sink_of(focused_surface)?;
                let pressed = state == WEnum::Value(wl_keyboard::KeyState::Pressed);

                let mods = keyboard_modifiers;
                let has_text_input_client =
                    cx.shell(focused_surface).is_some_and(|shell| shell.surface().has_text_input_client());
                // In the middle of a compose sequence: the whole event is suppressed.
                let info = self.resolve_key(key, pressed, pressed && has_text_input_client)?;

                if pressed {
                    self.held_key = Some(key);
                    sink.on_key_down(u64::from(time), mods, info);
                } else {
                    if self.held_key == Some(key) {
                        self.held_key = None;
                    }
                    sink.on_key_up(u64::from(time), mods, info);
                }
                None
            }
            wl_keyboard::Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } => {
                if let Some(keymap) = &self.keymap {
                    keymap.update_modifiers(mods_depressed, mods_latched, mods_locked, group);
                }
                let effective = mods_depressed | mods_latched | mods_locked;
                let modifiers = modifiers_from_xkb_mask(effective);
                self.effective_mods = effective;

                // The held key resolves to other values now: the repeat uses them.
                if let Some(held_key) = self.held_key {
                    let sink = self.focused_surface.and_then(|surface| cx.sink_of(surface));
                    if let (Some(sink), Some(info)) = (sink, self.resolve_key(held_key, true, false)) {
                        sink.on_held_key_changed(modifiers, info);
                    }
                }
                Some(modifiers)
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
            if let wl_keyboard::Event::Enter { serial, .. } | wl_keyboard::Event::Key { serial, .. } = &event {
                seat.note_input_serial(*serial);
            }
            let keyboard_modifiers = seat.keyboard_modifiers;
            if let Some(handler) = &mut seat.keyboard_handler {
                if let Some(modifiers) = handler.on_event(event, keyboard_modifiers, cx) {
                    seat.keyboard_modifiers = modifiers;
                }
            }
        });
    }
}
