use super::GlSkiaSharedTextureForComposition;
use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::ISkiaGrContext;
use crate::immutable_bitmap::ImmutableBitmap;
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle, IPlatformRenderInterfaceImportedImage,
    IPlatformRenderInterfaceImportedObject, IPlatformRenderInterfaceImportedSemaphore, PlatformGraphicsDrmFormat,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage,
};
use ferroui_base::utilities::ThreadBound;
use ferroui_opengl::gl_consts::*;
use ferroui_opengl::{
    ICompositionImportableOpenGlSharedTexture, IGlContext, IGlContextExternalObjectsFeature, IGlExternalImageTexture,
    IGlExternalSemaphore,
};
use skia_safe::gpu::gl::TextureInfo;
use skia_safe::gpu::{backend_textures, images, Mipmapped, SurfaceOrigin};
use skia_safe::{AlphaType, ColorType, Image};
use std::any::Any;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Imports images and semaphores that were created outside of the framework
/// into the Ganesh context of an OpenGL Skia GPU.
///
/// The feature and everything it imports are objects of the thread that
/// renders: they hold the OpenGL context and the Ganesh context of the GPU.
// The original holds its `GlSkiaGpu`; the GPU owns the feature here, so the
// feature holds what it reads of the GPU instead: the two contexts and what
// stands for the share group.
pub struct GlSkiaExternalObjectsFeature {
    this: Weak<GlSkiaExternalObjectsFeature>,
    gr_context: Rc<GaneshGrContext>,
    gl_context: Rc<dyn IGlContext>,
    share_group: Arc<()>,
    feature: Option<Rc<dyn IGlContextExternalObjectsFeature>>,
    fbo: Cell<i32>,
}

impl GlSkiaExternalObjectsFeature {
    /// Creates the feature of the GPU that renders with `gl_context` through
    /// `gr_context`. `feature` is the external objects feature of the
    /// OpenGL context, when it has one; `share_group` is what the shared
    /// textures of the GPU are marked with.
    pub fn new(
        gr_context: Rc<GaneshGrContext>,
        gl_context: Rc<dyn IGlContext>,
        share_group: Arc<()>,
        feature: Option<Rc<dyn IGlContextExternalObjectsFeature>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            gr_context,
            gl_context,
            share_group,
            feature,
            fbo: Cell::new(0),
        })
    }

    fn fbo_provider(&self) -> Rc<dyn IGlSkiaFboProvider> {
        self.this.upgrade().expect("a feature that is called is alive")
    }
}

impl IExternalObjectsRenderInterfaceContextFeature for GlSkiaExternalObjectsFeature {
    fn supported_image_handle_types(&self) -> Vec<String> {
        self.feature.as_ref().map(|feature| feature.supported_importable_external_image_types()).unwrap_or_default()
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        self.feature
            .as_ref()
            .map(|feature| feature.supported_importable_external_semaphore_types())
            .unwrap_or_default()
    }

    fn supported_dma_buf_formats(&self) -> Option<Vec<PlatformGraphicsDrmFormat>> {
        self.feature.as_ref().and_then(|feature| feature.supported_dma_buf_formats())
    }

    /// # Panics
    /// Panics when the OpenGL context has no external objects feature and
    /// when the context fails to import the handle (the exceptions of the
    /// original).
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        let Some(feature) = &self.feature else {
            panic!("Importing this platform handle is not supported");
        };
        let current = self.gl_context.ensure_current();
        let image = feature.import_image(handle, properties);
        current.dispose();
        let image = image.unwrap_or_else(|error| panic!("{}", error.message()));
        Rc::new(GlSkiaImportedImage::new(
            self.gr_context.clone(),
            self.gl_context.clone(),
            self.fbo_provider(),
            ImportedImageSource::External(image),
        ))
    }

    /// # Panics
    /// Panics when the image is not a shared texture of this backend and
    /// when its context does not share with the one of the GPU (the
    /// exceptions of the original).
    fn import_shared_image(
        &self,
        image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        let img = shared_texture(&image);
        // Deviation (DEVIATIONS.md, Skia backend): the context of the
        // texture belongs to the thread that made the texture. That thread
        // asks the context, as upstream; another one compares what the GPU
        // that made the texture marked it with.
        let is_shared = if img.is_context_on_thread() {
            img.context().is_shared_with(&*self.gl_context)
        } else {
            Arc::ptr_eq(img.share_group(), &self.share_group)
        };
        if !is_shared {
            panic!("Contexts do not belong to the same share group");
        }

        Rc::new(GlSkiaImportedImage::new(
            self.gr_context.clone(),
            self.gl_context.clone(),
            self.fbo_provider(),
            ImportedImageSource::Shared(image),
        ))
    }

    /// # Panics
    /// Panics when the OpenGL context has no external objects feature and
    /// when the context fails to import the handle (the exceptions of the
    /// original).
    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        let Some(feature) = &self.feature else {
            panic!("Importing this platform handle is not supported");
        };
        let current = self.gl_context.ensure_current();
        let semaphore = feature.import_semaphore(handle);
        current.dispose();
        let semaphore = semaphore.unwrap_or_else(|error| panic!("{}", error.message()));
        Rc::new(GlSkiaImportedSemaphore::new(semaphore))
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        self.feature
            .as_ref()
            .map(|feature| feature.get_synchronization_capabilities(image_handle_type))
            .unwrap_or_default()
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        self.feature.as_ref().and_then(|feature| feature.device_uuid())
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.feature.as_ref().and_then(|feature| feature.device_luid())
    }
}

impl IGlSkiaFboProvider for GlSkiaExternalObjectsFeature {
    fn fbo(&self) -> i32 {
        if self.fbo.get() == 0 {
            self.fbo.set(self.gl_context.gl_interface().gen_framebuffer());
        }

        self.fbo.get()
    }
}

/// Hands out the framebuffer object the imported images copy their textures
/// with.
pub(crate) trait IGlSkiaFboProvider {
    fn fbo(&self) -> i32;
}

/// The shared texture behind an image of a context of the share group (the
/// cast of the original).
fn shared_texture(image: &Arc<dyn ICompositionImportableSharedGpuContextImage>) -> &GlSkiaSharedTextureForComposition {
    image
        .as_any()
        .downcast_ref::<GlSkiaSharedTextureForComposition>()
        .unwrap_or_else(|| panic!("The image is not a shared texture of the Skia backend."))
}

/// The imported semaphore behind the contract (the cast of the original).
fn imported_semaphore(semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>) -> &GlSkiaImportedSemaphore {
    semaphore
        .as_any()
        .downcast_ref::<GlSkiaImportedSemaphore>()
        .unwrap_or_else(|| panic!("The semaphore was not imported by the OpenGL Skia GPU."))
}

/// Runs its function when it goes out of scope, also while a panic unwinds:
/// the `finally` of the original.
struct Finally<F: FnMut()>(F);

impl<F: FnMut()> Drop for Finally<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

/// A semaphore of another graphics API, imported into the OpenGL context.
pub struct GlSkiaImportedSemaphore {
    semaphore: Rc<dyn IGlExternalSemaphore>,
}

impl GlSkiaImportedSemaphore {
    pub fn new(semaphore: Rc<dyn IGlExternalSemaphore>) -> Self {
        Self { semaphore }
    }

    pub fn semaphore(&self) -> &Rc<dyn IGlExternalSemaphore> {
        &self.semaphore
    }
}

impl IPlatformRenderInterfaceImportedObject for GlSkiaImportedSemaphore {
    fn dispose(&self) {
        self.semaphore.dispose();
    }
}

impl IPlatformRenderInterfaceImportedSemaphore for GlSkiaImportedSemaphore {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// What an imported image is backed by: the two fields of the original, of
/// which one is set.
enum ImportedImageSource {
    /// The texture of an image of another graphics API.
    External(Rc<dyn IGlExternalImageTexture>),
    /// A texture of a context of the share group; a
    /// [`GlSkiaSharedTextureForComposition`].
    Shared(Arc<dyn ICompositionImportableSharedGpuContextImage>),
}

/// An image imported into the OpenGL context. A snapshot copies its texture
/// into a texture of the Ganesh context.
pub struct GlSkiaImportedImage {
    gr_context: Rc<GaneshGrContext>,
    gl_context: Rc<dyn IGlContext>,
    fbo_provider: Rc<dyn IGlSkiaFboProvider>,
    source: ImportedImageSource,
}

impl GlSkiaImportedImage {
    fn new(
        gr_context: Rc<GaneshGrContext>,
        gl_context: Rc<dyn IGlContext>,
        fbo_provider: Rc<dyn IGlSkiaFboProvider>,
        source: ImportedImageSource,
    ) -> Self {
        Self { gr_context, gl_context, fbo_provider, source }
    }

    /// The external image, for the snapshots that synchronize with it.
    ///
    /// # Panics
    /// Panics when the image is a shared texture.
    fn external_image(&self) -> &Rc<dyn IGlExternalImageTexture> {
        match &self.source {
            ImportedImageSource::External(image) => image,
            ImportedImageSource::Shared(_) => panic!("Only supported with an external image"),
        }
    }

    // Not called in the original either.
    #[allow(dead_code)]
    fn convert_color_type(format: PlatformGraphicsExternalImageFormat) -> ColorType {
        match format {
            PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm => ColorType::BGRA8888,
            PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm => ColorType::RGBA8888,
        }
    }

    fn try_create_image(
        &self,
        target: i32,
        texture_id: i32,
        format: i32,
        width: i32,
        height: i32,
        top_left: bool,
    ) -> Option<Image> {
        let origin = if top_left { SurfaceOrigin::TopLeft } else { SurfaceOrigin::BottomLeft };

        // SAFETY: `texture_id` names a texture of the current context that
        // was just created with this size (see `copy_to_new_texture`).
        let texture = unsafe {
            backend_textures::make_gl(
                (width, height),
                Mipmapped::No,
                TextureInfo { target: target as u32, id: texture_id as u32, format: format as u32, ..Default::default() },
                "",
            )
        };

        let image = self.gr_context.with_context(|context| {
            images::adopt_texture_from(context, &texture, origin, ColorType::RGBA8888, AlphaType::Premul, None)
        });
        if image.is_some() {
            return image;
        }

        // SAFETY: as above.
        let unformatted = unsafe {
            backend_textures::make_gl(
                (width, height),
                Mipmapped::No,
                TextureInfo::from_target_and_id(target as u32, texture_id as u32),
                "",
            )
        };

        self.gr_context.with_context(|context| {
            images::adopt_texture_from(context, &unformatted, origin, ColorType::RGBA8888, AlphaType::Premul, None)
        })
    }

    fn take_snapshot(&self) -> Arc<SharedBitmapImpl> {
        let (width, height, internal_format, texture_id, top_left, texture_type) = match &self.source {
            ImportedImageSource::External(image) => {
                let properties = image.properties();
                (
                    properties.width,
                    properties.height,
                    image.internal_format(),
                    image.texture_id(),
                    properties.top_left_origin,
                    image.texture_type(),
                )
            }
            ImportedImageSource::Shared(image) => {
                let texture = shared_texture(image);
                let size = texture.size();
                (size.width, size.height, texture.internal_format(), texture.texture_id(), false, GL_TEXTURE_2D)
            }
        };

        let context = self.gl_context.clone();
        let snapshot_texture_id = self.copy_to_new_texture(texture_type, texture_id, internal_format, width, height);
        let snapshot_image =
            self.try_create_image(texture_type, snapshot_texture_id, internal_format, width, height, top_left);

        let Some(snapshot_image) = snapshot_image else {
            context.gl_interface().delete_texture(snapshot_texture_id);
            panic!("Unable to consume provided texture");
        };

        // The bitmap is shared between threads by its contract, and the
        // context is an object of this one: on this thread the image is
        // released with the context current, as upstream; on another the
        // context cannot be reached and the image is released as it is.
        let dispose_context = ThreadBound::new(context.clone());
        let rv = ImmutableBitmap::from_image(
            snapshot_image,
            Some(Box::new(move |snapshot_image: Image| {
                // A context that cannot be made current is likely dead:
                // ignored, as upstream.
                let restore_context = if dispose_context.is_on_thread() {
                    catch_unwind(AssertUnwindSafe(|| dispose_context.get().ensure_current())).ok()
                } else {
                    None
                };

                drop(snapshot_image);

                if let Some(restore_context) = restore_context {
                    restore_context.dispose();
                }
            })),
        );

        self.gr_context.flush();
        context.gl_interface().flush();
        Arc::new(rv)
    }

    fn copy_to_new_texture(
        &self,
        texture_type: i32,
        source_texture_id: i32,
        internal_format: i32,
        width: i32,
        height: i32,
    ) -> i32 {
        let gl = self.gl_context.gl_interface();

        let current = self.gl_context.ensure_current();

        // Snapshot current values
        let old_fbo = gl.get_integerv(GL_FRAMEBUFFER_BINDING);
        let old_scissor_test = gl.get_integerv(GL_SCISSOR_TEST);

        // Bind source texture
        gl.bind_framebuffer(GL_FRAMEBUFFER, self.fbo_provider.fbo());
        gl.disable(GL_SCISSOR_TEST);
        gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, texture_type, source_texture_id, 0);

        // Create destination texture
        let dest_texture_id = gl.gen_texture();
        gl.bind_texture(texture_type, dest_texture_id);
        // SAFETY: a null data pointer makes OpenGL allocate the storage
        // without reading client memory.
        unsafe {
            gl.tex_image_2d(
                texture_type,
                0,
                internal_format,
                width,
                height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
        }

        // Copy
        gl.copy_tex_sub_image_2d(texture_type, 0, 0, 0, 0, 0, width, height);

        // Flush
        gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, texture_type, 0, 0);
        gl.flush();

        // Restore old values
        gl.bind_framebuffer(GL_FRAMEBUFFER, old_fbo);
        if old_scissor_test != 0 {
            gl.enable(GL_SCISSOR_TEST);
        }

        current.dispose();

        dest_texture_id
    }
}

impl IPlatformRenderInterfaceImportedObject for GlSkiaImportedImage {
    fn dispose(&self) {
        match &self.source {
            ImportedImageSource::External(image) => image.dispose(),
            ImportedImageSource::Shared(image) => shared_texture(image).dispose_with(&*self.gl_context),
        }
    }
}

impl IPlatformRenderInterfaceImportedImage for GlSkiaImportedImage {
    /// # Panics
    /// Panics when the image is a shared texture and when the texture
    /// cannot be consumed (the exceptions of the original).
    fn snapshot_with_keyed_mutex(&self, acquire_index: u32, release_index: u32) -> Arc<SharedBitmapImpl> {
        let image = self.external_image();

        let current = self.gl_context.ensure_current();
        image.acquire_keyed_mutex(acquire_index);
        let snapshot = {
            let _release = Finally(|| image.release_keyed_mutex(release_index));
            self.take_snapshot()
        };
        current.dispose();
        snapshot
    }

    /// # Panics
    /// Panics when the image is a shared texture, when a semaphore was not
    /// imported by this backend and when the texture cannot be consumed
    /// (the exceptions of the original).
    fn snapshot_with_semaphores(
        &self,
        wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<SharedBitmapImpl> {
        let image = self.external_image();

        let wait = imported_semaphore(wait_for_semaphore);
        let signal = imported_semaphore(signal_semaphore);
        let current = self.gl_context.ensure_current();
        wait.semaphore().wait_semaphore(&**image);
        let snapshot = {
            let _signal = Finally(|| signal.semaphore().signal_semaphore(&**image));
            self.take_snapshot()
        };
        current.dispose();
        snapshot
    }

    /// # Panics
    /// Panics when the image is a shared texture, when a semaphore was not
    /// imported by this backend and when the texture cannot be consumed
    /// (the exceptions of the original).
    fn snapshot_with_timeline_semaphores(
        &self,
        wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        wait_for_value: u64,
        signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        let image = self.external_image();

        let wait = imported_semaphore(wait_for_semaphore);
        let signal = imported_semaphore(signal_semaphore);
        let current = self.gl_context.ensure_current();
        wait.semaphore().wait_timeline_semaphore(&**image, wait_for_value);
        let snapshot = {
            let _signal = Finally(|| signal.semaphore().signal_timeline_semaphore(&**image, signal_value));
            self.take_snapshot()
        };
        current.dispose();
        snapshot
    }

    /// # Panics
    /// Panics when the texture cannot be consumed (the exception of the
    /// original).
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        let current = self.gl_context.ensure_current();
        let snapshot = self.take_snapshot();
        current.dispose();
        snapshot
    }
}
