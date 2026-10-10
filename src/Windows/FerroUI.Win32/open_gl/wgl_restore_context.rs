//! What was current before a context of WGL was made current, restored
//! when it is disposed (the port of `OpenGl/WglRestoreContext.cs`).
//!
//! The reference also takes a monitor of the context it makes current and
//! leaves it when it is disposed: its contexts are used from the threads of
//! a pool. A context here is an object of the thread that created it
//! (`Rc`), so there is nothing to guard, as for the contexts of EGL and of
//! GLX.

/// The message of a context that could not be made current.
pub(crate) fn make_current_error(last_error: u32, dc_valid: bool) -> String {
    format!("Unable to make the context current: {last_error}, DC valid: {}", if dc_valid { "True" } else { "False" })
}

#[cfg(windows)]
pub(crate) use imp::WglRestoreContext;

#[cfg(windows)]
mod imp {
    use super::make_current_error;
    use crate::interop::unmanaged_methods::{
        get_device_caps, get_last_error, wgl_get_current_context, wgl_get_current_dc, wgl_make_current,
    };
    use crate::interop::unmanaged_methods::DEVICECAP;
    use ferroui_base::reactive::IDisposable;
    use ferroui_opengl::OpenGlException;
    use std::cell::Cell;

    pub(crate) struct WglRestoreContext {
        old_dc: isize,
        old_context: isize,
        restored: Cell<bool>,
    }

    impl WglRestoreContext {
        /// Makes `context` current with the device context `gc`,
        /// remembering what was current.
        pub fn new(gc: isize, context: isize) -> Result<WglRestoreContext, OpenGlException> {
            let old_dc = wgl_get_current_dc();
            let old_context = wgl_get_current_context();

            if !wgl_make_current(gc, context) {
                let last_error = get_last_error();
                let caps = get_device_caps(gc, DEVICECAP::BITSPIXEL);
                return Err(OpenGlException::new(make_current_error(last_error, caps != 0)));
            }

            Ok(WglRestoreContext { old_dc, old_context, restored: Cell::new(false) })
        }
    }

    impl IDisposable for WglRestoreContext {
        fn dispose(&self) {
            // What was current is restored once.
            if self.restored.replace(true) {
                return;
            }
            if !wgl_make_current(self.old_dc, self.old_context) {
                wgl_make_current(0, 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn the_message_names_the_error_and_whether_the_device_context_is_valid() {
        assert_eq!("Unable to make the context current: 2000, DC valid: True", make_current_error(2000, true));
        assert_eq!("Unable to make the context current: 6, DC valid: False", make_current_error(6, false));
    }
}
