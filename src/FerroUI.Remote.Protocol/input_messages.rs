/*
We are keeping copies of core events here, so they can be used
without referencing the framework itself, e. g. from projects that
are using WPF, GTK#, etc
*/

use std::ops::{Deref, DerefMut};

use crate::key::Key;
use crate::physical_key::PhysicalKey;
use crate::{bson_class, bson_enum, ferro_remote_message_guid};

/// Keep this in sync with InputModifiers in the main library
// Deviation (DEVIATIONS.md, Remote protocol): the original marks the
// enumeration `[Flags]`, but its members are the numbers zero to six, not
// bits, and a message carries them as an array of members. It is a plain
// enumeration here, not a set of flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InputModifiers {
    #[default]
    Alt = 0,
    Control = 1,
    Shift = 2,
    Windows = 3,
    LeftMouseButton = 4,
    RightMouseButton = 5,
    MiddleMouseButton = 6,
}

impl InputModifiers {
    /// The member with the given numeric value, if one is defined.
    pub fn from_value(value: i32) -> Option<InputModifiers> {
        match value {
            0 => Some(InputModifiers::Alt),
            1 => Some(InputModifiers::Control),
            2 => Some(InputModifiers::Shift),
            3 => Some(InputModifiers::Windows),
            4 => Some(InputModifiers::LeftMouseButton),
            5 => Some(InputModifiers::RightMouseButton),
            6 => Some(InputModifiers::MiddleMouseButton),
            _ => None,
        }
    }
}

bson_enum!(InputModifiers);

/// Keep this in sync with InputModifiers in the main library
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MouseButton {
    #[default]
    None = 0,
    Left = 1,
    Right = 2,
    Middle = 3,
}

impl MouseButton {
    /// The member with the given numeric value, if one is defined.
    pub fn from_value(value: i32) -> Option<MouseButton> {
        match value {
            0 => Some(MouseButton::None),
            1 => Some(MouseButton::Left),
            2 => Some(MouseButton::Right),
            3 => Some(MouseButton::Middle),
            _ => None,
        }
    }
}

bson_enum!(MouseButton);

// The two key enumerations are compiled into this library from the sources
// of the base library (`lib.rs`); what they are on the wire is stated here.
bson_enum!(Key);
bson_enum!(PhysicalKey);

/// The base of the input messages. A class that derives from it embeds it as
/// `base` and writes its properties after its own, which is the order in
/// which reflection returns the properties of a derived class.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputEventMessageBase {
    pub modifiers: Option<Vec<InputModifiers>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointerEventMessageBase {
    pub base: InputEventMessageBase,
    pub x: f64,
    pub y: f64,
}

impl Deref for PointerEventMessageBase {
    type Target = InputEventMessageBase;

    fn deref(&self) -> &InputEventMessageBase {
        &self.base
    }
}

impl DerefMut for PointerEventMessageBase {
    fn deref_mut(&mut self) -> &mut InputEventMessageBase {
        &mut self.base
    }
}

/// `class X : Base`: the derived class reads and writes as its base.
macro_rules! derives {
    ($class:ident : $base:ident) => {
        impl Deref for $class {
            type Target = $base;

            fn deref(&self) -> &$base {
                &self.base
            }
        }

        impl DerefMut for $class {
            fn deref_mut(&mut self) -> &mut $base {
                &mut self.base
            }
        }
    };
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointerMovedEventMessage {
    pub base: PointerEventMessageBase,
}

derives!(PointerMovedEventMessage: PointerEventMessageBase);
ferro_remote_message_guid!(PointerMovedEventMessage, "6228F0B9-99F2-4F62-A621-414DA2881648");
bson_class!(PointerMovedEventMessage as "PointerMovedEventMessage" {
    "X" => base.x: f64,
    "Y" => base.y: f64,
    "Modifiers" => base.base.modifiers: Option<Vec<InputModifiers>>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointerPressedEventMessage {
    pub base: PointerEventMessageBase,
    pub button: MouseButton,
}

derives!(PointerPressedEventMessage: PointerEventMessageBase);
ferro_remote_message_guid!(PointerPressedEventMessage, "7E9E2818-F93F-411A-800E-6B1AEB11DA46");
bson_class!(PointerPressedEventMessage as "PointerPressedEventMessage" {
    "Button" => button: MouseButton,
    "X" => base.x: f64,
    "Y" => base.y: f64,
    "Modifiers" => base.base.modifiers: Option<Vec<InputModifiers>>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointerReleasedEventMessage {
    pub base: PointerEventMessageBase,
    pub button: MouseButton,
}

derives!(PointerReleasedEventMessage: PointerEventMessageBase);
ferro_remote_message_guid!(PointerReleasedEventMessage, "4ADC84EE-E7C8-4BCF-986C-DE3A2F78EDE4");
bson_class!(PointerReleasedEventMessage as "PointerReleasedEventMessage" {
    "Button" => button: MouseButton,
    "X" => base.x: f64,
    "Y" => base.y: f64,
    "Modifiers" => base.base.modifiers: Option<Vec<InputModifiers>>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollEventMessage {
    pub base: PointerEventMessageBase,
    pub delta_x: f64,
    pub delta_y: f64,
}

derives!(ScrollEventMessage: PointerEventMessageBase);
ferro_remote_message_guid!(ScrollEventMessage, "79301A05-F02D-4B90-BB39-472563B504AE");
bson_class!(ScrollEventMessage as "ScrollEventMessage" {
    "DeltaX" => delta_x: f64,
    "DeltaY" => delta_y: f64,
    "X" => base.x: f64,
    "Y" => base.y: f64,
    "Modifiers" => base.base.modifiers: Option<Vec<InputModifiers>>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyEventMessage {
    pub base: InputEventMessageBase,
    pub is_down: bool,
    pub key: Key,
    pub physical_key: PhysicalKey,
    pub key_symbol: Option<String>,
}

derives!(KeyEventMessage: InputEventMessageBase);
ferro_remote_message_guid!(KeyEventMessage, "1C3B691E-3D54-4237-BFB0-9FEA83BC1DB8");
bson_class!(KeyEventMessage as "KeyEventMessage" {
    "IsDown" => is_down: bool,
    "Key" => key: Key,
    "PhysicalKey" => physical_key: PhysicalKey,
    "KeySymbol" => key_symbol: Option<String>,
    "Modifiers" => base.modifiers: Option<Vec<InputModifiers>>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextInputEventMessage {
    pub base: InputEventMessageBase,
    pub text: String,
}

derives!(TextInputEventMessage: InputEventMessageBase);
ferro_remote_message_guid!(TextInputEventMessage, "C174102E-7405-4594-916F-B10B8248A17D");
bson_class!(TextInputEventMessage as "TextInputEventMessage" {
    "Text" => text: String,
    "Modifiers" => base.modifiers: Option<Vec<InputModifiers>>,
});
