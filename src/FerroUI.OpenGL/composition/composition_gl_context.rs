use super::composition_gl_texture::CompositionGlTexture;
use super::task_support::start;
use super::{ICompositionGlContext, ICompositionGlTexture};
use crate::{IGlContext, IGlContextExternalObjectsFeature, IOpenGlTextureSharingRenderInterfaceContextFeature};
use ferroui_base::rendering::composition::{CompositionDrawingSurface, Compositor, ICompositionGpuInterop};
use ferroui_base::threading::DispatcherTask;
use ferroui_base::PixelSize;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The OpenGL context that draws into the surfaces of a compositor: a context shared with
/// the rendering context of the compositor, or one that transfers its frames through
/// external objects.
pub(crate) struct CompositionGlContext {
    this: Weak<CompositionGlContext>,
    compositor: Rc<Compositor>,
    gl_context: Rc<dyn IGlContext>,
    interop: Rc<dyn ICompositionGpuInterop>,
    sharing_feature: Option<Rc<dyn IOpenGlTextureSharingRenderInterfaceContextFeature>>,
    external_objects: Option<Rc<dyn IGlContextExternalObjectsFeature>>,
    textures: RefCell<Vec<Rc<CompositionGlTexture>>>,
    disposed: Cell<bool>,
}

impl CompositionGlContext {
    pub(crate) fn new(
        compositor: Rc<Compositor>,
        gl_context: Rc<dyn IGlContext>,
        interop: Rc<dyn ICompositionGpuInterop>,
        sharing_feature: Option<Rc<dyn IOpenGlTextureSharingRenderInterfaceContextFeature>>,
        external_objects: Option<Rc<dyn IGlContextExternalObjectsFeature>>,
    ) -> Rc<CompositionGlContext> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            compositor,
            gl_context,
            interop,
            sharing_feature,
            external_objects,
            textures: RefCell::new(Vec::new()),
            disposed: Cell::new(false),
        })
    }

    pub(crate) fn on_texture_disposed(&self, texture: &CompositionGlTexture) {
        self.textures.borrow_mut().retain(|other| !std::ptr::eq(&**other, texture));
    }
}

impl ICompositionGlContext for CompositionGlContext {
    fn compositor(&self) -> Rc<Compositor> {
        self.compositor.clone()
    }

    fn gl_context(&self) -> Rc<dyn IGlContext> {
        self.gl_context.clone()
    }

    fn is_valid_for_interop(&self) -> bool {
        !self.disposed.get() && !self.gl_context.is_lost() && !self.interop.is_lost()
    }

    /// # Panics
    /// Panics if the context is disposed or no longer valid for interop, if the surface
    /// belongs to another compositor, or if the size is smaller than one pixel (the
    /// exceptions of the original).
    fn create_texture(&self, surface: &CompositionDrawingSurface, size: PixelSize) -> Rc<dyn ICompositionGlTexture> {
        self.compositor.dispatcher().verify_access();
        if self.disposed.get() {
            panic!("Cannot access a disposed object. Object name: 'ICompositionGlContext'.");
        }
        if !self.is_valid_for_interop() {
            panic!("This context is no longer valid for interop with the compositor");
        }
        if !Rc::ptr_eq(surface.compositor(), &self.compositor) {
            panic!("The surface belongs to a different Compositor");
        }
        if size.width < 1 || size.height < 1 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'size')");
        }

        let owner = self.this.upgrade().expect("a context that is called is alive");
        let texture = match &self.sharing_feature {
            Some(sharing_feature) => {
                CompositionGlTexture::new_shared(&owner, sharing_feature, self.interop.clone(), surface.clone(), size)
            }
            None => CompositionGlTexture::new_external_image(
                &owner,
                self.external_objects.as_ref().expect("a context without texture sharing has external objects"),
                self.interop.clone(),
                surface.clone(),
                size,
            ),
        };
        self.textures.borrow_mut().push(texture.clone());
        texture
    }

    fn dispose_async(&self) -> DispatcherTask<()> {
        self.compositor.dispatcher().verify_access();
        if self.disposed.get() {
            return start(async {});
        }
        self.disposed.set(true);

        let this = self.this.upgrade().expect("a context that is called is alive");
        start(async move {
            let textures = this.textures.borrow().clone();
            for texture in textures {
                let _ = texture.dispose_async().await;
            }
            this.textures.borrow_mut().clear();

            this.gl_context.dispose();
        })
    }
}
