use super::server::ServerObjectId;
use super::{CompositionObject, Compositor, ICompositionObject};
use std::any::Any;
use std::rc::Rc;

/// A surface that a surface visual shows. The server-side counterpart
/// provides the bitmap.
pub struct CompositionSurface {
    object: CompositionObject,
}

impl CompositionSurface {
    /// Creates the UI-thread side of a surface whose server side is the
    /// object with the given id.
    pub fn new(compositor: &Rc<Compositor>, server: ServerObjectId) -> Rc<CompositionSurface> {
        Rc::new(CompositionSurface { object: CompositionObject::new(compositor, Some(server)) })
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }
}

impl ICompositionObject for CompositionSurface {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
