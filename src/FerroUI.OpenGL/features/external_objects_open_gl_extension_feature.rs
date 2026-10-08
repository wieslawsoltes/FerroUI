use crate::entry_points::{gl_entry_points, GetProcAddress};
use crate::gl_consts::*;
use crate::{
    IGlContext, IGlContextExternalObjectsFeature, IGlExportableExternalImageTexture, IGlExternalImageTexture,
    IGlExternalSemaphore, OpenGlException,
};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{
    IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, KnownPlatformGraphicsExternalSemaphoreHandleTypes,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
};
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use ferroui_base::PixelSize;
use std::cell::Cell;
use std::ptr;
use std::rc::Rc;

/// The entry points of `GL_EXT_memory_object`, `GL_EXT_semaphore` and their file descriptor
/// forms.
struct ExternalObjectsInterface {
    ext: ExternalObjectsEntryPoints,
}

/// The entry points without which the interface cannot be created.
const REQUIRED_ENTRY_POINTS: [&str; 7] = [
    "glCreateMemoryObjectsEXT",
    "glDeleteMemoryObjectsEXT",
    "glTexStorageMem2DEXT",
    "glGenSemaphoresEXT",
    "glDeleteSemaphoresEXT",
    "glWaitSemaphoreEXT",
    "glSignalSemaphoreEXT",
];

gl_entry_points! {
    table ExternalObjectsEntryPoints for ExternalObjectsInterface.ext(get, info: ());

    optional safe pub(crate) fn import_memory_fd_ext(memory: u32, size: u64, handle_type: i32, fd: i32)
        = get("glImportMemoryFdEXT");

    optional safe pub(crate) fn import_semaphore_fd_ext(semaphore: u32, handle_type: i32, fd: i32)
        = get("glImportSemaphoreFdEXT");

    required unsafe pub(crate) fn create_memory_objects_ext(n: i32, memory_objects: *mut u32)
        = get("glCreateMemoryObjectsEXT");

    required unsafe pub(crate) fn delete_memory_objects_ext(n: i32, objects: *const u32)
        = get("glDeleteMemoryObjectsEXT");

    required safe pub(crate) fn tex_storage_mem_2d_ext(
        target: i32,
        levels: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        memory: u32,
        offset: u64
    ) = get("glTexStorageMem2DEXT");

    required unsafe pub(crate) fn gen_semaphores_ext(n: i32, semaphores: *mut u32) = get("glGenSemaphoresEXT");

    required unsafe pub(crate) fn delete_semaphores_ext(n: i32, semaphores: *const u32) = get("glDeleteSemaphoresEXT");

    required unsafe pub(crate) fn wait_semaphore_ext(
        semaphore: u32,
        num_buffer_barriers: u32,
        buffers: *const u32,
        num_texture_barriers: u32,
        textures: *const i32,
        src_layouts: *const i32
    ) = get("glWaitSemaphoreEXT");

    required unsafe pub(crate) fn signal_semaphore_ext(
        semaphore: u32,
        num_buffer_barriers: u32,
        buffers: *const u32,
        num_texture_barriers: u32,
        textures: *const i32,
        dst_layouts: *const i32
    ) = get("glSignalSemaphoreEXT");

    optional unsafe pub(crate) fn get_unsigned_bytei_v_ext(target: i32, index: u32, data: *mut u8)
        = get("glGetUnsignedBytei_vEXT");

    optional unsafe pub(crate) fn get_unsigned_bytev_ext(target: i32, data: *mut u8) = get("glGetUnsignedBytevEXT");
}

impl ExternalObjectsInterface {
    /// Resolves the entry points; the name of the first required one that is missing
    /// otherwise.
    fn try_new(get_proc_address: &GetProcAddress) -> Result<Self, &'static str> {
        if let Some(missing) = REQUIRED_ENTRY_POINTS.iter().find(|name| get_proc_address(name).is_null()) {
            return Err(*missing);
        }
        Ok(Self { ext: ExternalObjectsEntryPoints::load(get_proc_address, &()) })
    }
}

/// The external objects feature of a context that has the extensions
/// `GL_EXT_memory_object` and `GL_EXT_semaphore`.
pub struct ExternalObjectsOpenGlExtensionFeature {
    context: Rc<dyn IGlContext>,
    ext: Rc<ExternalObjectsInterface>,
    image_types: Vec<String>,
    semaphore_types: Vec<String>,
    device_luid: Option<Vec<u8>>,
    device_uuid: Option<Vec<u8>>,
}

impl ExternalObjectsOpenGlExtensionFeature {
    /// Creates the feature for a context that has the extensions; `None` for a context
    /// without them, or when the feature cannot be initialized (which is logged).
    pub fn try_create(context: &Rc<dyn IGlContext>) -> Option<Rc<ExternalObjectsOpenGlExtensionFeature>> {
        let extensions = context.gl_interface().get_extensions();
        let has = |extension: &str| extensions.iter().any(|e| e == extension);
        if has("GL_EXT_memory_object") && has("GL_EXT_semaphore") {
            match Self::new(context, &extensions) {
                Ok(feature) => return Some(Rc::new(feature)),
                Err(e) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                        logger.log(None, &format!("Unable to initialize EXT_external_objects extension: {e}"));
                    }
                }
            }
        }

        None
    }

    fn new(context: &Rc<dyn IGlContext>, extensions: &[String]) -> Result<Self, OpenGlException> {
        let has = |extension: &str| extensions.iter().any(|e| e == extension);
        let gl = context.gl_interface();
        let get_proc_address: GetProcAddress = {
            let gl = gl.clone();
            Rc::new(move |name: &str| gl.get_proc_address(name))
        };
        let ext = ExternalObjectsInterface::try_new(&get_proc_address)
            .map_err(|name| OpenGlException::new(format!("Unable to find an entry point named '{name}'.")))?;

        let mut device_uuid = None;
        if ext.is_get_unsigned_bytei_v_ext_available() {
            let num_uuids = gl.get_integerv(GL_NUM_DEVICE_UUIDS_EXT);
            if num_uuids > 0 {
                let mut uuid = vec![0u8; 16];
                // SAFETY: a device UUID is 16 bytes, which the buffer holds.
                unsafe { ext.get_unsigned_bytei_v_ext(GL_DEVICE_UUID_EXT, 0, uuid.as_mut_ptr()) };
                device_uuid = Some(uuid);
            }
        }

        let mut device_luid = None;
        if ext.is_get_unsigned_bytev_ext_available()
            && (has("GL_EXT_memory_object_win32") || has("GL_EXT_semaphore_win32"))
        {
            let mut luid = vec![0u8; 8];
            // SAFETY: a device LUID is 8 bytes, which the buffer holds.
            unsafe { ext.get_unsigned_bytev_ext(GL_DEVICE_LUID_EXT, luid.as_mut_ptr()) };
            device_luid = Some(luid);
        }

        let mut image_types = Vec::new();
        let mut semaphore_types = Vec::new();
        if has("GL_EXT_memory_object_fd") && has("GL_EXT_semaphore_fd") {
            image_types.push(KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR.to_string());
            semaphore_types
                .push(KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR.to_string());
        }

        Ok(Self { context: context.clone(), ext: Rc::new(ext), image_types, semaphore_types, device_luid, device_uuid })
    }
}

/// `handle.Handle.ToInt32()`.
///
/// # Panics
/// Panics if the handle is not a file descriptor (the overflow of the original).
fn file_descriptor(handle: &dyn IPlatformHandle) -> i32 {
    i32::try_from(handle.handle()).expect("the handle is a file descriptor")
}

impl IGlContextExternalObjectsFeature for ExternalObjectsOpenGlExtensionFeature {
    fn supported_importable_external_image_types(&self) -> Vec<String> {
        self.image_types.clone()
    }

    fn supported_exportable_external_image_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn supported_importable_external_semaphore_types(&self) -> Vec<String> {
        self.semaphore_types.clone()
    }

    fn supported_exportable_external_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_supported_formats_for_external_memory_type(&self, _type: &str) -> Vec<PlatformGraphicsExternalImageFormat> {
        vec![PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm]
    }

    fn create_image(
        &self,
        _type: &str,
        _size: PixelSize,
        _format: PlatformGraphicsExternalImageFormat,
    ) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlException> {
        Err(OpenGlException::new("Specified method is not supported."))
    }

    fn create_semaphore(&self, _type: &str) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlException> {
        Err(OpenGlException::new("Specified method is not supported."))
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Result<Rc<dyn IGlExternalImageTexture>, OpenGlException> {
        let handle_descriptor = match handle.handle_descriptor() {
            Some(descriptor) if !descriptor.is_empty() => descriptor.to_string(),
            _ => return Err(OpenGlException::new("The handle must have a descriptor")),
        };

        if !self.image_types.contains(&handle_descriptor) {
            return Err(OpenGlException::new(format!("{handle_descriptor} is not supported")));
        }

        if handle_descriptor == KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR {
            let gl = self.context.gl_interface();
            while gl.get_error() != 0 {
                // Skip existing errors
            }
            let mut memory_object = 0u32;
            // SAFETY: the pointer is valid for the one name written.
            unsafe { self.ext.create_memory_objects_ext(1, &mut memory_object) };
            self.ext.import_memory_fd_ext(
                memory_object,
                properties.memory_size,
                GL_HANDLE_TYPE_OPAQUE_FD_EXT,
                file_descriptor(&*handle),
            );

            let err = gl.get_error();
            if err != 0 {
                return Err(OpenGlException::get_formatted_exception_for_code("glImportMemoryFdEXT", err));
            }
            let old_texture = gl.get_integerv(GL_TEXTURE_BINDING_2D);

            let texture = gl.gen_texture();
            gl.bind_texture(GL_TEXTURE_2D, texture);
            self.ext.tex_storage_mem_2d_ext(
                GL_TEXTURE_2D,
                1,
                GL_RGBA8,
                properties.width,
                properties.height,
                memory_object,
                properties.memory_offset,
            );
            let err = gl.get_error();

            gl.bind_texture(GL_TEXTURE_2D, old_texture);
            if err != 0 {
                return Err(OpenGlException::get_formatted_exception_for_code("glTexStorageMem2DEXT", err));
            }

            return Ok(Rc::new(ExternalImageTexture {
                context: self.context.clone(),
                ext: self.ext.clone(),
                object_id: Cell::new(memory_object),
                texture_id: texture,
                properties,
            }));
        }

        Err(OpenGlException::new(format!("{handle_descriptor} is not supported")))
    }

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Result<Rc<dyn IGlExternalSemaphore>, OpenGlException> {
        let handle_descriptor = match handle.handle_descriptor() {
            Some(descriptor) if !descriptor.is_empty() => descriptor.to_string(),
            _ => return Err(OpenGlException::new("The handle must have a descriptor")),
        };

        if !self.semaphore_types.contains(&handle_descriptor) {
            return Err(OpenGlException::new(format!("{handle_descriptor} is not supported")));
        }

        if handle_descriptor == KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR {
            let mut semaphore = 0u32;
            // SAFETY: the pointer is valid for the one name written.
            unsafe { self.ext.gen_semaphores_ext(1, &mut semaphore) };
            self.ext.import_semaphore_fd_ext(semaphore, GL_HANDLE_TYPE_OPAQUE_FD_EXT, file_descriptor(&*handle));
            return Ok(Rc::new(ExternalSemaphore {
                context: self.context.clone(),
                ext: self.ext.clone(),
                semaphore: Cell::new(semaphore),
            }));
        }

        Err(OpenGlException::new(format!("{handle_descriptor} is not supported")))
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        if image_handle_type == KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR {
            return CompositionGpuImportedImageSynchronizationCapabilities::SEMAPHORES;
        }
        CompositionGpuImportedImageSynchronizationCapabilities::default()
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.device_luid.clone()
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        self.device_uuid.clone()
    }
}

struct ExternalSemaphore {
    context: Rc<dyn IGlContext>,
    ext: Rc<ExternalObjectsInterface>,
    semaphore: Cell<u32>,
}

impl IGlExternalSemaphore for ExternalSemaphore {
    fn dispose(&self) {
        if self.context.is_lost() {
            return;
        }
        let current = self.context.ensure_current();
        let semaphore = self.semaphore.get();
        // SAFETY: the pointer is valid for the one name read.
        unsafe { self.ext.delete_semaphores_ext(1, &semaphore) };
        current.dispose();
        self.semaphore.set(0);
    }

    // The original casts the texture to the texture class of this file and reads its
    // identifier; the identifier is part of the contract of a texture, so no cast is needed.
    fn wait_semaphore(&self, texture: &dyn IGlExternalImageTexture) {
        let tex_id = texture.texture_id();
        let src_layout = GL_LAYOUT_TRANSFER_SRC_EXT;
        // SAFETY: there are no buffer barriers; the texture and layout pointers are valid
        // for the one value read.
        unsafe { self.ext.wait_semaphore_ext(self.semaphore.get(), 0, ptr::null(), 1, &tex_id, &src_layout) };
    }

    fn signal_semaphore(&self, texture: &dyn IGlExternalImageTexture) {
        let tex_id = texture.texture_id();
        let dst_layout = 0;
        // SAFETY: there are no buffer barriers; the texture and layout pointers are valid
        // for the one value read.
        unsafe { self.ext.signal_semaphore_ext(self.semaphore.get(), 0, ptr::null(), 1, &tex_id, &dst_layout) };
    }

    fn wait_timeline_semaphore(&self, _texture: &dyn IGlExternalImageTexture, _value: u64) {
        panic!("This semaphore type doesn't support value-based wait");
    }

    fn signal_timeline_semaphore(&self, _texture: &dyn IGlExternalImageTexture, _value: u64) {
        panic!("This semaphore type doesn't support value-based signaling");
    }
}

struct ExternalImageTexture {
    context: Rc<dyn IGlContext>,
    ext: Rc<ExternalObjectsInterface>,
    object_id: Cell<u32>,
    texture_id: i32,
    properties: PlatformGraphicsExternalImageProperties,
}

impl IGlExternalImageTexture for ExternalImageTexture {
    fn dispose(&self) {
        if self.context.is_lost() {
            return;
        }
        let current = self.context.ensure_current();
        self.context.gl_interface().delete_texture(self.texture_id);
        let object_id = self.object_id.get();
        // SAFETY: the pointer is valid for the one name read.
        unsafe { self.ext.delete_memory_objects_ext(1, &object_id) };
        self.object_id.set(0);
        current.dispose();
    }

    fn acquire_keyed_mutex(&self, _key: u32) {
        panic!("Specified method is not supported.");
    }

    fn release_keyed_mutex(&self, _key: u32) {
        panic!("Specified method is not supported.");
    }

    fn texture_id(&self) -> i32 {
        self.texture_id
    }

    fn internal_format(&self) -> i32 {
        GL_RGBA8
    }

    fn texture_type(&self) -> i32 {
        GL_TEXTURE_2D
    }

    fn properties(&self) -> PlatformGraphicsExternalImageProperties {
        self.properties.clone()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream project has no tests of the feature.
    use super::*;
    use crate::testing::FakeGl;
    use crate::{GlInterface, GlProfileType, GlVersion};
    use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext};
    use ferroui_base::reactive::{Disposable, IDisposable};
    use std::any::{Any, TypeId};

    struct TestContext {
        version: GlVersion,
        gl: Rc<GlInterface>,
    }

    impl IOptionalFeatureProvider for TestContext {
        fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
            None
        }
    }

    impl IPlatformGraphicsContext for TestContext {
        fn is_lost(&self) -> bool {
            false
        }

        fn ensure_current(&self) -> Rc<dyn IDisposable> {
            Disposable::create(|| {})
        }

        fn dispose(&self) {}

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl IGlContext for TestContext {
        fn version(&self) -> GlVersion {
            self.version
        }

        fn gl_interface(&self) -> Rc<GlInterface> {
            self.gl.clone()
        }

        fn sample_count(&self) -> i32 {
            0
        }

        fn stencil_size(&self) -> i32 {
            0
        }

        fn make_current(&self) -> Rc<dyn IDisposable> {
            Disposable::create(|| {})
        }

        fn is_shared_with(&self, _context: &dyn IGlContext) -> bool {
            false
        }

        fn can_create_shared_context(&self) -> bool {
            false
        }

        fn create_shared_context(&self, _preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
            None
        }
    }

    fn context(fake: &FakeGl) -> Rc<dyn IGlContext> {
        Rc::new(TestContext { version: fake.version(), gl: fake.interface() })
    }

    #[test]
    fn a_context_without_the_extensions_has_no_feature() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 2, 0));
        fake.set_string(GL_EXTENSIONS, Some("GL_OES_vertex_array_object"));

        assert!(ExternalObjectsOpenGlExtensionFeature::try_create(&context(&fake)).is_none());
    }

    #[test]
    fn a_context_without_the_entry_points_of_the_extensions_has_no_feature() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 2, 0));
        fake.set_string(GL_EXTENSIONS, Some("GL_EXT_memory_object GL_EXT_semaphore"));
        fake.remove("glCreateMemoryObjectsEXT");

        assert!(ExternalObjectsOpenGlExtensionFeature::try_create(&context(&fake)).is_none());
    }
}
