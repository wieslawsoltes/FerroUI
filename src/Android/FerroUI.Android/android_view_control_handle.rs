use crate::interop::java::JavaObject;
use ferroui_controls::platform::{INativeControlHostDestroyableControlHandle, IPlatformHandle};
use std::any::Any;
use std::cell::RefCell;

/// The descriptor of the handle of a view.
pub(crate) const ANDROID_VIEW_DESCRIPTOR: &str = "android.view.View";

/// A view of the system as a platform handle.
pub struct AndroidViewControlHandle {
    view: RefCell<Option<JavaObject>>,
    handle: isize,
}

impl AndroidViewControlHandle {
    pub fn new(view: JavaObject) -> Self {
        let handle = view.handle();
        Self { view: RefCell::new(Some(view)), handle }
    }

    /// The view; `None` once the handle was destroyed.
    pub fn view(&self) -> Option<JavaObject> {
        self.view.borrow().clone()
    }
}

impl IPlatformHandle for AndroidViewControlHandle {
    /// The reference to the view as a number. It names the view while this
    /// handle holds it.
    fn handle(&self) -> isize {
        self.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(ANDROID_VIEW_DESCRIPTOR)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl INativeControlHostDestroyableControlHandle for AndroidViewControlHandle {
    /// Releases the reference to the view (the reference disposes the
    /// managed peer of the view).
    fn destroy(&self) {
        let view = self.view.borrow_mut().take();
        drop(view);
    }
}
