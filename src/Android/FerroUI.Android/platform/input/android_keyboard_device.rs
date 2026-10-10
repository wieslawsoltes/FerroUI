//! The keyboard device of the Android platform and the key codes of the system as keys of the framework.
//!
//! The reference derives its device from the keyboard device of the framework and adds nothing to it but the
//! table of the key codes, so here [`AndroidKeyboardDevice::new`] gives the keyboard device of the framework
//! itself. The reference takes a `Keycode` of the binding of the system; here the key code is the number
//! `KeyEvent.getKeyCode()` returns, which the Java view passes as it is.

use ferroui_base::input::{IKeyboardDevice, Key, KeyboardDevice};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// `KeyEvent.KEYCODE_*`: the key codes the table below knows.
pub(crate) mod keycode {
    pub const BACK: i32 = 4;
    pub const NUM_0: i32 = 7;
    pub const NUM_1: i32 = 8;
    pub const NUM_2: i32 = 9;
    pub const NUM_3: i32 = 10;
    pub const NUM_4: i32 = 11;
    pub const NUM_5: i32 = 12;
    pub const NUM_6: i32 = 13;
    pub const NUM_7: i32 = 14;
    pub const NUM_8: i32 = 15;
    pub const NUM_9: i32 = 16;
    pub const DPAD_UP: i32 = 19;
    pub const DPAD_DOWN: i32 = 20;
    pub const DPAD_LEFT: i32 = 21;
    pub const DPAD_RIGHT: i32 = 22;
    pub const DPAD_CENTER: i32 = 23;
    pub const VOLUME_UP: i32 = 24;
    pub const VOLUME_DOWN: i32 = 25;
    pub const CLEAR: i32 = 28;
    pub const A: i32 = 29;
    pub const B: i32 = 30;
    pub const C: i32 = 31;
    pub const D: i32 = 32;
    pub const E: i32 = 33;
    pub const F: i32 = 34;
    pub const G: i32 = 35;
    pub const H: i32 = 36;
    pub const I: i32 = 37;
    pub const J: i32 = 38;
    pub const K: i32 = 39;
    pub const L: i32 = 40;
    pub const M: i32 = 41;
    pub const N: i32 = 42;
    pub const O: i32 = 43;
    pub const P: i32 = 44;
    pub const Q: i32 = 45;
    pub const R: i32 = 46;
    pub const S: i32 = 47;
    pub const T: i32 = 48;
    pub const U: i32 = 49;
    pub const V: i32 = 50;
    pub const W: i32 = 51;
    pub const X: i32 = 52;
    pub const Y: i32 = 53;
    pub const Z: i32 = 54;
    pub const COMMA: i32 = 55;
    pub const PERIOD: i32 = 56;
    pub const ALT_LEFT: i32 = 57;
    pub const ALT_RIGHT: i32 = 58;
    pub const SHIFT_LEFT: i32 = 59;
    pub const SHIFT_RIGHT: i32 = 60;
    pub const TAB: i32 = 61;
    pub const SPACE: i32 = 62;
    pub const ENTER: i32 = 66;
    pub const DEL: i32 = 67;
    pub const GRAVE: i32 = 68;
    pub const MINUS: i32 = 69;
    pub const LEFT_BRACKET: i32 = 71;
    pub const RIGHT_BRACKET: i32 = 72;
    pub const BACKSLASH: i32 = 73;
    pub const SEMICOLON: i32 = 74;
    pub const APOSTROPHE: i32 = 75;
    pub const SLASH: i32 = 76;
    pub const PLUS: i32 = 81;
    pub const MEDIA_PLAY_PAUSE: i32 = 85;
    pub const MEDIA_STOP: i32 = 86;
    pub const MEDIA_NEXT: i32 = 87;
    pub const MEDIA_PREVIOUS: i32 = 88;
    pub const PAGE_UP: i32 = 92;
    pub const PAGE_DOWN: i32 = 93;
    pub const ESCAPE: i32 = 111;
    pub const FORWARD_DEL: i32 = 112;
    pub const CTRL_LEFT: i32 = 113;
    pub const CTRL_RIGHT: i32 = 114;
    pub const CAPS_LOCK: i32 = 115;
    pub const SCROLL_LOCK: i32 = 116;
    pub const MOVE_HOME: i32 = 122;
    pub const MOVE_END: i32 = 123;
    pub const INSERT: i32 = 124;
    pub const MEDIA_PLAY: i32 = 126;
    pub const MEDIA_PAUSE: i32 = 127;
    pub const F1: i32 = 131;
    pub const F2: i32 = 132;
    pub const F3: i32 = 133;
    pub const F4: i32 = 134;
    pub const F5: i32 = 135;
    pub const F6: i32 = 136;
    pub const F7: i32 = 137;
    pub const F8: i32 = 138;
    pub const F9: i32 = 139;
    pub const F10: i32 = 140;
    pub const F11: i32 = 141;
    pub const F12: i32 = 142;
    pub const NUM_LOCK: i32 = 143;
    pub const NUMPAD_0: i32 = 144;
    pub const NUMPAD_1: i32 = 145;
    pub const NUMPAD_2: i32 = 146;
    pub const NUMPAD_3: i32 = 147;
    pub const NUMPAD_4: i32 = 148;
    pub const NUMPAD_5: i32 = 149;
    pub const NUMPAD_6: i32 = 150;
    pub const NUMPAD_7: i32 = 151;
    pub const NUMPAD_8: i32 = 152;
    pub const NUMPAD_9: i32 = 153;
    pub const NUMPAD_DIVIDE: i32 = 154;
    pub const NUMPAD_MULTIPLY: i32 = 155;
    pub const NUMPAD_SUBTRACT: i32 = 156;
    pub const NUMPAD_ADD: i32 = 157;
    pub const NUMPAD_DOT: i32 = 158;
    pub const NUMPAD_COMMA: i32 = 159;
    pub const SLEEP: i32 = 223;
    pub const HELP: i32 = 259;
}

/// The keyboard device of the Android platform.
pub struct AndroidKeyboardDevice;

impl AndroidKeyboardDevice {
    /// The keyboard device that is registered with the locator, if there is one.
    pub fn instance() -> Option<Rc<dyn IKeyboardDevice>> {
        FerroLocator::current().get_service::<dyn IKeyboardDevice>()
    }

    /// A keyboard device: the one of the framework, to which the reference adds nothing.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Rc<dyn IKeyboardDevice> {
        KeyboardDevice::new()
    }

    /// The key of the framework for a key code of the system (`KeyEvent.KEYCODE_*`), [`Key::None`] for a key
    /// code the table does not have.
    pub fn convert_key(key: i32) -> Key {
        match key {
            //   keycode::CANCEL? => Key::Cancel,
            keycode::DEL => Key::Back,
            keycode::TAB => Key::Tab,
            //  keycode::LINEFEED? => Key::LineFeed,
            keycode::CLEAR => Key::Clear,
            keycode::ENTER => Key::Return,
            keycode::MEDIA_PAUSE => Key::Pause,
            keycode::CAPS_LOCK => Key::CapsLock,
            // ? => Key::HangulMode
            // ? => Key::JunjaMode
            // ? => Key::FinalMode
            // ? => Key::KanjiMode
            keycode::ESCAPE => Key::Escape,
            // ? => Key::ImeConvert
            // ? => Key::ImeNonConvert
            // ? => Key::ImeAccept
            // ? => Key::ImeModeChange
            keycode::SPACE => Key::Space,
            keycode::PAGE_UP => Key::Prior,
            keycode::PAGE_DOWN => Key::PageDown,
            keycode::MOVE_END => Key::End,
            keycode::MOVE_HOME => Key::Home,
            // keycode::BUTTON_SELECT? => Key::Select,
            // keycode::print? => Key::Print,
            // keycode::execute? => Key::Execute,
            // keycode::snap? => Key::Snapshot
            keycode::INSERT => Key::Insert,
            keycode::FORWARD_DEL => Key::Delete,
            keycode::HELP => Key::Help,
            keycode::NUM_0 => Key::D0,
            keycode::NUM_1 => Key::D1,
            keycode::NUM_2 => Key::D2,
            keycode::NUM_3 => Key::D3,
            keycode::NUM_4 => Key::D4,
            keycode::NUM_5 => Key::D5,
            keycode::NUM_6 => Key::D6,
            keycode::NUM_7 => Key::D7,
            keycode::NUM_8 => Key::D8,
            keycode::NUM_9 => Key::D9,
            keycode::A => Key::A,
            keycode::B => Key::B,
            keycode::C => Key::C,
            keycode::D => Key::D,
            keycode::E => Key::E,
            keycode::F => Key::F,
            keycode::G => Key::G,
            keycode::H => Key::H,
            keycode::I => Key::I,
            keycode::J => Key::J,
            keycode::K => Key::K,
            keycode::L => Key::L,
            keycode::M => Key::M,
            keycode::N => Key::N,
            keycode::O => Key::O,
            keycode::P => Key::P,
            keycode::Q => Key::Q,
            keycode::R => Key::R,
            keycode::S => Key::S,
            keycode::T => Key::T,
            keycode::U => Key::U,
            keycode::V => Key::V,
            keycode::W => Key::W,
            keycode::X => Key::X,
            keycode::Y => Key::Y,
            keycode::Z => Key::Z,
            // keycode::a => Key::A,
            // keycode::b => Key::B,
            // keycode::c => Key::C,
            // keycode::d => Key::D,
            // keycode::e => Key::E,
            // keycode::f => Key::F,
            // keycode::g => Key::G,
            // keycode::h => Key::H,
            // keycode::i => Key::I,
            // keycode::j => Key::J,
            // keycode::k => Key::K,
            // keycode::l => Key::L,
            // keycode::m => Key::M,
            // keycode::n => Key::N,
            // keycode::o => Key::O,
            // keycode::p => Key::P,
            // keycode::q => Key::Q,
            // keycode::r => Key::R,
            // keycode::s => Key::S,
            // keycode::t => Key::T,
            // keycode::u => Key::U,
            // keycode::v => Key::V,
            // keycode::w => Key::W,
            // keycode::x => Key::X,
            // keycode::y => Key::Y,
            // keycode::z => Key::Z,
            // ? => Key::LWin
            // ? => Key::RWin
            // ? => Key::Apps
            keycode::SLEEP => Key::Sleep,
            keycode::NUMPAD_0 => Key::NumPad0,
            keycode::NUMPAD_1 => Key::NumPad1,
            keycode::NUMPAD_2 => Key::NumPad2,
            keycode::NUMPAD_3 => Key::NumPad3,
            keycode::NUMPAD_4 => Key::NumPad4,
            keycode::NUMPAD_5 => Key::NumPad5,
            keycode::NUMPAD_6 => Key::NumPad6,
            keycode::NUMPAD_7 => Key::NumPad7,
            keycode::NUMPAD_8 => Key::NumPad8,
            keycode::NUMPAD_9 => Key::NumPad9,
            keycode::NUMPAD_MULTIPLY => Key::Multiply,
            keycode::NUMPAD_ADD => Key::Add,
            keycode::NUMPAD_COMMA => Key::Separator,
            keycode::NUMPAD_SUBTRACT => Key::Subtract,
            keycode::NUMPAD_DOT => Key::Decimal,
            keycode::NUMPAD_DIVIDE => Key::Divide,
            keycode::F1 => Key::F1,
            keycode::F2 => Key::F2,
            keycode::F3 => Key::F3,
            keycode::F4 => Key::F4,
            keycode::F5 => Key::F5,
            keycode::F6 => Key::F6,
            keycode::F7 => Key::F7,
            keycode::F8 => Key::F8,
            keycode::F9 => Key::F9,
            keycode::F10 => Key::F10,
            keycode::F11 => Key::F11,
            keycode::F12 => Key::F12,
            // keycode::f13 => Key::F13,
            // keycode::F14 => Key::F14,
            // keycode::L5 => Key::F15,
            // keycode::F16 => Key::F16,
            // keycode::F17 => Key::F17,
            // keycode::L8 => Key::F18,
            // keycode::L9 => Key::F19,
            // keycode::L10 => Key::F20,
            // keycode::R1 => Key::F21,
            // keycode::R2 => Key::F22,
            // keycode::F23 => Key::F23,
            // keycode::R4 => Key::F24,
            keycode::NUM_LOCK => Key::NumLock,
            keycode::SCROLL_LOCK => Key::Scroll,
            keycode::SHIFT_LEFT => Key::LeftShift,
            keycode::SHIFT_RIGHT => Key::RightShift,
            keycode::CTRL_LEFT => Key::LeftCtrl,
            keycode::CTRL_RIGHT => Key::RightCtrl,
            keycode::ALT_LEFT => Key::LeftAlt,
            keycode::ALT_RIGHT => Key::RightAlt,
            // ? => Key::BrowserBack
            // ? => Key::BrowserForward
            // ? => Key::BrowserRefresh
            // ? => Key::BrowserStop
            // ? => Key::BrowserSearch
            // ? => Key::BrowserFavorites
            // ? => Key::BrowserHome
            // ? => Key::VolumeMute
            keycode::VOLUME_DOWN => Key::VolumeDown,
            keycode::VOLUME_UP => Key::VolumeUp,
            keycode::MEDIA_NEXT => Key::MediaNextTrack,
            keycode::MEDIA_PREVIOUS => Key::MediaPreviousTrack,
            keycode::MEDIA_STOP => Key::MediaStop,
            keycode::MEDIA_PLAY_PAUSE => Key::MediaPlayPause,
            // ? => Key::LaunchMail
            // ? => Key::SelectMedia
            // ? => Key::LaunchApplication1
            // ? => Key::LaunchApplication2
            keycode::SEMICOLON => Key::OemSemicolon,
            keycode::PLUS => Key::OemPlus,
            keycode::COMMA => Key::OemComma,
            keycode::MINUS => Key::OemMinus,
            keycode::PERIOD => Key::OemPeriod,
            // ? => Key::Oem2
            keycode::GRAVE => Key::OemTilde,
            // ? => Key::AbntC1
            // ? => Key::AbntC2
            // ? => Key::OemPipe
            keycode::APOSTROPHE => Key::OemQuotes,
            keycode::SLASH => Key::OemQuestion,
            keycode::LEFT_BRACKET => Key::OemOpenBrackets,
            keycode::RIGHT_BRACKET => Key::OemCloseBrackets,
            // ? => Key::Oem7
            // ? => Key::Oem8
            // ? => Key::Oem102
            // ? => Key::ImeProcessed
            // ? => Key::System
            // ? => Key::OemAttn
            // ? => Key::OemFinish
            // ? => Key::DbeHiragana
            // ? => Key::OemAuto
            // ? => Key::DbeDbcsChar
            // ? => Key::OemBackTab
            // ? => Key::Attn
            // ? => Key::DbeEnterWordRegisterMode
            // ? => Key::DbeEnterImeConfigureMode
            // ? => Key::EraseEof
            keycode::MEDIA_PLAY => Key::Play,
            // ? => Key::Zoom
            // ? => Key::NoName
            // ? => Key::DbeEnterDialogConversionMode
            // ? => Key::OemClear
            // ? => Key::DeadCharProcessed
            keycode::BACKSLASH => Key::OemBackslash,

            // Loosely mapping DPad keys to the keys of the framework
            keycode::BACK => Key::Escape,
            keycode::DPAD_CENTER => Key::Space,
            keycode::DPAD_LEFT => Key::Left,
            keycode::DPAD_UP => Key::Up,
            keycode::DPAD_RIGHT => Key::Right,
            keycode::DPAD_DOWN => Key::Down,
            _ => Key::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_are_the_letter_keys() {
        let letters = [
            Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L, Key::M,
            Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X, Key::Y, Key::Z,
        ];
        // KEYCODE_A is 29 and the letters follow it in the order of the alphabet.
        for (index, letter) in letters.iter().enumerate() {
            assert_eq!(AndroidKeyboardDevice::convert_key(29 + index as i32), *letter);
        }
    }

    #[test]
    fn digits_are_the_digit_keys() {
        let digits = [Key::D0, Key::D1, Key::D2, Key::D3, Key::D4, Key::D5, Key::D6, Key::D7, Key::D8, Key::D9];
        // KEYCODE_0 is 7, KEYCODE_NUMPAD_0 is 144.
        for (index, digit) in digits.iter().enumerate() {
            assert_eq!(AndroidKeyboardDevice::convert_key(7 + index as i32), *digit);
        }
        assert_eq!(AndroidKeyboardDevice::convert_key(144), Key::NumPad0);
        assert_eq!(AndroidKeyboardDevice::convert_key(153), Key::NumPad9);
    }

    #[test]
    fn named_keys() {
        assert_eq!(AndroidKeyboardDevice::convert_key(66), Key::Return);
        assert_eq!(AndroidKeyboardDevice::convert_key(66), Key::Enter);
        assert_eq!(AndroidKeyboardDevice::convert_key(67), Key::Back);
        assert_eq!(AndroidKeyboardDevice::convert_key(112), Key::Delete);
        assert_eq!(AndroidKeyboardDevice::convert_key(111), Key::Escape);
        assert_eq!(AndroidKeyboardDevice::convert_key(61), Key::Tab);
        assert_eq!(AndroidKeyboardDevice::convert_key(62), Key::Space);
        assert_eq!(AndroidKeyboardDevice::convert_key(92), Key::PageUp);
        assert_eq!(AndroidKeyboardDevice::convert_key(93), Key::PageDown);
        assert_eq!(AndroidKeyboardDevice::convert_key(122), Key::Home);
        assert_eq!(AndroidKeyboardDevice::convert_key(123), Key::End);
        assert_eq!(AndroidKeyboardDevice::convert_key(131), Key::F1);
        assert_eq!(AndroidKeyboardDevice::convert_key(142), Key::F12);
        assert_eq!(AndroidKeyboardDevice::convert_key(59), Key::LeftShift);
        assert_eq!(AndroidKeyboardDevice::convert_key(114), Key::RightCtrl);
    }

    #[test]
    fn the_back_key_and_the_directional_pad() {
        assert_eq!(AndroidKeyboardDevice::convert_key(4), Key::Escape);
        assert_eq!(AndroidKeyboardDevice::convert_key(23), Key::Space);
        assert_eq!(AndroidKeyboardDevice::convert_key(21), Key::Left);
        assert_eq!(AndroidKeyboardDevice::convert_key(19), Key::Up);
        assert_eq!(AndroidKeyboardDevice::convert_key(22), Key::Right);
        assert_eq!(AndroidKeyboardDevice::convert_key(20), Key::Down);
    }

    #[test]
    fn an_unknown_key_code_is_no_key() {
        // KEYCODE_UNKNOWN, KEYCODE_HOME (which the table does not have), and numbers that are no key code.
        assert_eq!(AndroidKeyboardDevice::convert_key(0), Key::None);
        assert_eq!(AndroidKeyboardDevice::convert_key(3), Key::None);
        assert_eq!(AndroidKeyboardDevice::convert_key(-1), Key::None);
        assert_eq!(AndroidKeyboardDevice::convert_key(100_000), Key::None);
    }
}
