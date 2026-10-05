use crate::input::Key;
use crate::{FerroLocator, LocatorExtensions};
use std::collections::HashMap;
use std::rc::Rc;

/// Provides platform specific formatting information for the `KeyGesture`
/// class.
///
/// A format info without platform key overrides is the invariant one: keys
/// are formatted with their plain names.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyGestureFormatInfo {
    platform_key_overrides: Option<HashMap<Key, String>>,
    meta: String,
    ctrl: String,
    alt: String,
    shift: String,
}

impl Default for KeyGestureFormatInfo {
    fn default() -> Self {
        Self::new(None, "Cmd", "Ctrl", "Alt", "Shift")
    }
}

impl KeyGestureFormatInfo {
    /// Creates a format info.
    ///
    /// `platform_key_overrides` holds the platform specific key names;
    /// `meta`, `ctrl`, `alt` and `shift` are the names of the modifier
    /// keys.
    pub fn new(
        platform_key_overrides: Option<HashMap<Key, String>>,
        meta: &str,
        ctrl: &str,
        alt: &str,
        shift: &str,
    ) -> Self {
        Self {
            platform_key_overrides,
            meta: meta.to_string(),
            ctrl: ctrl.to_string(),
            alt: alt.to_string(),
            shift: shift.to_string(),
        }
    }

    /// The invariant format info, used as formatting fallback.
    pub fn invariant() -> Rc<KeyGestureFormatInfo> {
        thread_local! {
            static INVARIANT: Rc<KeyGestureFormatInfo> = Rc::new(KeyGestureFormatInfo::default());
        }
        INVARIANT.with(Rc::clone)
    }

    /// The string used to represent Meta on the platform.
    pub fn meta(&self) -> &str {
        &self.meta
    }

    /// The string used to represent Ctrl on the platform.
    pub fn ctrl(&self) -> &str {
        &self.ctrl
    }

    /// The string used to represent Alt on the platform.
    pub fn alt(&self) -> &str {
        &self.alt
    }

    /// The string used to represent Shift on the platform.
    pub fn shift(&self) -> &str {
        &self.shift
    }

    /// Returns `format_provider` if given, otherwise the format info
    /// registered with the locator, otherwise the invariant one.
    pub fn get_instance(format_provider: Option<Rc<KeyGestureFormatInfo>>) -> Rc<KeyGestureFormatInfo> {
        format_provider
            .or_else(|| FerroLocator::current().get_service::<KeyGestureFormatInfo>())
            .unwrap_or_else(Self::invariant)
    }

    /// A common source of key names, irrespective of platform, used when a
    /// platform does not override the name of a key.
    fn common_key_override(key: Key) -> Option<&'static str> {
        Some(match key {
            Key::Add => "+",
            Key::D0 => "0",
            Key::D1 => "1",
            Key::D2 => "2",
            Key::D3 => "3",
            Key::D4 => "4",
            Key::D5 => "5",
            Key::D6 => "6",
            Key::D7 => "7",
            Key::D8 => "8",
            Key::D9 => "9",
            Key::Decimal => ".",
            Key::Divide => "/",
            Key::Multiply => "*",
            Key::OemBackslash => "\\",
            Key::OemCloseBrackets => "]",
            Key::OemComma => ",",
            Key::OemMinus => "-",
            Key::OemOpenBrackets => "[",
            Key::OemPeriod => ".",
            Key::OemPipe => "|",
            Key::OemPlus => "+",
            Key::OemQuestion => "/",
            Key::OemQuotes => "\"",
            Key::OemSemicolon => ";",
            Key::OemTilde => "`",
            Key::Separator => "/",
            Key::Subtract => "-",
            Key::Back => "Backspace",
            Key::Down => "Down Arrow",
            Key::Left => "Left Arrow",
            Key::Right => "Right Arrow",
            Key::Up => "Up Arrow",
            _ => return None,
        })
    }

    /// Checks the platform overrides and the common overrides, in that
    /// order, and returns the name of the key in the appropriate style, or
    /// the key's plain name if there is none.
    pub fn format_key(&self, key: Key) -> String {
        // The absence of an overrides dictionary indicates this is the
        // invariant, and so should just return the default name.
        let Some(overrides) = &self.platform_key_overrides else { return key.name().to_string() };

        match overrides.get(&key) {
            Some(result) => result.clone(),
            None => Self::common_key_override(key).unwrap_or(key.name()).to_string(),
        }
    }
}
