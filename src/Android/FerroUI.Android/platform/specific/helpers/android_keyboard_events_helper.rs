//! Key events of the system to raw key and text events of the framework.
//!
//! The reference reads a `KeyEvent`; here the Java view copies what the
//! reference reads of it ([`KeyEventData`]), as it does for motion events,
//! and this helper translates the copy.

use super::android_key_interop::AndroidKeyInterop;
use crate::platform::input::android_keyboard_device::AndroidKeyboardDevice;
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawTextInputEventArgs};
use ferroui_base::input::{
    IInputDevice, IInputRoot, IKeyboardDevice, KeyDeviceType, KeySymbolHelper, PhysicalKey, RawInputModifiers,
};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// `KeyEvent.ACTION_*`.
pub(crate) mod key_event_actions {
    pub const DOWN: i32 = 0;
    pub const UP: i32 = 1;
    pub const MULTIPLE: i32 = 2;
}

/// `InputDevice.SOURCE_*`.
pub(crate) mod input_source_type {
    pub const DPAD: i32 = 0x0000_0201;
    pub const GAMEPAD: i32 = 0x0000_0401;
    pub const JOYSTICK: i32 = 0x0100_0010;
}

/// `InputDevice.KEYBOARD_TYPE_NON_ALPHABETIC`.
pub(crate) const KEYBOARD_TYPE_NON_ALPHABETIC: i32 = 1;

/// The device of a key event, as far as the reference reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyEventDevice {
    /// `InputDevice.getSources`.
    pub sources: i32,
    /// `InputDevice.getKeyboardType`.
    pub keyboard_type: i32,
}

/// The copy of a `KeyEvent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyEventData {
    /// `getEventTime`: milliseconds of the uptime clock.
    pub event_time: i64,
    pub action: i32,
    pub key_code: i32,
    pub scan_code: i32,
    /// `getUnicodeChar`: the character the key produces with the meta state
    /// of the event, or zero.
    pub unicode_char: i32,
    pub repeat_count: i32,
    pub is_ctrl_pressed: bool,
    pub is_shift_pressed: bool,
    /// `getCharacters`, which the system fills below API 29 only.
    pub characters: Option<String>,
    /// `getDevice`; `None` when the event has no device.
    pub device: Option<KeyEventDevice>,
}

/// What the helper needs of the top-level it serves.
pub(crate) trait IKeyboardEventsTopLevel {
    /// The input root the events are for; `None` before the top-level has
    /// one.
    fn key_input_root(&self) -> Option<Rc<dyn IInputRoot>>;

    /// Delivers a raw input event (`Input?.Invoke`).
    fn dispatch_key_input(&self, args: Rc<dyn IRawInputEventArgs>);
}

pub(crate) struct AndroidKeyboardEventsHelper {
    view: Weak<dyn IKeyboardEventsTopLevel>,
    /// The API level of the system (`OperatingSystem.IsAndroidVersionAtLeast`).
    api_level: i32,
    /// The keyboard device the events name (`AndroidKeyboardDevice.Instance`).
    device: fn() -> Option<Rc<dyn IKeyboardDevice>>,
    handle_events: Cell<bool>,
}

impl AndroidKeyboardEventsHelper {
    pub fn new(view: Weak<dyn IKeyboardEventsTopLevel>, api_level: i32) -> Self {
        Self::with_device(view, api_level, AndroidKeyboardDevice::instance)
    }

    pub(crate) fn with_device(
        view: Weak<dyn IKeyboardEventsTopLevel>,
        api_level: i32,
        device: fn() -> Option<Rc<dyn IKeyboardDevice>>,
    ) -> Self {
        Self { view, api_level, device, handle_events: Cell::new(true) }
    }

    pub fn handle_events(&self) -> bool {
        self.handle_events.get()
    }

    pub fn set_handle_events(&self, value: bool) {
        self.handle_events.set(value);
    }

    /// Translates a key event. The result is `None` when the event was not
    /// looked at; `call_base` says whether the base class dispatches the
    /// event too.
    pub fn dispatch_key_event(&self, e: Option<&KeyEventData>, call_base: &mut bool) -> Option<bool> {
        let Some(e) = e.filter(|_| self.handle_events.get()) else {
            *call_base = true;
            return None;
        };

        self.dispatch_key_event_internal(e, call_base)
    }

    fn unicode_text_input(key_event: &KeyEventData) -> Option<&str> {
        if key_event.action == key_event_actions::MULTIPLE && key_event.repeat_count == 0 {
            key_event.characters.as_deref().filter(|characters| !characters.is_empty())
        } else {
            None
        }
    }

    fn dispatch_key_event_internal(&self, e: &KeyEventData, call_base: &mut bool) -> Option<bool> {
        let unicode_text_input = if self.api_level >= 29 { None } else { Self::unicode_text_input(e) };
        let view = self.view.upgrade();
        let input_root = view.as_ref().and_then(|view| view.key_input_root());

        let (Some(view), Some(input_root)) = (view, input_root) else {
            *call_base = true;
            return None;
        };
        if e.action == key_event_actions::MULTIPLE && unicode_text_input.is_none() {
            *call_base = true;
            return None;
        }

        let physical_key = AndroidKeyInterop::physical_key_from_scan_code(e.scan_code);
        let key_symbol = Self::get_key_symbol(e.unicode_char, physical_key);
        let key_device_type = Self::get_key_device_type(e);
        let device: Rc<dyn IInputDevice> = match (self.device)() {
            Some(device) => device,
            None => panic!("The keyboard device of the platform is not registered. Make sure use_android() was executed."),
        };
        // `Convert.ToUInt64` of the reference fails for a negative time; the uptime clock
        // has none.
        let timestamp = u64::try_from(e.event_time).unwrap_or(0);

        let raw_key_event = Rc::new(RawKeyEventArgs::new(
            device.clone(),
            timestamp,
            input_root.clone(),
            if e.action == key_event_actions::DOWN { RawKeyEventType::KeyDown } else { RawKeyEventType::KeyUp },
            AndroidKeyboardDevice::convert_key(e.key_code),
            Self::get_modifier_keys(e),
            physical_key,
            key_symbol,
            key_device_type,
        ));

        view.dispatch_key_input(raw_key_event.clone());

        let mut handled = raw_key_event.handled();

        if (e.action == key_event_actions::DOWN && e.unicode_char >= 32) || unicode_text_input.is_some() {
            let text = match unicode_text_input {
                Some(text) => text.to_string(),
                None => Self::char_to_string(e.unicode_char),
            };
            let raw_text_event = Rc::new(RawTextInputEventArgs::new(device, timestamp, input_root, text));
            view.dispatch_key_input(raw_text_event.clone());

            handled = handled || raw_text_event.handled();
        }

        if e.action == key_event_actions::UP {
            //nothing to do here more call base no need of more events
            *call_base = true;
            return None;
        }

        *call_base = false;
        Some(handled)
    }

    pub(crate) fn get_modifier_keys(e: &KeyEventData) -> RawInputModifiers {
        let mut rv = RawInputModifiers::NONE;

        if e.is_ctrl_pressed {
            rv |= RawInputModifiers::CONTROL;
        }
        if e.is_shift_pressed {
            rv |= RawInputModifiers::SHIFT;
        }

        rv
    }

    /// The text of a code point (`char.ConvertFromUtf32`, which fails for a
    /// number that is no code point: the system gives none).
    fn char_to_string(unicode_char: i32) -> String {
        u32::try_from(unicode_char).ok().and_then(char::from_u32).map(String::from).unwrap_or_default()
    }

    pub(crate) fn get_key_symbol(unicode_char: i32, physical_key: PhysicalKey) -> Option<String> {
        // Handle a very limited set of control characters so that we're consistent with other platforms
        // (matches KeySymbolHelper.IsAllowedAsciiKeySymbol)
        match physical_key {
            PhysicalKey::Backspace => Some("\u{8}".to_string()),
            PhysicalKey::Tab => Some("\t".to_string()),
            PhysicalKey::Enter | PhysicalKey::NumPadEnter => Some("\r".to_string()),
            PhysicalKey::Escape => Some("\u{1b}".to_string()),
            _ => {
                if unicode_char <= 0x7f {
                    // The reference converts the number to a UTF-16 unit: a negative number
                    // (a combining accent of the key character map has the highest bit set)
                    // is above the range of ASCII there and is not a symbol.
                    let ascii_char = u8::try_from(unicode_char).ok().map(char::from)?;
                    return KeySymbolHelper::is_allowed_ascii_key_symbol(ascii_char).then(|| ascii_char.to_string());
                }
                Some(Self::char_to_string(unicode_char))
            }
        }
    }

    pub(crate) fn get_key_device_type(e: &KeyEventData) -> KeyDeviceType {
        let source = e.device.map_or(0, |device| device.sources);

        // Remote controller reports itself as "DPad | Keyboard", which is confusing,
        // so we need to double-check KeyboardType as well.

        if source & input_source_type::DPAD != 0
            && e.device.is_some_and(|device| device.keyboard_type == KEYBOARD_TYPE_NON_ALPHABETIC)
        {
            return KeyDeviceType::Remote;
        }

        // "Any bit in common", as the reference tests it (`HasAnyFlag`). Both sources contain
        // the bit of a source class (buttons, joystick), so this is true for every device
        // with keys, a keyboard included: kept as the reference has it.
        if source & (input_source_type::JOYSTICK | input_source_type::GAMEPAD) != 0 {
            return KeyDeviceType::Gamepad;
        }

        KeyDeviceType::Keyboard // fallback to the keyboard, if unknown.
    }

    pub fn dispose(&self) {
        self.handle_events.set(false);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the helper.
    use super::*;
    use crate::platform::input::android_keyboard_device::keycode;
    use ferroui_base::input::{FocusManager, InputElement, Key, KeyboardDevice};
    use ferroui_base::Ref;
    use std::cell::RefCell;

    struct TestRoot {
        element: Ref<InputElement>,
    }

    impl IInputRoot for TestRoot {
        fn focus_manager(&self) -> Option<Rc<FocusManager>> {
            None
        }

        fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
            None
        }

        fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}

        fn cursor_element(&self) -> Option<Ref<InputElement>> {
            None
        }

        fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}

        fn root_element(&self) -> Ref<InputElement> {
            self.element.clone()
        }

        fn focus_root(&self) -> Ref<InputElement> {
            self.element.clone()
        }

        fn pointer_over_invalidated(&self) {}
    }

    #[derive(Debug, PartialEq)]
    enum Seen {
        Key {
            type_: RawKeyEventType,
            key: Key,
            modifiers: RawInputModifiers,
            physical_key: PhysicalKey,
            symbol: Option<String>,
            device_type: KeyDeviceType,
            timestamp: u64,
        },
        Text(String),
    }

    struct TopLevel {
        root: RefCell<Option<Rc<dyn IInputRoot>>>,
        seen: RefCell<Vec<Seen>>,
        /// Whether the top-level marks what it is given as handled.
        handle: Cell<bool>,
    }

    impl IKeyboardEventsTopLevel for TopLevel {
        fn key_input_root(&self) -> Option<Rc<dyn IInputRoot>> {
            self.root.borrow().clone()
        }

        fn dispatch_key_input(&self, args: Rc<dyn IRawInputEventArgs>) {
            if let Some(key) = args.downcast_ref::<RawKeyEventArgs>() {
                self.seen.borrow_mut().push(Seen::Key {
                    type_: key.type_(),
                    key: key.key(),
                    modifiers: key.modifiers(),
                    physical_key: key.physical_key(),
                    symbol: key.key_symbol(),
                    device_type: key.key_device_type(),
                    timestamp: key.timestamp(),
                });
                key.set_handled(self.handle.get());
            } else if let Some(text) = args.downcast_ref::<RawTextInputEventArgs>() {
                self.seen.borrow_mut().push(Seen::Text(text.text().to_string()));
            } else {
                panic!("a key or a text event");
            }
        }
    }

    fn test_device() -> Option<Rc<dyn IKeyboardDevice>> {
        thread_local! {
            static DEVICE: Rc<KeyboardDevice> = KeyboardDevice::new();
        }
        Some(DEVICE.with(|device| device.clone()))
    }

    fn fixture(with_root: bool, api_level: i32) -> (Rc<TopLevel>, AndroidKeyboardEventsHelper) {
        let root: Option<Rc<dyn IInputRoot>> =
            with_root.then(|| Rc::new(TestRoot { element: InputElement::new() }) as Rc<dyn IInputRoot>);
        let top_level =
            Rc::new(TopLevel { root: RefCell::new(root), seen: RefCell::new(Vec::new()), handle: Cell::new(false) });
        let as_trait: Rc<dyn IKeyboardEventsTopLevel> = top_level.clone();
        (top_level, AndroidKeyboardEventsHelper::with_device(Rc::downgrade(&as_trait), api_level, test_device))
    }

    fn event(action: i32, key_code: i32, scan_code: i32, unicode_char: i32) -> KeyEventData {
        KeyEventData {
            event_time: 4321,
            action,
            key_code,
            scan_code,
            unicode_char,
            repeat_count: 0,
            is_ctrl_pressed: false,
            is_shift_pressed: false,
            characters: None,
            device: None,
        }
    }

    #[test]
    fn an_event_is_not_looked_at_without_an_event_after_dispose_or_before_the_root() {
        let (top_level, helper) = fixture(true, 36);
        let mut call_base = false;
        assert_eq!(helper.dispatch_key_event(None, &mut call_base), None);
        assert!(call_base);

        let (_, without_root) = fixture(false, 36);
        let mut call_base = false;
        let down = event(key_event_actions::DOWN, keycode::A, 30, 'a' as i32);
        assert_eq!(without_root.dispatch_key_event(Some(&down), &mut call_base), None);
        assert!(call_base);

        helper.dispose();
        assert!(!helper.handle_events());
        let mut call_base = false;
        assert_eq!(helper.dispatch_key_event(Some(&down), &mut call_base), None);
        assert!(call_base);
        assert!(top_level.seen.borrow().is_empty());
    }

    #[test]
    fn a_letter_goes_down_as_a_key_and_its_text_and_up_as_a_key() {
        let (top_level, helper) = fixture(true, 36);
        let mut call_base = true;
        let mut down = event(key_event_actions::DOWN, keycode::A, 30, 'A' as i32);
        down.is_shift_pressed = true;
        assert_eq!(helper.dispatch_key_event(Some(&down), &mut call_base), Some(false));
        assert!(!call_base);

        let up = event(key_event_actions::UP, keycode::A, 30, 'a' as i32);
        assert_eq!(helper.dispatch_key_event(Some(&up), &mut call_base), None);
        assert!(call_base);

        assert_eq!(
            *top_level.seen.borrow(),
            vec![
                Seen::Key {
                    type_: RawKeyEventType::KeyDown,
                    key: Key::A,
                    modifiers: RawInputModifiers::SHIFT,
                    physical_key: PhysicalKey::A,
                    symbol: Some("A".to_string()),
                    device_type: KeyDeviceType::Keyboard,
                    timestamp: 4321,
                },
                Seen::Text("A".to_string()),
                Seen::Key {
                    type_: RawKeyEventType::KeyUp,
                    key: Key::A,
                    modifiers: RawInputModifiers::NONE,
                    physical_key: PhysicalKey::A,
                    symbol: Some("a".to_string()),
                    device_type: KeyDeviceType::Keyboard,
                    timestamp: 4321,
                },
            ]
        );
    }

    #[test]
    fn a_handled_key_down_is_answered_as_handled() {
        let (top_level, helper) = fixture(true, 36);
        top_level.handle.set(true);
        let mut call_base = true;
        let down = event(key_event_actions::DOWN, keycode::DPAD_LEFT, 105, 0);
        assert_eq!(helper.dispatch_key_event(Some(&down), &mut call_base), Some(true));
        assert!(!call_base);
        // No character: no text event.
        assert_eq!(top_level.seen.borrow().len(), 1);
    }

    #[test]
    fn a_key_without_a_character_raises_no_text_and_enter_has_its_symbol() {
        let (top_level, helper) = fixture(true, 36);
        let mut call_base = true;
        // Enter produces a line feed (10), which is below 32: no text event.
        let down = event(key_event_actions::DOWN, keycode::ENTER, 28, 10);
        helper.dispatch_key_event(Some(&down), &mut call_base);
        assert_eq!(
            *top_level.seen.borrow(),
            vec![Seen::Key {
                type_: RawKeyEventType::KeyDown,
                key: Key::Enter,
                modifiers: RawInputModifiers::NONE,
                physical_key: PhysicalKey::Enter,
                symbol: Some("\r".to_string()),
                device_type: KeyDeviceType::Keyboard,
                timestamp: 4321,
            }]
        );
    }

    #[test]
    fn several_characters_are_text_below_api_29_and_nothing_from_there() {
        let mut multiple = event(key_event_actions::MULTIPLE, 0, 0, 0);
        multiple.characters = Some("zażółć".to_string());

        let (top_level, helper) = fixture(true, 28);
        let mut call_base = true;
        assert_eq!(helper.dispatch_key_event(Some(&multiple), &mut call_base), Some(false));
        assert!(!call_base);
        assert_eq!(top_level.seen.borrow().last(), Some(&Seen::Text("zażółć".to_string())));

        let (top_level, helper) = fixture(true, 29);
        let mut call_base = false;
        assert_eq!(helper.dispatch_key_event(Some(&multiple), &mut call_base), None);
        assert!(call_base);
        assert!(top_level.seen.borrow().is_empty());

        // A repeated event of several characters is not text.
        multiple.repeat_count = 2;
        let (top_level, helper) = fixture(true, 28);
        assert_eq!(helper.dispatch_key_event(Some(&multiple), &mut call_base), None);
        assert!(top_level.seen.borrow().is_empty());
    }

    #[test]
    fn the_key_symbol_is_the_character_unless_it_is_a_control_character() {
        let symbol = AndroidKeyboardEventsHelper::get_key_symbol;
        assert_eq!(symbol(0, PhysicalKey::Backspace), Some("\u{8}".to_string()));
        assert_eq!(symbol(9, PhysicalKey::Tab), Some("\t".to_string()));
        assert_eq!(symbol(10, PhysicalKey::NumPadEnter), Some("\r".to_string()));
        assert_eq!(symbol(0, PhysicalKey::Escape), Some("\u{1b}".to_string()));
        assert_eq!(symbol(0, PhysicalKey::ShiftLeft), None);
        assert_eq!(symbol('q' as i32, PhysicalKey::Q), Some("q".to_string()));
        assert_eq!(symbol(0x105, PhysicalKey::A), Some("ą".to_string()));
        assert_eq!(symbol(0x1f600, PhysicalKey::None), Some("\u{1f600}".to_string()));
        // A dead key: the combining accent flag of the key character map.
        assert_eq!(symbol(i32::MIN | 0x60, PhysicalKey::Backquote), None);
    }

    #[test]
    fn the_device_type_follows_the_sources_and_the_keyboard_type() {
        let mut e = event(key_event_actions::DOWN, keycode::DPAD_CENTER, 0, 0);
        assert_eq!(AndroidKeyboardEventsHelper::get_key_device_type(&e), KeyDeviceType::Keyboard);

        // SOURCE_KEYBOARD | SOURCE_DPAD of a remote control.
        e.device = Some(KeyEventDevice { sources: 0x101 | input_source_type::DPAD, keyboard_type: 1 });
        assert_eq!(AndroidKeyboardEventsHelper::get_key_device_type(&e), KeyDeviceType::Remote);

        // As the reference has it: its test is "any bit in common", and the sources of
        // every device with keys share the bit of the class of buttons with the sources of
        // a joystick or a gamepad, so an alphabetic keyboard (SOURCE_KEYBOARD) is a gamepad.
        e.device = Some(KeyEventDevice { sources: 0x101, keyboard_type: 2 });
        assert_eq!(AndroidKeyboardEventsHelper::get_key_device_type(&e), KeyDeviceType::Gamepad);

        // A device that is no source of keys at all (SOURCE_TOUCHSCREEN).
        e.device = Some(KeyEventDevice { sources: 0x1002, keyboard_type: 0 });
        assert_eq!(AndroidKeyboardEventsHelper::get_key_device_type(&e), KeyDeviceType::Keyboard);

        // A gamepad shares that bit with a directional pad too: with a keyboard that is not
        // alphabetic it is a remote control, in the reference as here.
        e.device = Some(KeyEventDevice { sources: input_source_type::GAMEPAD, keyboard_type: 1 });
        assert_eq!(AndroidKeyboardEventsHelper::get_key_device_type(&e), KeyDeviceType::Remote);
        e.device = Some(KeyEventDevice { sources: input_source_type::GAMEPAD, keyboard_type: 0 });
        assert_eq!(AndroidKeyboardEventsHelper::get_key_device_type(&e), KeyDeviceType::Gamepad);

        e.device = Some(KeyEventDevice { sources: input_source_type::JOYSTICK, keyboard_type: 0 });
        assert_eq!(AndroidKeyboardEventsHelper::get_key_device_type(&e), KeyDeviceType::Gamepad);
    }

    #[test]
    fn control_and_shift_are_the_modifiers() {
        let mut e = event(key_event_actions::DOWN, keycode::C, 46, 0);
        e.is_ctrl_pressed = true;
        e.is_shift_pressed = true;
        assert_eq!(
            AndroidKeyboardEventsHelper::get_modifier_keys(&e),
            RawInputModifiers::CONTROL | RawInputModifiers::SHIFT
        );
    }
}
