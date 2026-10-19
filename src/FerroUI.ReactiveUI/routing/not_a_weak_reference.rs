//! `NotAWeakReference` (`ReactiveUI.Core/Routing/NotAWeakReference.cs`).

use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// A reference to an object that is always alive (`NotAWeakReference`): it holds the object
/// strongly, for an API that expects something shaped like a weak reference. The message bus
/// holds its channels with it.
#[derive(Clone)]
pub struct NotAWeakReference {
    target: Rc<dyn Any>,
}

impl NotAWeakReference {
    /// A reference to `target` (`NotAWeakReference(object)`).
    pub fn new(target: Rc<dyn Any>) -> Self {
        Self { target }
    }

    /// The object (`Target`).
    pub fn target(&self) -> &Rc<dyn Any> {
        &self.target
    }

    /// Whether the object is alive: always, since the reference is strong (`IsAlive`).
    pub fn is_alive(&self) -> bool {
        true
    }
}

impl fmt::Debug for NotAWeakReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotAWeakReference")
            .field("target", &(Rc::as_ptr(&self.target) as *const ()))
            .field("is_alive", &self.is_alive())
            .finish()
    }
}
