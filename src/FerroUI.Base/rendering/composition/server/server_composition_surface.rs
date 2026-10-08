use super::IServerObject;
use crate::platform::IBitmapImpl;
use std::cell::RefCell;
use std::rc::Rc;

/// The `Changed` delegate of a server-side surface: the callbacks of the
/// visuals that show it.
#[derive(Default)]
pub struct ServerCompositionSurfaceChanged {
    handlers: RefCell<Vec<(usize, Rc<dyn Fn()>)>>,
}

impl ServerCompositionSurfaceChanged {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Changed += handler`; `key` identifies the subscriber.
    pub fn add(&self, key: usize, handler: Rc<dyn Fn()>) {
        self.handlers.borrow_mut().push((key, handler));
    }

    /// `Changed -= handler`.
    pub fn remove(&self, key: usize) {
        let mut handlers = self.handlers.borrow_mut();
        if let Some(index) = handlers.iter().rposition(|(k, _)| *k == key) {
            handlers.remove(index);
        }
    }

    /// `Changed?.Invoke()`.
    pub fn invoke(&self) {
        let handlers: Vec<_> = self.handlers.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler();
        }
    }
}

/// Server-side counterpart of a composition surface: something that
/// provides a bitmap to the surface visuals that show it.
pub trait IServerCompositionSurface: IServerObject {
    /// The current content of the surface.
    fn bitmap(&self) -> Option<std::sync::Arc<crate::platform::SharedBitmapImpl>>;

    /// Raised when the content changed.
    fn changed(&self) -> &ServerCompositionSurfaceChanged;
}
