//! A context of `libxkbcommon` (the port of `XkbContext.cs`).

use xkbcommon_dl::{xkb_context, xkb_context_flags, xkbcommon_option};

/// Managed wrapper around an `xkb_context*`.
///
/// The library is opened at run time. The reference fails without it (the
/// runtime cannot load the library); here a context cannot be made then, and
/// keys are translated without a keymap.
pub struct XkbContext {
    handle: *mut xkb_context,
}

impl XkbContext {
    /// A context, or `None` when the library is missing or refuses.
    pub fn new() -> Option<Self> {
        let xkb = xkbcommon_option()?;
        // SAFETY: the function takes flags only; the library stays loaded.
        let handle = unsafe { (xkb.xkb_context_new)(xkb_context_flags::XKB_CONTEXT_NO_FLAGS) };
        if handle.is_null() {
            return None;
        }
        Some(Self { handle })
    }

    pub(crate) fn handle(&self) -> *mut xkb_context {
        self.handle
    }
}

impl Drop for XkbContext {
    fn drop(&mut self) {
        if let Some(xkb) = xkbcommon_option() {
            // SAFETY: the handle is the one reference this value owns, given up once.
            unsafe { (xkb.xkb_context_unref)(self.handle) };
        }
    }
}
