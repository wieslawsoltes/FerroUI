use super::{IKeyModifiersEventArgs, Key, KeyDeviceType, KeyModifiers, PhysicalKey};
use crate::ferro_routed_event_args;
use crate::interactivity::RoutedEventArgs;

/// Provides information specific to a keyboard event.
#[derive(Clone, Default)]
pub struct KeyEventArgs {
    base: RoutedEventArgs,

    /// The virtual-key for the associated event.
    ///
    /// A given physical key can result in different virtual keys depending
    /// on the current keyboard layout. This is the key that is generally
    /// referred to when creating keyboard shortcuts. For example, when
    /// pressing the key located at the `Z` position on standard US English
    /// QWERTY keyboard, this property returns:
    ///
    /// - `Key::Z` for an English (QWERTY) layout
    /// - `Key::W` for a French (AZERTY) layout
    /// - `Key::Y` for a German (QWERTZ) layout
    /// - `Key::Z` for a Russian (JCUKEN) layout
    pub key: Key,

    /// The key modifiers for the associated event.
    pub key_modifiers: KeyModifiers,

    /// The physical key for the associated event.
    ///
    /// This value is independent of the current keyboard layout and usually
    /// corresponds to the key printed on a standard US English QWERTY
    /// keyboard. This is the key to use for position-dependent bindings
    /// such as WASD movement in games.
    pub physical_key: PhysicalKey,

    /// The unicode symbol of the key, or `None` if none is applicable.
    ///
    /// For example, when pressing the key located at the `Z` position on
    /// standard US English QWERTY keyboard, this property returns:
    ///
    /// - `z` for an English (QWERTY) layout
    /// - `w` for a French (AZERTY) layout
    /// - `y` for a German (QWERTZ) layout
    /// - `я` for a Russian (JCUKEN) layout
    pub key_symbol: Option<String>,

    /// The type of key device that sent this event.
    pub key_device_type: KeyDeviceType,
}

ferro_routed_event_args!(KeyEventArgs: RoutedEventArgs);

impl KeyEventArgs {
    /// Creates args with default values and no routed event.
    pub fn new() -> Self {
        Self::default()
    }
}

impl IKeyModifiersEventArgs for KeyEventArgs {
    fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }
}
