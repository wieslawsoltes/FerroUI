//! `BooleanDisposable`.

use super::ICancelable;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// A disposable that only records that it has been disposed (`BooleanDisposable`).
#[derive(Default)]
pub struct BooleanDisposable {
    disposed: Cell<bool>,
}

impl BooleanDisposable {
    /// A disposable that is not disposed yet.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}

impl IDisposable for BooleanDisposable {
    fn dispose(&self) {
        self.disposed.set(true);
    }
}

impl ICancelable for BooleanDisposable {
    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}
