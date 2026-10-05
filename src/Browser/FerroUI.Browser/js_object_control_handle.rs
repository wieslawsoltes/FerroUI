use crate::interop::JsObject;
use ferroui_controls::platform::{INativeControlHostDestroyableControlHandle, IPlatformHandle};
use std::any::Any;
use std::cell::RefCell;
use std::sync::atomic::{AtomicIsize, Ordering};

/// The descriptor of handles that wrap an object of the page.
pub(crate) const ELEMENT_REFERENCE_DESCRIPTOR: &str = "JSObject";

static NEXT_HANDLE: AtomicIsize = AtomicIsize::new(1);

/// A platform handle that wraps an object of the page.
///
/// The numeric handle only identifies the wrapper; the object itself is
/// reached through [`object`](Self::object).
pub struct JsObjectPlatformHandle {
    handle: isize,
    object: RefCell<Option<JsObject>>,
}

impl JsObjectPlatformHandle {
    pub(crate) fn new(reference: JsObject) -> Self {
        Self { handle: NEXT_HANDLE.fetch_add(1, Ordering::Relaxed), object: RefCell::new(Some(reference)) }
    }

    /// The object of the page; `None` once the handle has been destroyed.
    pub fn object(&self) -> Option<JsObject> {
        self.object.borrow().clone()
    }
}

impl IPlatformHandle for JsObjectPlatformHandle {
    fn handle(&self) -> isize {
        self.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(ELEMENT_REFERENCE_DESCRIPTOR)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The handle of a control of the page that the framework hosts and may
/// destroy.
pub struct JsObjectControlHandle {
    base: JsObjectPlatformHandle,
}

impl JsObjectControlHandle {
    /// Wraps an object of the page.
    pub fn new(reference: JsObject) -> Self {
        Self { base: JsObjectPlatformHandle::new(reference) }
    }

    /// The object of the page; `None` once the handle has been destroyed.
    pub fn object(&self) -> Option<JsObject> {
        self.base.object()
    }
}

impl IPlatformHandle for JsObjectControlHandle {
    fn handle(&self) -> isize {
        self.base.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(ELEMENT_REFERENCE_DESCRIPTOR)
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

impl INativeControlHostDestroyableControlHandle for JsObjectControlHandle {
    fn destroy(&self) {
        self.base.object.borrow_mut().take();
    }
}
