use crate::media::ITransform;
use crate::reactive::IDisposable;
use std::rc::Rc;

/// A transform whose value can change.
pub trait IMutableTransform: ITransform {
    /// Subscribes to changes of the transform. Disposing the returned handle
    /// unsubscribes.
    fn changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}
