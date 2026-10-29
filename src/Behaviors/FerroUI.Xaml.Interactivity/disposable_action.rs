//! Port of `DisposableAction.cs`.

use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// Represents an [`IDisposable`] that executes a specified action when disposed.
pub struct DisposableAction {
    dispose: Box<dyn Fn()>,
}

impl DisposableAction {
    /// Creates a disposable that executes `dispose` when disposed.
    pub fn new(dispose: impl Fn() + 'static) -> Self {
        Self { dispose: Box::new(dispose) }
    }

    /// An empty disposable that does nothing when disposed.
    pub fn empty() -> Rc<dyn IDisposable> {
        ferroui_base::ferro_thread_local! {
            static EMPTY: Rc<dyn IDisposable> = Rc::new(DisposableAction::new(|| {}));
        }
        EMPTY.with(Rc::clone)
    }

    /// Creates a new [`DisposableAction`] that executes the specified action when disposed.
    pub fn create(dispose: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        Rc::new(DisposableAction::new(dispose))
    }
}

impl IDisposable for DisposableAction {
    /// Executes the dispose action.
    fn dispose(&self) {
        (self.dispose)()
    }
}
