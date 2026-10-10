//! The modifier and capability bits of IBus (the port of `IBusEnums.cs`).

use bitflags::bitflags;

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
