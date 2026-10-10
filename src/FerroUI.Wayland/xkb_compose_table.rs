//! The compose sequences of a locale (the port of `XkbComposeTable.cs`).

use crate::xkb_context::XkbContext;
use std::ffi::CString;
use xkbcommon_dl::{xkb_compose_compile_flags, xkb_compose_table, xkbcommon_compose_option};

/// Managed wrapper around `xkb_compose_table*`.
/// Created once per keyboard context from the user's locale; shared across compose states.
pub struct XkbComposeTable {
    handle: *mut xkb_compose_table,
}

impl XkbComposeTable {
    /// The table of the locale of the environment (`LC_ALL`, then `LC_CTYPE`, then `LANG`,
    /// then `C`); `None` when the locale has none or the library is missing.
    pub fn try_create(context: &XkbContext) -> Option<Self> {
        let locale = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_CTYPE"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_else(|_| "C".to_string());
        let locale = CString::new(locale).ok()?;
        let xkb = xkbcommon_compose_option()?;
        // SAFETY: the context is alive; the locale is a terminated string for the call.
        let handle = unsafe {
            (xkb.xkb_compose_table_new_from_locale)(
                context.handle(),
                locale.as_ptr(),
                xkb_compose_compile_flags::XKB_COMPOSE_COMPILE_NO_FLAGS,
            )
        };
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    pub(crate) fn handle(&self) -> *mut xkb_compose_table {
        self.handle
    }
}

impl Drop for XkbComposeTable {
    fn drop(&mut self) {
        if let Some(xkb) = xkbcommon_compose_option() {
            // SAFETY: the one reference this value owns, given up once.
            unsafe { (xkb.xkb_compose_table_unref)(self.handle) };
        }
    }
}
