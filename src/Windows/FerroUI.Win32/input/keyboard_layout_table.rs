//! Port of `Input/KeyboardLayoutTable.cs` (upstream 12.2.0, `8dae868308` and `5ef345810e`).

use ferroui_base::input::RawInputModifiers;

use super::key_interop::KeyInterop;

const SHIFT: usize = 1;
const CONTROL: usize = 2;
const ALT: usize = 4;
const CAPS_LOCK: usize = 8;
const STATE_COUNT: usize = 16;

/// The text that a keyboard layout produces for each virtual key with each combination of
/// Shift, Ctrl, Alt and Caps Lock, read once per layout with `ToUnicodeEx`. Lookups do not touch
/// the keyboard state.
///
/// Deviation: a table is built on the UI thread, at the first key of its layout that needs it,
/// not for every installed layout on a background thread when the platform starts (no start-up
/// work, no thread). Building reads 255 keys in 16 states once.
pub(crate) struct KeyboardLayoutTable {
    entries: Box<[Option<Box<str>>]>,
}

impl KeyboardLayoutTable {
    /// A table of `entries`: `(virtual key, state, text)`, the state a combination of the bits
    /// Shift 1, Control 2, Alt 4 and Caps Lock 8.
    #[cfg_attr(windows, allow(dead_code))]
    pub(crate) fn from_entries(entries: &[(u8, usize, &str)]) -> Self {
        let mut table = Self::empty();
        for &(virtual_key, state, text) in entries {
            table.entries[usize::from(virtual_key) * STATE_COUNT + state] = Some(text.into());
        }
        table
    }

    fn empty() -> Self {
        Self { entries: vec![None; 256 * STATE_COUNT].into_boxed_slice() }
    }

    /// The text of a key with Shift and Caps Lock applied, Ctrl and Alt ignored. A dead key gives
    /// its dead character.
    pub(crate) fn text_ignoring_control_and_alt(&self, virtual_key: i32, shift: bool, caps_lock: bool) -> Option<&str> {
        self.entry(virtual_key, if shift { SHIFT } else { 0 } | if caps_lock { CAPS_LOCK } else { 0 })
    }

    /// The pressed modifiers that select the level of the key: a modifier is consumed if releasing
    /// it changes the character of the key. Control characters do not count as a level.
    pub(crate) fn consumed_modifiers(
        &self,
        virtual_key: i32,
        pressed: RawInputModifiers,
        caps_lock: bool,
    ) -> RawInputModifiers {
        let mut state = 0;
        for (modifier, bit) in
            [(RawInputModifiers::SHIFT, SHIFT), (RawInputModifiers::CONTROL, CONTROL), (RawInputModifiers::ALT, ALT)]
        {
            if pressed.contains(modifier) {
                state |= bit;
            }
        }
        if caps_lock {
            state |= CAPS_LOCK;
        }
        let level = self.level(virtual_key, state);
        let mut consumed = RawInputModifiers::NONE;
        for (modifier, bit) in
            [(RawInputModifiers::SHIFT, SHIFT), (RawInputModifiers::CONTROL, CONTROL), (RawInputModifiers::ALT, ALT)]
        {
            if state & bit != 0 && self.level(virtual_key, state & !bit) != level {
                consumed |= modifier;
            }
        }
        consumed
    }

    fn level(&self, virtual_key: i32, state: usize) -> Option<&str> {
        let text = self.entry(virtual_key, state);
        if KeyInterop::is_printable(text) {
            text
        } else {
            self.entry(virtual_key, state & !(CONTROL | ALT))
        }
    }

    fn entry(&self, virtual_key: i32, state: usize) -> Option<&str> {
        match usize::try_from(virtual_key) {
            Ok(virtual_key @ 1..=255) => self.entries[virtual_key * STATE_COUNT + state].as_deref(),
            _ => None,
        }
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use crate::interop::unmanaged_methods::{get_keyboard_layout, map_virtual_key_ex, to_unicode_ex};
    use crate::platform_constants::PlatformConstants;
    use crate::win32_platform::Win32Platform;
    use std::collections::HashMap;
    use std::rc::Rc;

    const VK_SHIFT: usize = 0x10;
    const VK_CONTROL: usize = 0x11;
    const VK_MENU: usize = 0x12;
    const VK_CAPITAL: usize = 0x14;
    const VK_LSHIFT: usize = 0xA0;
    const VK_LCONTROL: usize = 0xA2;
    const VK_LMENU: usize = 0xA4;

    ferroui_base::ferro_thread_local! {
        /// The tables by layout; `None` for a layout that could not be read.
        static TABLES: std::cell::RefCell<HashMap<isize, Option<Rc<KeyboardLayoutTable>>>> =
            std::cell::RefCell::new(HashMap::new());
    }

    impl KeyboardLayoutTable {
        /// The table of a layout, or `None` if it has to wait for a pending dead key.
        pub(crate) fn get(layout: isize) -> Option<Rc<KeyboardLayoutTable>> {
            if let Some(table) = TABLES.with(|tables| tables.borrow().get(&layout).cloned()) {
                return table;
            }
            // A pending dead key would combine with every entry, and ending it would lose the key
            // just pressed, so the table is built on a later call. Older versions cannot check
            // without ending it.
            if Win32Platform::windows_version() >= PlatformConstants::WINDOWS10_1607
                && KeyInterop::is_dead_key_pending(layout)
            {
                return None;
            }
            let table = Some(Rc::new(Self::build(layout)));
            TABLES.with(|tables| tables.borrow_mut().insert(layout, table.clone()));
            table
        }

        /// The table of the layout of the calling thread.
        pub(crate) fn current() -> Option<Rc<KeyboardLayoutTable>> {
            Self::get(get_keyboard_layout(0))
        }

        fn build(layout: isize) -> Self {
            const DO_NOT_CHANGE_KEYBOARD_STATE: u32 = 1 << 2;
            // These versions ignore the flag above, so every call can change the dead-key state.
            let changes_state = Win32Platform::windows_version() < PlatformConstants::WINDOWS10_1607;
            let mut table = Self::empty();
            if changes_state {
                // A pending dead key would combine with every entry.
                KeyInterop::end_pending_dead_key(layout);
            }
            for virtual_key in 1..256usize {
                let scan_code = map_virtual_key_ex(virtual_key as u32, 0, layout);
                if scan_code == 0 {
                    continue;
                }
                for state in 0..STATE_COUNT {
                    let mut key_states = [0u8; 256];
                    if state & SHIFT != 0 {
                        key_states[VK_SHIFT] = 0x80;
                        key_states[VK_LSHIFT] = 0x80;
                    }
                    if state & CONTROL != 0 {
                        key_states[VK_CONTROL] = 0x80;
                        key_states[VK_LCONTROL] = 0x80;
                    }
                    if state & ALT != 0 {
                        key_states[VK_MENU] = 0x80;
                        key_states[VK_LMENU] = 0x80;
                    }
                    if state & CAPS_LOCK != 0 {
                        key_states[VK_CAPITAL] = 0x01;
                    }
                    key_states[virtual_key] = 0x80;
                    let (length, buffer) = to_unicode_ex(
                        virtual_key as u32,
                        scan_code,
                        &key_states,
                        DO_NOT_CHANGE_KEYBOARD_STATE,
                        layout,
                    );
                    if length != 0 {
                        let count = (length.unsigned_abs() as usize).min(buffer.len());
                        table.entries[virtual_key * STATE_COUNT + state] =
                            Some(String::from_utf16_lossy(&buffer[..count]).into_boxed_str());
                    }
                    // A second press of a dead key clears the dead state.
                    if length < 0 && changes_state {
                        to_unicode_ex(virtual_key as u32, scan_code, &key_states, 0, layout);
                    }
                }
            }
            table
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VK_2: u8 = 0x32;
    const VK_Q: u8 = 0x51;
    const VK_Z: u8 = 0x5A;

    /// A German layout in part: AltGr (Ctrl+Alt) gives `@` on Q, Shift gives `"` on 2, Ctrl+Z
    /// gives a control character.
    fn german() -> KeyboardLayoutTable {
        KeyboardLayoutTable::from_entries(&[
            (VK_Q, 0, "q"),
            (VK_Q, SHIFT, "Q"),
            (VK_Q, CONTROL, "\u{11}"),
            (VK_Q, CONTROL | ALT, "@"),
            (VK_Q, CAPS_LOCK, "Q"),
            (VK_2, 0, "2"),
            (VK_2, SHIFT, "\""),
            (VK_Z, 0, "z"),
            (VK_Z, SHIFT, "Z"),
            (VK_Z, CONTROL, "\u{1a}"),
        ])
    }

    #[test]
    fn the_text_ignores_control_and_alt_and_keeps_shift_and_caps_lock() {
        let table = german();
        assert_eq!(table.text_ignoring_control_and_alt(VK_Q as i32, false, false), Some("q"));
        assert_eq!(table.text_ignoring_control_and_alt(VK_Q as i32, true, false), Some("Q"));
        assert_eq!(table.text_ignoring_control_and_alt(VK_Q as i32, false, true), Some("Q"));
        assert_eq!(table.text_ignoring_control_and_alt(0, false, false), None);
        assert_eq!(table.text_ignoring_control_and_alt(300, false, false), None);
    }

    #[test]
    fn a_modifier_is_consumed_when_releasing_it_changes_the_character() {
        let table = german();
        let (shift, ctrl, alt) = (RawInputModifiers::SHIFT, RawInputModifiers::CONTROL, RawInputModifiers::ALT);
        // AltGr+Q: Ctrl and Alt select `@`.
        assert_eq!(table.consumed_modifiers(VK_Q as i32, ctrl | alt, false), ctrl | alt);
        // Shift+2: Shift selects `"`.
        assert_eq!(table.consumed_modifiers(VK_2 as i32, shift, false), shift);
        // Ctrl+Z: a control character is no level, so Ctrl is not consumed.
        assert_eq!(table.consumed_modifiers(VK_Z as i32, ctrl, false), RawInputModifiers::NONE);
        // Ctrl+Shift+Z: only Shift.
        assert_eq!(table.consumed_modifiers(VK_Z as i32, ctrl | shift, false), shift);
        // A key the layout does not have.
        assert_eq!(table.consumed_modifiers(0x7B, shift, false), RawInputModifiers::NONE);
    }
}
