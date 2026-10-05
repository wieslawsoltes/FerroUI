use super::platform::KeyGestureFormatInfo;
use super::{Key, KeyEventArgs, KeyModifiers, ParseKeyModifiersError};
use std::rc::Rc;
use std::fmt;
use std::str::FromStr;

/// Defines a keyboard input combination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyGesture {
    key: Key,
    key_modifiers: KeyModifiers,
}

impl KeyGesture {
    /// Creates a gesture for a key with modifiers.
    pub const fn new(key: Key, modifiers: KeyModifiers) -> Self {
        Self { key, key_modifiers: modifiers }
    }

    /// Creates a gesture for a key without modifiers.
    pub const fn from_key(key: Key) -> Self {
        Self { key, key_modifiers: KeyModifiers::NONE }
    }

    /// The key of the gesture.
    #[inline]
    pub const fn key(&self) -> Key {
        self.key
    }

    /// The modifier keys of the gesture.
    #[inline]
    pub const fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }

    /// Parses a gesture such as `Ctrl+Shift+A`.
    ///
    /// Parts are separated by `+`. The last part that names a key decides
    /// the key (`+`, `-`, `.` and `,` name the corresponding OEM keys);
    /// every other part must be a modifier: `Ctrl`/`Control`, `Shift`,
    /// `Alt`, `Meta`/`Cmd`/`Win`/`⌘`.
    pub fn parse(gesture: &str) -> Result<KeyGesture, ParseKeyModifiersError> {
        // A plain split can't be used here because "Ctrl++" is a perfectly
        // valid key gesture.
        let mut key = Key::None;
        let mut key_modifiers = KeyModifiers::NONE;
        let mut cstart = 0;

        for (c, ch) in gesture.char_indices().chain(std::iter::once((gesture.len(), '\0'))) {
            let is_last = c == gesture.len();

            if is_last || (ch == '+' && cstart != c) {
                let part = gesture[cstart..c].trim();

                match Self::try_parse_key(part) {
                    Some(parsed) => key = parsed,
                    None => {
                        // A part that is not a key resets the key, as the
                        // failed key parse does upstream.
                        key = Key::None;
                        key_modifiers |= Self::parse_modifier(part)?;
                    }
                }

                cstart = c + 1;
            }
        }

        Ok(KeyGesture::new(key, key_modifiers))
    }

    /// Whether a key event matches the gesture: same modifiers and the
    /// same key, where the numpad operation keys match their regular
    /// counterparts.
    pub fn matches(&self, key_event: Option<&KeyEventArgs>) -> bool {
        key_event.is_some_and(|key_event| {
            key_event.key_modifiers == self.key_modifiers
                && Self::resolve_num_pad_operation_key(key_event.key) == Self::resolve_num_pad_operation_key(self.key)
        })
    }

    fn try_parse_key(key_str: &str) -> Option<Key> {
        match key_str {
            "+" => return Some(Key::OemPlus),
            "-" => return Some(Key::OemMinus),
            "." => return Some(Key::OemPeriod),
            "," => return Some(Key::OemComma),
            _ => {}
        }

        Key::parse(key_str).ok()
    }

    fn parse_modifier(modifier: &str) -> Result<KeyModifiers, ParseKeyModifiersError> {
        if modifier.eq_ignore_ascii_case("ctrl") {
            return Ok(KeyModifiers::CONTROL);
        }

        if modifier.eq_ignore_ascii_case("cmd") || modifier.eq_ignore_ascii_case("win") || modifier == "⌘" {
            return Ok(KeyModifiers::META);
        }

        KeyModifiers::parse(modifier)
    }

    fn resolve_num_pad_operation_key(key: Key) -> Key {
        match key {
            Key::Add => Key::OemPlus,
            Key::Subtract => Key::OemMinus,
            Key::Decimal => Key::OemPeriod,
            _ => key,
        }
    }
}

impl FromStr for KeyGesture {
    type Err = ParseKeyModifiersError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl KeyGesture {
    /// Formats the gesture with the platform's names for keys and
    /// modifiers (the `p` format): the given format info, or the one
    /// registered with the locator, or the invariant one.
    pub fn to_platform_string(&self, format_provider: Option<Rc<KeyGestureFormatInfo>>) -> String {
        self.format_with(&KeyGestureFormatInfo::get_instance(format_provider))
    }

    /// Formats the gesture with the given format info.
    pub fn format_with(&self, format_info: &KeyGestureFormatInfo) -> String {
        let mut s = String::new();
        let mut part = |text: &str| {
            if !s.is_empty() {
                s.push('+');
            }
            s.push_str(text);
        };

        if self.key_modifiers.contains(KeyModifiers::CONTROL) {
            part(format_info.ctrl());
        }

        if self.key_modifiers.contains(KeyModifiers::SHIFT) {
            part(format_info.shift());
        }

        if self.key_modifiers.contains(KeyModifiers::ALT) {
            part(format_info.alt());
        }

        if self.key_modifiers.contains(KeyModifiers::META) {
            part(format_info.meta());
        }

        if self.key != Key::None || self.key_modifiers == KeyModifiers::NONE {
            part(&format_info.format_key(self.key));
        }

        s
    }
}

/// Formats the gesture in the invariant format, e.g. `Ctrl+Shift+A`.
impl fmt::Display for KeyGesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut any = false;
        let mut part = |f: &mut fmt::Formatter<'_>, s: &str| -> fmt::Result {
            if any {
                f.write_str("+")?;
            }
            any = true;
            f.write_str(s)
        };

        if self.key_modifiers.contains(KeyModifiers::CONTROL) {
            part(f, "Ctrl")?;
        }

        if self.key_modifiers.contains(KeyModifiers::SHIFT) {
            part(f, "Shift")?;
        }

        if self.key_modifiers.contains(KeyModifiers::ALT) {
            part(f, "Alt")?;
        }

        if self.key_modifiers.contains(KeyModifiers::META) {
            part(f, "Cmd")?;
        }

        if self.key != Key::None || self.key_modifiers == KeyModifiers::NONE {
            part(f, self.key.name())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_gesture_is_able_to_parse_sample_data() {
        let data = [
            ("Ctrl+A", KeyGesture::new(Key::A, KeyModifiers::CONTROL)),
            ("  \tShift\t+Alt +B", KeyGesture::new(Key::B, KeyModifiers::SHIFT | KeyModifiers::ALT)),
            ("Control++", KeyGesture::new(Key::OemPlus, KeyModifiers::CONTROL)),
            ("Shift+⌘+A", KeyGesture::new(Key::A, KeyModifiers::META | KeyModifiers::SHIFT)),
            ("Shift+Cmd+A", KeyGesture::new(Key::A, KeyModifiers::META | KeyModifiers::SHIFT)),
            ("None", KeyGesture::from_key(Key::None)),
            ("Alt+Shift", KeyGesture::new(Key::None, KeyModifiers::ALT | KeyModifiers::SHIFT)),
        ];

        for (text, gesture) in data {
            assert_eq!(KeyGesture::parse(text), Ok(gesture), "{text}");
        }
    }

    #[test]
    fn key_gesture_parse_rejects_unknown_parts() {
        assert!(KeyGesture::parse("Ctrl+NotAKey").is_err());
        assert!("Bogus+A".parse::<KeyGesture>().is_err());
    }

    #[test]
    fn key_gesture_matches_num_pad_to_regular_digit() {
        let data = [(Key::OemMinus, Key::Subtract), (Key::OemPlus, Key::Add), (Key::OemPeriod, Key::Decimal)];

        for (gesture_key, pressed_key) in data {
            let key_gesture = KeyGesture::from_key(gesture_key);
            let mut args = KeyEventArgs::new();
            args.key = pressed_key;

            assert!(key_gesture.matches(Some(&args)));
        }

        assert!(!KeyGesture::from_key(Key::A).matches(None));
    }

    #[test]
    fn platform_format_uses_overrides() {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert(Key::Return, "\u{21A9}".to_string());
        let info = KeyGestureFormatInfo::new(Some(overrides), "\u{2318}", "\u{2303}", "\u{2325}", "\u{21E7}");

        assert_eq!(KeyGesture::new(Key::Return, KeyModifiers::META).format_with(&info), "\u{2318}+\u{21A9}");
        assert_eq!(KeyGesture::new(Key::OemPlus, KeyModifiers::CONTROL).format_with(&info), "\u{2303}++");
        assert_eq!(KeyGesture::new(Key::Left, KeyModifiers::SHIFT).format_with(&info), "\u{21E7}+Left Arrow");
        assert_eq!(KeyGesture::from_key(Key::A).format_with(&info), "A");

        // Without a registered format info the platform format is the
        // invariant one.
        let gesture = KeyGesture::new(Key::OemPlus, KeyModifiers::CONTROL | KeyModifiers::META);
        assert_eq!(gesture.to_platform_string(None), gesture.to_string());
        assert_eq!(gesture.to_string(), "Ctrl+Cmd+OemPlus");
    }

    #[test]
    fn to_string_produces_correct_results() {
        let data = [
            (KeyGesture::from_key(Key::A), "A"),
            (KeyGesture::new(Key::A, KeyModifiers::CONTROL), "Ctrl+A"),
            (KeyGesture::new(Key::A, KeyModifiers::CONTROL | KeyModifiers::SHIFT), "Ctrl+Shift+A"),
            (KeyGesture::new(Key::A, KeyModifiers::ALT | KeyModifiers::SHIFT), "Shift+Alt+A"),
            (
                KeyGesture::new(Key::A, KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT),
                "Ctrl+Shift+Alt+A",
            ),
            (KeyGesture::new(Key::A, KeyModifiers::META | KeyModifiers::SHIFT), "Shift+Cmd+A"),
            (KeyGesture::from_key(Key::None), "None"),
            (KeyGesture::new(Key::None, KeyModifiers::ALT | KeyModifiers::SHIFT), "Shift+Alt"),
        ];

        for (gesture, expected) in data {
            assert_eq!(gesture.to_string(), expected);
        }
    }
}
