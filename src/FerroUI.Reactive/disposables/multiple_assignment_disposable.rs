//! `MultipleAssignmentDisposable` and `MultipleAssignmentDisposableValue`.

use super::single_assignment_disposable::Slot;
use super::ICancelable;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// A slot whose disposable can be replaced without disposing the previous one
/// (`MultipleAssignmentDisposableValue`). A disposable assigned after the slot was disposed is
/// disposed at once.
pub struct MultipleAssignmentDisposableValue {
    slot: RefCell<Slot>,
}

impl Default for MultipleAssignmentDisposableValue {
    fn default() -> Self {
        Self::new()
    }
}

impl MultipleAssignmentDisposableValue {
    pub const fn new() -> Self {
        Self { slot: RefCell::new(Slot::Empty) }
    }

    pub fn is_disposed(&self) -> bool {
        matches!(*self.slot.borrow(), Slot::Disposed)
    }

    pub fn disposable(&self) -> Option<Rc<dyn IDisposable>> {
        match &*self.slot.borrow() {
            Slot::Set(d) => Some(d.clone()),
            _ => None,
        }
    }

    /// Replaces the disposable without disposing the previous one.
    pub fn set_disposable(&self, value: Option<Rc<dyn IDisposable>>) {
        let mut slot = self.slot.borrow_mut();
        if matches!(*slot, Slot::Disposed) {
            drop(slot);
            if let Some(value) = value {
                value.dispose();
            }
            return;
        }
        *slot = value.map_or(Slot::Empty, Slot::Set);
    }

    pub fn dispose(&self) {
        let previous = std::mem::replace(&mut *self.slot.borrow_mut(), Slot::Disposed);
        if let Slot::Set(d) = previous {
            d.dispose();
        }
    }
}

/// A disposable whose underlying disposable can be replaced without disposing the previous one
/// (`MultipleAssignmentDisposable`).
#[derive(Default)]
pub struct MultipleAssignmentDisposable {
    value: MultipleAssignmentDisposableValue,
}

impl MultipleAssignmentDisposable {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn disposable(&self) -> Option<Rc<dyn IDisposable>> {
        self.value.disposable()
    }

    pub fn set_disposable(&self, value: Option<Rc<dyn IDisposable>>) {
        self.value.set_disposable(value)
    }
}

impl IDisposable for MultipleAssignmentDisposable {
    fn dispose(&self) {
        self.value.dispose()
    }
}

impl ICancelable for MultipleAssignmentDisposable {
    fn is_disposed(&self) -> bool {
        self.value.is_disposed()
    }
}
