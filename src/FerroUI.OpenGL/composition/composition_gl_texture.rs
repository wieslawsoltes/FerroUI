use super::composition_gl_context::CompositionGlContext;
use super::task_support::{start, ServerJobTaskFuture};
use super::{CompositionGlTextureInfo, ICompositionGlContext, ICompositionGlTexture, ICompositionGlTextureLease};
use crate::gl_consts::GL_TEXTURE_2D;
use crate::{
    ICompositionImportableOpenGlSharedTexture, IGlContextExternalObjectsFeature, IGlExportableExternalImageTexture,
    IOpenGlTextureSharingRenderInterfaceContextFeature,
};
use ferroui_base::platform::{KnownPlatformGraphicsExternalImageHandleTypes, PlatformGraphicsExternalImageFormat};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{
    CompositionDrawingSurface, ICompositionGpuInterop, ICompositionImportableSharedGpuContextImage,
    ICompositionImportedGpuImage, RenderInterfaceFeature, ServerJobTask,
};
use ferroui_base::threading::DispatcherTask;
use ferroui_base::PixelSize;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// What a texture is backed by: the two classes that derive from `CompositionGlTexture` in
/// the original.
enum TextureKind {
    /// `SharedCompositionGlTexture`: a texture of the share group of the compositor. The
    /// thread that renders reads it during the import: it is shared with that thread.
    Shared(Arc<dyn ICompositionImportableOpenGlSharedTexture>),
    /// `ExternalImageCompositionGlTexture`: a texture whose image the compositor imports
    /// through a shared handle.
    ExternalImage(Rc<dyn IGlExportableExternalImageTexture>),
}

/// An OpenGL texture that is presented to a drawing surface of the compositor.
// The failures the original catches around the calls into the OpenGL context (a lost
// context while a draw is discarded or the resources are freed) are panics of those calls
// here, and are not caught.
pub(crate) struct CompositionGlTexture {
    this: Weak<CompositionGlTexture>,
    /// The owner holds its textures; a texture holds its owner weakly.
    owner: Weak<CompositionGlContext>,
    interop: Rc<dyn ICompositionGpuInterop>,
    surface: CompositionDrawingSurface,
    size: PixelSize,
    kind: TextureKind,
    /// The lease of the drawing session in progress. As in the original the texture and
    /// the lease hold each other until the session ends (the frame is presented or
    /// discarded, or the texture is disposed), so that a lease that was let go of without
    /// either is still discarded when the texture is disposed.
    active_lease: RefCell<Option<Rc<TextureLease>>>,
    last_present: RefCell<Option<ServerJobTask<()>>>,
    imported: RefCell<Option<Rc<dyn ICompositionImportedGpuImage>>>,
    disposed: Cell<bool>,
}

impl CompositionGlTexture {
    fn new(
        owner: &Rc<CompositionGlContext>,
        interop: Rc<dyn ICompositionGpuInterop>,
        surface: CompositionDrawingSurface,
        size: PixelSize,
        kind: TextureKind,
    ) -> Rc<CompositionGlTexture> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            owner: Rc::downgrade(owner),
            interop,
            surface,
            size,
            kind,
            active_lease: RefCell::new(None),
            last_present: RefCell::new(None),
            imported: RefCell::new(None),
            disposed: Cell::new(false),
        })
    }

    /// `new SharedCompositionGlTexture(owner, sharingFeature, interop, surface, size)`.
    ///
    /// The sharing feature is asked for the texture inside the compositor lock; the texture
    /// it answers with is shared between the threads by its contract.
    ///
    /// # Panics
    /// Panics if `sharing_feature` is not the texture sharing feature of a compositor that
    /// is alive (the interop only keeps one that is, with its compositor).
    pub(crate) fn new_shared(
        owner: &Rc<CompositionGlContext>,
        sharing_feature: &RenderInterfaceFeature,
        interop: Rc<dyn ICompositionGpuInterop>,
        surface: CompositionDrawingSurface,
        size: PixelSize,
    ) -> Rc<CompositionGlTexture> {
        let gl_context = owner.gl_context();
        let texture = sharing_feature
            .with::<dyn IOpenGlTextureSharingRenderInterfaceContextFeature, _>(|feature| {
                feature.create_shared_texture_for_composition(&gl_context, size)
            })
            .expect("the texture sharing feature of the compositor of the context");
        Self::new(owner, interop, surface, size, TextureKind::Shared(texture))
    }

    /// `new ExternalImageCompositionGlTexture(owner, externalObjects, interop, surface, size)`.
    ///
    /// # Panics
    /// Panics if the image cannot be created (the exception of the original).
    pub(crate) fn new_external_image(
        owner: &Rc<CompositionGlContext>,
        external_objects: &Rc<dyn IGlContextExternalObjectsFeature>,
        interop: Rc<dyn ICompositionGpuInterop>,
        surface: CompositionDrawingSurface,
        size: PixelSize,
    ) -> Rc<CompositionGlTexture> {
        let texture = external_objects
            .create_image(
                KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE,
                size,
                PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm,
            )
            .unwrap_or_else(|error| panic!("{error}"));
        Self::new(owner, interop, surface, size, TextureKind::ExternalImage(texture))
    }

    fn owner(&self) -> Rc<CompositionGlContext> {
        self.owner.upgrade().expect("the context of a texture in use is alive")
    }

    fn is_active_lease(&self, lease: &TextureLease) -> bool {
        self.active_lease.borrow().as_ref().is_some_and(|active| std::ptr::eq(Rc::as_ptr(active), lease))
    }

    fn present(&self, lease: &TextureLease) -> ServerJobTask<()> {
        debug_assert!(self.is_active_lease(lease));
        {
            // `finally`: the session ends whatever the calls into the context do.
            let _end_session = ClearOnDrop(&self.active_lease);
            let gl_context = self.owner().gl_context();
            let current = gl_context.ensure_current();
            gl_context.gl_interface().flush();
            self.present_gl_core();
            current.dispose();
        }
        let imported = self.imported.borrow().clone();
        let imported = match imported {
            Some(imported) => imported,
            None => {
                let imported = self.import();
                *self.imported.borrow_mut() = Some(imported.clone());
                imported
            }
        };
        let task = self.update_surface(&*imported);
        *self.last_present.borrow_mut() = Some(task.clone());
        task
    }

    fn discard_draw(&self, lease: &TextureLease) {
        if !self.is_active_lease(lease) {
            return;
        }
        let owner = self.owner();
        owner.compositor().dispatcher().verify_access();
        let ended = self.active_lease.borrow_mut().take();
        let gl_context = owner.gl_context();
        let current = gl_context.ensure_current();
        self.discard_draw_core();
        current.dispose();
        // The lease that is being disposed is released only now: it may be the last
        // reference to itself.
        drop(ended);
    }

    fn get_texture_info(&self) -> CompositionGlTextureInfo {
        match &self.kind {
            TextureKind::Shared(texture) => {
                CompositionGlTextureInfo::new(texture.texture_id(), GL_TEXTURE_2D, texture.internal_format(), self.size)
            }
            TextureKind::ExternalImage(texture) => CompositionGlTextureInfo::new(
                texture.texture_id(),
                texture.texture_type(),
                texture.internal_format(),
                self.size,
            ),
        }
    }

    fn begin_draw_core(&self) {
        match &self.kind {
            TextureKind::Shared(_) => {}
            TextureKind::ExternalImage(texture) => texture.acquire_keyed_mutex(0),
        }
    }

    fn discard_draw_core(&self) {
        match &self.kind {
            TextureKind::Shared(_) => {}
            TextureKind::ExternalImage(texture) => texture.release_keyed_mutex(0),
        }
    }

    fn present_gl_core(&self) {
        match &self.kind {
            TextureKind::Shared(_) => {}
            TextureKind::ExternalImage(texture) => texture.release_keyed_mutex(1),
        }
    }

    fn import(&self) -> Rc<dyn ICompositionImportedGpuImage> {
        match &self.kind {
            TextureKind::Shared(texture) => {
                let image: Arc<dyn ICompositionImportableSharedGpuContextImage> = texture.clone();
                self.interop.import_shared_image(image)
            }
            TextureKind::ExternalImage(texture) => self.interop.import_image(texture.get_handle(), texture.properties()),
        }
    }

    fn update_surface(&self, imported: &dyn ICompositionImportedGpuImage) -> ServerJobTask<()> {
        match &self.kind {
            TextureKind::Shared(_) => self.surface.update_async(imported),
            TextureKind::ExternalImage(_) => self.surface.update_with_keyed_mutex_async(imported, 1, 0),
        }
    }

    fn dispose_gl_resources(&self) {
        match &self.kind {
            TextureKind::Shared(texture) => texture.dispose(),
            TextureKind::ExternalImage(texture) => texture.dispose(),
        }
    }
}

/// Clears a slot when it goes out of scope.
struct ClearOnDrop<'a, T>(&'a RefCell<Option<T>>);

impl<T> Drop for ClearOnDrop<'_, T> {
    fn drop(&mut self) {
        // The value is dropped after the borrow of the slot has ended.
        let value = self.0.borrow_mut().take();
        drop(value);
    }
}

impl ICompositionGlTexture for CompositionGlTexture {
    fn size(&self) -> PixelSize {
        self.size
    }

    fn is_ready_for_draw(&self) -> bool {
        !self.disposed.get()
            && self.active_lease.borrow().is_none()
            && self.last_present.borrow().as_ref().map_or(true, |task| task.is_completed_successfully())
    }

    /// # Panics
    /// Panics if the texture is disposed, if its context is no longer valid for interop, if
    /// the previous drawing session is still active or if the previous presentation has
    /// failed or has not completed (the exceptions of the original).
    fn begin_draw(&self) -> Rc<dyn ICompositionGlTextureLease> {
        let owner = self.owner();
        owner.compositor().dispatcher().verify_access();
        if self.disposed.get() {
            panic!("Cannot access a disposed object. Object name: 'ICompositionGlTexture'.");
        }
        if !owner.is_valid_for_interop() {
            panic!("The owning context is no longer valid for interop with the compositor");
        }
        if self.active_lease.borrow().is_some() {
            panic!("The previous drawing session is still active");
        }
        let last_present = self.last_present.borrow().clone();
        if let Some(last_present) = last_present.filter(|task| !task.is_completed_successfully()) {
            panic!(
                "{}",
                if last_present.is_completed() {
                    "The previous presentation has failed"
                } else {
                    "The previous presentation hasn't been completed yet"
                }
            );
        }

        let gl_context = owner.gl_context();
        let current = gl_context.ensure_current();
        self.begin_draw_core();
        current.dispose();

        let this = self.this.upgrade().expect("a texture that is called is alive");
        let lease = Rc::new(TextureLease { texture: RefCell::new(Some(this)) });
        *self.active_lease.borrow_mut() = Some(lease.clone());
        lease
    }

    fn dispose_async(&self) -> DispatcherTask<()> {
        let owner = self.owner.upgrade();
        if let Some(owner) = &owner {
            owner.compositor().dispatcher().verify_access();
        }
        if self.disposed.get() {
            return start(async {});
        }
        let active_lease = self.active_lease.borrow().clone();
        if let Some(active_lease) = active_lease {
            active_lease.dispose();
        }
        self.disposed.set(true);
        if let Some(owner) = &owner {
            owner.on_texture_disposed(self);
        }

        let this = self.this.upgrade().expect("a texture that is called is alive");
        start(async move {
            // The texture might have already been sent to the compositor, so we need to wait for its
            // attempts to use the texture before destroying it
            let imported = this.imported.borrow().clone();
            if let Some(imported) = imported {
                let _ = ServerJobTaskFuture::new(imported.import_completed()).await;
                let last_present = this.last_present.borrow().clone();
                if let Some(last_present) = last_present {
                    let _ = ServerJobTaskFuture::new(last_present).await;
                }
                let _ = ServerJobTaskFuture::new(imported.dispose_async()).await;
            }
            this.dispose_gl_resources();
        })
    }
}

struct TextureLease {
    texture: RefCell<Option<Rc<CompositionGlTexture>>>,
}

impl ICompositionGlTextureLease for TextureLease {
    /// # Panics
    /// Panics if the lease is disposed or presented (the exception of the original).
    fn texture_info(&self) -> CompositionGlTextureInfo {
        let texture = self.texture.borrow().clone();
        let Some(texture) = texture else {
            panic!("Cannot access a disposed object. Object name: 'ICompositionGlTextureLease'.");
        };
        texture.owner().compositor().dispatcher().verify_access();
        texture.get_texture_info()
    }

    /// # Panics
    /// Panics if the lease is disposed or presented (the exception of the original).
    fn present_async(&self) -> ServerJobTask<()> {
        let texture = self.texture.borrow().clone();
        let Some(texture) = texture else {
            panic!("Cannot access a disposed object. Object name: 'ICompositionGlTextureLease'.");
        };
        texture.owner().compositor().dispatcher().verify_access();
        *self.texture.borrow_mut() = None;
        texture.present(self)
    }
}

impl IDisposable for TextureLease {
    fn dispose(&self) {
        let texture = self.texture.borrow_mut().take();
        if let Some(texture) = texture {
            texture.discard_draw(self);
        }
    }
}
