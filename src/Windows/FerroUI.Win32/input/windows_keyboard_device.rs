//! The keyboard of the Windows backend.

use crate::interop::unmanaged_methods::VirtualKeyStates;
use ferroui_base::input::RawInputModifiers;

/// The keyboard device of the Windows backend: the keyboard device of the
/// toolkit, and the state of the modifier keys as the system reports it.
///
/// In the reference this is a class that derives from the keyboard device.
/// Here the device stays the keyboard device of the base library, which is
/// what the input manager and the focus code recover from the device
/// contract; this type holds the one instance and adds the modifiers.
pub struct WindowsKeyboardDevice;

impl WindowsKeyboardDevice {
    /// The modifier keys that are down in a keyboard state of 256 virtual
    /// keys (the high bit of a key is set while it is down).
    pub fn modifiers_from_key_states(key_states: &[u8; 256]) -> RawInputModifiers {
        let down = |left: i32, right: i32| ((key_states[left as usize] | key_states[right as usize]) & 0x80) != 0;

        let mut result = RawInputModifiers::NONE;

        if down(VirtualKeyStates::VK_LMENU, VirtualKeyStates::VK_RMENU) {
            result |= RawInputModifiers::ALT;
        }

        if down(VirtualKeyStates::VK_LCONTROL, VirtualKeyStates::VK_RCONTROL) {
            result |= RawInputModifiers::CONTROL;
        }

        if down(VirtualKeyStates::VK_LSHIFT, VirtualKeyStates::VK_RSHIFT) {
            result |= RawInputModifiers::SHIFT;
        }

        if down(VirtualKeyStates::VK_LWIN, VirtualKeyStates::VK_RWIN) {
            result |= RawInputModifiers::META;
        }

        result
    }
}

#[cfg(windows)]
impl WindowsKeyboardDevice {
    /// The keyboard device of the calling thread.
    pub fn instance() -> std::rc::Rc<ferroui_base::input::KeyboardDevice> {
        thread_local! {
            static INSTANCE: std::rc::Rc<ferroui_base::input::KeyboardDevice> = ferroui_base::input::KeyboardDevice::new();
        }
        INSTANCE.with(std::rc::Rc::clone)
    }

    /// The modifier keys that are down, as the calling thread has seen
    /// them.
    pub fn modifiers() -> RawInputModifiers {
        Self::modifiers_from_key_states(&crate::interop::unmanaged_methods::get_keyboard_state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(down: &[i32]) -> [u8; 256] {
        let mut states = [0u8; 256];
        for &key in down {
            states[key as usize] = 0x80;
        }
        states
    }

    #[test]
    fn no_key_down_is_no_modifier() {
        assert_eq!(WindowsKeyboardDevice::modifiers_from_key_states(&[0u8; 256]), RawInputModifiers::NONE);
        // The low bit is the toggle state of a key, not whether it is down.
        let mut toggled = [0u8; 256];
        toggled[VirtualKeyStates::VK_LSHIFT as usize] = 0x01;
        assert_eq!(WindowsKeyboardDevice::modifiers_from_key_states(&toggled), RawInputModifiers::NONE);
    }

    #[test]
    fn either_key_of_a_pair_sets_its_modifier() {
        let cases = [
            (VirtualKeyStates::VK_LMENU, VirtualKeyStates::VK_RMENU, RawInputModifiers::ALT),
            (VirtualKeyStates::VK_LCONTROL, VirtualKeyStates::VK_RCONTROL, RawInputModifiers::CONTROL),
            (VirtualKeyStates::VK_LSHIFT, VirtualKeyStates::VK_RSHIFT, RawInputModifiers::SHIFT),
            (VirtualKeyStates::VK_LWIN, VirtualKeyStates::VK_RWIN, RawInputModifiers::META),
        ];
        for (left, right, modifier) in cases {
            assert_eq!(WindowsKeyboardDevice::modifiers_from_key_states(&states(&[left])), modifier);
            assert_eq!(WindowsKeyboardDevice::modifiers_from_key_states(&states(&[right])), modifier);
        }
    }

    #[test]
    fn modifiers_combine() {
        let modifiers = WindowsKeyboardDevice::modifiers_from_key_states(&states(&[
            VirtualKeyStates::VK_RCONTROL,
            VirtualKeyStates::VK_LSHIFT,
            VirtualKeyStates::VK_RMENU,
        ]));
        assert_eq!(modifiers, RawInputModifiers::CONTROL | RawInputModifiers::SHIFT | RawInputModifiers::ALT);
    }
}
