//! `SingleAssignmentDisposable` and `SingleAssignmentDisposableValue`.

use super::ICancelable;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// What a slot of a disposable holds.
pub(crate) enum Slot {
    Empty,
    Set(Rc<dyn IDisposable>),
    Disposed,
}

/// A slot for a disposable that can be assigned once (`SingleAssignmentDisposableValue`). A
/// disposable assigned after the slot was disposed is disposed at once; a second assignment panics,
/// where upstream throws `InvalidOperationException`.
pub struct SingleAssignmentDisposableValue {
    slot: RefCell<Slot>,
}

impl Default for SingleAssignmentDisposableValue {
    fn default() -> Self {
        Self::new()
    }
}

impl SingleAssignmentDisposableValue {
    pub const fn new() -> Self {
        Self { slot: RefCell::new(Slot::Empty) }
    }

    /// Whether the slot has been disposed.
    pub fn is_disposed(&self) -> bool {
        matches!(*self.slot.borrow(), Slot::Disposed)
    }

    /// The assigned disposable, if any (none once the slot is disposed).
    pub fn disposable(&self) -> Option<Rc<dyn IDisposable>> {
        match &*self.slot.borrow() {
            Slot::Set(d) => Some(d.clone()),
            _ => None,
        }
    }

    /// Assigns the disposable. Panics when one was assigned already.
    pub fn set_disposable(&self, value: Option<Rc<dyn IDisposable>>) {
        if !self.try_set(value) {
            panic!("Disposable has already been assigned.");
        }
    }

    /// Assigns the disposable unless one was assigned already; false when one was. A value assigned
    /// to a disposed slot is disposed and counts as assigned.
    pub(crate) fn try_set(&self, value: Option<Rc<dyn IDisposable>>) -> bool {
        let mut slot = self.slot.borrow_mut();
        match &*slot {
            Slot::Empty => {
                if let Some(value) = value {
                    *slot = Slot::Set(value);
                }
                true
            }
            Slot::Disposed => {
                drop(slot);
                if let Some(value) = value {
                    value.dispose();
                }
                true
            }
            Slot::Set(_) => false,
        }
    }

    /// Disposes the slot and the disposable it holds.
    pub fn dispose(&self) {
        let previous = std::mem::replace(&mut *self.slot.borrow_mut(), Slot::Disposed);
        if let Slot::Set(d) = previous {
            d.dispose();
        }
    }
}

/// A disposable whose underlying disposable can be assigned once (`SingleAssignmentDisposable`).
#[derive(Default)]
pub struct SingleAssignmentDisposable {
    value: SingleAssignmentDisposableValue,
}

impl SingleAssignmentDisposable {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The assigned disposable, if any.
    pub fn disposable(&self) -> Option<Rc<dyn IDisposable>> {
        self.value.disposable()
    }

    /// Assigns the underlying disposable. Panics when one was assigned already; a disposable
    /// assigned after this one was disposed is disposed at once.
    pub fn set_disposable(&self, value: Option<Rc<dyn IDisposable>>) {
        self.value.set_disposable(value)
    }
}

impl IDisposable for SingleAssignmentDisposable {
    fn dispose(&self) {
        self.value.dispose()
    }
}

impl ICancelable for SingleAssignmentDisposable {
    fn is_disposed(&self) -> bool {
        self.value.is_disposed()
    }
}
