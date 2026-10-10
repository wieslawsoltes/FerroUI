//! The keyboard input of a window (the port of the keyboard part of
//! `X11Window.Ime.cs`): the input context, the key of a key event and the
//! text it produces.
//!
//! The rest of that file, and `X11Window.Xim.cs`, are the input methods of
//! a window (the queue of key events an input method filters, the input
//! method of the server): stage 2 of docs/porting/x11-platform.md.

use crate::keysyms::X11Key;
use crate::x11_enum_extensions::X11EnumExtensions;
use crate::x11_enums::XModifierMask;
use crate::x11_key_transform::X11KeyTransform;
use crate::x11_structs::{XEventName, XIMProperties};
use crate::x11_window::X11Window;
use crate::xlib::{self, XDisplay, XEvent, XKeyEvent, XLookupStatus};
use ferroui_base::input::raw::{IRawInputEventArgs, RawInputEventArgs, RawKeyEventArgs, RawKeyEventType};
use ferroui_base::input::{
    IInputDevice, IInputRoot, Key, KeyDeviceType, KeySymbolHelper, PhysicalKey, RawInputModifiers,
};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

const IME_BUFFER_SIZE: usize = 64 * 1024;

thread_local! {
    /// The buffer text is looked up into (`ImeBuffer`): one per thread,
    /// made when a key first produces text.
    static IME_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// This class is used to attach the text value of the key to an asynchronously dispatched KeyDown event
pub(crate) struct RawKeyEventArgsWithText {
    base: RawKeyEventArgs,
    text: Option<String>,
}

impl RawKeyEventArgsWithText {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        type_: RawKeyEventType,
        key: Key,
        modifiers: RawInputModifiers,
        physical_key: PhysicalKey,
        key_symbol: Option<String>,
        text: Option<String>,
    ) -> Self {
        Self {
            base: RawKeyEventArgs::new(
                device,
                timestamp,
                root,
                type_,
                key,
                modifiers,
                physical_key,
                key_symbol,
                KeyDeviceType::Keyboard,
            ),
            text,
        }
    }

    pub(crate) fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

impl Deref for RawKeyEventArgsWithText {
    type Target = RawKeyEventArgs;

    fn deref(&self) -> &RawKeyEventArgs {
        &self.base
    }
}

impl IRawInputEventArgs for RawKeyEventArgsWithText {
    fn as_raw_input_event_args(&self) -> &RawInputEventArgs {
        self.base.as_raw_input_event_args()
    }

    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<RawKeyEventArgsWithText>() {
            Some(self)
        } else {
            self.base.query_args(type_id)
        }
    }
}

/// The text attached to raw input, when it is a key press with text.
pub(crate) fn text_of(args: &Rc<dyn IRawInputEventArgs>) -> Option<String> {
    args.downcast_ref::<RawKeyEventArgsWithText>().and_then(|args| args.text().map(str::to_string))
}

/// What a key event is, as far as the server and the keyboard mapping say.
type KeyLookup = (X11Key, Key, Option<String>);

/// The symbol of the bytes a key produced: nothing for no bytes and for a
/// single character that is not allowed as a symbol.
pub(crate) fn key_symbol_from_bytes(bytes: &[u8]) -> Option<String> {
    match bytes.len() {
        0 => None,
        1 if !KeySymbolHelper::is_allowed_ascii_key_symbol(bytes[0] as char) => None,
        _ => Some(String::from_utf8_lossy(bytes).into_owned()),
    }
}

/// The text of a key press, when it is text (`TranslateEventToString`,
/// its last part): a single control character or DEL is not.
pub(crate) fn filter_key_text(text: Option<String>) -> Option<String> {
    let text = text?;

    let mut units = text.encode_utf16();
    if let (Some(first), None) = (units.next(), units.next()) {
        if first < ' ' as u16 || first == 0x7f {
            // Control codes or DEL
            return None;
        }
    }

    Some(text)
}

/// Applies the rules that do not depend on the server to a looked up key
/// (`LookupKey`).
pub(crate) fn adjust_looked_up_key(lookup: KeyLookup, physical_key: PhysicalKey) -> KeyLookup {
    let (x11_key, mut key, symbol) = lookup;

    // Always use digits keys if possible, matching Windows/macOS.
    if physical_key >= PhysicalKey::Digit0 && physical_key <= PhysicalKey::Digit9 {
        key = physical_key.to_qwerty_key();
    }

    // No key sym matched a key (e.g. non-latin keyboard without US fallback): fallback to a basic QWERTY map.
    if x11_key.0 != 0 && key == Key::None {
        key = physical_key.to_qwerty_key();
    }

    (x11_key, key, symbol)
}

/// Looks a key up with the keyboard extension (`LookUpKeyXkb`), through
/// two functions of the server: the key symbol of a key code under a
/// state, and the symbol text of a key symbol.
pub(crate) fn look_up_key_xkb(
    key_code: u32,
    state: i32,
    lookup_key_sym: &dyn Fn(u32, i32) -> Option<u64>,
    key_symbol: &dyn Fn(X11Key) -> Option<String>,
) -> KeyLookup {
    // First lookup using the current keyboard layout group (contained in state).
    let Some(original_key_sym) = lookup_key_sym(key_code, state) else {
        return (X11Key(0), Key::None, None);
    };

    let x11_key = X11Key(original_key_sym as i32);
    let symbol = key_symbol(x11_key);

    let key = X11KeyTransform::key_from_x11_key(x11_key);
    if key != Key::None {
        return (x11_key, key, symbol);
    }

    let original_group = xlib::xkb_get_group_for_core_state(state);

    // We got a KeySym that doesn't match a key: try the other groups.
    // This is needed to get a latin key for non-latin keyboard layouts.
    for group in 0..4 {
        if group == original_group {
            continue;
        }

        let new_state = xlib::xkb_set_group_for_core_state(state, group);
        if let Some(group_key_sym) = lookup_key_sym(key_code, new_state) {
            let key = X11KeyTransform::key_from_x11_key(X11Key(group_key_sym as i32));
            if key != Key::None {
                return (x11_key, key, symbol);
            }
        }
    }

    (x11_key, Key::None, None)
}

fn get_key_symbol_xkb(display: XDisplay, x11_key: X11Key) -> Option<String> {
    let bytes = xlib::xkb_translate_key_sym(display, x11_key.0 as u32 as _);
    key_symbol_from_bytes(&bytes)
}

fn lookup_key_x_core(key_event: &mut XKeyEvent) -> KeyLookup {
    const BUFFER_SIZE: usize = 4;

    // We don't have Xkb enabled, which should be rare: use XLookupString which will map to the first keyboard
    // while handling modifiers for us (XKeycodeToKeysym doesn't).
    let (bytes, key_sym) = xlib::x_lookup_string(key_event, BUFFER_SIZE);

    let x11_key = X11Key(key_sym as i32);
    let key = X11KeyTransform::key_from_x11_key(x11_key);

    (x11_key, key, key_symbol_from_bytes(&bytes))
}

impl X11Window {
    /// Creates the input context of the window (`CreateIC`): with the
    /// pre-edit at the caret when the server has an input method that
    /// supports it, otherwise the plain one.
    pub(crate) fn create_ic(&self) {
        let x11 = &self.x11;
        if x11.has_xim() {
            let supported_styles = xlib::x_get_im_supported_styles(x11.xim());
            for style in supported_styles {
                let properties = XIMProperties::from_bits_retain(style as i32);
                if properties.contains(XIMProperties::XIM_PREEDIT_POSITION)
                    && properties.contains(XIMProperties::XIM_STATUS_NOTHING)
                {
                    self.xic.set(xlib::x_create_ic_preedit_position(
                        x11.xim(),
                        self.xid(),
                        style,
                        self.platform().options().wm_class.as_deref(),
                        x11.default_font_set(),
                    ));

                    break;
                }
            }
        }

        if self.xic.get().is_null() {
            self.xic.set(xlib::x_create_ic_simple(
                x11.xim(),
                self.xid(),
                (XIMProperties::XIM_PREEDIT_NOTHING | XIMProperties::XIM_STATUS_NOTHING).bits() as _,
            ));
        }
    }

    pub(crate) fn handle_key_event(&self, ev: &mut XEvent) {
        let key_event = *xlib::key_event(ev);
        let physical_key = X11KeyTransform::physical_key_from_scan_code(key_event.keycode as i32);
        let (x11_key, key, symbol) = self.lookup_key(xlib::key_event_mut(ev), physical_key);
        let modifiers = XModifierMask::from_bits_retain(key_event.state as i32).to_raw_input_modifiers();
        let timestamp = key_event.time as u64;
        let _ = x11_key;

        let args: Rc<dyn IRawInputEventArgs> = if xlib::event_type(ev) == XEventName::KeyPress as i32 {
            let text = self.translate_event_to_string(ev, symbol.clone());
            Rc::new(RawKeyEventArgsWithText::new(
                self.keyboard.clone(),
                timestamp,
                self.input_root(),
                RawKeyEventType::KeyDown,
                key,
                modifiers,
                physical_key,
                symbol,
                text,
            ))
        } else {
            Rc::new(RawKeyEventArgs::new(
                self.keyboard.clone(),
                timestamp,
                self.input_root(),
                RawKeyEventType::KeyUp,
                key,
                modifiers,
                physical_key,
                symbol,
                KeyDeviceType::Keyboard,
            ))
        };

        self.schedule_key_input(args, ev);
    }

    fn lookup_key(&self, key_event: &mut XKeyEvent, physical_key: PhysicalKey) -> KeyLookup {
        let display = self.x11.display();
        let lookup = if self.x11.has_xkb() {
            look_up_key_xkb(
                key_event.keycode,
                key_event.state as i32,
                &|key_code, state| xlib::xkb_lookup_key_sym(display, key_code as u8, state as u32).map(|sym| sym as u64),
                &|x11_key| get_key_symbol_xkb(display, x11_key),
            )
        } else {
            lookup_key_x_core(key_event)
        };

        adjust_looked_up_key(lookup, physical_key)
    }

    fn translate_event_to_string(&self, ev: &mut XEvent, symbol: Option<String>) -> Option<String> {
        let xic = self.xic.get();
        let text = if !self.x11.has_xkb() && xic.is_null() {
            symbol // We already got the symbol from XLookupString, no need to call it again.
        } else if xic.is_null() {
            let (bytes, _) = xlib::x_lookup_string(xlib::key_event_mut(ev), IME_BUFFER_SIZE);
            if bytes.is_empty() {
                return None;
            }
            Some(String::from_utf8_lossy(&bytes).into_owned())
        } else {
            let text = IME_BUFFER.with(|buffer| {
                let mut buffer = buffer.borrow_mut();
                if buffer.is_empty() {
                    buffer.resize(IME_BUFFER_SIZE, 0);
                }
                let (len, status) = xlib::xutf8_lookup_string(xic, xlib::key_event_mut(ev), &mut buffer);
                if len == 0 || status == XLookupStatus::XBufferOverflow.0 {
                    return None;
                }
                Some(String::from_utf8_lossy(&buffer[..len]).into_owned())
            });
            Some(text?)
        };

        filter_key_text(text)
    }

    fn schedule_key_input(&self, args: Rc<dyn IRawInputEventArgs>, xev: &XEvent) {
        // As the reference, which reads the time through the layout of a
        // button event: a key event has it at the same place.
        self.x11.set_last_activity_timestamp(xlib::button_event(xev).time);

        // Stage 2: with an input method that is enabled the event goes to
        // its queue first (`FilterIme`).

        self.schedule_input(args);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn a_symbol_is_text_or_nothing() {
        assert_eq!(key_symbol_from_bytes(b""), None);
        assert_eq!(key_symbol_from_bytes(b"a").as_deref(), Some("a"));
        assert_eq!(key_symbol_from_bytes("ą".as_bytes()).as_deref(), Some("ą"));
        // Backspace, tab, return and escape are symbols; the other control characters and
        // DEL are not.
        assert_eq!(key_symbol_from_bytes(b"\r").as_deref(), Some("\r"));
        assert_eq!(key_symbol_from_bytes(b"\x1b").as_deref(), Some("\u{1b}"));
        assert_eq!(key_symbol_from_bytes(b"\x01"), None);
        assert_eq!(key_symbol_from_bytes(b"\x7f"), None);
    }

    #[test]
    fn control_characters_are_not_text() {
        assert_eq!(filter_key_text(None), None);
        assert_eq!(filter_key_text(Some("a".into())).as_deref(), Some("a"));
        assert_eq!(filter_key_text(Some(" ".into())).as_deref(), Some(" "));
        assert_eq!(filter_key_text(Some("\r".into())), None);
        assert_eq!(filter_key_text(Some("\u{8}".into())), None);
        assert_eq!(filter_key_text(Some("\u{7f}".into())), None);
        // More than one character is text whatever it holds (a composed sequence).
        assert_eq!(filter_key_text(Some("\r\n".into())).as_deref(), Some("\r\n"));
        // One character outside the basic plane is two units long.
        assert_eq!(filter_key_text(Some("\u{1d11e}".into())).as_deref(), Some("\u{1d11e}"));
    }

    #[test]
    fn digit_keys_are_digits_whatever_the_layout_says() {
        // The French layout has "&" on the key of the digit one.
        let lookup = (X11Key::ampersand, X11KeyTransform::key_from_x11_key(X11Key::ampersand), Some("&".to_string()));
        let (x11_key, key, symbol) = adjust_looked_up_key(lookup, PhysicalKey::Digit1);
        assert_eq!(x11_key, X11Key::ampersand);
        assert_eq!(key, Key::D1);
        assert_eq!(symbol.as_deref(), Some("&"));
    }

    #[test]
    fn a_key_without_a_latin_symbol_falls_back_to_the_qwerty_map() {
        // A Cyrillic letter on the key of "A", and no key for it.
        let (_, key, _) = adjust_looked_up_key((X11Key::Cyrillic_ef, Key::None, Some("ф".into())), PhysicalKey::A);
        assert_eq!(key, Key::A);
        // No key symbol at all stays no key.
        let (_, key, _) = adjust_looked_up_key((X11Key(0), Key::None, None), PhysicalKey::A);
        assert_eq!(key, Key::None);
    }

    #[test]
    fn the_other_keyboard_groups_are_tried_for_a_latin_key() {
        // Group 1 (bits 13 and 14 of the state) is a Cyrillic layout, group 0 a Latin one.
        let cyrillic_state = 1 << 13;
        let lookup = |_key_code: u32, state: i32| -> Option<u64> {
            match xlib::xkb_get_group_for_core_state(state) {
                0 => Some(X11Key::a.0 as u64),
                1 => Some(X11Key::Cyrillic_ef.0 as u64),
                _ => None,
            }
        };
        let symbol = |key: X11Key| (key == X11Key::Cyrillic_ef).then(|| "ф".to_string());

        let (x11_key, key, text) = look_up_key_xkb(38, cyrillic_state, &lookup, &symbol);
        assert_eq!(x11_key, X11Key::Cyrillic_ef);
        assert_eq!(key, Key::A);
        assert_eq!(text.as_deref(), Some("ф"));

        // In the Latin group the key is found at once.
        let (x11_key, key, _) = look_up_key_xkb(38, 0, &lookup, &symbol);
        assert_eq!((x11_key, key), (X11Key::a, Key::A));

        // No group has a key: the symbol is dropped with it, as in the reference.
        let no_latin = |_: u32, _: i32| Some(X11Key::Cyrillic_ef.0 as u64);
        let (x11_key, key, text) = look_up_key_xkb(38, cyrillic_state, &no_latin, &symbol);
        assert_eq!((x11_key, key, text), (X11Key::Cyrillic_ef, Key::None, None));

        // A key code without a symbol.
        let nothing = |_: u32, _: i32| None;
        assert_eq!(look_up_key_xkb(38, 0, &nothing, &symbol), (X11Key(0), Key::None, None));
    }

    #[test]
    fn the_group_of_a_state_is_in_bits_thirteen_and_fourteen() {
        assert_eq!(xlib::xkb_get_group_for_core_state(0), 0);
        assert_eq!(xlib::xkb_get_group_for_core_state(1 << 13), 1);
        assert_eq!(xlib::xkb_get_group_for_core_state(3 << 13 | 5), 3);
        assert_eq!(xlib::xkb_set_group_for_core_state(3 << 13 | 5, 2), 2 << 13 | 5);
        assert_eq!(xlib::xkb_set_group_for_core_state(5, 7), 3 << 13 | 5);
    }

}
