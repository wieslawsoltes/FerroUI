use super::IInputDevice;
use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Defines the keyboard modifier keys.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct KeyModifiers: i32 {
        const ALT = 1;
        const CONTROL = 2;
        const SHIFT = 4;
        const META = 8;
    }
}

/// The error returned when a string does not name keyboard modifiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseKeyModifiersError(pub String);

impl fmt::Display for ParseKeyModifiersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Requested value '{}' was not found.", self.0)
    }
}

impl std::error::Error for ParseKeyModifiersError {}

impl KeyModifiers {
    /// No modifier key.
    pub const NONE: KeyModifiers = KeyModifiers::empty();

    /// Parses a modifier name (`None`, `Alt`, `Control`, `Shift`, `Meta`;
    /// ignoring case), a comma-separated list of names, or a numeric value.
    pub fn parse(s: &str) -> Result<KeyModifiers, ParseKeyModifiersError> {
        let mut result = KeyModifiers::empty();

        for part in s.split(',') {
            let part = part.trim();
            let value = if part.eq_ignore_ascii_case("None") {
                KeyModifiers::empty()
            } else if part.eq_ignore_ascii_case("Alt") {
                KeyModifiers::ALT
            } else if part.eq_ignore_ascii_case("Control") {
                KeyModifiers::CONTROL
            } else if part.eq_ignore_ascii_case("Shift") {
                KeyModifiers::SHIFT
            } else if part.eq_ignore_ascii_case("Meta") {
                KeyModifiers::META
            } else {
                part.parse::<i32>()
                    .ok()
                    .and_then(KeyModifiers::from_bits)
                    .ok_or_else(|| ParseKeyModifiersError(s.to_string()))?
            };
            result |= value;
        }

        Ok(result)
    }
}

bitflags! {
    /// Defines the state of a key.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct KeyStates: i32 {
        const DOWN = 1;
        const TOGGLED = 2;
    }
}

impl KeyStates {
    /// The key is neither down nor toggled.
    pub const NONE: KeyStates = KeyStates::empty();
}

bitflags! {
    /// The modifier keys and pointer buttons reported by the platform with
    /// a raw input event.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct RawInputModifiers: i32 {
        const ALT = 1;
        const CONTROL = 2;
        const SHIFT = 4;
        const META = 8;

        const LEFT_MOUSE_BUTTON = 16;
        const RIGHT_MOUSE_BUTTON = 32;
        const MIDDLE_MOUSE_BUTTON = 64;
        const X_BUTTON_1_MOUSE_BUTTON = 128;
        const X_BUTTON_2_MOUSE_BUTTON = 256;
        const KEYBOARD_MASK = Self::ALT.bits() | Self::CONTROL.bits() | Self::SHIFT.bits() | Self::META.bits();

        const PEN_INVERTED = 512;
        const PEN_ERASER = 1024;
        const PEN_BARREL_BUTTON = 2048;
    }
}

impl RawInputModifiers {
    /// No modifier.
    pub const NONE: RawInputModifiers = RawInputModifiers::empty();
}

/// Represents a keyboard device.
pub trait IKeyboardDevice: IInputDevice {}
