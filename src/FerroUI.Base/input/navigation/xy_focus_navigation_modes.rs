use bitflags::bitflags;

bitflags! {
    /// Any combination of key device types allowed to move focus with
    /// directional (XY) navigation.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct XYFocusNavigationModes: i32 {
        /// Keyboard arrow keys can be used for 2D directional navigation.
        const KEYBOARD = 1;
        /// Gamepad controller DPad keys can be used for 2D directional
        /// navigation.
        const GAMEPAD = 2;
        /// Remote controller DPad keys can be used for 2D directional
        /// navigation.
        const REMOTE = 4;
        /// All key device XY navigation is enabled.
        const ENABLED = Self::GAMEPAD.bits() | Self::REMOTE.bits() | Self::KEYBOARD.bits();
    }
}

impl XYFocusNavigationModes {
    /// Any key device XY navigation is disabled.
    pub const DISABLED: XYFocusNavigationModes = XYFocusNavigationModes::empty();
}
