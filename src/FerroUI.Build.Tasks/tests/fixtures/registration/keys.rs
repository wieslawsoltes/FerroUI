use bitflags::bitflags;
use ferroui_base::ferro_markup_enum;

bitflags! {
    /// A set of flags whose constants are constant expressions.
    pub struct Modes: i32 {
        const KEYBOARD = 1;
        const GAMEPAD = 1 << 1;
        const REMOTE = Self::GAMEPAD.bits() << 1;
        const ENABLED = Self::KEYBOARD.bits() | Self::GAMEPAD.bits() | Self::REMOTE.bits();
    }
}

impl Modes {
    /// Constants of the type that are no constants of the `bitflags!` invocation.
    pub const DISABLED: Modes = Modes::empty();
    pub const EVERY: Modes = Modes::all();
    pub const POINTING: Modes = Modes::GAMEPAD.union(Modes::REMOTE);
    pub const COMPUTED: Modes = compute();
}

ferro_markup_enum!(flags Modes {
    Disabled = Modes::DISABLED,
    Keyboard = Modes::KEYBOARD,
    Remote = Modes::REMOTE,
    Enabled = Modes::ENABLED,
    Every = Modes::EVERY,
    Pointing = Modes::POINTING,
    Computed = Modes::COMPUTED,
});

#[repr(i32)]
pub enum Key {
    None = 0,
    Return = 6,
    Pause,
    Shifted = 1 << 4,
    Both = Self::Return as i32 | Self::Shifted as i32,
    Last = i32::MAX,
    Odd = odd(),
    After,
}

#[allow(non_upper_case_globals)]
impl Key {
    /// A second name of a variant.
    pub const Enter: Key = Key::Return;
    pub const Other: Key = other();
}

ferro_markup_enum!(Key { None, Return, Enter, Pause, Shifted, Both, Last, Odd, After, Other });
