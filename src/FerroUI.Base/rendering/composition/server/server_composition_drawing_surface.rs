use super::{IServerCompositionSurface, IServerObject, ServerCompositor, ServerCompositionSurfaceChanged};
use crate::platform::{IBitmapImpl, IPlatformRenderInterfaceContext, PlatformGraphicsContextLostException};
use crate::rendering::composition::{CompositionImportedGpuImage, CompositionImportedGpuSemaphore};
use crate::utilities::{RefCountable, RefCounted};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The error of an update that cannot proceed (`InvalidOperationException`).
#[derive(Clone, Debug)]
pub struct DrawingSurfaceUpdateError(&'static str);

impl std::fmt::Display for DrawingSurfaceUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for DrawingSurfaceUpdateError {}

type UpdateResult = Result<(), Rc<dyn std::error::Error>>;

fn context_lost() -> Rc<dyn std::error::Error> {
    Rc::new(PlatformGraphicsContextLostException)
}

/// The server-side counterpart of a
/// [`CompositionDrawingSurface`](crate::rendering::composition::CompositionDrawingSurface):
/// a surface whose content is a snapshot of an imported GPU image.
pub struct ServerCompositionDrawingSurface {
    compositor: Weak<ServerCompositor>,
    bitmap: RefCell<Option<RefCounted<crate::platform::SharedBitmapImpl>>>,
    created_with_context: RefCell<Option<Rc<dyn IPlatformRenderInterfaceContext>>>,
    disposed: Cell<bool>,
    changed: ServerCompositionSurfaceChanged,
}

impl ServerCompositionDrawingSurface {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionDrawingSurface> {
        Rc::new(ServerCompositionDrawingSurface {
            compositor: Rc::downgrade(compositor),
            bitmap: RefCell::new(None),
            created_with_context: RefCell::new(None),
            disposed: Cell::new(false),
            changed: ServerCompositionSurfaceChanged::new(),
        })
    }

    fn compositor(&self) -> Rc<ServerCompositor> {
        self.compositor.upgrade().expect("the compositor of a server object is alive while the object is used")
    }

    fn current_context(&self) -> Rc<dyn IPlatformRenderInterfaceContext> {
        self.compositor().render_interface().value()
    }

    /// The counted reference to the content of the surface.
    pub fn bitmap_ref(&self) -> Option<RefCounted<crate::platform::SharedBitmapImpl>> {
        // Failsafe to avoid consuming an image imported with a different context
        let current = self.current_context();
        let same = self
            .created_with_context
            .borrow()
            .as_ref()
            .is_some_and(|context| std::ptr::addr_eq(Rc::as_ptr(context), Rc::as_ptr(&current)));
        if !same {
            return None;
        }
        self.bitmap.borrow().as_ref().map(|bitmap| bitmap.clone_ref())
    }

    fn perform_sanity_checks(&self, image: &CompositionImportedGpuImage) -> UpdateResult {
        // Failsafe to avoid consuming an image imported with a different context
        if !image.is_usable() {
            return Err(context_lost());
        }

        // This should never happen, but check for it anyway to avoid a deadlock
        let import_completed = super::super::ICompositionGpuImportedObject::import_completed(image);
        if !import_completed.is_completed() {
            return Err(Rc::new(DrawingSurfaceUpdateError("The import operation is not completed yet")));
        }

        // Rethrow the import here exception
        if let Some(error) = import_completed.exception() {
            return Err(error);
        }
        Ok(())
    }

    fn update(&self, new_image: std::sync::Arc<crate::platform::SharedBitmapImpl>, context: Rc<dyn IPlatformRenderInterfaceContext>) {
        if self.disposed.get() {
            // Batches are processed with disposals before server jobs, so an update job
            // can be processed after this surface was disposed in the same batch.
            // Dispose the snapshot here on the render thread (context is current)
            // instead of orphaning it.
            new_image.dispose();
            return;
        }
        if let Some(bitmap) = self.bitmap.borrow_mut().take() {
            bitmap.dispose();
        }
        let item = new_image.clone();
        *self.bitmap.borrow_mut() = Some(RefCountable::create(new_image, move || item.dispose()));
        *self.created_with_context.borrow_mut() = Some(context);
        self.changed.invoke();
    }

    /// Runs `f` with the graphics context current.
    fn with_current_context(&self, f: impl FnOnce() -> UpdateResult) -> UpdateResult {
        let current = self.compositor().render_interface().ensure_current();
        let result = f();
        current.dispose();
        result
    }

    pub fn update_with_automatic_sync(&self, image: &CompositionImportedGpuImage) -> UpdateResult {
        self.with_current_context(|| {
            self.perform_sanity_checks(image)?;
            self.update(image.image().snapshot_with_automatic_sync(), image.context().clone());
            Ok(())
        })
    }

    pub fn update_with_keyed_mutex(
        &self,
        image: &CompositionImportedGpuImage,
        acquire_index: u32,
        release_index: u32,
    ) -> UpdateResult {
        self.with_current_context(|| {
            self.perform_sanity_checks(image)?;
            self.update(image.image().snapshot_with_keyed_mutex(acquire_index, release_index), image.context().clone());
            Ok(())
        })
    }

    pub fn update_with_semaphores(
        &self,
        image: &CompositionImportedGpuImage,
        wait: &CompositionImportedGpuSemaphore,
        signal: &CompositionImportedGpuSemaphore,
    ) -> UpdateResult {
        self.with_current_context(|| {
            self.perform_sanity_checks(image)?;
            if !wait.is_usable() || !signal.is_usable() {
                return Err(context_lost());
            }
            self.update(
                image.image().snapshot_with_semaphores(&wait.semaphore(), &signal.semaphore()),
                image.context().clone(),
            );
            Ok(())
        })
    }

    pub fn update_with_timeline_semaphores(
        &self,
        image: &CompositionImportedGpuImage,
        wait: &CompositionImportedGpuSemaphore,
        wait_for_value: u64,
        signal: &CompositionImportedGpuSemaphore,
        signal_value: u64,
    ) -> UpdateResult {
        self.with_current_context(|| {
            self.perform_sanity_checks(image)?;
            if !wait.is_usable() || !signal.is_usable() {
                return Err(context_lost());
            }
            self.update(
                image.image().snapshot_with_timeline_semaphores(
                    &wait.semaphore(),
                    wait_for_value,
                    &signal.semaphore(),
                    signal_value,
                ),
                image.context().clone(),
            );
            Ok(())
        })
    }
}

impl IServerObject for ServerCompositionDrawingSurface {
    fn dispose(&self) {
        if let Some(bitmap) = self.bitmap.borrow_mut().take() {
            bitmap.dispose();
        }
        self.disposed.set(true);
    }

    fn as_surface(self: Rc<Self>) -> Option<Rc<dyn IServerCompositionSurface>> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl IServerCompositionSurface for ServerCompositionDrawingSurface {
    fn bitmap(&self) -> Option<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        self.bitmap_ref().map(|bitmap| bitmap.item())
    }

    fn changed(&self) -> &ServerCompositionSurfaceChanged {
        &self.changed
    }
}
