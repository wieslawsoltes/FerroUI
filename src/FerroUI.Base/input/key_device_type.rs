/// Enumerates key device types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyDeviceType {
    /// The key device is a keyboard.
    #[default]
    Keyboard,
    /// The key device is a gamepad.
    Gamepad,
    /// The key device is a remote control.
    Remote,
}
