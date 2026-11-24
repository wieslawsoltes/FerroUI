//! Port of `Input/PlatformKeyInfo.cs` (upstream 12.2.0, `8dae868308`).

/// Known values of [`PlatformKeyInfo::format`]. Each value identifies the platform that
/// produced a [`PlatformKeyInfo`], and so the meaning of its values. Other platforms can report
/// other formats.
pub struct KnownPlatformKeyInfoFormats;

impl KnownPlatformKeyInfoFormats {
    /// Win32 keyboard messages.
    pub const WIN32: &'static str = "Win32";
    /// X11 key events.
    pub const X11: &'static str = "X11";
    /// Wayland `wl_keyboard` events.
    pub const WAYLAND: &'static str = "Wayland";
    /// macOS `NSEvent` key events.
    pub const MACOS: &'static str = "MacOS";
}

/// Platform-specific data of a key event, for code that knows the platform, for example
/// terminal emulators that encode keys the way a native terminal does. Each value is `None`
/// when the platform does not provide it.
///
/// Deviation: the format is a `&'static str` (a backend names its format with a constant),
/// so the value is `Copy` and a key event allocates nothing for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlatformKeyInfo {
    /// The platform that produced the values, usually one of [`KnownPlatformKeyInfoFormats`].
    /// It is empty only for a default instance.
    pub format: &'static str,

    /// The scan code of the key.
    ///
    /// Win32: the hardware scan code, plus `0xE000` for extended keys. X11 and Wayland: the XKB
    /// keycode (evdev code + 8). macOS: `NSEvent.keyCode`, which identifies the physical key on
    /// every layout.
    pub scan_code: Option<u32>,

    /// The virtual key.
    ///
    /// Win32: the virtual-key code. X11 and Wayland: the keysym. macOS: `None`, because macOS
    /// has no layout-dependent key code (see [`scan_code`](Self::scan_code)).
    pub virtual_key: Option<u32>,

    /// The modifier state.
    ///
    /// Win32: console `dwControlKeyState` flags (`LEFT_CTRL_PRESSED`, `RIGHT_ALT_PRESSED`,
    /// `ENHANCED_KEY`, ...), including the key being pressed. X11: `XKeyEvent.state`, the state
    /// before the event. Wayland: the effective XKB modifier mask, the state before the event.
    /// macOS: `NSEventModifierFlags`. Only the core XKB modifiers (Shift, Lock, Control, Mod1 to
    /// Mod5) have fixed bits; what Mod1 to Mod5 mean depends on the XKB keymap.
    pub modifiers: Option<u64>,
}

impl PlatformKeyInfo {
    /// A value of the platform `format` with no data yet.
    pub const fn new(format: &'static str) -> Self {
        Self { format, scan_code: None, virtual_key: None, modifiers: None }
    }

    /// Sets [`scan_code`](Self::scan_code).
    pub const fn with_scan_code(mut self, scan_code: u32) -> Self {
        self.scan_code = Some(scan_code);
        self
    }

    /// Sets [`virtual_key`](Self::virtual_key).
    pub const fn with_virtual_key(mut self, virtual_key: u32) -> Self {
        self.virtual_key = Some(virtual_key);
        self
    }

    /// Sets [`modifiers`](Self::modifiers).
    pub const fn with_modifiers(mut self, modifiers: u64) -> Self {
        self.modifiers = Some(modifiers);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_value_has_only_its_format() {
        let info = PlatformKeyInfo::new(KnownPlatformKeyInfoFormats::X11);
        assert_eq!(info.format, "X11");
        assert_eq!((info.scan_code, info.virtual_key, info.modifiers), (None, None, None));
        assert_eq!(PlatformKeyInfo::default().format, "");
    }

    #[test]
    fn the_builders_set_each_value() {
        let info = PlatformKeyInfo::new(KnownPlatformKeyInfoFormats::WIN32)
            .with_scan_code(0xE01D)
            .with_virtual_key(0x11)
            .with_modifiers(0x0108);
        assert_eq!(info.scan_code, Some(0xE01D));
        assert_eq!(info.virtual_key, Some(0x11));
        assert_eq!(info.modifiers, Some(0x0108));
        assert_ne!(info, PlatformKeyInfo::new(KnownPlatformKeyInfoFormats::WIN32));
    }
}
