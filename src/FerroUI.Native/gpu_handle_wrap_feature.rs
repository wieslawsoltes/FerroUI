//! The feature that takes a reference of its own to an IOSurface or to a
//! Metal shared event at the moment it is handed over for an import, so the
//! one who imports may release its own at once.

use crate::interop::*;
use ferroui_base::platform::{
    IExternalObjectsHandleWrapRenderInterfaceContextFeature, IExternalObjectsWrappedGpuHandle, IPlatformHandle,
    KnownPlatformGraphicsExternalImageHandleTypes, KnownPlatformGraphicsExternalSemaphoreHandleTypes,
    PlatformGraphicsExternalImageProperties,
};
use ferroui_microcom::ComPtr;
use std::any::Any;
use std::cell::Cell;
use std::ffi::c_void;
use std::rc::Rc;

/// Wraps the handles of GPU objects that are counted by reference: an
/// `IOSurfaceRef` and an `id<MTLSharedEvent>`.
pub(crate) struct GpuHandleWrapFeature {
    helper: ComPtr<IFrnNativeObjectsMemoryManagement>,
}

impl GpuHandleWrapFeature {
    // The original is handed the factory and asks it for the helper. Here
    // the platform graphics ask the factory once, on the UI thread, and the
    // feature of every device gets a reference to that helper (DEVIATIONS.md,
    // Native backend).
    pub(crate) fn new(helper: ComPtr<IFrnNativeObjectsMemoryManagement>) -> GpuHandleWrapFeature {
        GpuHandleWrapFeature { helper }
    }
}

impl IExternalObjectsHandleWrapRenderInterfaceContextFeature for GpuHandleWrapFeature {
    fn wrap_image_handle_on_any_thread(
        &self,
        handle: &Rc<dyn IPlatformHandle>,
        _properties: PlatformGraphicsExternalImageProperties,
    ) -> Option<Rc<dyn IExternalObjectsWrappedGpuHandle>> {
        if handle.handle_descriptor() == Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF) {
            // SAFETY: by its descriptor the handle is an `IOSurfaceRef`,
            // which the caller keeps alive for the length of this call; the
            // reference taken here is given back by the wrapper.
            unsafe { self.helper.retain_cf_object(handle.handle() as *mut c_void) };
            return Some(Rc::new(CFObjectWrapper {
                helper: self.helper.clone(),
                handle: handle.handle(),
                descriptor: KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF.to_string(),
                released: Cell::new(false),
            }));
        }

        None
    }

    fn wrap_semaphore_handle_on_any_thread(
        &self,
        handle: &Rc<dyn IPlatformHandle>,
    ) -> Option<Rc<dyn IExternalObjectsWrappedGpuHandle>> {
        if handle.handle_descriptor() == Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT) {
            // SAFETY: by its descriptor the handle is an
            // `id<MTLSharedEvent>`, which the caller keeps alive for the
            // length of this call; the reference taken here is given back
            // by the wrapper.
            unsafe { self.helper.retain_ns_object(handle.handle() as *mut c_void) };
            return Some(Rc::new(NSObjectWrapper {
                helper: self.helper.clone(),
                handle: handle.handle(),
                descriptor: KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT.to_string(),
                released: Cell::new(false),
            }));
        }

        None
    }
}

/// A reference to an Objective-C object, given back when the wrapper is
/// disposed.
// The original releases at every call of `Dispose`. A second release would
// free an object this wrapper has no reference to, which a safe function may
// not do: only the first call releases. As in the original, a wrapper that
// is never disposed keeps its reference.
struct NSObjectWrapper {
    helper: ComPtr<IFrnNativeObjectsMemoryManagement>,
    handle: isize,
    descriptor: String,
    released: Cell<bool>,
}

impl IPlatformHandle for NSObjectWrapper {
    fn handle(&self) -> isize {
        self.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(&self.descriptor)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IExternalObjectsWrappedGpuHandle for NSObjectWrapper {
    fn dispose(&self) {
        if self.released.replace(true) {
            return;
        }
        // SAFETY: the wrapper holds the reference `retain_ns_object` took
        // when it was made, and gives it back once.
        unsafe { self.helper.release_ns_object(self.handle as *mut c_void) };
    }

    fn as_platform_handle(self: Rc<Self>) -> Rc<dyn IPlatformHandle> {
        self
    }
}

/// A reference to a Core Foundation object, given back when the wrapper is
/// disposed.
// One release, as for `NSObjectWrapper`.
struct CFObjectWrapper {
    helper: ComPtr<IFrnNativeObjectsMemoryManagement>,
    handle: isize,
    descriptor: String,
    released: Cell<bool>,
}

impl IPlatformHandle for CFObjectWrapper {
    fn handle(&self) -> isize {
        self.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(&self.descriptor)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IExternalObjectsWrappedGpuHandle for CFObjectWrapper {
    fn dispose(&self) {
        if self.released.replace(true) {
            return;
        }
        // SAFETY: the wrapper holds the reference `retain_cf_object` took
        // when it was made, and gives it back once.
        unsafe { self.helper.release_cf_object(self.handle as *mut c_void) };
    }

    fn as_platform_handle(self: Rc<Self>) -> Rc<dyn IPlatformHandle> {
        self
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from upstream, which has no tests of the feature: the feature over
    // a memory management helper that is implemented here and records its
    // calls.
    use super::*;
    use ferroui_base::platform::PlatformHandle;
    use std::cell::RefCell;

    pub(crate) type Log = Rc<RefCell<Vec<String>>>;

    /// A helper of the native side that records what it is asked to retain
    /// and release.
    pub(crate) struct FakeMemoryManagement {
        pub(crate) log: Log,
    }

    impl IFrnNativeObjectsMemoryManagementImpl for FakeMemoryManagement {
        fn retain_ns_object(&self, obj: *mut c_void) {
            self.log.borrow_mut().push(format!("RetainNSObject({})", obj as usize));
        }

        fn release_ns_object(&self, obj: *mut c_void) {
            self.log.borrow_mut().push(format!("ReleaseNSObject({})", obj as usize));
        }

        fn get_retain_count_for_ns_object(&self, _obj: *mut c_void) -> u64 {
            0
        }

        fn retain_cf_object(&self, obj: *mut c_void) {
            self.log.borrow_mut().push(format!("RetainCFObject({})", obj as usize));
        }

        fn release_cf_object(&self, obj: *mut c_void) {
            self.log.borrow_mut().push(format!("ReleaseCFObject({})", obj as usize));
        }

        fn get_retain_count_for_cf_object(&self, _obj: *mut c_void) -> i64 {
            0
        }
    }

    pub(crate) fn helper(log: &Log) -> ComPtr<IFrnNativeObjectsMemoryManagement> {
        IFrnNativeObjectsMemoryManagement::from_impl(FakeMemoryManagement { log: log.clone() })
    }

    fn feature() -> (GpuHandleWrapFeature, Log) {
        let log = Log::default();
        (GpuHandleWrapFeature::new(helper(&log)), log)
    }

    fn take(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    fn handle(handle: isize, descriptor: Option<&str>) -> Rc<dyn IPlatformHandle> {
        Rc::new(PlatformHandle::new(handle, descriptor))
    }

    #[test]
    fn an_io_surface_is_retained_until_its_wrapper_is_disposed() {
        let (feature, log) = feature();
        let surface = handle(5, Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF));

        let wrapped = feature
            .wrap_image_handle_on_any_thread(&surface, PlatformGraphicsExternalImageProperties::default())
            .expect("an IOSurface is wrapped");
        assert_eq!(vec!["RetainCFObject(5)"], take(&log));
        assert_eq!(5, wrapped.handle());
        assert_eq!(Some("IOSurfaceRef"), wrapped.handle_descriptor());

        // The one who imports may let go of its handle at once.
        drop(surface);
        assert!(take(&log).is_empty());

        // The wrapper is what the import is handed.
        let as_handle = wrapped.clone().as_platform_handle();
        assert_eq!(5, as_handle.handle());
        assert_eq!(Some("IOSurfaceRef"), as_handle.handle_descriptor());

        wrapped.dispose();
        assert_eq!(vec!["ReleaseCFObject(5)"], take(&log));
    }

    #[test]
    fn a_shared_event_is_retained_until_its_wrapper_is_disposed() {
        let (feature, log) = feature();
        let event = handle(9, Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT));

        let wrapped = feature.wrap_semaphore_handle_on_any_thread(&event).expect("a shared event is wrapped");
        assert_eq!(vec!["RetainNSObject(9)"], take(&log));
        assert_eq!(9, wrapped.handle());
        assert_eq!(Some("MetalSharedEvent"), wrapped.handle_descriptor());

        wrapped.dispose();
        assert_eq!(vec!["ReleaseNSObject(9)"], take(&log));
    }

    #[test]
    fn a_wrapper_gives_its_reference_back_once() {
        let (feature, log) = feature();

        let surface = handle(5, Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF));
        let wrapped = feature
            .wrap_image_handle_on_any_thread(&surface, PlatformGraphicsExternalImageProperties::default())
            .expect("an IOSurface is wrapped");
        wrapped.dispose();
        wrapped.dispose();
        assert_eq!(vec!["RetainCFObject(5)", "ReleaseCFObject(5)"], take(&log));

        let event = handle(9, Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT));
        let wrapped = feature.wrap_semaphore_handle_on_any_thread(&event).expect("a shared event is wrapped");
        wrapped.dispose();
        wrapped.dispose();
        assert_eq!(vec!["RetainNSObject(9)", "ReleaseNSObject(9)"], take(&log));
    }

    #[test]
    fn a_wrapper_that_is_not_disposed_keeps_its_reference() {
        let (feature, log) = feature();
        let surface = handle(5, Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF));

        let wrapped = feature.wrap_image_handle_on_any_thread(&surface, PlatformGraphicsExternalImageProperties::default());
        assert_eq!(vec!["RetainCFObject(5)"], take(&log));
        drop(wrapped);
        assert!(take(&log).is_empty());
    }

    #[test]
    fn a_handle_of_another_kind_is_not_wrapped() {
        let (feature, log) = feature();
        let surface = handle(5, Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF));
        let event = handle(9, Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT));
        let descriptor = handle(3, Some(KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR));
        let plain = handle(4, None);

        // Each kind is wrapped by its own method only.
        assert!(feature.wrap_image_handle_on_any_thread(&event, Default::default()).is_none());
        assert!(feature.wrap_semaphore_handle_on_any_thread(&surface).is_none());
        // What is not counted by reference is left to the one who imports.
        assert!(feature.wrap_image_handle_on_any_thread(&descriptor, Default::default()).is_none());
        assert!(feature.wrap_semaphore_handle_on_any_thread(&descriptor).is_none());
        assert!(feature.wrap_image_handle_on_any_thread(&plain, Default::default()).is_none());
        assert!(feature.wrap_semaphore_handle_on_any_thread(&plain).is_none());

        assert!(take(&log).is_empty());
    }
}
