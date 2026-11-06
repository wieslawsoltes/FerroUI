// Derived from ScreenKeyboardControl; see NOTICE.md.
/// Source Int32 enum, including undeclared values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KeyType(pub i32);
#[allow(non_upper_case_globals)]
impl KeyType {
    pub const Character: Self = Self(0);
    pub const EnterEditing: Self = Self(1);
    pub const Function: Self = Self(2);
    pub const Lock: Self = Self(3);
    pub const Modifier: Self = Self(4);
    pub const Navigation: Self = Self(5);
    pub const SystemGui: Self = Self(6);
    pub const Numeric: Self = Self(7);
    pub const Placeholder: Self = Self(8);
    pub const Unspecified: Self = Self(9);
}
/// Source Int32 enum. Unknown values are retained until the operation that rejects them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KeyboardMode(pub i32);
#[allow(non_upper_case_globals)]
impl KeyboardMode {
    pub const Characters: Self = Self(0);
    pub const Symbols: Self = Self(1);
    pub const Symbols2: Self = Self(2);
    pub const Numeric: Self = Self(3);
    pub const NumericAdvanced: Self = Self(4);
    pub const Phone: Self = Self(5);
    pub const Phone2: Self = Self(6);
}
/// Source Int32 enum, including undeclared values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KeyVariation(pub i32);
#[allow(non_upper_case_globals)]
impl KeyVariation {
    pub const All: Self = Self(0);
    pub const Normal: Self = Self(1);
    pub const EmailAddress: Self = Self(2);
    pub const Uri: Self = Self(3);
    pub const Password: Self = Self(4);
}
/// Source Int32 enum, including undeclared values.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShiftState(pub i32);
#[allow(non_upper_case_globals)]
impl ShiftState {
    pub const Unshifted: Self = Self(0);
    pub const ShiftedManual: Self = Self(1);
    pub const ShiftedAutomatic: Self = Self(2);
    pub const CapsLock: Self = Self(3);
}
/// Source Int32 enum; only exactly Rtl selects the RTL behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LayoutDirection(pub i32);
#[allow(non_upper_case_globals)]
impl LayoutDirection {
    pub const Ltr: Self = Self(0);
    pub const Rtl: Self = Self(1);
}
/// Source Int32 enum, including undeclared values.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KeyboardUiMode(pub i32);
#[allow(non_upper_case_globals)]
impl KeyboardUiMode {
    pub const Text: Self = Self(0);
    pub const Media: Self = Self(1);
    pub const Clipboard: Self = Self(2);
    pub const Editing: Self = Self(3);
}
impl KeyType {
    pub fn parse_key_type(value: Option<&str>) -> Self {
        match value {
            None | Some("" | "character") => Self::Character,
            Some("enter_editing") => Self::EnterEditing,
            Some("function") => Self::Function,
            Some("lock") => Self::Lock,
            Some("modifier") => Self::Modifier,
            Some("navigation") => Self::Navigation,
            Some("system_gui") => Self::SystemGui,
            Some("numeric") => Self::Numeric,
            Some("placeholder") => Self::Placeholder,
            _ => Self::Unspecified,
        }
    }
    /// Additional nullable UTF-16 bridge for the exact, case-sensitive source string input.
    pub fn parse_key_type_utf16(value: Option<&crate::text::Utf16Text>) -> Self {
        let Some(value) = value else {
            return Self::Character;
        };
        if value.is_empty() {
            return Self::Character;
        }
        [
            Self::Character,
            Self::EnterEditing,
            Self::Function,
            Self::Lock,
            Self::Modifier,
            Self::Navigation,
            Self::SystemGui,
            Self::Numeric,
            Self::Placeholder,
        ]
        .into_iter()
        .find(|v| {
            let name = v.to_json_string();
            name.len() == value.len()
                && name
                    .bytes()
                    .zip(value.units())
                    .all(|(a, &b)| u16::from(a) == b)
        })
        .unwrap_or(Self::Unspecified)
    }
    pub fn to_json_string(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::EnterEditing => "enter_editing",
            Self::Function => "function",
            Self::Lock => "lock",
            Self::Modifier => "modifier",
            Self::Navigation => "navigation",
            Self::SystemGui => "system_gui",
            Self::Numeric => "numeric",
            Self::Placeholder => "placeholder",
            _ => "unspecified",
        }
    }
}
impl ShiftState {
    pub fn is_uppercase(self) -> bool {
        self != Self::Unshifted
    }
}

impl std::fmt::Display for ShiftState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Unshifted => f.write_str("Unshifted"),
            Self::ShiftedManual => f.write_str("ShiftedManual"),
            Self::ShiftedAutomatic => f.write_str("ShiftedAutomatic"),
            Self::CapsLock => f.write_str("CapsLock"),
            _ => std::fmt::Display::fmt(&self.0, f),
        }
    }
}
impl std::fmt::Debug for ShiftState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for KeyboardUiMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Text => f.write_str("Text"),
            Self::Media => f.write_str("Media"),
            Self::Clipboard => f.write_str("Clipboard"),
            Self::Editing => f.write_str("Editing"),
            _ => std::fmt::Display::fmt(&self.0, f),
        }
    }
}
impl std::fmt::Debug for KeyboardUiMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
