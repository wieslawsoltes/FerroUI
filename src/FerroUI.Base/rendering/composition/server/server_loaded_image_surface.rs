use super::{IServerCompositionSurface, IServerObject, ServerCompositionSurfaceChanged};
use crate::utilities::RefCounted;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The server-side counterpart of a
/// [`LoadedImageSurface`](crate::rendering::composition::LoadedImageSurface):
/// a surface whose content is a bitmap given when it is created.
pub struct ServerLoadedImageSurface {
    bitmap: RefCell<Option<RefCounted<crate::platform::SharedBitmapImpl>>>,
    changed: ServerCompositionSurfaceChanged,
}

impl ServerLoadedImageSurface {
    /// Creates the surface with its bitmap (`None` for an image that did not
    /// load). The surface holds a reference to the bitmap until it is
    /// disposed.
    pub fn new(bitmap: Option<RefCounted<crate::platform::SharedBitmapImpl>>) -> Rc<ServerLoadedImageSurface> {
        Rc::new(ServerLoadedImageSurface { bitmap: RefCell::new(bitmap), changed: ServerCompositionSurfaceChanged::new() })
    }
}

impl IServerObject for ServerLoadedImageSurface {
    fn dispose(&self) {
        if let Some(bitmap) = self.bitmap.borrow_mut().take() {
            bitmap.dispose();
        }
        self.changed.invoke();
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

impl IServerCompositionSurface for ServerLoadedImageSurface {
    fn bitmap(&self) -> Option<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        self.bitmap.borrow().as_ref().filter(|bitmap| bitmap.is_alive()).map(|bitmap| bitmap.item())
    }

    fn changed(&self) -> &ServerCompositionSurfaceChanged {
        &self.changed
    }
}
