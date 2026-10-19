//! The disposables Splat takes from ReactiveUI.Primitives (`ActionDisposable`, `BooleanDisposable`,
//! `EmptyDisposable`). The crate depends only on `ferroui-system`, so it keeps the three small ones
//! it needs here.

use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Runs an action the first time it is disposed (`ActionDisposable`).
pub(crate) struct ActionDisposable {
    action: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl ActionDisposable {
    pub(crate) fn new(action: impl FnOnce() + 'static) -> Rc<Self> {
        Rc::new(Self { action: RefCell::new(Some(Box::new(action))) })
    }
}

impl IDisposable for ActionDisposable {
    fn dispose(&self) {
        let action = self.action.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
    }
}

/// Records that it has been disposed (`BooleanDisposable`).
#[derive(Default)]
pub(crate) struct BooleanDisposable {
    disposed: Cell<bool>,
}

impl BooleanDisposable {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}

impl IDisposable for BooleanDisposable {
    fn dispose(&self) {
        self.disposed.set(true);
    }
}

/// Does nothing when disposed (`EmptyDisposable.Instance`).
pub(crate) struct EmptyDisposable;

impl EmptyDisposable {
    pub(crate) fn instance() -> Rc<dyn IDisposable> {
        Rc::new(EmptyDisposable)
    }
}

impl IDisposable for EmptyDisposable {
    fn dispose(&self) {}
}
