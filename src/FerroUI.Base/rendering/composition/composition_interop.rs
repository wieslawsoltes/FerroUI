//! The interop of a compositor with GPU objects made outside of the
//! framework.
//!
//! Upstream one object serves both threads: the UI thread creates a
//! `CompositionImportedGpuImage` and keeps it, jobs of the render thread
//! import, use and dispose what is inside. Here each of these objects has
//! two parts. The part the caller holds is an object of its thread. What the
//! render thread works with (the context, the feature of the render
//! interface, the imported object and the outcome of the import) is the
//! server part, confined to the compositor lock in a [`LockBound`]: the
//! jobs reach it there, and this thread enters the lock for the few things
//! it reads from it.

use super::server::{LockBound, ServerCompositor};
use super::{
    CompositionGpuImportedImageSynchronizationCapabilities, Compositor, ICompositionGpuImportedObject,
    ICompositionGpuInterop, ICompositionImportableSharedGpuContextImage, ICompositionImportableSharedGpuContextSemaphore,
    ICompositionImportedGpuImage, ICompositionImportedGpuSemaphore, ServerJobError, ServerJobTask,
};
use crate::platform::{
    IExternalObjectsHandleWrapRenderInterfaceContextFeature, IExternalObjectsRenderInterfaceContextFeature,
    IExternalObjectsWrappedGpuHandle, IPlatformHandle, IPlatformRenderInterfaceContext,
    IPlatformRenderInterfaceImportedImage, IPlatformRenderInterfaceImportedSemaphore, PlatformGraphicsContextLostException,
    PlatformGraphicsDrmFormat, PlatformGraphicsExternalImageProperties, PlatformHandle,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

fn same_context(a: &Rc<dyn IPlatformRenderInterfaceContext>, b: &Rc<dyn IPlatformRenderInterfaceContext>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// The server part of the interop: the context of the render interface and
/// its features. Objects of the server side, kept inside the compositor
/// lock.
struct InteropState {
    context: Rc<dyn IPlatformRenderInterfaceContext>,
    external_objects: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
    external_objects_with_handle_wrap: Option<Rc<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>>,
}

impl InteropState {
    fn imported_object_base(&self) -> ServerGpuImportedObjectBase {
        ServerGpuImportedObjectBase {
            context: self.context.clone(),
            feature: self.external_objects.clone(),
            import_result: RefCell::new(None),
        }
    }
}

/// The interop of a compositor with the GPU objects of its render
/// interface context.
pub struct CompositionInterop {
    compositor: Rc<Compositor>,
    state: LockBound<InteropState>,
    device_luid: RefCell<Option<Vec<u8>>>,
    device_uuid: RefCell<Option<Vec<u8>>>,
}

impl CompositionInterop {
    /// Creates the interop over a feature of the render interface.
    ///
    /// The feature is an object of the server side and is kept inside the
    /// compositor lock from here on: a caller that is not inside the lock
    /// hands it over whole, without keeping a clone. (Upstream the
    /// constructor runs in a job of the render thread.)
    ///
    /// # Panics
    ///
    /// When the compositor is confined to its render thread and that thread
    /// has no context of the render interface: see [`try_new`](Self::try_new).
    pub fn new(
        compositor: &Rc<Compositor>,
        external_objects: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
    ) -> Rc<CompositionInterop> {
        match Self::try_new(compositor, external_objects) {
            Some(interop) => interop,
            None => panic!("the context of the render interface is created by the render thread, which has none"),
        }
    }

    /// [`new`](Self::new), for a caller that can do without the interop.
    ///
    /// Upstream the constructor takes `RenderInterface.Value`, which creates
    /// the context when there is none. The thread of a compositor that is
    /// confined to its render thread may not create it: there the context
    /// the render thread has is taken as it is, and without one the answer
    /// is `None`.
    pub(crate) fn try_new(
        compositor: &Rc<Compositor>,
        external_objects: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
    ) -> Option<Rc<CompositionInterop>> {
        compositor.with_server(|server| {
            let context = if compositor.is_confined_to_render_thread() {
                server.render_interface().existing_backend_context()?
            } else {
                server.render_interface().value()
            };
            let external_objects_with_handle_wrap = {
                let features: &dyn crate::platform::IOptionalFeatureProvider = &*context;
                features.try_get::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>()
            };
            Some(Rc::new(CompositionInterop {
                compositor: compositor.clone(),
                device_luid: RefCell::new(external_objects.device_luid()),
                device_uuid: RefCell::new(external_objects.device_uuid()),
                state: compositor.bind_to_lock(
                    server,
                    InteropState { context, external_objects, external_objects_with_handle_wrap },
                ),
            }))
        })
    }

    /// Runs `f` with the server part, inside the compositor lock. Upstream
    /// reads the feature from the UI thread as it is.
    fn with_state<R>(&self, f: impl FnOnce(&ServerCompositor, &InteropState) -> R) -> R {
        self.compositor.with_server(|server| {
            let server: &ServerCompositor = server;
            f(server, self.state.get(server))
        })
    }
}

impl ICompositionGpuInterop for CompositionInterop {
    fn supported_image_handle_types(&self) -> Vec<String> {
        self.with_state(|_, state| state.external_objects.supported_image_handle_types())
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        self.with_state(|_, state| state.external_objects.supported_semaphore_types())
    }

    fn supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        self.with_state(|_, state| state.external_objects.supported_dma_buf_formats())
    }

    fn get_synchronization_capabilities(&self, image_handle_type: &str) -> CompositionGpuImportedImageSynchronizationCapabilities {
        self.with_state(|_, state| state.external_objects.get_synchronization_capabilities(image_handle_type))
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn ICompositionImportedGpuImage> {
        let server = self.with_state(|server, state| {
            let wrapped = state
                .external_objects_with_handle_wrap
                .as_ref()
                .and_then(|wrap| wrap.wrap_image_handle_on_any_thread(&handle, properties.clone()));
            let source = ImageSource::Handle(ImportHandle::new(wrapped, &*handle), properties);
            Arc::new(self.compositor.bind_to_lock(server, ServerImportedGpuImage::new(state, source)))
        });
        CompositionImportedGpuImage::new(&self.compositor, server)
    }

    fn import_shared_image(&self, image: Arc<dyn ICompositionImportableSharedGpuContextImage>) -> Rc<dyn ICompositionImportedGpuImage> {
        // The image is shared with the caller, who keeps it (see
        // `ImageSource::Shared`).
        let source = ImageSource::Shared(image);
        let server = self.with_state(|server, state| {
            Arc::new(self.compositor.bind_to_lock(server, ServerImportedGpuImage::new(state, source)))
        });
        CompositionImportedGpuImage::new(&self.compositor, server)
    }

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn ICompositionImportedGpuSemaphore> {
        let server = self.with_state(|server, state| {
            let wrapped = state
                .external_objects_with_handle_wrap
                .as_ref()
                .and_then(|wrap| wrap.wrap_semaphore_handle_on_any_thread(&handle));
            let handle = ImportHandle::new(wrapped, &*handle);
            Arc::new(self.compositor.bind_to_lock(server, ServerImportedGpuSemaphore::new(state, handle)))
        });
        CompositionImportedGpuSemaphore::new(&self.compositor, server)
    }

    /// Not supported (`NotSupportedException` upstream).
    fn import_shared_semaphore(
        &self,
        _image: Rc<dyn ICompositionImportableSharedGpuContextSemaphore>,
    ) -> Rc<dyn ICompositionImportedGpuImage> {
        panic!("Specified method is not supported.");
    }

    fn is_lost(&self) -> bool {
        self.with_state(|_, state| state.context.is_lost())
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.device_luid.borrow().clone()
    }

    fn set_device_luid(&self, value: Option<Vec<u8>>) {
        *self.device_luid.borrow_mut() = value;
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        self.device_uuid.borrow().clone()
    }

    fn set_device_uuid(&self, value: Option<Vec<u8>>) {
        *self.device_uuid.borrow_mut() = value;
    }
}

/// The handle an object is imported from, as the render thread gets it.
enum ImportHandle {
    /// The handle the backend wrapped for the import. Made inside the
    /// compositor lock and never handed to the caller: an object of the
    /// server side.
    Wrapped(Rc<dyn IExternalObjectsWrappedGpuHandle>),
    /// A copy of the value and the descriptor of the caller's handle.
    /// Upstream the render thread reads the caller's object; here that
    /// object stays on its thread, and the backend is handed a
    /// [`PlatformHandle`] with the same contents.
    Copied(PlatformHandle),
}

impl ImportHandle {
    fn new(wrapped: Option<Rc<dyn IExternalObjectsWrappedGpuHandle>>, handle: &dyn IPlatformHandle) -> ImportHandle {
        match wrapped {
            Some(wrapped) => ImportHandle::Wrapped(wrapped),
            None => ImportHandle::Copied(PlatformHandle::new(handle.handle(), handle.handle_descriptor())),
        }
    }

    fn platform_handle(&self) -> Rc<dyn IPlatformHandle> {
        match self {
            ImportHandle::Wrapped(wrapped) => wrapped.clone().as_platform_handle(),
            ImportHandle::Copied(handle) => Rc::new(handle.clone()),
        }
    }

    /// Releases a wrapped handle: once the import has run.
    fn dispose(self) {
        if let ImportHandle::Wrapped(wrapped) = self {
            wrapped.dispose();
        }
    }
}

/// What an image is imported from.
enum ImageSource {
    Handle(ImportHandle, PlatformGraphicsExternalImageProperties),
    /// An image of a context that shares with the one of the compositor.
    /// The caller keeps a reference and disposes it, and the render thread
    /// reads it during the import, as upstream: the image is an object the
    /// two threads share (`Send + Sync`), not one of the lock.
    Shared(Arc<dyn ICompositionImportableSharedGpuContextImage>),
}

impl ImageSource {
    fn import(
        &self,
        feature: &dyn IExternalObjectsRenderInterfaceContextFeature,
    ) -> Result<Rc<dyn IPlatformRenderInterfaceImportedImage>, ServerJobError> {
        match self {
            ImageSource::Handle(handle, properties) => Ok(feature.import_image(handle.platform_handle(), properties.clone())),
            ImageSource::Shared(image) => Ok(feature.import_shared_image(image.clone())),
        }
    }

    fn dispose(self) {
        if let ImageSource::Handle(handle, _) = self {
            handle.dispose();
        }
    }
}

/// What the server part of every imported object holds: the context and the
/// feature the object is imported with, and the outcome of the import.
pub struct ServerGpuImportedObjectBase {
    context: Rc<dyn IPlatformRenderInterfaceContext>,
    feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
    /// The outcome of the import job, `None` until it has run: what the
    /// status of `ImportCompleted` tells the jobs upstream.
    import_result: RefCell<Option<Result<(), ServerJobError>>>,
}

impl ServerGpuImportedObjectBase {
    pub fn context(&self) -> &Rc<dyn IPlatformRenderInterfaceContext> {
        &self.context
    }

    pub fn feature(&self) -> &Rc<dyn IExternalObjectsRenderInterfaceContextFeature> {
        &self.feature
    }

    /// The outcome of the import, `None` until the import job has run.
    pub fn import_result(&self) -> Option<Result<(), ServerJobError>> {
        self.import_result.borrow().clone()
    }

    /// Whether the context of the server compositor is the one the object
    /// was imported with.
    fn is_current_context(&self, server: &ServerCompositor) -> bool {
        same_context(&server.render_interface().value(), &self.context)
    }
}

/// The server part of an imported object: what the jobs of the render
/// thread import, use and dispose.
pub trait IServerGpuImportedObject: 'static {
    fn base(&self) -> &ServerGpuImportedObjectBase;

    /// Imports the object (`Import`).
    fn import(&self, server: &ServerCompositor) -> Result<(), ServerJobError>;

    /// Releases the imported object (`Dispose`).
    fn dispose(&self);
}

/// What upstream's `CompositionGpuImportedObjectBase` holds on the side of
/// the caller: the compositor, the task of the import and the handle of the
/// server part.
pub struct CompositionGpuImportedObjectBase<T: IServerGpuImportedObject> {
    compositor: Rc<Compositor>,
    server: Arc<LockBound<T>>,
    import_completed: ServerJobTask<()>,
}

impl<T: IServerGpuImportedObject> CompositionGpuImportedObjectBase<T> {
    /// Schedules the import on the render thread.
    fn new(compositor: &Rc<Compositor>, server: Arc<LockBound<T>>) -> Self {
        let object = server.clone();
        let import_completed = compositor.invoke_server_job_async(
            move |compositor| {
                let object = object.get(compositor);
                let result = object.import(compositor);
                *object.base().import_result.borrow_mut() = Some(result.clone());
                result
            },
            false,
        );
        Self { compositor: compositor.clone(), server, import_completed }
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// The server part: for a job of the render thread to capture.
    pub(crate) fn server(&self) -> Arc<LockBound<T>> {
        self.server.clone()
    }

    pub fn import_completed(&self) -> ServerJobTask<()> {
        self.import_completed.clone()
    }

    /// Upstream reads the context from the UI thread as it is; here this
    /// thread enters the compositor lock for it.
    pub fn is_lost(&self) -> bool {
        self.compositor.with_server(|server| self.server.get(server).base().context.is_lost())
    }

    fn dispose_async(&self) -> ServerJobTask<()> {
        // The job holds the server part, as upstream it captures `this`: the
        // imported object is released even when the caller drops its object
        // before the job runs.
        let object = self.server.clone();
        self.compositor.invoke_server_job_async(
            move |compositor| {
                let object = object.get(compositor);
                if matches!(object.base().import_result(), Some(Ok(()))) {
                    object.dispose();
                }
                Ok(())
            },
            false,
        )
    }
}

/// The server part of an imported GPU image.
pub struct ServerImportedGpuImage {
    base: ServerGpuImportedObjectBase,
    source: RefCell<Option<ImageSource>>,
    image: RefCell<Option<Rc<dyn IPlatformRenderInterfaceImportedImage>>>,
}

impl ServerImportedGpuImage {
    fn new(state: &InteropState, source: ImageSource) -> ServerImportedGpuImage {
        ServerImportedGpuImage {
            base: state.imported_object_base(),
            source: RefCell::new(Some(source)),
            image: RefCell::new(None),
        }
    }

    /// The imported image. Panics once the image has been disposed
    /// (`ObjectDisposedException`).
    pub fn image(&self) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        match self.image.borrow().clone() {
            Some(image) => image,
            None => panic!("Cannot access a disposed object.\nObject name: 'CompositionImportedGpuImage'."),
        }
    }

    pub fn context(&self) -> &Rc<dyn IPlatformRenderInterfaceContext> {
        &self.base.context
    }

    pub fn is_usable(&self, server: &ServerCompositor) -> bool {
        self.image.borrow().is_some() && self.base.is_current_context(server)
    }
}

impl IServerGpuImportedObject for ServerImportedGpuImage {
    fn base(&self) -> &ServerGpuImportedObjectBase {
        &self.base
    }

    fn import(&self, server: &ServerCompositor) -> Result<(), ServerJobError> {
        let source = self.source.borrow_mut().take();
        let current = server.render_interface().ensure_current();
        let result = (|| {
            // The original context was lost and the new one might have different capabilities
            if !self.base.is_current_context(server) {
                return Err(Arc::new(PlatformGraphicsContextLostException) as ServerJobError);
            }
            if let Some(source) = &source {
                *self.image.borrow_mut() = Some(source.import(&*self.base.feature)?);
            }
            Ok(())
        })();
        current.dispose();
        // A wrapped handle is released once the import has run.
        if let Some(source) = source {
            source.dispose();
        }
        result
    }

    fn dispose(&self) {
        if let Some(image) = self.image.borrow_mut().take() {
            image.dispose();
        }
    }
}

/// A GPU image imported into the compositor.
pub struct CompositionImportedGpuImage {
    base: CompositionGpuImportedObjectBase<ServerImportedGpuImage>,
}

impl CompositionImportedGpuImage {
    fn new(compositor: &Rc<Compositor>, server: Arc<LockBound<ServerImportedGpuImage>>) -> Rc<CompositionImportedGpuImage> {
        Rc::new(CompositionImportedGpuImage { base: CompositionGpuImportedObjectBase::new(compositor, server) })
    }

    pub fn base(&self) -> &CompositionGpuImportedObjectBase<ServerImportedGpuImage> {
        &self.base
    }

    /// The server part of the image: for a job of the render thread.
    pub(crate) fn server(&self) -> Arc<LockBound<ServerImportedGpuImage>> {
        self.base.server()
    }
}

impl ICompositionGpuImportedObject for CompositionImportedGpuImage {
    fn import_completed(&self) -> ServerJobTask<()> {
        self.base.import_completed()
    }

    fn is_lost(&self) -> bool {
        self.base.is_lost()
    }

    fn dispose_async(&self) -> ServerJobTask<()> {
        self.base.dispose_async()
    }
}

impl ICompositionImportedGpuImage for CompositionImportedGpuImage {
    fn as_imported_gpu_image(&self) -> Option<&CompositionImportedGpuImage> {
        Some(self)
    }
}

/// The server part of an imported GPU semaphore.
pub struct ServerImportedGpuSemaphore {
    base: ServerGpuImportedObjectBase,
    handle: RefCell<Option<ImportHandle>>,
    semaphore: RefCell<Option<Rc<dyn IPlatformRenderInterfaceImportedSemaphore>>>,
}

impl ServerImportedGpuSemaphore {
    fn new(state: &InteropState, handle: ImportHandle) -> ServerImportedGpuSemaphore {
        ServerImportedGpuSemaphore {
            base: state.imported_object_base(),
            handle: RefCell::new(Some(handle)),
            semaphore: RefCell::new(None),
        }
    }

    /// The imported semaphore. Panics once the semaphore has been disposed
    /// (`ObjectDisposedException`).
    pub fn semaphore(&self) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        match self.semaphore.borrow().clone() {
            Some(semaphore) => semaphore,
            None => panic!("Cannot access a disposed object.\nObject name: 'CompositionImportedGpuSemaphore'."),
        }
    }

    pub fn is_usable(&self, server: &ServerCompositor) -> bool {
        self.semaphore.borrow().is_some() && self.base.is_current_context(server)
    }
}

impl IServerGpuImportedObject for ServerImportedGpuSemaphore {
    fn base(&self) -> &ServerGpuImportedObjectBase {
        &self.base
    }

    fn import(&self, _server: &ServerCompositor) -> Result<(), ServerJobError> {
        let handle = self.handle.borrow_mut().take();
        if let Some(handle) = handle {
            *self.semaphore.borrow_mut() = Some(self.base.feature.import_semaphore(handle.platform_handle()));
            // A wrapped handle is released once the import has run.
            handle.dispose();
        }
        Ok(())
    }

    fn dispose(&self) {
        if let Some(semaphore) = self.semaphore.borrow_mut().take() {
            semaphore.dispose();
        }
    }
}

/// A GPU semaphore imported into the compositor.
pub struct CompositionImportedGpuSemaphore {
    base: CompositionGpuImportedObjectBase<ServerImportedGpuSemaphore>,
}

impl CompositionImportedGpuSemaphore {
    fn new(
        compositor: &Rc<Compositor>,
        server: Arc<LockBound<ServerImportedGpuSemaphore>>,
    ) -> Rc<CompositionImportedGpuSemaphore> {
        Rc::new(CompositionImportedGpuSemaphore { base: CompositionGpuImportedObjectBase::new(compositor, server) })
    }

    pub fn base(&self) -> &CompositionGpuImportedObjectBase<ServerImportedGpuSemaphore> {
        &self.base
    }

    /// The server part of the semaphore: for a job of the render thread.
    pub(crate) fn server(&self) -> Arc<LockBound<ServerImportedGpuSemaphore>> {
        self.base.server()
    }
}

impl ICompositionGpuImportedObject for CompositionImportedGpuSemaphore {
    fn import_completed(&self) -> ServerJobTask<()> {
        self.base.import_completed()
    }

    fn is_lost(&self) -> bool {
        self.base.is_lost()
    }

    fn dispose_async(&self) -> ServerJobTask<()> {
        self.base.dispose_async()
    }
}

impl ICompositionImportedGpuSemaphore for CompositionImportedGpuSemaphore {
    fn as_imported_gpu_semaphore(&self) -> Option<&CompositionImportedGpuSemaphore> {
        Some(self)
    }
}
