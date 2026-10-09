//! Metal: the platform graphics, the device (graphics context) and its
//! external objects feature, the render surface of a top-level and its
//! render target and drawing session.

use crate::gpu_handle_wrap_feature::GpuHandleWrapFeature;
use crate::helpers::ComResultExt;
use crate::interop::*;
use crate::top_level_impl::SurfaceTopLevel;
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{
    IExternalObjectsHandleWrapRenderInterfaceContextFeature, IOptionalFeatureProvider, IPlatformGraphics,
    IPlatformGraphicsContext, IPlatformHandle,
    KnownPlatformGraphicsExternalImageHandleTypes, KnownPlatformGraphicsExternalSemaphoreHandleTypes,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use ferroui_base::threading::Dispatcher;
use ferroui_base::PixelSize;
use ferroui_microcom::{ComPtr, HResult};
use ferroui_metal::{
    IMetalDevice, IMetalExternalObjectsFeature, IMetalExternalTexture, IMetalPlatformSurface,
    IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession, IMetalSharedEvent,
};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::{Arc, Weak};

/// The Metal platform graphics of the macOS backend.
// The original holds the factory and hands it to every device, whose
// handle wrapping feature asks it for a memory management helper. The
// graphics are shared with the thread that renders, and the factory is an
// object of the UI thread (releasing its last reference ends the native
// application state): the helper is asked for once, here, and every device
// gets a reference to it (DEVIATIONS.md, Native backend).
pub struct MetalPlatformGraphics {
    display: ComPtr<IFrnMetalDisplay>,
    memory_helper: ComPtr<IFrnNativeObjectsMemoryManagement>,
}

// SAFETY: the platform graphics are shared by the UI thread and the thread
// that renders (`IPlatformGraphics: Send + Sync`), and each may call
// `CreateDevice` and drop its handle. The display is a process-lifetime
// singleton of the native side (`FrnMetalDisplay`, held by a `ComStaticPtr`
// in `native/FerroUI.Native/src/OSX/metal.mm`) without state: `CreateDevice`
// reads nothing of the object and makes a new device and command queue per
// call, so two threads may be inside it at once. The pointer is never
// replaced, and the reference this object holds is taken once and given
// back once, by whichever thread drops the last handle: the reference count
// of a native object is atomic (`ComObject` in
// `native/FerroUI.Native/inc/comimpl.h`). The device a call returns belongs
// to the thread that asked.
//
// The memory management helper (`MemHelper` in
// `native/FerroUI.Native/src/OSX/memhelp.mm`) has no state either: its
// methods retain and release the object they are handed (`CFRetain`,
// `CFRelease`, `retain` and `release` of an Objective-C object, all of which
// any thread may call), so a thread that creates a device may take a
// reference to the helper, and the features and wrappers of that device may
// call it and release it, while another thread does the same.
unsafe impl Send for MetalPlatformGraphics {}
// SAFETY: see `Send`.
unsafe impl Sync for MetalPlatformGraphics {}

impl MetalPlatformGraphics {
    /// Obtains the Metal display of the native side; fails when Metal is
    /// not available.
    pub fn new(factory: &IFerroNativeFactory) -> Result<MetalPlatformGraphics, HResult> {
        let display = factory.obtain_metal_display()?.ok_or(HResult::POINTER)?;
        let memory_helper = factory.create_memory_management_helper()?.ok_or(HResult::POINTER)?;
        Ok(MetalPlatformGraphics { display, memory_helper })
    }

    /// Creates a Metal device; fails when the system has none.
    pub fn try_create_context(&self) -> Result<Rc<MetalDevice>, HResult> {
        let native = self.display.create_device()?.ok_or(HResult::POINTER)?;
        Ok(MetalDevice::new(self.memory_helper.clone(), native))
    }
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MetalPlatformGraphics>();
};

impl IPlatformGraphics for MetalPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.try_create_context().check()
    }

    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.");
    }
}

/// A Metal device and its command queue.
pub struct MetalDevice {
    weak_self: std::rc::Weak<MetalDevice>,
    sync_root: ferroui_base::utilities::DisposableLock,
    handle_wrap_feature: Rc<GpuHandleWrapFeature>,
    external_objects_feature: Rc<MetalExternalObjectsFeature>,
    native: RefCell<Option<ComPtr<IFrnMetalDevice>>>,
}

impl MetalDevice {
    fn new(
        memory_helper: ComPtr<IFrnNativeObjectsMemoryManagement>,
        native: ComPtr<IFrnMetalDevice>,
    ) -> Rc<MetalDevice> {
        Rc::new_cyclic(|weak_self| MetalDevice {
            weak_self: weak_self.clone(),
            sync_root: ferroui_base::utilities::DisposableLock::new(),
            handle_wrap_feature: Rc::new(GpuHandleWrapFeature::new(memory_helper)),
            external_objects_feature: Rc::new(MetalExternalObjectsFeature::new(&native, weak_self.clone())),
            native: RefCell::new(Some(native)),
        })
    }

    /// The native device.
    ///
    /// # Panics
    /// Panics when the device is disposed.
    #[track_caller]
    pub fn native(&self) -> ComPtr<IFrnMetalDevice> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: MetalDevice"),
        }
    }

    /// Calls the native device where it is held, without taking a reference
    /// of its own: for the getters, which neither take time nor release the
    /// device.
    ///
    /// # Panics
    /// Panics when the device is disposed.
    #[track_caller]
    fn with_native<R>(&self, f: impl FnOnce(&IFrnMetalDevice) -> R) -> R {
        match self.native.borrow().as_ref() {
            Some(native) => f(&**native),
            None => panic!("Cannot access a disposed object: MetalDevice"),
        }
    }
}

impl IOptionalFeatureProvider for MetalDevice {
    /// The device announces itself as a Metal device, the feature that
    /// wraps the handle of an IOSurface or of a shared event for the time
    /// of its import, and its external objects feature (the import of an
    /// IOSurface and of a shared event).
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IMetalDevice>() {
            let this: Rc<dyn IMetalDevice> = self.weak_self.upgrade()?;
            return Some(Rc::new(this));
        }
        if feature_type == TypeId::of::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>() {
            let feature: Rc<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature> =
                self.handle_wrap_feature.clone();
            return Some(Rc::new(feature));
        }
        if feature_type == TypeId::of::<dyn IMetalExternalObjectsFeature>() {
            let feature: Rc<dyn IMetalExternalObjectsFeature> = self.external_objects_feature.clone();
            return Some(Rc::new(feature));
        }
        None
    }
}

impl IPlatformGraphicsContext for MetalDevice {
    fn is_lost(&self) -> bool {
        false
    }

    /// Takes the lock of the device: the UI thread and the render thread
    /// both use it, one at a time.
    ///
    /// The device lives in the graph of the server compositor, so every
    /// caller already holds the compositor lock: the order is always the
    /// compositor lock first, then this one. Something that used the device
    /// outside the compositor would have to keep that order.
    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.sync_root.lock()
    }

    fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        drop(native);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalDevice for MetalDevice {
    fn device(&self) -> *mut c_void {
        self.with_native(|native| native.get_device())
    }

    fn command_queue(&self) -> *mut c_void {
        self.with_native(|native| native.get_queue())
    }
}

/// The Metal render surface of a top-level.
///
/// The surface is shared with the thread that renders
/// (`IPlatformRenderSurface: Send + Sync`). What it holds of the native side
/// is the native top-level, which only the UI thread may use (see
/// `SurfaceTopLevel`): the render target is created on the UI thread, as in
/// the reference, and the native side refuses anything else
/// (`TopLevelImpl::CreateMetalRenderTarget` in
/// `native/FerroUI.Native/src/OSX/TopLevelImpl.mm` returns
/// `COR_E_INVALIDOPERATION` off the main thread).
pub struct MetalPlatformSurface {
    weak_self: Weak<MetalPlatformSurface>,
    top_level: SurfaceTopLevel,
}

impl MetalPlatformSurface {
    pub(crate) fn new(top_level: ComPtr<IFrnTopLevel>) -> Arc<MetalPlatformSurface> {
        Arc::new_cyclic(|weak_self| MetalPlatformSurface {
            weak_self: weak_self.clone(),
            top_level: SurfaceTopLevel::new(top_level),
        })
    }

    /// Releases the native top-level; called by the top-level when it is
    /// disposed, on the UI thread.
    pub(crate) fn close(&self) {
        self.top_level.release();
    }
}

impl IPlatformRenderSurface for MetalPlatformSurface {
    /// Deviation (DEVIATIONS.md, Native backend): the reference throws
    /// `RenderTargetNotReadyException` from `CreateMetalRenderTarget` off the
    /// UI thread and the composition target catches it; here the surface
    /// answers that it is not ready, which the composition target treats the
    /// same way, without a panic to catch.
    fn is_ready(&self) -> bool {
        self.top_level.get().is_some()
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IMetalPlatformSurface>() {
            // The contract hands out an `Rc`, and the surface lives in an
            // `Arc`: the view forwards to the surface.
            let this: Rc<dyn IMetalPlatformSurface> = Rc::new(MetalPlatformSurfaceView(self.weak_self.upgrade()?));
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalPlatformSurface for MetalPlatformSurface {
    /// # Panics
    /// Panics when called off the UI thread or when the top-level is
    /// disposed (the render target is not ready) and when `device` is not a
    /// device of this backend.
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        if !Dispatcher::ui_thread().check_access() {
            panic!("The render target is not ready.");
        }

        let Some(top_level) = self.top_level.get() else {
            panic!("The render target is not ready.");
        };

        let Some(dev) = device.as_any().downcast_ref::<MetalDevice>() else {
            panic!("The Metal device belongs to a different platform backend.");
        };
        let native_device = dev.native();
        let target = top_level.create_metal_render_target(Some(&native_device)).check();
        Rc::new(MetalRenderTarget { native: RefCell::new(target) })
    }
}

/// The Metal surface as the Metal contract hands it out
/// (`Rc<dyn IMetalPlatformSurface>`); it forwards to the shared surface.
struct MetalPlatformSurfaceView(Arc<MetalPlatformSurface>);

impl IPlatformRenderSurface for MetalPlatformSurfaceView {
    fn is_ready(&self) -> bool {
        self.0.is_ready()
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        self.0.try_get_surface_kind(kind)
    }

    fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }
}

impl IMetalPlatformSurface for MetalPlatformSurfaceView {
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        self.0.create_metal_render_target(device)
    }
}

/// The external objects of a Metal device: the import of an IOSurface as a
/// texture and of a shared event, and the commands that wait for an event
/// and signal one.
///
/// The feature is an object of the device: it is created with it, inside
/// the graph of the server compositor, and used under the compositor lock
/// like the device.
// The original holds the native device, which its device releases when it
// is disposed. Here every holder of a native object has a reference of its
// own, so the feature reaches the native device through its device: a
// disposed device fails the feature as it does upstream.
pub(crate) struct MetalExternalObjectsFeature {
    device: std::rc::Weak<MetalDevice>,
    device_luid: Option<Vec<u8>>,
}

impl MetalExternalObjectsFeature {
    fn new(native: &IFrnMetalDevice, device: std::rc::Weak<MetalDevice>) -> MetalExternalObjectsFeature {
        let mut registry_id = 0u64;
        // SAFETY: the pointer is valid for the one value the native method
        // writes.
        let has_registry_id = unsafe { native.get_io_kit_registry_id(&mut registry_id) };
        // The bytes of the identifier, the most significant first.
        let device_luid = has_registry_id.then(|| registry_id.to_be_bytes().to_vec());
        MetalExternalObjectsFeature { device, device_luid }
    }

    /// The native device.
    ///
    /// # Panics
    /// Panics when the device is disposed.
    #[track_caller]
    fn native(&self) -> ComPtr<IFrnMetalDevice> {
        match self.device.upgrade() {
            Some(device) => device.native(),
            None => panic!("Cannot access a disposed object: MetalDevice"),
        }
    }
}

impl IMetalExternalObjectsFeature for MetalExternalObjectsFeature {
    fn supported_image_handle_types(&self) -> Vec<String> {
        vec![KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF.to_string()]
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        vec![KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT.to_string()]
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.device_luid.clone()
    }

    fn get_synchronization_capabilities(
        &self,
        _image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        CompositionGpuImportedImageSynchronizationCapabilities::TIMELINE_SEMAPHORES
    }

    /// # Panics
    /// Panics when the handle is not an IOSurface and when the native
    /// device cannot make a texture of it.
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IMetalExternalTexture> {
        // Every format of the enumeration is supported: the original
        // refuses the values it does not name.
        let format = match properties.format {
            PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm => FrnPixelFormat::kFrnRgba8888,
            PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm => FrnPixelFormat::kFrnBgra8888,
        };

        if handle.handle_descriptor() != Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF) {
            panic!("Specified method is not supported.");
        }

        // SAFETY: by its descriptor the handle is an `IOSurfaceRef`. The
        // compositor hands over the handle the wrapping feature of the
        // device made of it, which holds a reference until the import has
        // run; another caller keeps the surface alive for the call. The
        // texture the native device makes of it retains the surface.
        let texture = unsafe { self.native().import_io_surface(handle.handle() as *mut c_void, format) }
            .and_then(|texture| texture.ok_or(HResult::POINTER))
            .check();
        Rc::new(ImportedTexture { texture: RefCell::new(Some(texture)) })
    }

    /// # Panics
    /// Panics when the handle is not a Metal shared event and when the
    /// native device cannot import it.
    fn import_shared_event(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IMetalSharedEvent> {
        if handle.handle_descriptor() != Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT) {
            panic!("Specified method is not supported.");
        }
        // SAFETY: by its descriptor the handle is an `id<MTLSharedEvent>`.
        // The compositor hands over the handle the wrapping feature of the
        // device made of it, which holds a reference until the import has
        // run; another caller keeps the event alive for the call. The
        // native device makes an event of its own from it.
        let inner = unsafe { self.native().import_shared_event(handle.handle() as *mut c_void) }
            .and_then(|event| event.ok_or(HResult::POINTER))
            .check();
        Rc::new(SharedEvent { inner: RefCell::new(Some(inner)) })
    }

    /// # Panics
    /// Panics when the event was not imported by a device of this backend.
    fn submit_wait(&self, event: &dyn IMetalSharedEvent, wait_for_value: u64) {
        let event = SharedEvent::from_contract(event).native();
        self.native().submit_wait(Some(&*event), wait_for_value).check();
    }

    /// # Panics
    /// Panics when the event was not imported by a device of this backend.
    fn submit_signal(&self, event: &dyn IMetalSharedEvent, signal_value: u64) {
        let event = SharedEvent::from_contract(event).native();
        self.native().submit_signal(Some(&*event), signal_value).check();
    }
}

/// A texture the native device made of an IOSurface.
struct ImportedTexture {
    texture: RefCell<Option<ComPtr<IFrnMetalTexture>>>,
}

impl ImportedTexture {
    /// Calls the native texture where it is held.
    ///
    /// # Panics
    /// Panics when the texture is disposed.
    #[track_caller]
    fn with_texture<R>(&self, f: impl FnOnce(&IFrnMetalTexture) -> R) -> R {
        match self.texture.borrow().as_ref() {
            Some(texture) => f(&**texture),
            None => panic!("Cannot access a disposed object: ImportedTexture"),
        }
    }
}

impl IMetalExternalTexture for ImportedTexture {
    fn width(&self) -> i32 {
        self.with_texture(|texture| texture.get_width())
    }

    fn height(&self) -> i32 {
        self.with_texture(|texture| texture.get_height())
    }

    fn samples(&self) -> i32 {
        self.with_texture(|texture| texture.get_sample_count())
    }

    fn handle(&self) -> *mut c_void {
        self.with_texture(|texture| texture.get_native_handle())
    }

    fn dispose(&self) {
        let texture = self.texture.borrow_mut().take();
        drop(texture);
    }
}

/// A shared event the native device imported.
struct SharedEvent {
    inner: RefCell<Option<ComPtr<IFrnMTLSharedEvent>>>,
}

impl SharedEvent {
    /// The event behind the contract (the cast of the original).
    #[track_caller]
    fn from_contract(event: &dyn IMetalSharedEvent) -> &SharedEvent {
        match event.as_any().downcast_ref::<SharedEvent>() {
            Some(event) => event,
            None => panic!("The shared event belongs to a different platform backend."),
        }
    }

    /// The native event.
    ///
    /// # Panics
    /// Panics when the event is disposed.
    #[track_caller]
    fn native(&self) -> ComPtr<IFrnMTLSharedEvent> {
        match self.inner.borrow().clone() {
            Some(inner) => inner,
            None => panic!("Cannot access a disposed object: SharedEvent"),
        }
    }
}

impl IMetalSharedEvent for SharedEvent {
    fn handle(&self) -> *mut c_void {
        self.native().get_native_handle()
    }

    fn dispose(&self) {
        let inner = self.inner.borrow_mut().take();
        drop(inner);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The Metal render target of a top-level.
pub struct MetalRenderTarget {
    native: RefCell<Option<ComPtr<IFrnMetalRenderTarget>>>,
}

impl MetalRenderTarget {
    #[track_caller]
    fn native(&self) -> ComPtr<IFrnMetalRenderTarget> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: MetalRenderTarget"),
        }
    }
}

impl IPlatformRenderSurfaceRenderTarget for MetalRenderTarget {}

impl IMetalPlatformSurfaceRenderTarget for MetalRenderTarget {
    fn begin_rendering(&self) -> Rc<dyn IMetalPlatformSurfaceRenderingSession> {
        let session = self.native().begin_drawing().check();
        Rc::new(MetalDrawingSession { session: RefCell::new(session) })
    }

    fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        drop(native);
    }
}

/// One frame drawn to the Metal render target of a top-level; releasing the
/// native session presents the frame.
pub struct MetalDrawingSession {
    session: RefCell<Option<ComPtr<IFrnMetalRenderingSession>>>,
}

impl MetalDrawingSession {
    /// Calls the native session where it is held, without taking a reference
    /// of its own: its members are getters, and releasing the session is
    /// what presents the frame.
    ///
    /// # Panics
    /// Panics when the session is disposed.
    #[track_caller]
    fn with_session<R>(&self, f: impl FnOnce(&IFrnMetalRenderingSession) -> R) -> R {
        match self.session.borrow().as_ref() {
            Some(session) => f(&**session),
            None => panic!("Cannot access a disposed object: MetalDrawingSession"),
        }
    }
}

impl IMetalPlatformSurfaceRenderingSession for MetalDrawingSession {
    fn texture(&self) -> *mut c_void {
        self.with_session(|session| session.get_texture())
    }

    fn size(&self) -> PixelSize {
        let size = self.with_session(|session| session.get_pixel_size()).check();
        PixelSize::new(size.width, size.height)
    }

    fn scaling(&self) -> f64 {
        self.with_session(|session| session.get_scaling())
    }

    fn is_y_flipped(&self) -> bool {
        false
    }

    fn dispose(&self) {
        let session = self.session.borrow_mut().take();
        drop(session);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream, which has no tests of the feature: the feature over
    // a native device that is implemented here and records its calls.
    use super::*;
    use crate::gpu_handle_wrap_feature::tests::helper;
    use ferroui_base::platform::PlatformHandle;

    type Log = Rc<RefCell<Vec<String>>>;

    struct FakeNativeDevice {
        log: Log,
        registry_id: Option<u64>,
    }

    impl IFrnMetalDeviceImpl for FakeNativeDevice {
        fn get_device(&self) -> *mut c_void {
            std::ptr::null_mut()
        }

        fn get_queue(&self) -> *mut c_void {
            std::ptr::null_mut()
        }

        fn get_io_kit_registry_id(&self, value: *mut u64) -> bool {
            match self.registry_id {
                Some(registry_id) => {
                    // SAFETY: the caller passes storage for one value.
                    unsafe { *value = registry_id };
                    true
                }
                None => false,
            }
        }

        fn import_io_surface(
            &self,
            handle: *mut c_void,
            pixel_format: FrnPixelFormat,
        ) -> Result<Option<ComPtr<IFrnMetalTexture>>, HResult> {
            self.log.borrow_mut().push(format!("ImportIOSurface({}, {pixel_format:?})", handle as usize));
            if handle.is_null() {
                return Err(HResult::FAIL);
            }
            Ok(Some(IFrnMetalTexture::from_impl(FakeNativeTexture { log: self.log.clone() })))
        }

        fn import_shared_event(
            &self,
            mtl_shared_event_instance: *mut c_void,
        ) -> Result<Option<ComPtr<IFrnMTLSharedEvent>>, HResult> {
            let handle = mtl_shared_event_instance as usize;
            self.log.borrow_mut().push(format!("ImportSharedEvent({handle})"));
            Ok(Some(IFrnMTLSharedEvent::from_impl(FakeNativeEvent { log: self.log.clone(), handle })))
        }

        fn submit_wait(&self, ev: Option<&IFrnMTLSharedEvent>, value: u64) -> Result<(), HResult> {
            let handle = ev.map(|ev| ev.get_native_handle() as usize);
            self.log.borrow_mut().push(format!("SubmitWait({handle:?}, {value})"));
            Ok(())
        }

        fn submit_signal(&self, ev: Option<&IFrnMTLSharedEvent>, value: u64) -> Result<(), HResult> {
            let handle = ev.map(|ev| ev.get_native_handle() as usize);
            self.log.borrow_mut().push(format!("SubmitSignal({handle:?}, {value})"));
            Ok(())
        }
    }

    struct FakeNativeTexture {
        log: Log,
    }

    impl IFrnMetalTextureImpl for FakeNativeTexture {
        fn get_native_handle(&self) -> *mut c_void {
            77 as *mut c_void
        }

        fn get_width(&self) -> i32 {
            30
        }

        fn get_height(&self) -> i32 {
            20
        }

        fn get_sample_count(&self) -> i32 {
            1
        }
    }

    impl Drop for FakeNativeTexture {
        fn drop(&mut self) {
            self.log.borrow_mut().push("the texture is released".to_string());
        }
    }

    struct FakeNativeEvent {
        log: Log,
        handle: usize,
    }

    impl IFrnMTLSharedEventImpl for FakeNativeEvent {
        fn get_native_handle(&self) -> *mut c_void {
            // The native event is one of the device, not the one imported.
            (self.handle + 1000) as *mut c_void
        }

        fn wait(&self, _value: u64, _timeout_ms: u64) -> bool {
            false
        }

        fn set_signaled_value(&self, _value: u64) {}

        fn get_signaled_value(&self) -> u64 {
            0
        }
    }

    impl Drop for FakeNativeEvent {
        fn drop(&mut self) {
            self.log.borrow_mut().push("the event is released".to_string());
        }
    }

    fn device(registry_id: Option<u64>) -> (Rc<MetalDevice>, Log) {
        let log = Log::default();
        let native = IFrnMetalDevice::from_impl(FakeNativeDevice { log: log.clone(), registry_id });
        (MetalDevice::new(helper(&log), native), log)
    }

    fn feature(device: &Rc<MetalDevice>) -> Rc<dyn IMetalExternalObjectsFeature> {
        let features: &dyn IOptionalFeatureProvider = &**device;
        features.try_get::<dyn IMetalExternalObjectsFeature>().expect("the device has the feature")
    }

    fn take(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    fn io_surface(handle: isize) -> Rc<dyn IPlatformHandle> {
        Rc::new(PlatformHandle::new(handle, Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF)))
    }

    fn shared_event(handle: isize) -> Rc<dyn IPlatformHandle> {
        Rc::new(PlatformHandle::new(
            handle,
            Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT),
        ))
    }

    fn properties(format: PlatformGraphicsExternalImageFormat) -> PlatformGraphicsExternalImageProperties {
        PlatformGraphicsExternalImageProperties { width: 30, height: 20, format, ..Default::default() }
    }

    #[test]
    fn the_device_announces_what_it_imports_and_how_it_is_synchronized() {
        let (device, _) = device(Some(0x0102_0304_0506_0708));
        let feature = feature(&device);

        assert_eq!(vec!["IOSurfaceRef".to_string()], feature.supported_image_handle_types());
        assert_eq!(vec!["MetalSharedEvent".to_string()], feature.supported_semaphore_types());
        assert_eq!(
            CompositionGpuImportedImageSynchronizationCapabilities::TIMELINE_SEMAPHORES,
            feature.get_synchronization_capabilities("IOSurfaceRef")
        );
        // The registry identifier, the most significant byte first.
        assert_eq!(Some(vec![1, 2, 3, 4, 5, 6, 7, 8]), feature.device_luid());
    }

    #[test]
    fn a_device_without_a_registry_identifier_has_no_luid() {
        let (device, _) = device(None);

        assert_eq!(None, feature(&device).device_luid());
    }

    #[test]
    fn an_io_surface_is_imported_with_the_format_of_the_image() {
        let (device, log) = device(None);
        let feature = feature(&device);

        let texture = feature.import_image(io_surface(5), properties(PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm));
        assert_eq!(vec!["ImportIOSurface(5, kFrnRgba8888)"], take(&log));
        assert_eq!(30, texture.width());
        assert_eq!(20, texture.height());
        assert_eq!(1, texture.samples());
        assert_eq!(77, texture.handle() as usize);

        texture.dispose();
        assert_eq!(vec!["the texture is released"], take(&log));
        texture.dispose();
        assert!(take(&log).is_empty());

        let texture = feature.import_image(io_surface(6), properties(PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm));
        assert_eq!(vec!["ImportIOSurface(6, kFrnBgra8888)"], take(&log));
        drop(texture);
    }

    #[test]
    #[should_panic(expected = "Cannot access a disposed object: ImportedTexture")]
    fn a_disposed_texture_cannot_be_read() {
        let (device, _) = device(None);
        let texture = feature(&device).import_image(io_surface(5), properties(Default::default()));
        texture.dispose();
        let _ = texture.width();
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported.")]
    fn a_handle_that_is_not_an_io_surface_is_not_imported_as_an_image() {
        let (device, _) = device(None);
        let _ = feature(&device).import_image(shared_event(5), properties(Default::default()));
    }

    #[test]
    #[should_panic(expected = "Native call failed")]
    fn a_failed_import_of_the_native_device_fails_the_import() {
        let (device, _) = device(None);
        let _ = feature(&device).import_image(io_surface(0), properties(Default::default()));
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported.")]
    fn a_handle_that_is_not_a_shared_event_is_not_imported_as_an_event() {
        let (device, _) = device(None);
        let _ = feature(&device).import_shared_event(io_surface(5));
    }

    #[test]
    fn the_commands_wait_for_and_signal_the_imported_event() {
        let (device, log) = device(None);
        let feature = feature(&device);

        let event = feature.import_shared_event(shared_event(9));
        assert_eq!(vec!["ImportSharedEvent(9)"], take(&log));
        assert_eq!(1009, event.handle() as usize);

        feature.submit_wait(&*event, 3);
        feature.submit_signal(&*event, 4);
        assert_eq!(vec!["SubmitWait(Some(1009), 3)", "SubmitSignal(Some(1009), 4)"], take(&log));

        event.dispose();
        assert_eq!(vec!["the event is released"], take(&log));
        event.dispose();
        assert!(take(&log).is_empty());
    }

    #[test]
    #[should_panic(expected = "Cannot access a disposed object: SharedEvent")]
    fn a_disposed_event_cannot_be_waited_for() {
        let (device, _) = device(None);
        let feature = feature(&device);
        let event = feature.import_shared_event(shared_event(9));
        event.dispose();
        feature.submit_wait(&*event, 3);
    }

    #[test]
    #[should_panic(expected = "The shared event belongs to a different platform backend.")]
    fn an_event_of_another_backend_is_refused() {
        struct ForeignEvent;

        impl IMetalSharedEvent for ForeignEvent {
            fn handle(&self) -> *mut c_void {
                std::ptr::null_mut()
            }

            fn dispose(&self) {}

            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        let (device, _) = device(None);
        feature(&device).submit_signal(&ForeignEvent, 1);
    }

    #[test]
    fn the_device_wraps_the_handles_it_imports() {
        let (device, log) = device(None);
        let features: &dyn IOptionalFeatureProvider = &*device;
        let wrap = features
            .try_get::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>()
            .expect("the device has the feature");
        let feature = feature(&device);

        // An IOSurface: retained when it is handed over, imported from the
        // wrapper, released when the import has run.
        let surface = io_surface(5);
        let wrapped = wrap
            .wrap_image_handle_on_any_thread(&surface, properties(Default::default()))
            .expect("an IOSurface is wrapped");
        drop(surface);
        let texture = feature.import_image(wrapped.clone().as_platform_handle(), properties(Default::default()));
        wrapped.dispose();
        assert_eq!(
            vec!["RetainCFObject(5)", "ImportIOSurface(5, kFrnRgba8888)", "ReleaseCFObject(5)"],
            take(&log)
        );
        drop(texture);
        let _ = take(&log);

        // A shared event likewise.
        let event = shared_event(9);
        let wrapped = wrap.wrap_semaphore_handle_on_any_thread(&event).expect("a shared event is wrapped");
        drop(event);
        let imported = feature.import_shared_event(wrapped.clone().as_platform_handle());
        wrapped.dispose();
        assert_eq!(vec!["RetainNSObject(9)", "ImportSharedEvent(9)", "ReleaseNSObject(9)"], take(&log));
        drop(imported);
    }

    #[test]
    fn the_wrapping_feature_outlives_a_disposed_device() {
        // As in the original, the feature holds the helper and not the
        // device: a handle wrapped before the device went is still released.
        let (device, log) = device(None);
        let features: &dyn IOptionalFeatureProvider = &*device;
        let wrap = features
            .try_get::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>()
            .expect("the device has the feature");
        let wrapped = wrap.wrap_semaphore_handle_on_any_thread(&shared_event(9)).expect("a shared event is wrapped");

        device.dispose();
        wrapped.dispose();
        assert_eq!(vec!["RetainNSObject(9)", "ReleaseNSObject(9)"], take(&log));
    }

    #[test]
    #[should_panic(expected = "Cannot access a disposed object: MetalDevice")]
    fn the_feature_of_a_disposed_device_imports_nothing() {
        let (device, _) = device(None);
        let feature = feature(&device);
        device.dispose();
        let _ = feature.import_shared_event(shared_event(9));
    }
}
