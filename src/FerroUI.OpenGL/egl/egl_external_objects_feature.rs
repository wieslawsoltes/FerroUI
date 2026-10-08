use super::egl_external_objects_feature_drm::DmaBufImport;
use super::EglContext;
use crate::features::ExternalObjectsOpenGlExtensionFeature;
use crate::{
    IGlContext, IGlContextExternalObjectsFeature, IGlExportableExternalImageTexture, IGlExternalImageTexture,
    IGlExternalSemaphore, OpenGlException,
};
use ferroui_base::platform::{
    IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, PlatformGraphicsDrmFormat,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
};
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use ferroui_base::PixelSize;
use std::rc::{Rc, Weak};

/// The external objects feature of an EGL context: what the extensions of OpenGL offer
/// ([`ExternalObjectsOpenGlExtensionFeature`]), and the import of dma-buf images through
/// `EGL_EXT_image_dma_buf_import`.
///
/// The part of the class that imports dma-buf images (the second file of the partial class
/// of the original) is in `egl_external_objects_feature_drm.rs`.
// The context holds the feature among its features, so the feature refers to its context
// weakly: the original holds the context, which a garbage collector allows.
pub(crate) struct EglExternalObjectsFeature {
    pub(super) context: Weak<EglContext>,
    gl_external_objects: Option<Rc<ExternalObjectsOpenGlExtensionFeature>>,
    image_types: Vec<String>,
    /// The fields of the second file of the partial class.
    pub(super) drm: DmaBufImport,
}

impl EglExternalObjectsFeature {
    /// Creates the feature for a context that is current; `None` when the context can
    /// neither import dma-buf images nor has the extensions of OpenGL for external objects.
    pub(crate) fn try_create(context: &Rc<EglContext>) -> Option<Rc<EglExternalObjectsFeature>> {
        let feature = Self::new(context);
        if !feature.image_types.is_empty() || feature.gl_external_objects.is_some() {
            Some(Rc::new(feature))
        } else {
            None
        }
    }

    fn new(context: &Rc<EglContext>) -> Self {
        let gl_context: Rc<dyn IGlContext> = context.clone();
        let gl_external_objects = ExternalObjectsOpenGlExtensionFeature::try_create(&gl_context);
        let mut image_types = Vec::new();
        if let Some(gl_external_objects) = &gl_external_objects {
            image_types.extend(gl_external_objects.supported_importable_external_image_types());
        }

        let drm = Self::try_initialize_drm(context);
        if drm.dma_buf_supported() {
            image_types.push(KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR.to_string());
        }

        Self { context: Rc::downgrade(context), gl_external_objects, image_types, drm }
    }

    pub(super) fn is_dma_buf(type_: Option<&str>) -> bool {
        type_ == Some(KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR)
    }

    /// The failure of an import of a handle whose type nothing imports
    /// (`ArgumentException(handle.HandleDescriptor + " is not supported")`).
    pub(super) fn not_supported(handle: &dyn IPlatformHandle) -> OpenGlException {
        OpenGlException::new(format!("{} is not supported", handle.handle_descriptor().unwrap_or("")))
    }
}

impl IGlContextExternalObjectsFeature for EglExternalObjectsFeature {
    fn supported_importable_external_image_types(&self) -> Vec<String> {
        self.image_types.clone()
    }

    fn supported_exportable_external_image_types(&self) -> Vec<String> {
        self.gl_external_objects
            .as_ref()
            .map(|gl_external_objects| gl_external_objects.supported_exportable_external_image_types())
            .unwrap_or_default()
    }

    fn supported_importable_external_semaphore_types(&self) -> Vec<String> {
        self.gl_external_objects
            .as_ref()
            .map(|gl_external_objects| gl_external_objects.supported_importable_external_semaphore_types())
            .unwrap_or_default()
    }

    fn supported_exportable_external_semaphore_types(&self) -> Vec<String> {
        self.gl_external_objects
            .as_ref()
            .map(|gl_external_objects| gl_external_objects.supported_exportable_external_semaphore_types())
            .unwrap_or_default()
    }

    fn get_supported_formats_for_external_memory_type(&self, type_: &str) -> Vec<PlatformGraphicsExternalImageFormat> {
        if Self::is_dma_buf(Some(type_)) {
            return Self::get_supported_dma_buf_image_formats();
        }
        self.gl_external_objects
            .as_ref()
            .map(|gl_external_objects| gl_external_objects.get_supported_formats_for_external_memory_type(type_))
            .unwrap_or_default()
    }

    // Declared in the second file of the partial class of the original (`SupportedDmaBufFormats`);
    // it is a member of the contract, so it is implemented with the others.
    fn supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        self.get_supported_dma_buf_formats()
    }

    fn create_image(
        &self,
        type_: &str,
        size: PixelSize,
        format: PlatformGraphicsExternalImageFormat,
    ) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlException> {
        match &self.gl_external_objects {
            Some(gl_external_objects) if !Self::is_dma_buf(Some(type_)) => {
                gl_external_objects.create_image(type_, size, format)
            }
            _ => Err(OpenGlException::new("Specified method is not supported.")),
        }
    }

    fn create_semaphore(&self, type_: &str) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlException> {
        match &self.gl_external_objects {
            Some(gl_external_objects) => gl_external_objects.create_semaphore(type_),
            None => Err(OpenGlException::new("Specified method is not supported.")),
        }
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Result<Rc<dyn IGlExternalImageTexture>, OpenGlException> {
        if Self::is_dma_buf(handle.handle_descriptor()) {
            return self.import_dma_buf_image(handle, properties);
        }
        match &self.gl_external_objects {
            Some(gl_external_objects) => gl_external_objects.import_image(handle, properties),
            None => Err(Self::not_supported(&*handle)),
        }
    }

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Result<Rc<dyn IGlExternalSemaphore>, OpenGlException> {
        match &self.gl_external_objects {
            Some(gl_external_objects) => gl_external_objects.import_semaphore(handle),
            None => Err(Self::not_supported(&*handle)),
        }
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        if Self::is_dma_buf(Some(image_handle_type)) {
            return CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC;
        }
        self.gl_external_objects
            .as_ref()
            .map(|gl_external_objects| gl_external_objects.get_synchronization_capabilities(image_handle_type))
            .unwrap_or_default()
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.gl_external_objects.as_ref().and_then(|gl_external_objects| gl_external_objects.device_luid())
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        self.gl_external_objects.as_ref().and_then(|gl_external_objects| gl_external_objects.device_uuid())
    }
}
