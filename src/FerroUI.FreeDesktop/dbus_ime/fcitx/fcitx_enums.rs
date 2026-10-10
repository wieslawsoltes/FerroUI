//! The key event types, capability flags and key states of Fcitx (the
//! port of `FcitxEnums.cs`).

use bitflags::bitflags;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FcitxKeyEventType {
    FcitxPressKey = 0,
    FcitxReleaseKey = 1,
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct FcitxCapabilityFlags: u32 {
        const CAPACITY_NONE = 0;
        const CAPACITY_CLIENT_SIDE_UI = 1 << 0;
        const CAPACITY_PREEDIT = 1 << 1;
        const CAPACITY_CLIENT_SIDE_CONTROL_STATE = 1 << 2;
        const CAPACITY_PASSWORD = 1 << 3;
        const CAPACITY_FORMATTED_PREEDIT = 1 << 4;
        const CAPACITY_CLIENT_UNFOCUS_COMMIT = 1 << 5;
        const CAPACITY_SURROUNDING_TEXT = 1 << 6;
        const CAPACITY_EMAIL = 1 << 7;
        const CAPACITY_DIGIT = 1 << 8;
        const CAPACITY_UPPERCASE = 1 << 9;
        const CAPACITY_LOWERCASE = 1 << 10;
        const CAPACITY_NOAUTOUPPERCASE = 1 << 11;
        const CAPACITY_URL = 1 << 12;
        const CAPACITY_DIALABLE = 1 << 13;
        const CAPACITY_NUMBER = 1 << 14;
        const CAPACITY_NO_ON_SCREEN_KEYBOARD = 1 << 15;
        const CAPACITY_SPELLCHECK = 1 << 16;
        const CAPACITY_NO_SPELLCHECK = 1 << 17;
        const CAPACITY_WORD_COMPLETION = 1 << 18;
        const CAPACITY_UPPERCASE_WORDS = 1 << 19;
        const CAPACITY_UPPERCASE_SENTENCES = 1 << 20;
        const CAPACITY_ALPHA = 1 << 21;
        const CAPACITY_NAME = 1 << 22;
        const CAPACITY_GET_IM_INFO_ON_FOCUS = 1 << 23;
        const CAPACITY_RELATIVE_CURSOR_RECT = 1 << 24;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct FcitxKeyState: u32 {
        const NONE = 0;
        const SHIFT = 1 << 0;
        const CAPS_LOCK = 1 << 1;
        const CTRL = 1 << 2;
        const ALT = 1 << 3;
        const ALT_SHIFT = Self::ALT.bits() | Self::SHIFT.bits();
        const CTRL_SHIFT = Self::CTRL.bits() | Self::SHIFT.bits();
        const CTRL_ALT = Self::CTRL.bits() | Self::ALT.bits();
        const CTRL_ALT_SHIFT = Self::CTRL.bits() | Self::ALT.bits() | Self::SHIFT.bits();
        const NUM_LOCK = 1 << 4;
        const SUPER = 1 << 6;
        const SCROLL_LOCK = 1 << 7;
        const MOUSE_PRESSED = 1 << 8;
        const HANDLED_MASK = 1 << 24;
        const IGNORED_MASK = 1 << 25;
        const SUPER2 = 1 << 26;
        const HYPER = 1 << 27;
        const META = 1 << 28;
        const USED_MASK = 0x5c001fff;
    }
}
