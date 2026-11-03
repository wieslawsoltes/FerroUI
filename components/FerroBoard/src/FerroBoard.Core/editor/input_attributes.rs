// Derived from ScreenKeyboardControl; see NOTICE.md.
/// Source Int32 enum, including undeclared values.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InputKind(pub i32);
#[allow(non_upper_case_globals)]
impl InputKind {
    pub const Text: Self = Self(0);
    pub const Number: Self = Self(1);
    pub const Decimal: Self = Self(2);
    pub const Phone: Self = Self(3);
    pub const DateTime: Self = Self(4);
    pub const Email: Self = Self(5);
    pub const Uri: Self = Self(6);
    pub const Password: Self = Self(7);
    pub const NumericPassword: Self = Self(8);
    pub const Search: Self = Self(9);
    pub const Chat: Self = Self(10);
}

/// Source Int32 enum, including undeclared values.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EnterAction(pub i32);
#[allow(non_upper_case_globals)]
impl EnterAction {
    pub const Default: Self = Self(0);
    pub const NewLine: Self = Self(1);
    pub const Go: Self = Self(2);
    pub const Search: Self = Self(3);
    pub const Send: Self = Self(4);
    pub const Next: Self = Self(5);
    pub const Previous: Self = Self(6);
    pub const Done: Self = Self(7);
}

/// Source Int32 enum, including undeclared values.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CapitalizationMode(pub i32);
#[allow(non_upper_case_globals)]
impl CapitalizationMode {
    pub const None: Self = Self(0);
    pub const Characters: Self = Self(1);
    pub const Words: Self = Self(2);
    pub const Sentences: Self = Self(3);
}

use crate::keys::{KeyVariation, KeyboardMode};
use std::sync::{Arc, LazyLock};
/// Native constructor/value snapshot, independent of the shared configuration handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputAttributesValues {
    pub kind: InputKind,
    pub enter_action: EnterAction,
    pub capitalization: CapitalizationMode,
    pub is_multiline: bool,
    pub allow_suggestions: bool,
    pub allow_auto_correct: bool,
    pub is_private: bool,
}
impl Default for InputAttributesValues {
    fn default() -> Self {
        Self {
            kind: InputKind::Text,
            enter_action: EnterAction::Default,
            capitalization: CapitalizationMode::Sentences,
            is_multiline: false,
            allow_suggestions: true,
            allow_auto_correct: true,
            is_private: false,
        }
    }
}
/// Shared native input configuration. Setters update the existing value; cloning
/// copies a snapshot. Compiled XAML uses the same ordinary setters as Rust callers.
pub struct InputAttributes {
    values: parking_lot::RwLock<InputAttributesValues>,
}
impl Default for InputAttributes {
    fn default() -> Self {
        Self::from_values(InputAttributesValues::default())
    }
}
impl Clone for InputAttributes {
    fn clone(&self) -> Self {
        Self::from_values(self.values())
    }
}
impl PartialEq for InputAttributes {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.values() == other.values()
    }
}
impl Eq for InputAttributes {}
impl std::hash::Hash for InputAttributes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.values().hash(state);
    }
}
impl InputAttributes {
    pub fn from_values(values: InputAttributesValues) -> Self {
        Self {
            values: parking_lot::RwLock::new(values),
        }
    }
    pub fn values(&self) -> InputAttributesValues {
        *self.values.read()
    }
    pub fn shared_default() -> Arc<Self> {
        static DEFAULT: LazyLock<Arc<InputAttributes>> =
            LazyLock::new(|| Arc::new(InputAttributes::default()));
        DEFAULT.clone()
    }
    pub fn kind(&self) -> InputKind {
        self.values.read().kind
    }
    pub fn set_kind(&self, value: InputKind) {
        self.values.write().kind = value;
    }
    pub fn enter_action(&self) -> EnterAction {
        self.values.read().enter_action
    }
    pub fn set_enter_action(&self, value: EnterAction) {
        self.values.write().enter_action = value;
    }
    pub fn capitalization(&self) -> CapitalizationMode {
        self.values.read().capitalization
    }
    pub fn set_capitalization(&self, value: CapitalizationMode) {
        self.values.write().capitalization = value;
    }
    pub fn is_multiline(&self) -> bool {
        self.values.read().is_multiline
    }
    pub fn set_is_multiline(&self, value: bool) {
        self.values.write().is_multiline = value;
    }
    pub fn allow_suggestions(&self) -> bool {
        self.values.read().allow_suggestions
    }
    pub fn set_allow_suggestions(&self, value: bool) {
        self.values.write().allow_suggestions = value;
    }
    pub fn allow_auto_correct(&self) -> bool {
        self.values.read().allow_auto_correct
    }
    pub fn set_allow_auto_correct(&self, value: bool) {
        self.values.write().allow_auto_correct = value;
    }
    pub fn is_private(&self) -> bool {
        self.values.read().is_private
    }
    pub fn set_is_private(&self, value: bool) {
        self.values.write().is_private = value;
    }
    pub fn is_password(&self) -> bool {
        matches!(
            self.kind(),
            InputKind::Password | InputKind::NumericPassword
        )
    }
    pub fn variation(&self) -> KeyVariation {
        match self.kind() {
            InputKind::Email => KeyVariation::EmailAddress,
            InputKind::Uri => KeyVariation::Uri,
            InputKind::Password => KeyVariation::Password,
            _ => KeyVariation::Normal,
        }
    }
    pub fn initial_mode(&self) -> KeyboardMode {
        match self.kind() {
            InputKind::Number | InputKind::NumericPassword => KeyboardMode::Numeric,
            InputKind::Decimal | InputKind::DateTime => KeyboardMode::NumericAdvanced,
            InputKind::Phone => KeyboardMode::Phone,
            _ => KeyboardMode::Characters,
        }
    }
    pub fn effective_enter_action(&self) -> EnterAction {
        let action = self.enter_action();
        if action != EnterAction::Default {
            return action;
        }
        if self.is_multiline() {
            return EnterAction::NewLine;
        }
        match self.kind() {
            InputKind::Search => EnterAction::Search,
            InputKind::Uri => EnterAction::Go,
            InputKind::Chat => EnterAction::Send,
            _ => EnterAction::Done,
        }
    }
    pub fn supports_nlp(&self) -> bool {
        self.allow_suggestions()
            && !self.is_password()
            && matches!(
                self.kind(),
                InputKind::Text | InputKind::Search | InputKind::Chat
            )
    }
    /// Native equivalent of the source with-expression's fresh record clone.
    pub fn with_values(&self, change: impl FnOnce(&mut InputAttributesValues)) -> Arc<Self> {
        let mut values = self.values();
        change(&mut values);
        Arc::new(Self::from_values(values))
    }
}
impl std::fmt::Debug for InputAttributes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.values().fmt(f)
    }
}
impl std::fmt::Display for InputAttributes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::fmt::Display for InputKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Text => f.write_str("Text"),
            Self::Number => f.write_str("Number"),
            Self::Decimal => f.write_str("Decimal"),
            Self::Phone => f.write_str("Phone"),
            Self::DateTime => f.write_str("DateTime"),
            Self::Email => f.write_str("Email"),
            Self::Uri => f.write_str("Uri"),
            Self::Password => f.write_str("Password"),
            Self::NumericPassword => f.write_str("NumericPassword"),
            Self::Search => f.write_str("Search"),
            Self::Chat => f.write_str("Chat"),
            _ => std::fmt::Display::fmt(&self.0, f),
        }
    }
}
impl std::fmt::Debug for InputKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for EnterAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Default => f.write_str("Default"),
            Self::NewLine => f.write_str("NewLine"),
            Self::Go => f.write_str("Go"),
            Self::Search => f.write_str("Search"),
            Self::Send => f.write_str("Send"),
            Self::Next => f.write_str("Next"),
            Self::Previous => f.write_str("Previous"),
            Self::Done => f.write_str("Done"),
            _ => std::fmt::Display::fmt(&self.0, f),
        }
    }
}
impl std::fmt::Debug for EnterAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for CapitalizationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::None => f.write_str("None"),
            Self::Characters => f.write_str("Characters"),
            Self::Words => f.write_str("Words"),
            Self::Sentences => f.write_str("Sentences"),
            _ => std::fmt::Display::fmt(&self.0, f),
        }
    }
}
impl std::fmt::Debug for CapitalizationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
