//! `StableCompositeDisposable`.

use super::ICancelable;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A fixed group of disposables that are disposed together (`StableCompositeDisposable`): unlike a
/// [`CompositeDisposable`](super::CompositeDisposable) nothing can be added or removed.
pub struct StableCompositeDisposable {
    disposables: RefCell<Vec<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl StableCompositeDisposable {
    /// A group of two disposables (`Create(IDisposable, IDisposable)`).
    pub fn create(first: Rc<dyn IDisposable>, second: Rc<dyn IDisposable>) -> Rc<dyn ICancelable> {
        Self::create_from([first, second])
    }

    /// A group of the given disposables (`Create(params IDisposable[])`).
    pub fn create_from(disposables: impl IntoIterator<Item = Rc<dyn IDisposable>>) -> Rc<dyn ICancelable> {
        Rc::new(Self { disposables: RefCell::new(disposables.into_iter().collect()), disposed: Cell::new(false) })
    }
}

impl IDisposable for StableCompositeDisposable {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let disposables = std::mem::take(&mut *self.disposables.borrow_mut());
        for d in disposables {
            d.dispose();
        }
    }
}

impl ICancelable for StableCompositeDisposable {
    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}
