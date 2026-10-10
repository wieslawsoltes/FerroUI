//! Port of `Embedding/MacOSViewHandle.cs`.

use super::objc::{self, Id};
use ferroui_controls::platform::{INativeControlHostDestroyableControlHandle, IPlatformHandle};
use std::any::Any;
use std::cell::Cell;

/// The handle of a native view the sample created, which the native control host destroys.
pub struct MacOSViewHandle {
    /// The view, owned by the handle; null once destroyed.
    view: Cell<Id>,
}

impl MacOSViewHandle {
    /// The handle of `view`, which takes the ownership of the view (the reference its creation
    /// returned).
    pub fn new(view: Id) -> Self {
        Self { view: Cell::new(view) }
    }
}

impl IPlatformHandle for MacOSViewHandle {
    fn handle(&self) -> isize {
        self.view.get() as isize
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some("NSView")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_native_control_host_destroyable_control_handle(
        &self,
    ) -> Option<&dyn INativeControlHostDestroyableControlHandle> {
        Some(self)
    }
}

impl INativeControlHostDestroyableControlHandle for MacOSViewHandle {
    fn destroy(&self) {
        let view = self.view.replace(std::ptr::null_mut());
        if !view.is_null() {
            // `_view.Dispose()`: gives up the reference the handle owns.
            objc::send_void(view, "release");
        }
    }
}
