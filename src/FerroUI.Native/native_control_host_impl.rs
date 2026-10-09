use crate::helpers::ComResultExt;
use crate::interop::*;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Size};
use ferroui_controls::platform::{
    INativeControlHostControlTopLevelAttachment, INativeControlHostDestroyableControlHandle, INativeControlHostImpl,
    IPlatformHandle, PlatformHandle,
};
use ferroui_microcom::ComPtr;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::{Rc, Weak};

/// Hosts native views (`NSView`) inside a top-level of this backend.
pub(crate) struct NativeControlHostImpl {
    this: Weak<NativeControlHostImpl>,
    host: RefCell<Option<ComPtr<IFrnNativeControlHost>>>,
}

impl NativeControlHostImpl {
    pub(crate) fn new(host: Option<ComPtr<IFrnNativeControlHost>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), host: RefCell::new(host) })
    }

    #[track_caller]
    fn host(&self) -> ComPtr<IFrnNativeControlHost> {
        match self.host.borrow().clone() {
            Some(host) => host,
            None => panic!("Cannot access a disposed object: NativeControlHostImpl"),
        }
    }

    fn this(&self) -> Rc<dyn INativeControlHostImpl> {
        self.this.upgrade().expect("the native control host is alive while it is used")
    }

    /// Releases the native host.
    pub(crate) fn dispose(&self) {
        let host = self.host.borrow_mut().take();
        drop(host);
    }
}

impl IDisposable for NativeControlHostImpl {
    fn dispose(&self) {
        NativeControlHostImpl::dispose(self);
    }
}

/// The default child view of a native control host; the host destroys it.
struct DestroyableNSView {
    host: RefCell<Option<ComPtr<IFrnNativeControlHost>>>,
    ns_view: Cell<isize>,
}

impl DestroyableNSView {
    fn new(host: ComPtr<IFrnNativeControlHost>) -> Self {
        // SAFETY: the native side creates a view of its own; it does not
        // read the parent, which is null as in the reference.
        let ns_view = unsafe { host.create_default_child(std::ptr::null_mut()) }.check();
        Self { host: RefCell::new(Some(host)), ns_view: Cell::new(ns_view as isize) }
    }
}

impl IPlatformHandle for DestroyableNSView {
    fn handle(&self) -> isize {
        self.ns_view.get()
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

impl INativeControlHostDestroyableControlHandle for DestroyableNSView {
    fn destroy(&self) {
        let host = self.host.borrow_mut().take();
        if let Some(host) = host {
            // SAFETY: the view was created by `create_default_child` of this
            // host and is destroyed once: the host is taken above.
            unsafe { host.destroy_default_child(self.ns_view.get() as *mut c_void) };
            drop(host);
            self.ns_view.set(0);
        }
    }
}

impl INativeControlHostImpl for NativeControlHostImpl {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_default_child(&self, _parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostDestroyableControlHandle> {
        Rc::new(DestroyableNSView::new(self.host()))
    }

    fn create_new_attachment_with(
        &self,
        create: Rc<dyn Fn(Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle>>,
    ) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        let a = Rc::new(Attachment::new(self.host().create_attachment()));
        // The attachment is disposed when creating or initializing the
        // child fails, and the failure goes on to the caller.
        let attach = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let child = create(a.get_parent_handle());
            a.init_with_child(&*child);
            a.set_attached_to(Some(self.this()));
        }));
        match attach {
            Ok(()) => a,
            Err(payload) => {
                a.dispose();
                std::panic::resume_unwind(payload)
            }
        }
    }

    fn create_new_attachment(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeControlHostControlTopLevelAttachment> {
        let a = Rc::new(Attachment::new(self.host().create_attachment()));
        a.init_with_child(&*handle);
        a.set_attached_to(Some(self.this()));
        a
    }

    fn is_compatible_with(&self, handle: &dyn IPlatformHandle) -> bool {
        handle.handle_descriptor() == Some("NSView")
    }
}

/// The attachment of a native view to a top-level of this backend.
struct Attachment {
    native: RefCell<Option<ComPtr<IFrnNativeControlHostTopLevelAttachment>>>,
    attached_to: RefCell<Option<Rc<dyn INativeControlHostImpl>>>,
}

impl Attachment {
    fn new(native: Option<ComPtr<IFrnNativeControlHostTopLevelAttachment>>) -> Self {
        Self { native: RefCell::new(native), attached_to: RefCell::new(None) }
    }

    #[track_caller]
    fn native(&self) -> ComPtr<IFrnNativeControlHostTopLevelAttachment> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: Attachment"),
        }
    }

    fn get_parent_handle(&self) -> Rc<dyn IPlatformHandle> {
        Rc::new(PlatformHandle::new(self.native().get_parent_handle() as isize, Some("NSView")))
    }

    fn init_with_child(&self, handle: &dyn IPlatformHandle) {
        // SAFETY: the handle is the `NSView` the attachment is asked to
        // host (see `is_compatible_with` of the host); the native side
        // retains it.
        unsafe { self.native().initialize_with_child_handle(handle.handle() as *mut c_void) }.check()
    }
}

impl IDisposable for Attachment {
    fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        if let Some(native) = native {
            native.release_child();
            drop(native);
        }
    }
}

impl INativeControlHostControlTopLevelAttachment for Attachment {
    fn attached_to(&self) -> Option<Rc<dyn INativeControlHostImpl>> {
        self.attached_to.borrow().clone()
    }

    fn set_attached_to(&self, value: Option<Rc<dyn INativeControlHostImpl>>) {
        let host = value.as_ref().map(|value| {
            value
                .as_any()
                .downcast_ref::<NativeControlHostImpl>()
                .expect("the native control host belongs to this backend")
                .host
                .borrow()
                .clone()
        });
        self.native().attach_to(host.flatten().as_deref()).check();
        *self.attached_to.borrow_mut() = value;
    }

    fn is_compatible_with(&self, host: &dyn INativeControlHostImpl) -> bool {
        host.as_any().is::<NativeControlHostImpl>()
    }

    fn hide_with_size(&self, size: Size) {
        self.native().hide_with_size((size.width as f32).max(1.0), (size.height as f32).max(1.0));
    }

    fn show_in_bounds(&self, bounds: Rect) {
        if self.attached_to.borrow().is_none() {
            panic!("Native control isn't attached to a toplevel");
        }
        let bounds = Rect::new(bounds.x, bounds.y, bounds.width.max(1.0), bounds.height.max(1.0));
        self.native().show_in_bounds(bounds.x as f32, bounds.y as f32, bounds.width as f32, bounds.height as f32);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream, which has no tests of the host: the default child
    // over a native host that is implemented here and records its calls.
    use super::*;
    use ferroui_microcom::HResult;

    type Log = Rc<RefCell<Vec<String>>>;

    struct FakeNativeHost {
        log: Log,
    }

    impl IFrnNativeControlHostImpl for FakeNativeHost {
        fn create_default_child(&self, parent: *mut c_void) -> Result<*mut c_void, HResult> {
            self.log.borrow_mut().push(format!("CreateDefaultChild({})", parent as usize));
            Ok(77 as *mut c_void)
        }

        fn create_attachment(&self) -> Option<ComPtr<IFrnNativeControlHostTopLevelAttachment>> {
            None
        }

        fn destroy_default_child(&self, child: *mut c_void) {
            self.log.borrow_mut().push(format!("DestroyDefaultChild({})", child as usize));
        }
    }

    fn take(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    #[test]
    fn the_default_child_is_a_handle_that_can_be_destroyed() {
        let log = Log::default();
        let host = NativeControlHostImpl::new(Some(IFrnNativeControlHost::from_impl(FakeNativeHost { log: log.clone() })));

        let parent: Rc<dyn IPlatformHandle> = Rc::new(PlatformHandle::new(5, Some("NSView")));
        let child = host.create_default_child(parent);
        // The native side makes a view of its own and is not told the parent.
        assert_eq!(vec!["CreateDefaultChild(0)"], take(&log));
        assert_eq!(77, child.handle());
        assert_eq!(Some("NSView"), child.handle_descriptor());

        // What the control that hosts it asks of the handle it holds
        // (`NativeControlHost::destroy_native_control_core`).
        let destroyable = child
            .as_native_control_host_destroyable_control_handle()
            .expect("the default child can be destroyed");
        destroyable.destroy();
        assert_eq!(vec!["DestroyDefaultChild(77)"], take(&log));
        assert_eq!(0, child.handle());

        // Destroyed once.
        destroyable.destroy();
        assert!(take(&log).is_empty());
    }
}
