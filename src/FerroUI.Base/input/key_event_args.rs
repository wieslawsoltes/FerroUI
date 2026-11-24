use super::{IKeyModifiersEventArgs, Key, KeyDeviceType, KeyModifiers, PhysicalKey, PlatformKeyInfo};
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

    /// The text that the platform attached to this key event, with all active modifiers
    /// applied, when it is available. It includes C0 control characters and DEL.
    ///
    /// For example, Ctrl+Z produces `"\x1a"` on most layouts. On a German layout, AltGr+Q
    /// produces `"@"` on Windows and Linux, and Option+L produces `"@"` on macOS.
    ///
    /// It is meant for encoding keys, for example in terminal emulators. It is not a
    /// replacement for the text input event:
    ///
    /// - `None` does not mean that no text input follows. Some platforms do not provide the
    ///   value, and keys that an input method handles have none.
    /// - It is not a preview of, or a guarantee for, the committed text.
    /// - It can contain several characters, surrogate pairs and NUL characters.
    /// - The result of a dead key or compose sequence depends on the platform: some platforms
    ///   report the composed text, others the text before the input method handles the event.
    ///
    /// The value comes from the `WM_CHAR` messages of the key on Windows, `XLookupString` on
    /// X11, `xkb_state_key_get_utf8` on Wayland and `NSEvent.characters` on macOS. On macOS,
    /// Option types characters, so Option+Q on a German layout gives `"«"`. It is only set for
    /// the key down event.
    pub key_text: Option<String>,

    /// An alternate text of the key, for chords where [`key_text`](Self::key_text) is not the
    /// key's character: the text with Ctrl, Alt and Meta ignored and the other modifiers
    /// applied, or `None` if the key has no text. Like `NSEvent.charactersIgnoringModifiers`
    /// on macOS, the value keeps Shift.
    ///
    /// For example, Ctrl+Z produces `"z"`, Ctrl+Shift+2 on a US layout produces `"@"`, Ctrl+Я
    /// on a Russian layout produces `"я"`, AltGr+Q on a German layout produces `"@"` on Windows
    /// and Linux, and Option+B on macOS produces `"b"`.
    ///
    /// - Shift, Caps Lock and AltGr (ISO level 3) are applied.
    /// - Ctrl, Alt and Meta are ignored, except where they select the key's level. On Windows,
    ///   AltGr is reported as Ctrl+Alt, and a Ctrl+Alt chord that produces printable text gives
    ///   that text. A layout can also use Ctrl or Alt to select a level, and then the text is
    ///   the text of that level.
    /// - On macOS, Option counts as Alt and is always ignored, so a terminal can use it as a
    ///   Meta key: Option+Q on a German layout gives `"q"`. Use
    ///   [`key_text`](Self::key_text) for the character that Option types (`"«"`).
    ///
    /// A terminal emulator can use it to send `ESC` and the text for Alt chords, or a C0
    /// control character for Ctrl chords that [`key_text`](Self::key_text) does not cover.
    /// [`consumed_modifiers`](Self::consumed_modifiers) tells which of the pressed modifiers
    /// the layout used. It is only set for the key down event.
    pub alternate_key_text: Option<String>,

    /// The pressed modifiers that the keyboard layout used to produce the key's text, or
    /// `None` if the platform does not report them.
    ///
    /// For example, Shift is consumed for Shift+2 (`"@"`) on a US layout, and Ctrl and Alt are
    /// consumed for AltGr+Q (`"@"`) on a German layout on Windows. Ctrl is not consumed for
    /// Ctrl+Z. `key_modifiers & !consumed_modifiers` gives the modifiers that are left for the
    /// application, for example to decide whether a key types text or is a shortcut.
    ///
    /// On X11 and Wayland, the value follows the GTK rule (`XKB_CONSUMED_MODE_GTK` of
    /// xkbcommon): a modifier is consumed only when it changes the key's keysym. On macOS,
    /// Option is consumed when it changes the character, and Ctrl and Command are never
    /// consumed. On Windows, a modifier is consumed when releasing it changes the key's
    /// character in the layout. It is only set for the key down event.
    pub consumed_modifiers: Option<KeyModifiers>,

    /// Platform-specific data of the key event, or `None` if the platform provides none.
    pub platform_key_info: Option<PlatformKeyInfo>,
}

ferro_routed_event_args!(KeyEventArgs: RoutedEventArgs);

impl KeyEventArgs {
    /// Creates args with default values and no routed event.
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyEventArgs {
    /// The modifiers left for the application: [`key_modifiers`](Self::key_modifiers) without
    /// the [`consumed_modifiers`](Self::consumed_modifiers) (all of them when the platform
    /// does not report the consumed ones).
    pub fn unconsumed_modifiers(&self) -> KeyModifiers {
        self.key_modifiers & !self.consumed_modifiers.unwrap_or(KeyModifiers::NONE)
    }
}

impl IKeyModifiersEventArgs for KeyEventArgs {
    fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }
}
