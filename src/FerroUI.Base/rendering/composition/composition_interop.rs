use super::{
    CompositionGpuImportedImageSynchronizationCapabilities, Compositor, ICompositionGpuImportedObject,
    ICompositionGpuInterop, ICompositionImportableSharedGpuContextImage, ICompositionImportableSharedGpuContextSemaphore,
    ICompositionImportedGpuImage, ICompositionImportedGpuSemaphore, ServerJobTask,
};
use crate::platform::{
    IExternalObjectsHandleWrapRenderInterfaceContextFeature, IExternalObjectsRenderInterfaceContextFeature,
    IExternalObjectsWrappedGpuHandle, IPlatformHandle, IPlatformRenderInterfaceContext,
    IPlatformRenderInterfaceImportedImage, IPlatformRenderInterfaceImportedSemaphore, PlatformGraphicsContextLostException,
    PlatformGraphicsDrmFormat, PlatformGraphicsExternalImageProperties,
};
use std::cell::{OnceCell, RefCell};
use std::rc::{Rc, Weak};

fn same_context(a: &Rc<dyn IPlatformRenderInterfaceContext>, b: &Rc<dyn IPlatformRenderInterfaceContext>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// The interop of a compositor with the GPU objects of its render
/// interface context.
pub struct CompositionInterop {
    compositor: Rc<Compositor>,
    context: Rc<dyn IPlatformRenderInterfaceContext>,
    external_objects: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
    external_objects_with_handle_wrap: Option<Rc<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>>,
    device_luid: RefCell<Option<Vec<u8>>>,
    device_uuid: RefCell<Option<Vec<u8>>>,
}

impl CompositionInterop {
    pub fn new(
        compositor: &Rc<Compositor>,
        external_objects: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
    ) -> Rc<CompositionInterop> {
        let context = compositor.server().render_interface().value();
        let external_objects_with_handle_wrap = {
            let features: &dyn crate::platform::IOptionalFeatureProvider = &*context;
            features.try_get::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>()
        };
        Rc::new(CompositionInterop {
            compositor: compositor.clone(),
            device_luid: RefCell::new(external_objects.device_luid()),
            device_uuid: RefCell::new(external_objects.device_uuid()),
            context,
            external_objects,
            external_objects_with_handle_wrap,
        })
    }
}

impl ICompositionGpuInterop for CompositionInterop {
    fn supported_image_handle_types(&self) -> Vec<String> {
        self.external_objects.supported_image_handle_types()
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        self.external_objects.supported_semaphore_types()
    }

    fn supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        self.external_objects.supported_dma_buf_formats()
    }

    fn get_synchronization_capabilities(&self, image_handle_type: &str) -> CompositionGpuImportedImageSynchronizationCapabilities {
        self.external_objects.get_synchronization_capabilities(image_handle_type)
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn ICompositionImportedGpuImage> {
        let wrapped = self
            .external_objects_with_handle_wrap
            .as_ref()
            .and_then(|wrap| wrap.wrap_image_handle_on_any_thread(&handle, properties.clone()));
        let handle = wrapped.clone().map(|wrapped| wrapped.as_platform_handle()).unwrap_or(handle);
        let external_objects = self.external_objects.clone();
        CompositionImportedGpuImage::new(
            &self.compositor,
            self.context.clone(),
            self.external_objects.clone(),
            Box::new(move || external_objects.import_image(handle, properties)),
            wrapped,
        )
    }

    fn import_shared_image(&self, image: Rc<dyn ICompositionImportableSharedGpuContextImage>) -> Rc<dyn ICompositionImportedGpuImage> {
        let external_objects = self.external_objects.clone();
        CompositionImportedGpuImage::new(
            &self.compositor,
            self.context.clone(),
            self.external_objects.clone(),
            Box::new(move || external_objects.import_shared_image(image)),
            None,
        )
    }

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn ICompositionImportedGpuSemaphore> {
        let wrapped =
            self.external_objects_with_handle_wrap.as_ref().and_then(|wrap| wrap.wrap_semaphore_handle_on_any_thread(&handle));
        let handle = wrapped.clone().map(|wrapped| wrapped.as_platform_handle()).unwrap_or(handle);
        CompositionImportedGpuSemaphore::new(handle, &self.compositor, self.context.clone(), self.external_objects.clone(), wrapped)
    }

    /// Not supported (`NotSupportedException` upstream).
    fn import_shared_semaphore(
        &self,
        _image: Rc<dyn ICompositionImportableSharedGpuContextSemaphore>,
    ) -> Rc<dyn ICompositionImportedGpuImage> {
        panic!("Specified method is not supported.");
    }

    fn is_lost(&self) -> bool {
        self.context.is_lost()
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

/// What upstream's `CompositionGpuImportedObjectBase` holds: the
/// compositor, the context and the feature the object was imported with,
/// and the task of the import.
pub struct CompositionGpuImportedObjectBase {
    compositor: Rc<Compositor>,
    context: Rc<dyn IPlatformRenderInterfaceContext>,
    feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
    import_completed: OnceCell<ServerJobTask<()>>,
}

impl CompositionGpuImportedObjectBase {
    fn new(
        compositor: &Rc<Compositor>,
        context: Rc<dyn IPlatformRenderInterfaceContext>,
        feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
    ) -> Self {
        Self { compositor: compositor.clone(), context, feature, import_completed: OnceCell::new() }
    }

    /// Schedules the import on the render thread. A wrapped handle is
    /// released once the import has run.
    fn start_import(
        &self,
        handle: Option<Rc<dyn IExternalObjectsWrappedGpuHandle>>,
        import: impl FnOnce() -> Result<(), Rc<dyn std::error::Error>> + 'static,
    ) {
        let task = self.compositor.invoke_server_job_async(
            move |_| {
                let result = import();
                if let Some(handle) = handle {
                    handle.dispose();
                }
                result
            },
            false,
        );
        let _ = self.import_completed.set(task);
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    pub fn context(&self) -> &Rc<dyn IPlatformRenderInterfaceContext> {
        &self.context
    }

    pub fn feature(&self) -> &Rc<dyn IExternalObjectsRenderInterfaceContextFeature> {
        &self.feature
    }

    pub fn import_completed(&self) -> ServerJobTask<()> {
        self.import_completed.get().cloned().expect("the import is started by the constructor")
    }

    pub fn is_lost(&self) -> bool {
        self.context.is_lost()
    }

    /// Whether the context of the server compositor is the one the object
    /// was imported with.
    fn is_current_context(&self) -> bool {
        same_context(&self.compositor.server().render_interface().value(), &self.context)
    }

    fn dispose_async(&self, dispose: impl FnOnce() + 'static) -> ServerJobTask<()> {
        let import_completed = self.import_completed();
        self.compositor.invoke_server_job_async(
            move |_| {
                if import_completed.is_completed_successfully() {
                    dispose();
                }
                Ok(())
            },
            false,
        )
    }
}

type ImageImporter = Box<dyn FnOnce() -> Rc<dyn IPlatformRenderInterfaceImportedImage>>;

/// A GPU image imported into the compositor.
pub struct CompositionImportedGpuImage {
    this: Weak<CompositionImportedGpuImage>,
    base: CompositionGpuImportedObjectBase,
    importer: RefCell<Option<ImageImporter>>,
    image: RefCell<Option<Rc<dyn IPlatformRenderInterfaceImportedImage>>>,
}

impl CompositionImportedGpuImage {
    fn new(
        compositor: &Rc<Compositor>,
        context: Rc<dyn IPlatformRenderInterfaceContext>,
        feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
        importer: ImageImporter,
        handle: Option<Rc<dyn IExternalObjectsWrappedGpuHandle>>,
    ) -> Rc<CompositionImportedGpuImage> {
        let image = Rc::new_cyclic(|this: &Weak<CompositionImportedGpuImage>| CompositionImportedGpuImage {
            this: this.clone(),
            base: CompositionGpuImportedObjectBase::new(compositor, context, feature),
            importer: RefCell::new(Some(importer)),
            image: RefCell::new(None),
        });
        let this = image.clone();
        image.base.start_import(handle, move || this.import());
        image
    }

    pub fn base(&self) -> &CompositionGpuImportedObjectBase {
        &self.base
    }

    /// The handle of the image.
    pub(crate) fn rc(&self) -> Rc<CompositionImportedGpuImage> {
        self.this.upgrade().expect("the image is alive while it is used")
    }

    fn import(&self) -> Result<(), Rc<dyn std::error::Error>> {
        let server = self.base.compositor.server().clone();
        let current = server.render_interface().ensure_current();
        let result = (|| {
            // The original context was lost and the new one might have different capabilities
            if !self.base.is_current_context() {
                return Err(Rc::new(PlatformGraphicsContextLostException) as Rc<dyn std::error::Error>);
            }
            if let Some(importer) = self.importer.borrow_mut().take() {
                *self.image.borrow_mut() = Some(importer());
            }
            Ok(())
        })();
        current.dispose();
        result
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

    pub fn is_usable(&self) -> bool {
        self.image.borrow().is_some() && self.base.is_current_context()
    }

    pub fn dispose(&self) {
        if let Some(image) = self.image.borrow_mut().take() {
            image.dispose();
        }
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
        // The job holds the object, as upstream it captures `this`: the
        // imported object is released even when the last other reference
        // is dropped before the job runs.
        let this = self.this.upgrade();
        self.base.dispose_async(move || {
            if let Some(this) = this {
                this.dispose();
            }
        })
    }
}

impl ICompositionImportedGpuImage for CompositionImportedGpuImage {
    fn as_imported_gpu_image(&self) -> Option<&CompositionImportedGpuImage> {
        Some(self)
    }
}

/// A GPU semaphore imported into the compositor.
pub struct CompositionImportedGpuSemaphore {
    this: Weak<CompositionImportedGpuSemaphore>,
    base: CompositionGpuImportedObjectBase,
    handle: Rc<dyn IPlatformHandle>,
    semaphore: RefCell<Option<Rc<dyn IPlatformRenderInterfaceImportedSemaphore>>>,
}

impl CompositionImportedGpuSemaphore {
    fn new(
        handle: Rc<dyn IPlatformHandle>,
        compositor: &Rc<Compositor>,
        context: Rc<dyn IPlatformRenderInterfaceContext>,
        feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature>,
        wrapped: Option<Rc<dyn IExternalObjectsWrappedGpuHandle>>,
    ) -> Rc<CompositionImportedGpuSemaphore> {
        let semaphore = Rc::new_cyclic(|this: &Weak<CompositionImportedGpuSemaphore>| CompositionImportedGpuSemaphore {
            this: this.clone(),
            base: CompositionGpuImportedObjectBase::new(compositor, context, feature),
            handle,
            semaphore: RefCell::new(None),
        });
        let this = semaphore.clone();
        semaphore.base.start_import(wrapped, move || {
            this.import();
            Ok(())
        });
        semaphore
    }

    pub fn base(&self) -> &CompositionGpuImportedObjectBase {
        &self.base
    }

    /// The handle of the semaphore.
    pub(crate) fn rc(&self) -> Rc<CompositionImportedGpuSemaphore> {
        self.this.upgrade().expect("the semaphore is alive while it is used")
    }

    fn import(&self) {
        *self.semaphore.borrow_mut() = Some(self.base.feature.import_semaphore(self.handle.clone()));
    }

    /// The imported semaphore. Panics once the semaphore has been disposed
    /// (`ObjectDisposedException`).
    pub fn semaphore(&self) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        match self.semaphore.borrow().clone() {
            Some(semaphore) => semaphore,
            None => panic!("Cannot access a disposed object.\nObject name: 'CompositionImportedGpuSemaphore'."),
        }
    }

    pub fn is_usable(&self) -> bool {
        self.semaphore.borrow().is_some() && self.base.is_current_context()
    }

    pub fn dispose(&self) {
        if let Some(semaphore) = self.semaphore.borrow_mut().take() {
            semaphore.dispose();
        }
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
        // The job holds the object, as upstream it captures `this`: the
        // imported object is released even when the last other reference
        // is dropped before the job runs.
        let this = self.this.upgrade();
        self.base.dispose_async(move || {
            if let Some(this) = this {
                this.dispose();
            }
        })
    }
}

impl ICompositionImportedGpuSemaphore for CompositionImportedGpuSemaphore {
    fn as_imported_gpu_semaphore(&self) -> Option<&CompositionImportedGpuSemaphore> {
        Some(self)
    }
}
