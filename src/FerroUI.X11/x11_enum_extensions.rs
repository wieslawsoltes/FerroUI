//! Conversions of protocol values (the port of `X11EnumExtensions.cs`).

use crate::x11_enums::XModifierMask;
use ferroui_base::input::RawInputModifiers;

/// The conversions of [`XModifierMask`].
pub trait X11EnumExtensions {
    /// The pressed buttons and modifier keys of a core event state.
    fn to_raw_input_modifiers(self) -> RawInputModifiers;
}

impl X11EnumExtensions for XModifierMask {
    fn to_raw_input_modifiers(self) -> RawInputModifiers {
        let state = self;
        let mut rv = RawInputModifiers::empty();
        if state.contains(XModifierMask::BUTTON1_MASK) {
            rv |= RawInputModifiers::LEFT_MOUSE_BUTTON;
        }
        // As the reference: the second button of the protocol (the middle
        // one) is reported as the right button and the third as the
        // middle one. Pointer input normally arrives through the X Input
        // extension, whose button state is read the other way round.
        if state.contains(XModifierMask::BUTTON2_MASK) {
            rv |= RawInputModifiers::RIGHT_MOUSE_BUTTON;
        }
        if state.contains(XModifierMask::BUTTON3_MASK) {
            rv |= RawInputModifiers::MIDDLE_MOUSE_BUTTON;
        }
        if state.contains(XModifierMask::BUTTON4_MASK) {
            rv |= RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON;
        }
        if state.contains(XModifierMask::BUTTON5_MASK) {
            rv |= RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON;
        }
        if state.contains(XModifierMask::SHIFT_MASK) {
            rv |= RawInputModifiers::SHIFT;
        }
        if state.contains(XModifierMask::CONTROL_MASK) {
            rv |= RawInputModifiers::CONTROL;
        }
        if state.contains(XModifierMask::MOD1_MASK) {
            rv |= RawInputModifiers::ALT;
        }
        if state.contains(XModifierMask::MOD4_MASK) {
            rv |= RawInputModifiers::META;
        }
        rv
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn modifier_keys_and_buttons_are_mapped() {
        assert_eq!(XModifierMask::empty().to_raw_input_modifiers(), RawInputModifiers::empty());
        assert_eq!(
            (XModifierMask::SHIFT_MASK | XModifierMask::CONTROL_MASK | XModifierMask::MOD1_MASK | XModifierMask::MOD4_MASK)
                .to_raw_input_modifiers(),
            RawInputModifiers::SHIFT | RawInputModifiers::CONTROL | RawInputModifiers::ALT | RawInputModifiers::META
        );
        assert_eq!(XModifierMask::BUTTON1_MASK.to_raw_input_modifiers(), RawInputModifiers::LEFT_MOUSE_BUTTON);
        // The quirk of the reference: buttons two and three are swapped.
        assert_eq!(XModifierMask::BUTTON2_MASK.to_raw_input_modifiers(), RawInputModifiers::RIGHT_MOUSE_BUTTON);
        assert_eq!(XModifierMask::BUTTON3_MASK.to_raw_input_modifiers(), RawInputModifiers::MIDDLE_MOUSE_BUTTON);
        assert_eq!(XModifierMask::BUTTON4_MASK.to_raw_input_modifiers(), RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON);
        assert_eq!(XModifierMask::BUTTON5_MASK.to_raw_input_modifiers(), RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON);
        // Lock keys and the other modifiers are not reported.
        assert_eq!(
            (XModifierMask::LOCK_MASK | XModifierMask::MOD2_MASK | XModifierMask::MOD3_MASK | XModifierMask::MOD5_MASK)
                .to_raw_input_modifiers(),
            RawInputModifiers::empty()
        );
    }
}
