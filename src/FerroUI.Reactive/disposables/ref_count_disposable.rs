//! `RefCountDisposable`.

use super::{Disposable, ICancelable};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A disposable that disposes its underlying disposable once it has been disposed itself and all
/// the dependent disposables it handed out have been disposed (`RefCountDisposable`).
pub struct RefCountDisposable {
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    count: Cell<usize>,
    primary_disposed: Cell<bool>,
    throw_when_disposed: bool,
}

impl RefCountDisposable {
    /// Counts the references to `disposable`.
    pub fn new(disposable: Rc<dyn IDisposable>) -> Rc<Self> {
        Self::with_throw_when_disposed(disposable, false)
    }

    /// As [`new`](Self::new); with `throw_when_disposed`, asking a disposed instance for a
    /// dependent disposable panics (upstream throws `ObjectDisposedException`) instead of returning
    /// an empty one.
    pub fn with_throw_when_disposed(disposable: Rc<dyn IDisposable>, throw_when_disposed: bool) -> Rc<Self> {
        Rc::new(Self {
            disposable: RefCell::new(Some(disposable)),
            count: Cell::new(0),
            primary_disposed: Cell::new(false),
            throw_when_disposed,
        })
    }

    /// A dependent disposable, which keeps the underlying disposable alive until it is disposed
    /// (`GetDisposable`).
    pub fn get_disposable(self: &Rc<Self>) -> Rc<dyn IDisposable> {
        if self.is_disposed() {
            if self.throw_when_disposed {
                panic!("Cannot access a disposed object.\nObject name: 'RefCountDisposable'.");
            }
            return Disposable::empty();
        }
        self.count.set(self.count.get() + 1);
        Rc::new(InnerDisposable { parent: RefCell::new(Some(self.clone())) })
    }

    fn release(&self) {
        let count = self.count.get() - 1;
        self.count.set(count);
        if count == 0 && self.primary_disposed.get() {
            self.dispose_underlying();
        }
    }

    fn dispose_underlying(&self) {
        let disposable = self.disposable.borrow_mut().take();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }
}

impl IDisposable for RefCountDisposable {
    fn dispose(&self) {
        if self.primary_disposed.replace(true) {
            return;
        }
        if self.count.get() == 0 {
            self.dispose_underlying();
        }
    }
}

impl ICancelable for RefCountDisposable {
    /// Whether the instance and every dependent disposable have been disposed.
    fn is_disposed(&self) -> bool {
        self.primary_disposed.get() && self.count.get() == 0
    }
}

struct InnerDisposable {
    parent: RefCell<Option<Rc<RefCountDisposable>>>,
}

impl IDisposable for InnerDisposable {
    fn dispose(&self) {
        let parent = self.parent.borrow_mut().take();
        if let Some(parent) = parent {
            parent.release();
        }
    }
}
