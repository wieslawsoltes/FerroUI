//! The modifier and capability bits of IBus (the port of `IBusEnums.cs`).

use bitflags::bitflags;

/// IBus input purposes, from IBus 1.5.32 `src/ibustypes.h` at
/// 0fd4e408a52d3d15179c643fd54d310aafd8875f. These are input-method hints,
/// not validation rules or a request to change the control's rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum IBusInputPurpose {
    #[default]
    FreeForm = 0,
    Alpha = 1,
    Digits = 2,
    Number = 3,
    Phone = 4,
    Url = 5,
    Email = 6,
    Name = 7,
    Password = 8,
    Pin = 9,
    Terminal = 10,
}

bitflags! {
    /// Exact IBus 1.5.32 `IBusInputHints` bits (`src/ibustypes.h`).
    /// Older engines ignore unknown hints; PRIVATE was added in 1.5.26.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct IBusInputHints: u32 {
        const SPELLCHECK = 1 << 0;
        const NO_SPELLCHECK = 1 << 1;
        const WORD_COMPLETION = 1 << 2;
        const LOWERCASE = 1 << 3;
        const UPPERCASE_CHARS = 1 << 4;
        const UPPERCASE_WORDS = 1 << 5;
        const UPPERCASE_SENTENCES = 1 << 6;
        const INHIBIT_OSK = 1 << 7;
        const VERTICAL_WRITING = 1 << 8;
        const EMOJI = 1 << 9;
        const NO_EMOJI = 1 << 10;
        const PRIVATE = 1 << 11;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct IBusModifierMask: u32 {
        const SHIFT_MASK    = 1 << 0;
        const LOCK_MASK     = 1 << 1;
        const CONTROL_MASK  = 1 << 2;
        const MOD1_MASK     = 1 << 3;
        const MOD2_MASK     = 1 << 4;
        const MOD3_MASK     = 1 << 5;
        const MOD4_MASK     = 1 << 6;
        const MOD5_MASK     = 1 << 7;
        const BUTTON1_MASK  = 1 << 8;
        const BUTTON2_MASK  = 1 << 9;
        const BUTTON3_MASK  = 1 << 10;
        const BUTTON4_MASK  = 1 << 11;
        const BUTTON5_MASK  = 1 << 12;

        const HANDLED_MASK  = 1 << 24;
        const FORWARD_MASK  = 1 << 25;
        const IGNORED_MASK  = Self::FORWARD_MASK.bits();

        const SUPER_MASK    = 1 << 26;
        const HYPER_MASK    = 1 << 27;
        const META_MASK     = 1 << 28;

        const RELEASE_MASK  = 1 << 30;

        const MODIFIER_MASK = 0x5c001fff;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct IBusCapability: u32 {
        const CAP_PREEDIT_TEXT = 1 << 0;
        const CAP_AUXILIARY_TEXT = 1 << 1;
        const CAP_LOOKUP_TABLE = 1 << 2;
        const CAP_FOCUS = 1 << 3;
        const CAP_PROPERTY = 1 << 4;
        const CAP_SURROUNDING_TEXT = 1 << 5;
    }
}
