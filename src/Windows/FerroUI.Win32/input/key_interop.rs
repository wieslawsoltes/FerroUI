//! Translates the virtual keys and scan codes of Windows to the keys of the
//! toolkit.
//!
//! The tables and the arithmetic on the key data of a message are plain data
//! and are compiled on every host; the members that ask the system (the
//! mapping of a scan code to a virtual key, the symbol of a key in the
//! current layout) exist on Windows only. The members with a `_with` suffix
//! take the system call as a function, which is how the rest is tested
//! without a system.

use crate::interop::unmanaged_methods::{MapVirtualKeyMapTypes, VirtualKeyStates};
use ferroui_base::input::{Key, KeySymbolHelper, PhysicalKey};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Contains methods used to translate a Windows virtual/physical key to a
/// [`Key`] of the toolkit.
pub struct KeyInterop;

// source: https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
static VIRTUAL_KEY_FROM_KEY: &[(Key, i32)] = &[
    (Key::Cancel, VirtualKeyStates::VK_CANCEL),
    (Key::Back, VirtualKeyStates::VK_BACK),
    (Key::Tab, VirtualKeyStates::VK_TAB),
    (Key::Clear, VirtualKeyStates::VK_CLEAR),
    (Key::Return, VirtualKeyStates::VK_RETURN),
    (Key::Pause, VirtualKeyStates::VK_PAUSE),
    (Key::Capital, VirtualKeyStates::VK_CAPITAL),
    (Key::KanaMode, VirtualKeyStates::VK_KANA),
    (Key::JunjaMode, VirtualKeyStates::VK_JUNJA),
    (Key::FinalMode, VirtualKeyStates::VK_FINAL),
    (Key::HanjaMode, VirtualKeyStates::VK_HANJA),
    (Key::Escape, VirtualKeyStates::VK_ESCAPE),
    (Key::ImeConvert, VirtualKeyStates::VK_CONVERT),
    (Key::ImeNonConvert, VirtualKeyStates::VK_NONCONVERT),
    (Key::ImeAccept, VirtualKeyStates::VK_ACCEPT),
    (Key::ImeModeChange, VirtualKeyStates::VK_MODECHANGE),
    (Key::Space, VirtualKeyStates::VK_SPACE),
    (Key::PageUp, VirtualKeyStates::VK_PRIOR),
    (Key::PageDown, VirtualKeyStates::VK_NEXT),
    (Key::End, VirtualKeyStates::VK_END),
    (Key::Home, VirtualKeyStates::VK_HOME),
    (Key::Left, VirtualKeyStates::VK_LEFT),
    (Key::Up, VirtualKeyStates::VK_UP),
    (Key::Right, VirtualKeyStates::VK_RIGHT),
    (Key::Down, VirtualKeyStates::VK_DOWN),
    (Key::Select, VirtualKeyStates::VK_SELECT),
    (Key::Print, VirtualKeyStates::VK_PRINT),
    (Key::Execute, VirtualKeyStates::VK_EXECUTE),
    (Key::Snapshot, VirtualKeyStates::VK_SNAPSHOT),
    (Key::Insert, VirtualKeyStates::VK_INSERT),
    (Key::Delete, VirtualKeyStates::VK_DELETE),
    (Key::Help, VirtualKeyStates::VK_HELP),
    (Key::D0, '0' as i32),
    (Key::D1, '1' as i32),
    (Key::D2, '2' as i32),
    (Key::D3, '3' as i32),
    (Key::D4, '4' as i32),
    (Key::D5, '5' as i32),
    (Key::D6, '6' as i32),
    (Key::D7, '7' as i32),
    (Key::D8, '8' as i32),
    (Key::D9, '9' as i32),
    (Key::A, 'A' as i32),
    (Key::B, 'B' as i32),
    (Key::C, 'C' as i32),
    (Key::D, 'D' as i32),
    (Key::E, 'E' as i32),
    (Key::F, 'F' as i32),
    (Key::G, 'G' as i32),
    (Key::H, 'H' as i32),
    (Key::I, 'I' as i32),
    (Key::J, 'J' as i32),
    (Key::K, 'K' as i32),
    (Key::L, 'L' as i32),
    (Key::M, 'M' as i32),
    (Key::N, 'N' as i32),
    (Key::O, 'O' as i32),
    (Key::P, 'P' as i32),
    (Key::Q, 'Q' as i32),
    (Key::R, 'R' as i32),
    (Key::S, 'S' as i32),
    (Key::T, 'T' as i32),
    (Key::U, 'U' as i32),
    (Key::V, 'V' as i32),
    (Key::W, 'W' as i32),
    (Key::X, 'X' as i32),
    (Key::Y, 'Y' as i32),
    (Key::Z, 'Z' as i32),
    (Key::LWin, VirtualKeyStates::VK_LWIN),
    (Key::RWin, VirtualKeyStates::VK_RWIN),
    (Key::Apps, VirtualKeyStates::VK_APPS),
    (Key::Sleep, VirtualKeyStates::VK_SLEEP),
    (Key::NumPad0, VirtualKeyStates::VK_NUMPAD0),
    (Key::NumPad1, VirtualKeyStates::VK_NUMPAD1),
    (Key::NumPad2, VirtualKeyStates::VK_NUMPAD2),
    (Key::NumPad3, VirtualKeyStates::VK_NUMPAD3),
    (Key::NumPad4, VirtualKeyStates::VK_NUMPAD4),
    (Key::NumPad5, VirtualKeyStates::VK_NUMPAD5),
    (Key::NumPad6, VirtualKeyStates::VK_NUMPAD6),
    (Key::NumPad7, VirtualKeyStates::VK_NUMPAD7),
    (Key::NumPad8, VirtualKeyStates::VK_NUMPAD8),
    (Key::NumPad9, VirtualKeyStates::VK_NUMPAD9),
    (Key::Multiply, VirtualKeyStates::VK_MULTIPLY),
    (Key::Add, VirtualKeyStates::VK_ADD),
    (Key::Separator, VirtualKeyStates::VK_SEPARATOR),
    (Key::Subtract, VirtualKeyStates::VK_SUBTRACT),
    (Key::Decimal, VirtualKeyStates::VK_DECIMAL),
    (Key::Divide, VirtualKeyStates::VK_DIVIDE),
    (Key::F1, VirtualKeyStates::VK_F1),
    (Key::F2, VirtualKeyStates::VK_F2),
    (Key::F3, VirtualKeyStates::VK_F3),
    (Key::F4, VirtualKeyStates::VK_F4),
    (Key::F5, VirtualKeyStates::VK_F5),
    (Key::F6, VirtualKeyStates::VK_F6),
    (Key::F7, VirtualKeyStates::VK_F7),
    (Key::F8, VirtualKeyStates::VK_F8),
    (Key::F9, VirtualKeyStates::VK_F9),
    (Key::F10, VirtualKeyStates::VK_F10),
    (Key::F11, VirtualKeyStates::VK_F11),
    (Key::F12, VirtualKeyStates::VK_F12),
    (Key::F13, VirtualKeyStates::VK_F13),
    (Key::F14, VirtualKeyStates::VK_F14),
    (Key::F15, VirtualKeyStates::VK_F15),
    (Key::F16, VirtualKeyStates::VK_F16),
    (Key::F17, VirtualKeyStates::VK_F17),
    (Key::F18, VirtualKeyStates::VK_F18),
    (Key::F19, VirtualKeyStates::VK_F19),
    (Key::F20, VirtualKeyStates::VK_F20),
    (Key::F21, VirtualKeyStates::VK_F21),
    (Key::F22, VirtualKeyStates::VK_F22),
    (Key::F23, VirtualKeyStates::VK_F23),
    (Key::F24, VirtualKeyStates::VK_F24),
    (Key::NumLock, VirtualKeyStates::VK_NUMLOCK),
    (Key::Scroll, VirtualKeyStates::VK_SCROLL),
    (Key::LeftShift, VirtualKeyStates::VK_LSHIFT),
    (Key::RightShift, VirtualKeyStates::VK_RSHIFT),
    (Key::LeftCtrl, VirtualKeyStates::VK_LCONTROL),
    (Key::RightCtrl, VirtualKeyStates::VK_RCONTROL),
    (Key::LeftAlt, VirtualKeyStates::VK_LMENU),
    (Key::RightAlt, VirtualKeyStates::VK_RMENU),
    (Key::BrowserBack, VirtualKeyStates::VK_BROWSER_BACK),
    (Key::BrowserForward, VirtualKeyStates::VK_BROWSER_FORWARD),
    (Key::BrowserRefresh, VirtualKeyStates::VK_BROWSER_REFRESH),
    (Key::BrowserStop, VirtualKeyStates::VK_BROWSER_STOP),
    (Key::BrowserSearch, VirtualKeyStates::VK_BROWSER_SEARCH),
    (Key::BrowserFavorites, VirtualKeyStates::VK_BROWSER_FAVORITES),
    (Key::BrowserHome, VirtualKeyStates::VK_BROWSER_HOME),
    (Key::VolumeMute, VirtualKeyStates::VK_VOLUME_MUTE),
    (Key::VolumeDown, VirtualKeyStates::VK_VOLUME_DOWN),
    (Key::VolumeUp, VirtualKeyStates::VK_VOLUME_UP),
    (Key::MediaNextTrack, VirtualKeyStates::VK_MEDIA_NEXT_TRACK),
    (Key::MediaPreviousTrack, VirtualKeyStates::VK_MEDIA_PREV_TRACK),
    (Key::MediaStop, VirtualKeyStates::VK_MEDIA_STOP),
    (Key::MediaPlayPause, VirtualKeyStates::VK_MEDIA_PLAY_PAUSE),
    (Key::LaunchMail, VirtualKeyStates::VK_LAUNCH_MAIL),
    (Key::SelectMedia, VirtualKeyStates::VK_LAUNCH_MEDIA_SELECT),
    (Key::LaunchApplication1, VirtualKeyStates::VK_LAUNCH_APP1),
    (Key::LaunchApplication2, VirtualKeyStates::VK_LAUNCH_APP2),
    (Key::Oem1, VirtualKeyStates::VK_OEM_1),
    (Key::OemPlus, VirtualKeyStates::VK_OEM_PLUS),
    (Key::OemComma, VirtualKeyStates::VK_OEM_COMMA),
    (Key::OemMinus, VirtualKeyStates::VK_OEM_MINUS),
    (Key::OemPeriod, VirtualKeyStates::VK_OEM_PERIOD),
    (Key::OemQuestion, VirtualKeyStates::VK_OEM_2),
    (Key::Oem3, VirtualKeyStates::VK_OEM_3),
    (Key::AbntC1, VirtualKeyStates::VK_ABNT_C1),
    (Key::AbntC2, VirtualKeyStates::VK_ABNT_C2),
    (Key::OemOpenBrackets, VirtualKeyStates::VK_OEM_4),
    (Key::Oem5, VirtualKeyStates::VK_OEM_5),
    (Key::Oem6, VirtualKeyStates::VK_OEM_6),
    (Key::OemQuotes, VirtualKeyStates::VK_OEM_7),
    (Key::Oem8, VirtualKeyStates::VK_OEM_8),
    (Key::OemBackslash, VirtualKeyStates::VK_OEM_102),
    (Key::ImeProcessed, VirtualKeyStates::VK_PROCESSKEY),
    (Key::OemAttn, VirtualKeyStates::VK_OEM_ATTN),
    (Key::OemFinish, VirtualKeyStates::VK_OEM_FINISH),
    (Key::OemCopy, VirtualKeyStates::VK_OEM_COPY),
    (Key::DbeSbcsChar, VirtualKeyStates::VK_OEM_AUTO),
    (Key::OemEnlw, VirtualKeyStates::VK_OEM_ENLW),
    (Key::OemBackTab, VirtualKeyStates::VK_OEM_BACKTAB),
    (Key::DbeNoRoman, VirtualKeyStates::VK_ATTN),
    (Key::DbeEnterWordRegisterMode, VirtualKeyStates::VK_CRSEL),
    (Key::DbeEnterImeConfigureMode, VirtualKeyStates::VK_EXSEL),
    (Key::EraseEof, VirtualKeyStates::VK_EREOF),
    (Key::Play, VirtualKeyStates::VK_PLAY),
    (Key::DbeNoCodeInput, VirtualKeyStates::VK_ZOOM),
    (Key::NoName, VirtualKeyStates::VK_NONAME),
    (Key::Pa1, VirtualKeyStates::VK_PA1),
    (Key::OemClear, VirtualKeyStates::VK_OEM_CLEAR),
];

// https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input#scan-codes
// https://github.com/chromium/chromium/blob/main/ui/events/keycodes/dom/dom_code_data.inc
// This list has the same order as the PhysicalKey enum.
static PHYSICAL_KEY_FROM_EXTENDED_SCAN_CODE: &[(u16, PhysicalKey)] = &[

    // Writing System Keys
    (0x0029, PhysicalKey::Backquote),
    (0x002B, PhysicalKey::Backslash),
    (0x001A, PhysicalKey::BracketLeft),
    (0x001B, PhysicalKey::BracketRight),
    (0x0033, PhysicalKey::Comma),
    (0x000B, PhysicalKey::Digit0),
    (0x0002, PhysicalKey::Digit1),
    (0x0003, PhysicalKey::Digit2),
    (0x0004, PhysicalKey::Digit3),
    (0x0005, PhysicalKey::Digit4),
    (0x0006, PhysicalKey::Digit5),
    (0x0007, PhysicalKey::Digit6),
    (0x0008, PhysicalKey::Digit7),
    (0x0009, PhysicalKey::Digit8),
    (0x000A, PhysicalKey::Digit9),
    (0x000D, PhysicalKey::Equal),
    (0x0056, PhysicalKey::IntlBackslash),
    (0x0073, PhysicalKey::IntlRo),
    (0x007D, PhysicalKey::IntlYen),
    (0x001E, PhysicalKey::A),
    (0x0030, PhysicalKey::B),
    (0x002E, PhysicalKey::C),
    (0x0020, PhysicalKey::D),
    (0x0012, PhysicalKey::E),
    (0x0021, PhysicalKey::F),
    (0x0022, PhysicalKey::G),
    (0x0023, PhysicalKey::H),
    (0x0017, PhysicalKey::I),
    (0x0024, PhysicalKey::J),
    (0x0025, PhysicalKey::K),
    (0x0026, PhysicalKey::L),
    (0x0032, PhysicalKey::M),
    (0x0031, PhysicalKey::N),
    (0x0018, PhysicalKey::O),
    (0x0019, PhysicalKey::P),
    (0x0010, PhysicalKey::Q),
    (0x0013, PhysicalKey::R),
    (0x001F, PhysicalKey::S),
    (0x0014, PhysicalKey::T),
    (0x0016, PhysicalKey::U),
    (0x002F, PhysicalKey::V),
    (0x0011, PhysicalKey::W),
    (0x002D, PhysicalKey::X),
    (0x0015, PhysicalKey::Y),
    (0x002C, PhysicalKey::Z),
    (0x000C, PhysicalKey::Minus),
    (0x0034, PhysicalKey::Period),
    (0x0028, PhysicalKey::Quote),
    (0x0027, PhysicalKey::Semicolon),
    (0x0035, PhysicalKey::Slash),

    // Functional Keys
    (0x0038, PhysicalKey::AltLeft),
    (0xE038, PhysicalKey::AltRight),
    (0x000E, PhysicalKey::Backspace),
    (0x003A, PhysicalKey::CapsLock),
    (0xE05D, PhysicalKey::ContextMenu),
    (0x001D, PhysicalKey::ControlLeft),
    (0xE01D, PhysicalKey::ControlRight),
    (0x001C, PhysicalKey::Enter),
    (0xE05B, PhysicalKey::MetaLeft),
    (0xE05C, PhysicalKey::MetaRight),
    (0x002A, PhysicalKey::ShiftLeft),
    (0x0036, PhysicalKey::ShiftRight),
    (0x0039, PhysicalKey::Space),
    (0x000F, PhysicalKey::Tab),
    (0x0079, PhysicalKey::Convert),
    (0x0070, PhysicalKey::KanaMode),
    (0x0072, PhysicalKey::Lang1),
    (0x0071, PhysicalKey::Lang2),
    (0x0078, PhysicalKey::Lang3),
    (0x0077, PhysicalKey::Lang4),
    // {     , PhysicalKey::Lang5 }, Not mapped on Windows since it's the same as F24 (see Chromium remarks)
    (0x007B, PhysicalKey::NonConvert),

    // Control Pad Section
    (0xE053, PhysicalKey::Delete),
    (0xE04F, PhysicalKey::End),
    (0xE03B, PhysicalKey::Help),
    (0xE047, PhysicalKey::Home),
    (0xE052, PhysicalKey::Insert),
    (0xE051, PhysicalKey::PageDown),
    (0xE049, PhysicalKey::PageUp),

    // Arrow Pad Section
    (0xE050, PhysicalKey::ArrowDown),
    (0xE04B, PhysicalKey::ArrowLeft),
    (0xE04D, PhysicalKey::ArrowRight),
    (0xE048, PhysicalKey::ArrowUp),

    // Numpad Section
    (0xE045, PhysicalKey::NumLock),
    (0x0052, PhysicalKey::NumPad0),
    (0x004F, PhysicalKey::NumPad1),
    (0x0050, PhysicalKey::NumPad2),
    (0x0051, PhysicalKey::NumPad3),
    (0x004B, PhysicalKey::NumPad4),
    (0x004C, PhysicalKey::NumPad5),
    (0x004D, PhysicalKey::NumPad6),
    (0x0047, PhysicalKey::NumPad7),
    (0x0048, PhysicalKey::NumPad8),
    (0x0049, PhysicalKey::NumPad9),
    (0x004E, PhysicalKey::NumPadAdd),
    // {     , PhysicalKey::NumPadClear },
    (0x007E, PhysicalKey::NumPadComma),
    (0x0053, PhysicalKey::NumPadDecimal),
    (0xE035, PhysicalKey::NumPadDivide),
    (0xE01C, PhysicalKey::NumPadEnter),
    (0x0059, PhysicalKey::NumPadEqual),
    (0x0037, PhysicalKey::NumPadMultiply),
    // {     , PhysicalKey::NumPadParenLeft },
    // {     , PhysicalKey::NumPadParenRight },
    (0x004A, PhysicalKey::NumPadSubtract),

    // Function Section
    (0x0001, PhysicalKey::Escape),
    (0x003B, PhysicalKey::F1),
    (0x003C, PhysicalKey::F2),
    (0x003D, PhysicalKey::F3),
    (0x003E, PhysicalKey::F4),
    (0x003F, PhysicalKey::F5),
    (0x0040, PhysicalKey::F6),
    (0x0041, PhysicalKey::F7),
    (0x0042, PhysicalKey::F8),
    (0x0043, PhysicalKey::F9),
    (0x0044, PhysicalKey::F10),
    (0x0057, PhysicalKey::F11),
    (0x0058, PhysicalKey::F12),
    (0x0064, PhysicalKey::F13),
    (0x0065, PhysicalKey::F14),
    (0x0066, PhysicalKey::F15),
    (0x0067, PhysicalKey::F16),
    (0x0068, PhysicalKey::F17),
    (0x0069, PhysicalKey::F18),
    (0x006A, PhysicalKey::F19),
    (0x006B, PhysicalKey::F20),
    (0x006C, PhysicalKey::F21),
    (0x006D, PhysicalKey::F22),
    (0x006E, PhysicalKey::F23),
    (0x0076, PhysicalKey::F24),
    (0xE037, PhysicalKey::PrintScreen),
    (0x0046, PhysicalKey::ScrollLock),
    (0x0045, PhysicalKey::Pause),

    // Media Keys
    (0xE06A, PhysicalKey::BrowserBack),
    (0xE066, PhysicalKey::BrowserFavorites),
    (0xE069, PhysicalKey::BrowserForward),
    (0xE032, PhysicalKey::BrowserHome),
    (0xE067, PhysicalKey::BrowserRefresh),
    (0xE065, PhysicalKey::BrowserSearch),
    (0xE068, PhysicalKey::BrowserStop),
    (0xE02C, PhysicalKey::Eject),
    (0xE06B, PhysicalKey::LaunchApp1),
    (0xE021, PhysicalKey::LaunchApp2),
    (0xE06C, PhysicalKey::LaunchMail),
    (0xE022, PhysicalKey::MediaPlayPause),
    (0xE06D, PhysicalKey::MediaSelect),
    (0xE024, PhysicalKey::MediaStop),
    (0xE019, PhysicalKey::MediaTrackNext),
    (0xE010, PhysicalKey::MediaTrackPrevious),
    (0xE05E, PhysicalKey::Power),
    (0xE05F, PhysicalKey::Sleep),
    (0xE02E, PhysicalKey::AudioVolumeDown),
    (0xE020, PhysicalKey::AudioVolumeMute),
    (0xE030, PhysicalKey::AudioVolumeUp),
    (0xE063, PhysicalKey::WakeUp),

    // Legacy Keys
    (0xE018, PhysicalKey::Copy),
    (0xE017, PhysicalKey::Cut),
    // {     , PhysicalKey::Find },
    // {     , PhysicalKey::Open },
    (0xE00A, PhysicalKey::Paste),
    // {     , PhysicalKey::Props },
    // {     , PhysicalKey::Select },
    (0xE008, PhysicalKey::Undo),
];

fn virtual_key_from_key_table() -> &'static HashMap<Key, i32> {
    static TABLE: OnceLock<HashMap<Key, i32>> = OnceLock::new();
    TABLE.get_or_init(|| VIRTUAL_KEY_FROM_KEY.iter().copied().collect())
}

fn key_from_virtual_key_table() -> &'static HashMap<i32, Key> {
    static TABLE: OnceLock<HashMap<i32, Key>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let table: HashMap<i32, Key> = VIRTUAL_KEY_FROM_KEY.iter().map(|&(key, virtual_key)| (virtual_key, key)).collect();
        // Two keys with one virtual key would make the reverse table lose
        // one of them, where the reference fails to build its dictionary.
        assert_eq!(table.len(), VIRTUAL_KEY_FROM_KEY.len(), "a virtual key is mapped to more than one key");
        table
    })
}

fn physical_key_table() -> &'static HashMap<u16, PhysicalKey> {
    static TABLE: OnceLock<HashMap<u16, PhysicalKey>> = OnceLock::new();
    TABLE.get_or_init(|| PHYSICAL_KEY_FROM_EXTENDED_SCAN_CODE.iter().copied().collect())
}

impl KeyInterop {
    /// Indicates whether the key is an extended key, such as the right-hand
    /// ALT and CTRL keys. According to
    /// https://docs.microsoft.com/en-us/windows/win32/inputdev/wm-keydown.
    fn is_extended(key_data: i32) -> bool {
        const EXTENDED_MASK: i32 = 1 << 24;

        (key_data & EXTENDED_MASK) != 0
    }

    fn get_scan_code(key_data: i32) -> u8 {
        // Bits from 16 to 23 represent scan code.
        const SCAN_CODE_MASK: i32 = 0xFF0000;

        ((key_data & SCAN_CODE_MASK) >> 16) as u8
    }

    fn get_virtual_key_with(virtual_key: i32, key_data: i32, map_virtual_key: &dyn Fn(u32, u32) -> u32) -> i32 {
        // Adapted from https://github.com/dotnet/wpf/blob/master/src/Microsoft.DotNet.Wpf/src/PresentationCore/System/Windows/InterOp/HwndKeyboardInputProvider.cs.

        if virtual_key == VirtualKeyStates::VK_SHIFT {
            let scan_code = Self::get_scan_code(key_data);

            let virtual_key = map_virtual_key(u32::from(scan_code), MapVirtualKeyMapTypes::MAPVK_VSC_TO_VK_EX) as i32;

            if virtual_key == 0 {
                return VirtualKeyStates::VK_LSHIFT;
            }

            virtual_key
        } else if virtual_key == VirtualKeyStates::VK_MENU {
            if Self::is_extended(key_data) {
                VirtualKeyStates::VK_RMENU
            } else {
                VirtualKeyStates::VK_LMENU
            }
        } else if virtual_key == VirtualKeyStates::VK_CONTROL {
            if Self::is_extended(key_data) {
                VirtualKeyStates::VK_RCONTROL
            } else {
                VirtualKeyStates::VK_LCONTROL
            }
        } else {
            virtual_key
        }
    }

    /// [`key_from_virtual_key`](Self::key_from_virtual_key) with the
    /// mapping of the system (`MapVirtualKey`) given as a function.
    pub fn key_from_virtual_key_with(virtual_key: i32, key_data: i32, map_virtual_key: &dyn Fn(u32, u32) -> u32) -> Key {
        let virtual_key = Self::get_virtual_key_with(virtual_key, key_data, map_virtual_key);

        key_from_virtual_key_table().get(&virtual_key).copied().unwrap_or(Key::None)
    }

    /// Gets a Windows virtual-key from a key of the toolkit: a Windows
    /// virtual-key code, or 0 if none matched.
    pub fn virtual_key_from_key(key: Key) -> i32 {
        virtual_key_from_key_table().get(&key).copied().unwrap_or(0)
    }

    /// [`physical_key_from_virtual_key`](Self::physical_key_from_virtual_key)
    /// with the mapping of the system (`MapVirtualKey`) given as a function.
    pub fn physical_key_from_virtual_key_with(
        virtual_key: i32,
        key_data: i32,
        map_virtual_key: &dyn Fn(u32, u32) -> u32,
    ) -> PhysicalKey {
        let mut scan_code = u32::from(Self::get_scan_code(key_data));
        if scan_code == 0 {
            // in some cases, the scan code contained in the keyData might be zero:
            // try to get one from the virtual key instead
            scan_code = map_virtual_key(virtual_key as u32, MapVirtualKeyMapTypes::MAPVK_VK_TO_VSC);
            if scan_code == 0 {
                return PhysicalKey::None;
            }
        }

        if Self::is_extended(key_data) {
            scan_code |= 0xE000;
        }

        if scan_code > 0 && scan_code <= 0xE0FF {
            physical_key_table().get(&(scan_code as u16)).copied().unwrap_or(PhysicalKey::None)
        } else {
            PhysicalKey::None
        }
    }

    /// [`get_key_symbol_from_virtual_key`](Self::get_key_symbol_from_virtual_key)
    /// with the mapping of the system (`MapVirtualKey`) given as a function.
    pub fn get_key_symbol_from_virtual_key_with(virtual_key: i32, map_virtual_key: &dyn Fn(u32, u32) -> u32) -> Option<String> {
        let ch = map_virtual_key(virtual_key as u32, MapVirtualKeyMapTypes::MAPVK_VK_TO_CHAR);
        if ch == 0 {
            return None;
        }

        // Bit 31 is set for dead keys: strip it to get the base character.
        // The reference then narrows the value to a UTF-16 code unit.
        let c = char::from_u32((ch & 0x7FFF_FFFF) & 0xFFFF)?;
        KeySymbolHelper::is_allowed_ascii_key_symbol(c).then(|| c.to_string())
    }

    /// The key symbol of what `ToUnicodeEx` returned: `length` characters in
    /// `buffer`, a negative length for a dead key.
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    fn key_symbol_from_to_unicode(length: i32, buffer: &[u16; 4]) -> Option<String> {
        let text = |count: usize| String::from_utf16_lossy(&buffer[..count.min(buffer.len())]);
        match length {
            // dead key
            length if length < 0 => Some(text(length.unsigned_abs() as usize)),
            0 => None,
            1 if !char::from_u32(u32::from(buffer[0])).is_some_and(KeySymbolHelper::is_allowed_ascii_key_symbol) => None,
            // dead key second press repeats symbol
            2 if buffer[0] == buffer[1] => Some(text(1)),
            length => Some(text(length as usize)),
        }
    }
}

#[cfg(windows)]
impl KeyInterop {
    /// Gets a key of the toolkit from a Windows virtual-key and key data
    /// (in the same format as lParam for WM_KEYDOWN), or [`Key::None`] if
    /// none matched.
    pub fn key_from_virtual_key(virtual_key: i32, key_data: i32) -> Key {
        Self::key_from_virtual_key_with(virtual_key, key_data, &crate::interop::unmanaged_methods::map_virtual_key)
    }

    /// Gets a physical key of the toolkit from a Windows virtual-key and
    /// key data (in the same format as lParam for WM_KEYDOWN), or
    /// [`PhysicalKey::None`] if none matched.
    pub fn physical_key_from_virtual_key(virtual_key: i32, key_data: i32) -> PhysicalKey {
        Self::physical_key_from_virtual_key_with(virtual_key, key_data, &crate::interop::unmanaged_methods::map_virtual_key)
    }

    /// Gets a key symbol from a Windows virtual-key using MapVirtualKey.
    /// Unlike [`get_key_symbol`](Self::get_key_symbol), this does not call
    /// ToUnicodeEx and is safe to use during WM_SYSKEYDOWN/UP where
    /// ToUnicodeEx would corrupt keyboard state.
    pub fn get_key_symbol_from_virtual_key(virtual_key: i32) -> Option<String> {
        Self::get_key_symbol_from_virtual_key_with(virtual_key, &crate::interop::unmanaged_methods::map_virtual_key)
    }

    /// Gets a key symbol from a Windows virtual-key and key data (in the
    /// same format as lParam for WM_KEYDOWN), or `None` if none matched.
    pub fn get_key_symbol(virtual_key: i32, key_data: i32) -> Option<String> {
        use crate::interop::unmanaged_methods::{get_keyboard_layout, get_keyboard_state, to_unicode_ex};

        const DO_NOT_CHANGE_KEYBOARD_STATE: u32 = 1 << 2;

        let key_states = get_keyboard_state();
        let (length, buffer) = to_unicode_ex(
            virtual_key as u32,
            u32::from(Self::get_scan_code(key_data)),
            &key_states,
            DO_NOT_CHANGE_KEYBOARD_STATE,
            get_keyboard_layout(0),
        );

        Self::key_symbol_from_to_unicode(length, &buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The key data of a key message: the scan code in bits 16 to 23 and
    /// the extended flag in bit 24.
    fn key_data(scan_code: u8, extended: bool) -> i32 {
        (i32::from(scan_code) << 16) | if extended { 1 << 24 } else { 0 }
    }

    fn no_mapping(_code: u32, _map_type: u32) -> u32 {
        0
    }

    #[test]
    fn every_key_of_the_table_maps_back_to_itself() {
        assert_eq!(VIRTUAL_KEY_FROM_KEY.len(), 169);
        for &(key, virtual_key) in VIRTUAL_KEY_FROM_KEY {
            assert_eq!(KeyInterop::virtual_key_from_key(key), virtual_key);
            assert_eq!(KeyInterop::key_from_virtual_key_with(virtual_key, 0, &no_mapping), key);
        }
        assert_eq!(KeyInterop::virtual_key_from_key(Key::None), 0);
        assert_eq!(KeyInterop::key_from_virtual_key_with(0x07, 0, &no_mapping), Key::None);
    }

    #[test]
    fn letters_and_digits_are_their_ascii_codes() {
        assert_eq!(KeyInterop::virtual_key_from_key(Key::A), 0x41);
        assert_eq!(KeyInterop::virtual_key_from_key(Key::Z), 0x5A);
        assert_eq!(KeyInterop::virtual_key_from_key(Key::D0), 0x30);
        assert_eq!(KeyInterop::key_from_virtual_key_with(0x39, 0, &no_mapping), Key::D9);
    }

    #[test]
    fn the_generic_modifiers_are_told_apart_by_the_key_data() {
        let vk = VirtualKeyStates::VK_MENU;
        assert_eq!(KeyInterop::key_from_virtual_key_with(vk, key_data(0x38, false), &no_mapping), Key::LeftAlt);
        assert_eq!(KeyInterop::key_from_virtual_key_with(vk, key_data(0x38, true), &no_mapping), Key::RightAlt);

        let vk = VirtualKeyStates::VK_CONTROL;
        assert_eq!(KeyInterop::key_from_virtual_key_with(vk, key_data(0x1D, false), &no_mapping), Key::LeftCtrl);
        assert_eq!(KeyInterop::key_from_virtual_key_with(vk, key_data(0x1D, true), &no_mapping), Key::RightCtrl);

        // Shift is told apart by its scan code, which the system maps; the
        // left key is the answer when the system has none.
        let vk = VirtualKeyStates::VK_SHIFT;
        let map = |code: u32, map_type: u32| {
            assert_eq!(map_type, MapVirtualKeyMapTypes::MAPVK_VSC_TO_VK_EX);
            if code == 0x36 {
                VirtualKeyStates::VK_RSHIFT as u32
            } else {
                0
            }
        };
        assert_eq!(KeyInterop::key_from_virtual_key_with(vk, key_data(0x36, false), &map), Key::RightShift);
        assert_eq!(KeyInterop::key_from_virtual_key_with(vk, key_data(0x2A, false), &map), Key::LeftShift);
    }

    #[test]
    fn physical_keys_come_from_the_scan_code_and_the_extended_flag() {
        assert_eq!(PHYSICAL_KEY_FROM_EXTENDED_SCAN_CODE.len(), 155);
        let physical =
            |scan_code, extended| KeyInterop::physical_key_from_virtual_key_with(0, key_data(scan_code, extended), &no_mapping);

        assert_eq!(physical(0x1E, false), PhysicalKey::A);
        assert_eq!(physical(0x1C, false), PhysicalKey::Enter);
        assert_eq!(physical(0x1C, true), PhysicalKey::NumPadEnter);
        assert_eq!(physical(0x38, true), PhysicalKey::AltRight);
        assert_eq!(physical(0x4B, false), PhysicalKey::NumPad4);
        assert_eq!(physical(0x4B, true), PhysicalKey::ArrowLeft);
        assert_eq!(physical(0x7F, false), PhysicalKey::None);
    }

    #[test]
    fn a_missing_scan_code_is_asked_from_the_virtual_key() {
        let map = |code: u32, map_type: u32| {
            assert_eq!(map_type, MapVirtualKeyMapTypes::MAPVK_VK_TO_VSC);
            if code == 0x41 {
                0x1E
            } else {
                0
            }
        };
        assert_eq!(KeyInterop::physical_key_from_virtual_key_with(0x41, 0, &map), PhysicalKey::A);
        assert_eq!(KeyInterop::physical_key_from_virtual_key_with(0x42, 0, &map), PhysicalKey::None);
    }

    #[test]
    fn the_symbol_of_a_virtual_key_drops_the_dead_key_bit() {
        let map = |code: u32, _map_type: u32| match code {
            0x41 => u32::from('A'),
            0xDE => 0x8000_0000 | u32::from('^'),
            0x0D => 0x01,
            _ => 0,
        };
        assert_eq!(KeyInterop::get_key_symbol_from_virtual_key_with(0x41, &map).as_deref(), Some("A"));
        assert_eq!(KeyInterop::get_key_symbol_from_virtual_key_with(0xDE, &map).as_deref(), Some("^"));
        // A control character is not a key symbol.
        assert_eq!(KeyInterop::get_key_symbol_from_virtual_key_with(0x0D, &map), None);
        assert_eq!(KeyInterop::get_key_symbol_from_virtual_key_with(0x70, &map), None);
    }

    #[test]
    fn the_symbol_of_a_translated_key() {
        let buffer = |text: &str| {
            let mut buffer = [0u16; 4];
            for (slot, unit) in buffer.iter_mut().zip(text.encode_utf16()) {
                *slot = unit;
            }
            buffer
        };

        assert_eq!(KeyInterop::key_symbol_from_to_unicode(1, &buffer("a")).as_deref(), Some("a"));
        assert_eq!(KeyInterop::key_symbol_from_to_unicode(0, &buffer("")), None);
        // A dead key reports a negative length.
        assert_eq!(KeyInterop::key_symbol_from_to_unicode(-1, &buffer("^")).as_deref(), Some("^"));
        // The second press of a dead key repeats its symbol.
        assert_eq!(KeyInterop::key_symbol_from_to_unicode(2, &buffer("^^")).as_deref(), Some("^"));
        assert_eq!(KeyInterop::key_symbol_from_to_unicode(2, &buffer("^a")).as_deref(), Some("^a"));
        // A control character is not a key symbol.
        assert_eq!(KeyInterop::key_symbol_from_to_unicode(1, &buffer("\u{1}")).as_deref(), None);
    }
}
