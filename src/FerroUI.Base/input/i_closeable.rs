use crate::reactive::IDisposable;
use std::rc::Rc;

/// Something that tells when it has been closed.
pub trait ICloseable {
    /// Raised when the object has been closed. Disposing the returned
    /// handle unsubscribes.
    fn closed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}
