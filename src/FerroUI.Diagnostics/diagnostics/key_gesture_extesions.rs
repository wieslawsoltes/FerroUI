//! Port of `Diagnostics/KeyGestureExtesions.cs` (the misspelt name is upstream's).

use ferroui_base::input::raw::RawKeyEventArgs;
use ferroui_base::input::{Key, KeyGesture, KeyModifiers, RawInputModifiers};

/// Extension methods of [`KeyGesture`] for raw key events.
pub trait KeyGestureExtesions {
    /// C# `Matches(this KeyGesture gesture, RawKeyEventArgs keyEvent)`: whether a raw key event
    /// matches the gesture: same keyboard modifiers and the same key, where the numpad operation
    /// keys match their regular counterparts. Named `matches_raw` because [`KeyGesture::matches`]
    /// is the overload for a routed key event.
    fn matches_raw(&self, key_event: &RawKeyEventArgs) -> bool;
}

impl KeyGestureExtesions for KeyGesture {
    fn matches_raw(&self, key_event: &RawKeyEventArgs) -> bool {
        KeyModifiers::from_bits_truncate((key_event.modifiers() & RawInputModifiers::KEYBOARD_MASK).bits())
            == self.key_modifiers()
            && resolve_num_pad_operation_key(key_event.key()) == resolve_num_pad_operation_key(self.key())
    }
}

fn resolve_num_pad_operation_key(key: Key) -> Key {
    match key {
        Key::Add => Key::OemPlus,
        Key::Subtract => Key::OemMinus,
        Key::Decimal => Key::OemPeriod,
        _ => key,
    }
}
