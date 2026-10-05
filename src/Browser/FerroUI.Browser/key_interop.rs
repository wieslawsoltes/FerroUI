//! Maps the `code` and `key` values of the keyboard events of the page to
//! the keys of the framework.

use ferroui_base::input::{Key, PhysicalKey};
use std::collections::HashMap;
use std::sync::OnceLock;

// https://www.w3.org/TR/uievents-code/
// https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values
// This list has the same order as the PhysicalKey enum.
const PHYSICAL_KEY_FROM_DOM_CODE: &[(&str, PhysicalKey)] = &[
    ("Unidentified", PhysicalKey::None),

    // Writing System Keys
    ("Backquote", PhysicalKey::Backquote),
    ("Backslash", PhysicalKey::Backslash),
    ("BracketLeft", PhysicalKey::BracketLeft),
    ("BracketRight", PhysicalKey::BracketRight),
    ("Comma", PhysicalKey::Comma),
    ("Digit0", PhysicalKey::Digit0),
    ("Digit1", PhysicalKey::Digit1),
    ("Digit2", PhysicalKey::Digit2),
    ("Digit3", PhysicalKey::Digit3),
    ("Digit4", PhysicalKey::Digit4),
    ("Digit5", PhysicalKey::Digit5),
    ("Digit6", PhysicalKey::Digit6),
    ("Digit7", PhysicalKey::Digit7),
    ("Digit8", PhysicalKey::Digit8),
    ("Digit9", PhysicalKey::Digit9),
    ("Equal", PhysicalKey::Equal),
    ("IntlBackslash", PhysicalKey::IntlBackslash),
    ("IntlRo", PhysicalKey::IntlRo),
    ("IntlYen", PhysicalKey::IntlYen),
    ("KeyA", PhysicalKey::A),
    ("KeyB", PhysicalKey::B),
    ("KeyC", PhysicalKey::C),
    ("KeyD", PhysicalKey::D),
    ("KeyE", PhysicalKey::E),
    ("KeyF", PhysicalKey::F),
    ("KeyG", PhysicalKey::G),
    ("KeyH", PhysicalKey::H),
    ("KeyI", PhysicalKey::I),
    ("KeyJ", PhysicalKey::J),
    ("KeyK", PhysicalKey::K),
    ("KeyL", PhysicalKey::L),
    ("KeyM", PhysicalKey::M),
    ("KeyN", PhysicalKey::N),
    ("KeyO", PhysicalKey::O),
    ("KeyP", PhysicalKey::P),
    ("KeyQ", PhysicalKey::Q),
    ("KeyR", PhysicalKey::R),
    ("KeyS", PhysicalKey::S),
    ("KeyT", PhysicalKey::T),
    ("KeyU", PhysicalKey::U),
    ("KeyV", PhysicalKey::V),
    ("KeyW", PhysicalKey::W),
    ("KeyX", PhysicalKey::X),
    ("KeyY", PhysicalKey::Y),
    ("KeyZ", PhysicalKey::Z),
    ("Minus", PhysicalKey::Minus),
    ("Period", PhysicalKey::Period),
    ("Quote", PhysicalKey::Quote),
    ("Semicolon", PhysicalKey::Semicolon),
    ("Slash", PhysicalKey::Slash),

    // Functional Keys
    ("AltLeft", PhysicalKey::AltLeft),
    ("AltRight", PhysicalKey::AltRight),
    ("Backspace", PhysicalKey::Backspace),
    ("CapsLock", PhysicalKey::CapsLock),
    ("ContextMenu", PhysicalKey::ContextMenu),
    ("ControlLeft", PhysicalKey::ControlLeft),
    ("ControlRight", PhysicalKey::ControlRight),
    ("Enter", PhysicalKey::Enter),
    ("MetaLeft", PhysicalKey::MetaLeft),
    ("OSLeft", PhysicalKey::MetaLeft),
    ("MetaRight", PhysicalKey::MetaRight),
    ("OSRight", PhysicalKey::MetaRight),
    ("ShiftLeft", PhysicalKey::ShiftLeft),
    ("ShiftRight", PhysicalKey::ShiftRight),
    ("Space", PhysicalKey::Space),
    ("Tab", PhysicalKey::Tab),
    ("Convert", PhysicalKey::Convert),
    ("KanaMode", PhysicalKey::KanaMode),
    ("Lang1", PhysicalKey::Lang1),
    ("Lang2", PhysicalKey::Lang2),
    ("Lang3", PhysicalKey::Lang3),
    ("Lang4", PhysicalKey::Lang4),
    ("Lang5", PhysicalKey::Lang5),
    ("NonConvert", PhysicalKey::NonConvert),

    // Control Pad Section
    ("Delete", PhysicalKey::Delete),
    ("End", PhysicalKey::End),
    ("Help", PhysicalKey::Help),
    ("Home", PhysicalKey::Home),
    ("Insert", PhysicalKey::Insert),
    ("PageDown", PhysicalKey::PageDown),
    ("PageUp", PhysicalKey::PageUp),

    // Arrow Pad Section
    ("ArrowDown", PhysicalKey::ArrowDown),
    ("ArrowLeft", PhysicalKey::ArrowLeft),
    ("ArrowRight", PhysicalKey::ArrowRight),
    ("ArrowUp", PhysicalKey::ArrowUp),

    // Numpad Section
    ("NumLock", PhysicalKey::NumLock),
    ("Numpad0", PhysicalKey::NumPad0),
    ("Numpad1", PhysicalKey::NumPad1),
    ("Numpad2", PhysicalKey::NumPad2),
    ("Numpad3", PhysicalKey::NumPad3),
    ("Numpad4", PhysicalKey::NumPad4),
    ("Numpad5", PhysicalKey::NumPad5),
    ("Numpad6", PhysicalKey::NumPad6),
    ("Numpad7", PhysicalKey::NumPad7),
    ("Numpad8", PhysicalKey::NumPad8),
    ("Numpad9", PhysicalKey::NumPad9),
    ("NumpadAdd", PhysicalKey::NumPadAdd),
    ("NumpadClear", PhysicalKey::NumPadClear),
    ("NumpadComma", PhysicalKey::NumPadComma),
    ("NumpadDecimal", PhysicalKey::NumPadDecimal),
    ("NumpadDivide", PhysicalKey::NumPadDivide),
    ("NumpadEnter", PhysicalKey::NumPadEnter),
    ("NumpadEqual", PhysicalKey::NumPadEqual),
    ("NumpadMultiply", PhysicalKey::NumPadMultiply),
    ("NumpadParenLeft", PhysicalKey::NumPadParenLeft),
    ("NumpadParenRight", PhysicalKey::NumPadParenRight),
    ("NumpadSubtract", PhysicalKey::NumPadSubtract),

    // Function Section
    ("Escape", PhysicalKey::Escape),
    ("F1", PhysicalKey::F1),
    ("F2", PhysicalKey::F2),
    ("F3", PhysicalKey::F3),
    ("F4", PhysicalKey::F4),
    ("F5", PhysicalKey::F5),
    ("F6", PhysicalKey::F6),
    ("F7", PhysicalKey::F7),
    ("F8", PhysicalKey::F8),
    ("F9", PhysicalKey::F9),
    ("F10", PhysicalKey::F10),
    ("F11", PhysicalKey::F11),
    ("F12", PhysicalKey::F12),
    ("F13", PhysicalKey::F13),
    ("F14", PhysicalKey::F14),
    ("F15", PhysicalKey::F15),
    ("F16", PhysicalKey::F16),
    ("F17", PhysicalKey::F17),
    ("F18", PhysicalKey::F18),
    ("F19", PhysicalKey::F19),
    ("F20", PhysicalKey::F20),
    ("F21", PhysicalKey::F21),
    ("F22", PhysicalKey::F22),
    ("F23", PhysicalKey::F23),
    ("F24", PhysicalKey::F24),
    ("PrintScreen", PhysicalKey::PrintScreen),
    ("ScrollLock", PhysicalKey::ScrollLock),
    ("Pause", PhysicalKey::Pause),

    // Media Keys
    ("BrowserBack", PhysicalKey::BrowserBack),
    ("BrowserFavorites", PhysicalKey::BrowserFavorites),
    ("BrowserForward", PhysicalKey::BrowserForward),
    ("BrowserHome", PhysicalKey::BrowserHome),
    ("BrowserRefresh", PhysicalKey::BrowserRefresh),
    ("BrowserSearch", PhysicalKey::BrowserSearch),
    ("BrowserStop", PhysicalKey::BrowserStop),
    ("Abort", PhysicalKey::BrowserStop),
    ("Eject", PhysicalKey::Eject),
    ("LaunchApp1", PhysicalKey::LaunchApp1),
    ("LaunchApp2", PhysicalKey::LaunchApp2),
    ("LaunchMail", PhysicalKey::LaunchMail),
    ("MediaPlayPause", PhysicalKey::MediaPlayPause),
    ("MediaSelect", PhysicalKey::MediaSelect),
    ("MediaStop", PhysicalKey::MediaStop),
    ("MediaTrackNext", PhysicalKey::MediaTrackNext),
    ("MediaTrackPrevious", PhysicalKey::MediaTrackPrevious),
    ("Power", PhysicalKey::Power),
    ("Sleep", PhysicalKey::Sleep),
    ("AudioVolumeDown", PhysicalKey::AudioVolumeDown),
    ("VolumeDown", PhysicalKey::AudioVolumeDown),
    ("AudioVolumeMute", PhysicalKey::AudioVolumeMute),
    ("VolumeMute", PhysicalKey::AudioVolumeMute),
    ("AudioVolumeUp", PhysicalKey::AudioVolumeUp),
    ("VolumeUp", PhysicalKey::AudioVolumeUp),
    ("WakeUp", PhysicalKey::WakeUp),

    // Legacy Keys
    ("Copy", PhysicalKey::Copy),
    ("Cut", PhysicalKey::Cut),
    ("Find", PhysicalKey::Find),
    ("Open", PhysicalKey::Open),
    ("Paste", PhysicalKey::Paste),
    ("Props", PhysicalKey::Props),
    ("Select", PhysicalKey::Select),
    ("Undo", PhysicalKey::Undo),
];

// https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_key_values
const KEY_FROM_DOM_KEY: &[(&str, Key)] = &[
    // Alphabetic keys
    ("A", Key::A),
    ("B", Key::B),
    ("C", Key::C),
    ("D", Key::D),
    ("E", Key::E),
    ("F", Key::F),
    ("G", Key::G),
    ("H", Key::H),
    ("I", Key::I),
    ("J", Key::J),
    ("K", Key::K),
    ("L", Key::L),
    ("M", Key::M),
    ("N", Key::N),
    ("O", Key::O),
    ("P", Key::P),
    ("Q", Key::Q),
    ("R", Key::R),
    ("S", Key::S),
    ("T", Key::T),
    ("U", Key::U),
    ("V", Key::V),
    ("W", Key::W),
    ("X", Key::X),
    ("Y", Key::Y),
    ("Z", Key::Z),
    ("a", Key::A),
    ("b", Key::B),
    ("c", Key::C),
    ("d", Key::D),
    ("e", Key::E),
    ("f", Key::F),
    ("g", Key::G),
    ("h", Key::H),
    ("i", Key::I),
    ("j", Key::J),
    ("k", Key::K),
    ("l", Key::L),
    ("m", Key::M),
    ("n", Key::N),
    ("o", Key::O),
    ("p", Key::P),
    ("q", Key::Q),
    ("r", Key::R),
    ("s", Key::S),
    ("t", Key::T),
    ("u", Key::U),
    ("v", Key::V),
    ("w", Key::W),
    ("x", Key::X),
    ("y", Key::Y),
    ("z", Key::Z),

    // Modifier keys (left/right keys are handled separately)
    ("AltGr", Key::RightAlt),
    ("CapsLock", Key::CapsLock),
    ("NumLock", Key::NumLock),
    ("ScrollLock", Key::Scroll),

    // Whitespace keys
    ("Enter", Key::Enter),
    ("Tab", Key::Tab),
    (" ", Key::Space),

    // Navigation keys
    ("ArrowDown", Key::Down),
    ("ArrowLeft", Key::Left),
    ("ArrowRight", Key::Right),
    ("ArrowUp", Key::Up),
    ("End", Key::End),
    ("Home", Key::Home),
    ("PageDown", Key::PageDown),
    ("PageUp", Key::PageUp),

    // Editing keys
    ("Backspace", Key::Back),
    ("Clear", Key::Clear),
    ("CrSel", Key::CrSel),
    ("Delete", Key::Delete),
    ("EraseEof", Key::EraseEof),
    ("ExSel", Key::ExSel),
    ("Insert", Key::Insert),

    // UI keys
    ("Accept", Key::ImeAccept),
    ("Attn", Key::OemAttn),
    ("Cancel", Key::Cancel),
    ("ContextMenu", Key::Apps),
    ("Escape", Key::Escape),
    ("Execute", Key::Execute),
    ("Finish", Key::OemFinish),
    ("Help", Key::Help),
    ("Pause", Key::Pause),
    ("Play", Key::Play),
    ("Select", Key::Select),
    ("ZoomIn", Key::Zoom),

    // Device keys
    ("PrintScreen", Key::PrintScreen),

    // IME keys
    ("Convert", Key::ImeConvert),
    ("FinalMode", Key::FinalMode),
    ("ModeChange", Key::ImeModeChange),
    ("NonConvert", Key::ImeNonConvert),
    ("Process", Key::ImeProcessed),
    ("HangulMode", Key::HangulMode),
    ("HanjaMode", Key::HanjaMode),
    ("JunjaMode", Key::JunjaMode),
    ("Hankaku", Key::OemAuto),
    ("Hiragana", Key::DbeHiragana),
    ("KanaMode", Key::KanaMode),
    ("KanjiMode", Key::KanjiMode),
    ("Katakana", Key::DbeKatakana),
    ("Romaji", Key::OemBackTab),
    ("Zenkaku", Key::OemEnlw),

    // Function keys
    ("F1", Key::F1),
    ("F2", Key::F2),
    ("F3", Key::F3),
    ("F4", Key::F4),
    ("F5", Key::F5),
    ("F6", Key::F6),
    ("F7", Key::F7),
    ("F8", Key::F8),
    ("F9", Key::F9),
    ("F10", Key::F10),
    ("F11", Key::F11),
    ("F12", Key::F12),
    ("F13", Key::F13),
    ("F14", Key::F14),
    ("F15", Key::F15),
    ("F16", Key::F16),
    ("F17", Key::F17),
    ("F18", Key::F18),
    ("F19", Key::F19),
    ("F20", Key::F20),

    // Multimedia keys
    ("MediaPlayPause", Key::MediaPlayPause),
    ("MediaStop", Key::MediaStop),
    ("MediaTrackNext", Key::MediaNextTrack),
    ("MediaTrackPrevious", Key::MediaPreviousTrack),

    // Audio control keys
    ("AudioVolumeDown", Key::VolumeDown),
    ("AudioVolumeMute", Key::VolumeMute),
    ("AudioVolumeUp", Key::VolumeUp),

    // Application selector keys
    ("LaunchCalculator", Key::LaunchApplication2),
    ("LaunchMail", Key::LaunchMail),
    ("LaunchMyComputer", Key::LaunchApplication1),
    ("LaunchApplication1", Key::LaunchApplication1),
    ("LaunchApplication2", Key::LaunchApplication2),

    // Browser control keys
    ("BrowserBack", Key::BrowserBack),
    ("BrowserFavorites", Key::BrowserFavorites),
    ("BrowserForward", Key::BrowserForward),
    ("BrowserHome", Key::BrowserHome),
    ("BrowserRefresh", Key::BrowserRefresh),
    ("BrowserSearch", Key::BrowserSearch),
    ("BrowserStop", Key::BrowserStop),

    // Numeric keypad keys
    ("Decimal", Key::Decimal),
    ("Multiply", Key::Multiply),
    ("Add", Key::Add),
    ("Divide", Key::Divide),
    ("Subtract", Key::Subtract),
    ("Separator", Key::Separator),
];

fn physical_key_from_dom_code_table() -> &'static HashMap<&'static str, PhysicalKey> {
    static TABLE: OnceLock<HashMap<&'static str, PhysicalKey>> = OnceLock::new();
    TABLE.get_or_init(|| PHYSICAL_KEY_FROM_DOM_CODE.iter().copied().collect())
}

fn key_from_dom_key_table() -> &'static HashMap<&'static str, Key> {
    static TABLE: OnceLock<HashMap<&'static str, Key>> = OnceLock::new();
    TABLE.get_or_init(|| KEY_FROM_DOM_KEY.iter().copied().collect())
}

/// The physical key of the `code` of a keyboard event.
pub(crate) fn physical_key_from_dom_code(dom_code: Option<&str>) -> PhysicalKey {
    match dom_code {
        Some(dom_code) if !dom_code.is_empty() => {
            physical_key_from_dom_code_table().get(dom_code).copied().unwrap_or(PhysicalKey::None)
        }
        _ => PhysicalKey::None,
    }
}

/// The key of the `key` of a keyboard event; `physical_key` tells the
/// left keys from the right ones and the keys of the numeric keypad
/// from the others.
pub(crate) fn key_from_dom_key(dom_key: Option<&str>, physical_key: PhysicalKey) -> Key {
    let dom_key = match dom_key {
        Some(dom_key) if !dom_key.is_empty() => dom_key,
        _ => return Key::None,
    };

    if let Some(key) = key_from_dom_key_table().get(dom_key) {
        return *key;
    }

    let select = |condition: bool, when_true: Key, when_false: Key| if condition { when_true } else { when_false };

    let key = match dom_key {
        "Alt" => select(physical_key == PhysicalKey::AltRight, Key::RightAlt, Key::LeftAlt),
        "Control" => select(physical_key == PhysicalKey::ControlRight, Key::RightCtrl, Key::LeftCtrl),
        "Shift" => select(physical_key == PhysicalKey::ShiftRight, Key::RightShift, Key::LeftShift),
        "Meta" => select(physical_key == PhysicalKey::MetaRight, Key::RWin, Key::LWin),
        "0" => select(physical_key == PhysicalKey::NumPad0, Key::NumPad0, Key::D0),
        "1" => select(physical_key == PhysicalKey::NumPad1, Key::NumPad1, Key::D1),
        "2" => select(physical_key == PhysicalKey::NumPad2, Key::NumPad2, Key::D2),
        "3" => select(physical_key == PhysicalKey::NumPad3, Key::NumPad3, Key::D3),
        "4" => select(physical_key == PhysicalKey::NumPad4, Key::NumPad4, Key::D4),
        "5" => select(physical_key == PhysicalKey::NumPad5, Key::NumPad5, Key::D5),
        "6" => select(physical_key == PhysicalKey::NumPad6, Key::NumPad6, Key::D6),
        "7" => select(physical_key == PhysicalKey::NumPad7, Key::NumPad7, Key::D7),
        "8" => select(physical_key == PhysicalKey::NumPad8, Key::NumPad8, Key::D8),
        "9" => select(physical_key == PhysicalKey::NumPad9, Key::NumPad9, Key::D9),
        "+" => select(physical_key == PhysicalKey::NumPadAdd, Key::Add, Key::OemPlus),
        "-" => select(physical_key == PhysicalKey::NumPadSubtract, Key::Subtract, Key::OemMinus),
        "*" => select(physical_key == PhysicalKey::NumPadMultiply, Key::Multiply, Key::None),
        "/" => select(physical_key == PhysicalKey::NumPadDivide, Key::Divide, Key::None),
        _ => Key::None,
    };

    if key != Key::None {
        return key;
    }

    physical_key.to_qwerty_key()
}

/// The symbol of the `key` of a keyboard event: the value itself when
/// it is one character (one UTF-16 unit or one surrogate pair).
pub(crate) fn key_symbol_from_dom_key(dom_key: Option<&str>) -> Option<String> {
    let dom_key = match dom_key {
        Some(dom_key) if !dom_key.is_empty() => dom_key,
        _ => return None,
    };

    // A string of the framework cannot hold half a surrogate pair, so one character is
    // either one UTF-16 unit or one whole pair: the two cases the original tells apart.
    let mut chars = dom_key.chars();
    match (chars.next(), chars.next()) {
        (Some(_), None) => Some(dom_key.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn physical(code: &str) -> PhysicalKey {
        physical_key_from_dom_code(Some(code))
    }

    fn key(dom_key: &str, physical_key: PhysicalKey) -> Key {
        key_from_dom_key(Some(dom_key), physical_key)
    }

    #[test]
    fn the_tables_are_complete_and_have_no_duplicate_entries() {
        assert_eq!(170, PHYSICAL_KEY_FROM_DOM_CODE.len());
        assert_eq!(147, KEY_FROM_DOM_KEY.len());

        let codes: HashSet<_> = PHYSICAL_KEY_FROM_DOM_CODE.iter().map(|(code, _)| *code).collect();
        let keys: HashSet<_> = KEY_FROM_DOM_KEY.iter().map(|(key, _)| *key).collect();
        assert_eq!(PHYSICAL_KEY_FROM_DOM_CODE.len(), codes.len());
        assert_eq!(KEY_FROM_DOM_KEY.len(), keys.len());
        assert_eq!(170, physical_key_from_dom_code_table().len());
        assert_eq!(147, key_from_dom_key_table().len());
    }

    #[test]
    fn every_physical_key_except_again_has_a_code() {
        // "Again" is the one legacy key the table leaves out.
        let mapped: HashSet<_> = PHYSICAL_KEY_FROM_DOM_CODE.iter().map(|(_, key)| *key).collect();
        assert_eq!(164, mapped.len());
        assert!(!mapped.contains(&PhysicalKey::Again));
    }

    #[test]
    fn a_missing_or_unknown_code_is_no_physical_key() {
        assert_eq!(PhysicalKey::None, physical_key_from_dom_code(None));
        assert_eq!(PhysicalKey::None, physical(""));
        assert_eq!(PhysicalKey::None, physical("Unidentified"));
        assert_eq!(PhysicalKey::None, physical("NoSuchCode"));
        // Codes are case sensitive.
        assert_eq!(PhysicalKey::None, physical("keya"));
    }

    #[test]
    fn codes_of_every_section_map_to_their_physical_key() {
        for (code, expected) in [
            // Writing System Keys
            ("Backquote", PhysicalKey::Backquote),
            ("Digit0", PhysicalKey::Digit0),
            ("Digit9", PhysicalKey::Digit9),
            ("IntlYen", PhysicalKey::IntlYen),
            ("KeyA", PhysicalKey::A),
            ("KeyZ", PhysicalKey::Z),
            ("Slash", PhysicalKey::Slash),
            // Functional Keys
            ("AltLeft", PhysicalKey::AltLeft),
            ("AltRight", PhysicalKey::AltRight),
            ("Enter", PhysicalKey::Enter),
            ("MetaLeft", PhysicalKey::MetaLeft),
            ("OSLeft", PhysicalKey::MetaLeft),
            ("MetaRight", PhysicalKey::MetaRight),
            ("OSRight", PhysicalKey::MetaRight),
            ("Space", PhysicalKey::Space),
            ("Lang5", PhysicalKey::Lang5),
            ("NonConvert", PhysicalKey::NonConvert),
            // Control Pad Section
            ("Delete", PhysicalKey::Delete),
            ("PageUp", PhysicalKey::PageUp),
            // Arrow Pad Section
            ("ArrowDown", PhysicalKey::ArrowDown),
            ("ArrowUp", PhysicalKey::ArrowUp),
            // Numpad Section
            ("NumLock", PhysicalKey::NumLock),
            ("Numpad0", PhysicalKey::NumPad0),
            ("Numpad9", PhysicalKey::NumPad9),
            ("NumpadEnter", PhysicalKey::NumPadEnter),
            ("NumpadSubtract", PhysicalKey::NumPadSubtract),
            // Function Section
            ("Escape", PhysicalKey::Escape),
            ("F1", PhysicalKey::F1),
            ("F24", PhysicalKey::F24),
            ("Pause", PhysicalKey::Pause),
            // Media Keys
            ("BrowserBack", PhysicalKey::BrowserBack),
            ("BrowserStop", PhysicalKey::BrowserStop),
            ("Abort", PhysicalKey::BrowserStop),
            ("AudioVolumeDown", PhysicalKey::AudioVolumeDown),
            ("VolumeDown", PhysicalKey::AudioVolumeDown),
            ("VolumeMute", PhysicalKey::AudioVolumeMute),
            ("VolumeUp", PhysicalKey::AudioVolumeUp),
            ("WakeUp", PhysicalKey::WakeUp),
            // Legacy Keys
            ("Copy", PhysicalKey::Copy),
            ("Undo", PhysicalKey::Undo),
        ] {
            assert_eq!(expected, physical(code), "{code}");
        }
    }

    #[test]
    fn a_missing_key_is_no_key() {
        assert_eq!(Key::None, key_from_dom_key(None, PhysicalKey::A));
        assert_eq!(Key::None, key("", PhysicalKey::A));
    }

    #[test]
    fn keys_of_every_group_map_to_their_key() {
        for (dom_key, expected) in [
            // Alphabetic keys
            ("A", Key::A),
            ("Z", Key::Z),
            ("a", Key::A),
            ("z", Key::Z),
            // Modifier keys
            ("AltGr", Key::RightAlt),
            ("CapsLock", Key::CapsLock),
            ("ScrollLock", Key::Scroll),
            // Whitespace keys
            ("Enter", Key::Enter),
            ("Tab", Key::Tab),
            (" ", Key::Space),
            // Navigation keys
            ("ArrowDown", Key::Down),
            ("ArrowLeft", Key::Left),
            ("PageUp", Key::PageUp),
            // Editing keys
            ("Backspace", Key::Back),
            ("Delete", Key::Delete),
            ("Insert", Key::Insert),
            // UI keys
            ("Accept", Key::ImeAccept),
            ("ContextMenu", Key::Apps),
            ("Escape", Key::Escape),
            ("ZoomIn", Key::Zoom),
            // Device keys
            ("PrintScreen", Key::PrintScreen),
            // IME keys
            ("Convert", Key::ImeConvert),
            ("Process", Key::ImeProcessed),
            ("HanjaMode", Key::HanjaMode),
            ("Hankaku", Key::OemAuto),
            ("KanaMode", Key::KanaMode),
            ("Katakana", Key::DbeKatakana),
            ("Zenkaku", Key::OemEnlw),
            // Function keys
            ("F1", Key::F1),
            ("F20", Key::F20),
            // Multimedia keys
            ("MediaTrackNext", Key::MediaNextTrack),
            ("MediaTrackPrevious", Key::MediaPreviousTrack),
            // Audio control keys
            ("AudioVolumeMute", Key::VolumeMute),
            // Application selector keys
            ("LaunchCalculator", Key::LaunchApplication2),
            ("LaunchMyComputer", Key::LaunchApplication1),
            // Browser control keys
            ("BrowserBack", Key::BrowserBack),
            ("BrowserStop", Key::BrowserStop),
            // Numeric keypad keys
            ("Decimal", Key::Decimal),
            ("Separator", Key::Separator),
        ] {
            assert_eq!(expected, key(dom_key, PhysicalKey::None), "{dom_key}");
        }
    }

    #[test]
    fn the_aliases_of_the_key_table_are_the_keys_they_stand_for() {
        assert_eq!(Key::Return, key("Enter", PhysicalKey::None));
        assert_eq!(Key::Snapshot, key("PrintScreen", PhysicalKey::None));
        assert_eq!(Key::HangulMode, key("KanaMode", PhysicalKey::None));
        assert_eq!(Key::KanjiMode, key("HanjaMode", PhysicalKey::None));
        assert_eq!(Key::DbeSbcsChar, key("Hankaku", PhysicalKey::None));
        assert_eq!(Key::OemFinish, key("Katakana", PhysicalKey::None));
        assert_eq!(Key::DbeDbcsChar, key("Zenkaku", PhysicalKey::None));
        assert_eq!(Key::DbeNoCodeInput, key("ZoomIn", PhysicalKey::None));
    }

    #[test]
    fn the_physical_key_tells_left_from_right_modifiers() {
        assert_eq!(Key::LeftAlt, key("Alt", PhysicalKey::AltLeft));
        assert_eq!(Key::RightAlt, key("Alt", PhysicalKey::AltRight));
        assert_eq!(Key::LeftCtrl, key("Control", PhysicalKey::ControlLeft));
        assert_eq!(Key::RightCtrl, key("Control", PhysicalKey::ControlRight));
        assert_eq!(Key::LeftShift, key("Shift", PhysicalKey::ShiftLeft));
        assert_eq!(Key::RightShift, key("Shift", PhysicalKey::ShiftRight));
        assert_eq!(Key::LWin, key("Meta", PhysicalKey::MetaLeft));
        assert_eq!(Key::RWin, key("Meta", PhysicalKey::MetaRight));
        // Without a physical key the left one is assumed.
        assert_eq!(Key::LeftShift, key("Shift", PhysicalKey::None));
    }

    #[test]
    fn the_physical_key_tells_the_numeric_keypad_from_the_digit_row() {
        let digits = [
            ("0", PhysicalKey::NumPad0, Key::NumPad0, Key::D0),
            ("1", PhysicalKey::NumPad1, Key::NumPad1, Key::D1),
            ("2", PhysicalKey::NumPad2, Key::NumPad2, Key::D2),
            ("3", PhysicalKey::NumPad3, Key::NumPad3, Key::D3),
            ("4", PhysicalKey::NumPad4, Key::NumPad4, Key::D4),
            ("5", PhysicalKey::NumPad5, Key::NumPad5, Key::D5),
            ("6", PhysicalKey::NumPad6, Key::NumPad6, Key::D6),
            ("7", PhysicalKey::NumPad7, Key::NumPad7, Key::D7),
            ("8", PhysicalKey::NumPad8, Key::NumPad8, Key::D8),
            ("9", PhysicalKey::NumPad9, Key::NumPad9, Key::D9),
        ];
        for (dom_key, num_pad, num_pad_key, digit_key) in digits {
            assert_eq!(num_pad_key, key(dom_key, num_pad), "{dom_key}");
            assert_eq!(digit_key, key(dom_key, PhysicalKey::None), "{dom_key}");
        }

        assert_eq!(Key::Add, key("+", PhysicalKey::NumPadAdd));
        assert_eq!(Key::OemPlus, key("+", PhysicalKey::Equal));
        assert_eq!(Key::Subtract, key("-", PhysicalKey::NumPadSubtract));
        assert_eq!(Key::OemMinus, key("-", PhysicalKey::Minus));
        assert_eq!(Key::Multiply, key("*", PhysicalKey::NumPadMultiply));
        assert_eq!(Key::Divide, key("/", PhysicalKey::NumPadDivide));
    }

    #[test]
    fn a_key_without_an_entry_falls_back_to_the_key_of_the_physical_key() {
        // "*" and "/" away from the numeric keypad, punctuation and the characters of other
        // layouts have no entry: the key is the one the physical key has on a QWERTY layout.
        assert_eq!(PhysicalKey::Digit8.to_qwerty_key(), key("*", PhysicalKey::Digit8));
        assert_eq!(PhysicalKey::Slash.to_qwerty_key(), key("/", PhysicalKey::Slash));
        assert_eq!(PhysicalKey::Semicolon.to_qwerty_key(), key(";", PhysicalKey::Semicolon));
        assert_eq!(PhysicalKey::Q.to_qwerty_key(), key("\u{0439}", PhysicalKey::Q));
        assert_eq!(PhysicalKey::None.to_qwerty_key(), key("Dead", PhysicalKey::None));
    }

    #[test]
    fn the_key_symbol_is_a_key_of_one_character() {
        assert_eq!(None, key_symbol_from_dom_key(None));
        assert_eq!(None, key_symbol_from_dom_key(Some("")));
        assert_eq!(Some("a".to_string()), key_symbol_from_dom_key(Some("a")));
        assert_eq!(Some(" ".to_string()), key_symbol_from_dom_key(Some(" ")));
        assert_eq!(Some("\u{00e9}".to_string()), key_symbol_from_dom_key(Some("\u{00e9}")));
        // One surrogate pair.
        assert_eq!(Some("\u{1f600}".to_string()), key_symbol_from_dom_key(Some("\u{1f600}")));
        // Two UTF-16 units that are not a pair, and named keys.
        assert_eq!(None, key_symbol_from_dom_key(Some("ab")));
        assert_eq!(None, key_symbol_from_dom_key(Some("Enter")));
        assert_eq!(None, key_symbol_from_dom_key(Some("F1")));
    }
}
