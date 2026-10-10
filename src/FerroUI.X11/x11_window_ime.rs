//! The keyboard input of a window (the port of `X11Window.Ime.cs`): the
//! input context, the key of a key event and the text it produces, the
//! input method of the window and the queue of key events an enabled
//! input method is offered before the application gets them.
//!
//! The input method of the server is in `x11_window_xim.rs`; the input
//! methods over D-Bus are in the crate `ferroui-freedesktop`.

use crate::keysyms::X11Key;
use crate::x11_enum_extensions::X11EnumExtensions;
use crate::x11_enums::XModifierMask;
use crate::x11_key_transform::X11KeyTransform;
use crate::x11_structs::{XEventName, XIMProperties};
use crate::x11_window::X11Window;
use crate::x11_window_xim::XimInputMethod;
use crate::xlib::{self, XDisplay, XEvent, XKeyEvent, XLookupStatus};
use ferroui_base::input::raw::{
    IRawInputEventArgs, RawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawTextInputEventArgs,
};
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::input::{
    IInputDevice, IInputRoot, Key, KeyDeviceType, KeySymbolHelper, PhysicalKey, RawInputModifiers,
};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::ops::Deref;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_freedesktop::{IX11InputMethodControl, IX11InputMethodFactory, X11InputMethodForwardedKey};
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

        self.schedule_key_input(args, ev, x11_key.0, key_event.keycode as i32);
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

    fn schedule_key_input(&self, args: Rc<dyn IRawInputEventArgs>, xev: &XEvent, keyval: i32, keycode: i32) {
        // As the reference, which reads the time through the layout of a
        // button event: a key event has it at the same place.
        self.x11.set_last_activity_timestamp(xlib::button_event(xev).time);

        let enabled = self.ime_control.borrow().as_ref().is_some_and(|ime_control| ime_control.is_enabled());
        if enabled && self.filter_ime(args.clone(), keyval, keycode) {
            return;
        }

        self.schedule_input(args);
    }

    /// Makes the input method of the window (`InitializeIme`): the one
    /// over D-Bus the platform registered a factory for, else the one of
    /// the server when the platform opened it.
    pub(crate) fn initialize_ime(&self) {
        let factory = FerroLocator::current().get_service::<dyn IX11InputMethodFactory>();
        let mut ime: Option<(Rc<dyn ITextInputMethodImpl>, Rc<dyn IX11InputMethodControl>)> =
            factory.map(|factory| factory.create_client(self.xid() as usize));

        if ime.is_none() && self.x11.has_xim() {
            let xim = XimInputMethod::new(self.this.clone());
            ime = Some((xim.clone(), xim));
        }

        if let Some((ime, ime_control)) = ime {
            {
                let weak = self.this.clone();
                ime_control.commit().subscribe(move |s| {
                    if let Some(window) = weak.upgrade() {
                        let Some(input_root) = window.input_root_or_none() else {
                            return;
                        };
                        window.schedule_input(Rc::new(RawTextInputEventArgs::new(
                            window.keyboard.clone(),
                            window.x11.last_activity_timestamp() as u64,
                            input_root,
                            s,
                        )));
                    }
                });
            }
            {
                let weak = self.this.clone();
                ime_control.forward_key().subscribe(move |forwarded_key| {
                    if let Some(window) = weak.upgrade() {
                        window.on_ime_control_forward_key(forwarded_key);
                    }
                });
            }
            *self.ime.borrow_mut() = Some(ime);
            *self.ime_control.borrow_mut() = Some(ime_control);
        }
    }

    fn on_ime_control_forward_key(&self, forwarded_key: X11InputMethodForwardedKey) {
        let Some(input_root) = self.input_root_or_none() else {
            return;
        };
        let x11_key = X11Key(forwarded_key.key_val);
        let key_symbol = if self.x11.has_xkb() {
            get_key_symbol_xkb(self.x11.display(), x11_key)
        } else {
            get_key_symbol_x_core(x11_key)
        };
        let timestamp = self.x11.last_activity_timestamp() as u64;

        self.schedule_input(forwarded_key_args(
            forwarded_key,
            key_symbol,
            self.keyboard.clone(),
            timestamp,
            input_root,
        ));
    }

    /// Queues a key event for the input method (`FilterIme`); false when
    /// the window has none.
    fn filter_ime(&self, args: Rc<dyn IRawInputEventArgs>, keyval: i32, keycode: i32) -> bool {
        if self.ime.borrow().is_none() {
            return false;
        }
        self.ime_queue.borrow_mut().push_back(QueuedImeKey { args, keyval, keycode });
        if !self.processing_ime.get() {
            self.process_next_ime_event();
        }
        true
    }

    /// Offers the queued key events to the input method, one after the
    /// other, and passes on those it does not consume
    /// (`ProcessNextImeEvent`, an `async void` of the reference: a task of
    /// the UI dispatcher here).
    fn process_next_ime_event(&self) {
        if self.processing_ime.get() {
            return;
        }
        let Some(window) = self.this.clone().upgrade() else {
            return;
        };
        self.processing_ime.set(true);

        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            // The `finally` of the reference.
            struct ResetProcessing(Rc<X11Window>);
            impl Drop for ResetProcessing {
                fn drop(&mut self) {
                    self.0.processing_ime.set(false);
                }
            }
            let guard = ResetProcessing(window);
            let window = &guard.0;

            loop {
                let Some(ev) = window.ime_queue.borrow_mut().pop_front() else {
                    break;
                };
                let ime_control = window.ime_control.borrow().clone();
                if let Some(ime_control) = ime_control {
                    let handled_by_ime = ime_control.handle_event_async(ev.args.clone(), ev.keyval, ev.keycode).await;
                    if handled_by_ime && !passes_although_handled(&ev.args) {
                        continue;
                    }
                }

                window.schedule_input(ev.args);
            }
        }));
    }
}

/// A key event that waits for the input method (an element of
/// `_imeQueue`; the X event the reference keeps with it is never read).
pub(crate) struct QueuedImeKey {
    pub(crate) args: Rc<dyn IRawInputEventArgs>,
    pub(crate) keyval: i32,
    pub(crate) keycode: i32,
}

/// Whether a key event goes on to the application although the input
/// method consumed it.
pub(crate) fn passes_although_handled(args: &Rc<dyn IRawInputEventArgs>) -> bool {
    let Some(key) = args.downcast_ref::<RawKeyEventArgs>() else {
        return false;
    };

    // We let filtered modifier-key KeyUp events through
    // since some apps rely on the order of events to track individual (left/right)
    // modifier keys states rather than relying on general key modifiers
    key.type_() == RawKeyEventType::KeyUp
        && matches!(
            key.key(),
            Key::LeftCtrl
                | Key::RightCtrl
                | Key::LeftAlt
                | Key::RightAlt
                | Key::LeftShift
                | Key::RightShift
                | Key::LWin
                | Key::RWin
        )
}

/// The raw input of a key an input method forwards
/// (`OnImeControlForwardKey`): a key event without a physical key, with
/// the symbol as its text when the input method says the key has text.
pub(crate) fn forwarded_key_args(
    forwarded_key: X11InputMethodForwardedKey,
    key_symbol: Option<String>,
    keyboard: Rc<dyn IInputDevice>,
    timestamp: u64,
    input_root: Rc<dyn IInputRoot>,
) -> Rc<dyn IRawInputEventArgs> {
    let x11_key = X11Key(forwarded_key.key_val);
    let key = X11KeyTransform::key_from_x11_key(x11_key);
    let modifiers = RawInputModifiers::from_bits_retain(forwarded_key.modifiers.bits());

    if forwarded_key.with_text {
        Rc::new(RawKeyEventArgsWithText::new(
            keyboard,
            timestamp,
            input_root,
            forwarded_key.type_,
            key,
            modifiers,
            PhysicalKey::None,
            key_symbol.clone(),
            key_symbol,
        ))
    } else {
        Rc::new(RawKeyEventArgs::new(
            keyboard,
            timestamp,
            input_root,
            forwarded_key.type_,
            key,
            modifiers,
            PhysicalKey::None,
            key_symbol,
            KeyDeviceType::Keyboard,
        ))
    }
}

fn get_key_symbol_x_core(x11_key: X11Key) -> Option<String> {
    let bytes = xlib::x_keysym_to_string(x11_key.0 as u32 as _)?;
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
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

    struct TestDevice;

    impl IInputDevice for TestDevice {
        fn process_raw_event(&self, _ev: &dyn IRawInputEventArgs) {}

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct TestRoot;

    impl IInputRoot for TestRoot {
        fn focus_manager(&self) -> Option<Rc<ferroui_base::input::FocusManager>> {
            None
        }

        fn pointer_over_element(&self) -> Option<ferroui_base::Ref<ferroui_base::input::InputElement>> {
            None
        }

        fn set_pointer_over_element(&self, _value: Option<ferroui_base::Ref<ferroui_base::input::InputElement>>) {}

        fn cursor_element(&self) -> Option<ferroui_base::Ref<ferroui_base::input::InputElement>> {
            None
        }

        fn set_cursor_element(&self, _value: Option<ferroui_base::Ref<ferroui_base::input::InputElement>>) {}

        fn root_element(&self) -> ferroui_base::Ref<ferroui_base::input::InputElement> {
            unreachable!("key events do not ask for the root element")
        }

        fn focus_root(&self) -> ferroui_base::Ref<ferroui_base::input::InputElement> {
            unreachable!("key events do not ask for the focus root")
        }

        fn pointer_over_invalidated(&self) {}
    }

    fn key_args(type_: RawKeyEventType, key: Key) -> Rc<dyn IRawInputEventArgs> {
        Rc::new(RawKeyEventArgs::new(
            Rc::new(TestDevice),
            1,
            Rc::new(TestRoot),
            type_,
            key,
            RawInputModifiers::empty(),
            PhysicalKey::None,
            None,
            KeyDeviceType::Keyboard,
        ))
    }

    #[test]
    fn the_release_of_a_modifier_key_passes_although_the_input_method_consumed_it() {
        for key in [
            Key::LeftCtrl,
            Key::RightCtrl,
            Key::LeftAlt,
            Key::RightAlt,
            Key::LeftShift,
            Key::RightShift,
            Key::LWin,
            Key::RWin,
        ] {
            assert!(passes_although_handled(&key_args(RawKeyEventType::KeyUp, key)), "{key:?}");
            // Its press does not.
            assert!(!passes_although_handled(&key_args(RawKeyEventType::KeyDown, key)), "{key:?}");
        }
        assert!(!passes_although_handled(&key_args(RawKeyEventType::KeyUp, Key::A)));
        assert!(!passes_although_handled(&key_args(RawKeyEventType::KeyDown, Key::A)));
        // A press with text is a key event like the others.
        let with_text: Rc<dyn IRawInputEventArgs> = Rc::new(RawKeyEventArgsWithText::new(
            Rc::new(TestDevice),
            1,
            Rc::new(TestRoot),
            RawKeyEventType::KeyUp,
            Key::LeftShift,
            RawInputModifiers::empty(),
            PhysicalKey::None,
            None,
            None,
        ));
        assert!(passes_although_handled(&with_text));
    }

    #[test]
    fn a_forwarded_key_is_a_key_event_without_a_physical_key() {
        use ferroui_base::input::KeyModifiers;

        // The return key, released, with modifiers: no text.
        let forwarded = X11InputMethodForwardedKey {
            key_val: 0xff0d,
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            type_: RawKeyEventType::KeyUp,
            with_text: false,
        };
        let args = forwarded_key_args(forwarded, None, Rc::new(TestDevice), 77, Rc::new(TestRoot));
        let key = args.downcast_ref::<RawKeyEventArgs>().unwrap();
        assert_eq!(key.key(), Key::Enter);
        assert_eq!(key.type_(), RawKeyEventType::KeyUp);
        assert_eq!(key.modifiers(), RawInputModifiers::CONTROL | RawInputModifiers::SHIFT);
        assert_eq!(key.physical_key(), PhysicalKey::None);
        assert_eq!(args.timestamp(), 77);
        assert_eq!(text_of(&args), None);

        // A press with text: the symbol of the key is its text.
        let forwarded = X11InputMethodForwardedKey {
            key_val: 0x61,
            modifiers: KeyModifiers::empty(),
            type_: RawKeyEventType::KeyDown,
            with_text: true,
        };
        let args =
            forwarded_key_args(forwarded, Some("a".to_string()), Rc::new(TestDevice), 78, Rc::new(TestRoot));
        let key = args.downcast_ref::<RawKeyEventArgs>().unwrap();
        assert_eq!(key.key(), Key::A);
        assert_eq!(key.key_symbol().as_deref(), Some("a"));
        assert_eq!(text_of(&args).as_deref(), Some("a"));
    }
}
