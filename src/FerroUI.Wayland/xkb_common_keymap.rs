//! A keymap of the compositor and its state (the port of
//! `XkbCommonKeymap.cs`).

use crate::server::interop::unsafe_native_methods::MemoryMapping;
use crate::xkb_context::XkbContext;
use crate::xkb_key_transform::IXkbKeymap;
use std::os::fd::{AsFd, OwnedFd};
use xkbcommon_dl::{xkb_keymap, xkb_keymap_compile_flags, xkb_keymap_format, xkb_keysym_t, xkb_state, xkbcommon_option};

/// Managed wrapper around `xkb_keymap` + `xkb_state` for key resolution.
/// Lives on the Wayland thread — must not be accessed from the UI thread.
pub struct XkbCommonKeymap {
    keymap: *mut xkb_keymap,
    state: *mut xkb_state,
}

impl XkbCommonKeymap {
    /// Compiles the keymap a compositor sent as a descriptor of `size` bytes of text.
    pub fn new(context: &XkbContext, fd: OwnedFd, size: u32) -> Result<Self, &'static str> {
        let xkb = xkbcommon_option().ok_or("libxkbcommon is not available")?;
        if size == 0 {
            return Err("The keymap is empty");
        }
        let mapped = MemoryMapping::private_read(fd.as_fd(), size as usize).map_err(|_| "Failed to mmap keymap fd")?;
        drop(fd);

        // size includes null terminator; xkb_keymap_new_from_buffer expects length without it
        // SAFETY: the mapping is `size` readable bytes that live until the end of this
        // function, the length given is one less; the context is alive.
        let keymap = unsafe {
            (xkb.xkb_keymap_new_from_buffer)(
                context.handle(),
                mapped.address().cast(),
                size as usize - 1,
                xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
                xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
            )
        };
        if keymap.is_null() {
            return Err("Failed to create xkb keymap from buffer");
        }

        // SAFETY: the keymap was just made and is not null.
        let state = unsafe { (xkb.xkb_state_new)(keymap) };
        if state.is_null() {
            // SAFETY: the one reference of the keymap, given up once.
            unsafe { (xkb.xkb_keymap_unref)(keymap) };
            return Err("Failed to create xkb state");
        }

        Ok(Self { keymap, state })
    }

    /// Compiles a keymap from its text. Not in the reference: what the tests and the example
    /// make a keymap with.
    pub fn from_text(context: &XkbContext, text: &str) -> Result<Self, &'static str> {
        let xkb = xkbcommon_option().ok_or("libxkbcommon is not available")?;
        // SAFETY: the buffer is `text.len()` readable bytes for the call; the context is alive.
        let keymap = unsafe {
            (xkb.xkb_keymap_new_from_buffer)(
                context.handle(),
                text.as_ptr().cast(),
                text.len(),
                xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
                xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
            )
        };
        if keymap.is_null() {
            return Err("Failed to create xkb keymap from buffer");
        }
        // SAFETY: the keymap was just made and is not null.
        let state = unsafe { (xkb.xkb_state_new)(keymap) };
        if state.is_null() {
            // SAFETY: the one reference of the keymap, given up once.
            unsafe { (xkb.xkb_keymap_unref)(keymap) };
            return Err("Failed to create xkb state");
        }
        Ok(Self { keymap, state })
    }

    /// Updates modifier and layout group state from wl_keyboard.modifiers event.
    pub fn update_modifiers(&self, mods_depressed: u32, mods_latched: u32, mods_locked: u32, group: u32) {
        let Some(xkb) = xkbcommon_option() else {
            return;
        };
        // SAFETY: the state is alive; the rest are values.
        unsafe { (xkb.xkb_state_update_mask)(self.state, mods_depressed, mods_latched, mods_locked, 0, 0, group) };
    }
}

impl IXkbKeymap for XkbCommonKeymap {
    /// Resolves an evdev keycode to a keysym and UTF-8 text using current modifier/layout state.
    fn resolve_key(&self, evdev_keycode: u32) -> (u32, Option<String>) {
        let Some(xkb) = xkbcommon_option() else {
            return (0, None);
        };
        let xkb_key = evdev_keycode + 8;

        // SAFETY: the state is alive; a key code outside the keymap gives no symbol.
        let keysym = unsafe { (xkb.xkb_state_key_get_one_sym)(self.state, xkb_key) };

        // Get UTF-8 text: first call with null to get required size
        // SAFETY: a null buffer of size zero asks for the length only.
        let needed = unsafe { (xkb.xkb_state_key_get_utf8)(self.state, xkb_key, std::ptr::null_mut(), 0) };
        if needed <= 0 {
            return (keysym, None);
        }

        // Allocate buffer (+1 for null terminator written by xkbcommon)
        let mut buffer = vec![0u8; needed as usize + 1];
        // SAFETY: the buffer has the size given, which the function does not write beyond.
        unsafe { (xkb.xkb_state_key_get_utf8)(self.state, xkb_key, buffer.as_mut_ptr().cast(), buffer.len()) };
        buffer.truncate(needed as usize);
        (keysym, Some(String::from_utf8_lossy(&buffer).into_owned()))
    }

    /// Number of layout groups in the keymap.
    fn layout_count(&self) -> u32 {
        match xkbcommon_option() {
            // SAFETY: the keymap is alive.
            Some(xkb) => unsafe { (xkb.xkb_keymap_num_layouts)(self.keymap) },
            None => 0,
        }
    }

    /// Finds a keysym for the given evdev keycode in a specific layout group (level 0).
    /// Used for non-latin keyboard fallback.
    fn find_keysym_in_layout(&self, evdev_keycode: u32, layout: u32) -> u32 {
        let Some(xkb) = xkbcommon_option() else {
            return 0;
        };
        let xkb_key = evdev_keycode + 8;
        let mut syms: *const xkb_keysym_t = std::ptr::null();
        // SAFETY: the keymap is alive; the function stores a pointer into the keymap, valid
        // while the keymap lives, and returns how many symbols are behind it.
        let count = unsafe { (xkb.xkb_keymap_key_get_syms_by_level)(self.keymap, xkb_key, layout, 0, &mut syms) };
        if count > 0 && !syms.is_null() {
            // SAFETY: at least one symbol is behind the pointer (the count says so).
            return unsafe { *syms };
        }
        0
    }
}

impl Drop for XkbCommonKeymap {
    fn drop(&mut self) {
        if let Some(xkb) = xkbcommon_option() {
            // SAFETY: the two references this value owns, each given up once, the state first.
            unsafe {
                (xkb.xkb_state_unref)(self.state);
                (xkb.xkb_keymap_unref)(self.keymap);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. They need libxkbcommon, and
    // pass without checking anything where it is missing.
    use super::*;
    use crate::xkb_key_transform::XkbKeyTransform;
    use ferroui_base::input::{Key, PhysicalKey};

    /// A keymap of two layouts, written out so that no rules have to be found: "a" and "q" on
    /// the first, Cyrillic letters on the second.
    pub(crate) const TEST_KEYMAP: &str = r#"xkb_keymap {
        xkb_keycodes "test" { minimum = 8; maximum = 255; <AD01> = 24; <AC01> = 38; <LFSH> = 50; <AE02> = 11; };
        xkb_types "test" {
            virtual_modifiers NumLock;
            type "ONE_LEVEL" { modifiers = none; map[None] = Level1; };
            type "ALPHABETIC" { modifiers = Shift+Lock; map[Shift] = Level2; map[Lock] = Level2; };
            type "TWO_LEVEL" { modifiers = Shift; map[Shift] = Level2; };
        };
        xkb_compatibility "test" {
            interpret Shift_L { action = SetMods(modifiers=Shift); };
        };
        xkb_symbols "test" {
            name[Group1] = "Latin";
            name[Group2] = "Cyrillic";
            key <AD01> { type = "ALPHABETIC", symbols[Group1] = [ q, Q ], symbols[Group2] = [ Cyrillic_shorti, Cyrillic_SHORTI ] };
            key <AC01> { type = "ALPHABETIC", symbols[Group1] = [ a, A ], symbols[Group2] = [ Cyrillic_ef, Cyrillic_EF ] };
            key <AE02> { type = "TWO_LEVEL", symbols[Group1] = [ 2, at ], symbols[Group2] = [ 2, quotedbl ] };
            key <LFSH> { type = "ONE_LEVEL", symbols[Group1] = [ Shift_L ], symbols[Group2] = [ Shift_L ] };
        };
    };"#;

    #[test]
    fn a_keymap_resolves_keys_in_its_layouts_and_with_its_modifiers() {
        let Some(context) = XkbContext::new() else {
            return;
        };
        let keymap = XkbCommonKeymap::from_text(&context, TEST_KEYMAP).expect("the keymap of the test compiles");

        assert_eq!(keymap.layout_count(), 2);
        // evdev 30 is the key of "a".
        assert_eq!(keymap.resolve_key(30), (0x61, Some("a".to_string())));
        assert_eq!(XkbKeyTransform::resolve_key_with_fallback(&keymap, 30, PhysicalKey::A), (Key::A, Some("a".to_string())));

        // Shift.
        keymap.update_modifiers(1, 0, 0, 0);
        assert_eq!(keymap.resolve_key(30), (0x41, Some("A".to_string())));
        assert_eq!(XkbKeyTransform::resolve_key_with_fallback(&keymap, 3, PhysicalKey::Digit2), (Key::D2, Some("@".to_string())));

        // The second layout: the symbol has no key, the first layout answers.
        keymap.update_modifiers(0, 0, 0, 1);
        let (keysym, text) = keymap.resolve_key(16);
        assert_eq!(keysym, 0x6ca);
        assert_eq!(text.as_deref(), Some("й"));
        assert_eq!(keymap.find_keysym_in_layout(16, 0), 0x71);
        assert_eq!(XkbKeyTransform::resolve_key_with_fallback(&keymap, 16, PhysicalKey::Q), (Key::Q, Some("й".to_string())));

        // A key the keymap does not have.
        assert_eq!(keymap.resolve_key(100), (0, None));
        assert_eq!(keymap.find_keysym_in_layout(100, 0), 0);
    }
}
