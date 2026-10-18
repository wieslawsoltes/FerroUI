//! `SerialDisposable` and `SerialDisposableValue`.

use super::single_assignment_disposable::Slot;
use super::ICancelable;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// A slot whose disposable can be replaced, disposing the previous one (`SerialDisposableValue`).
/// A disposable assigned after the slot was disposed is disposed at once.
pub struct SerialDisposableValue {
    slot: RefCell<Slot>,
}

impl Default for SerialDisposableValue {
    fn default() -> Self {
        Self::new()
    }
}

impl SerialDisposableValue {
    pub const fn new() -> Self {
        Self { slot: RefCell::new(Slot::Empty) }
    }

    pub fn is_disposed(&self) -> bool {
        matches!(*self.slot.borrow(), Slot::Disposed)
    }

    /// The current disposable, if any.
    pub fn disposable(&self) -> Option<Rc<dyn IDisposable>> {
        match &*self.slot.borrow() {
            Slot::Set(d) => Some(d.clone()),
            _ => None,
        }
    }

    /// Replaces the disposable and disposes the previous one.
    pub fn set_disposable(&self, value: Option<Rc<dyn IDisposable>>) {
        let mut slot = self.slot.borrow_mut();
        if matches!(*slot, Slot::Disposed) {
            drop(slot);
            if let Some(value) = value {
                value.dispose();
            }
            return;
        }
        let previous = std::mem::replace(&mut *slot, value.map_or(Slot::Empty, Slot::Set));
        drop(slot);
        if let Slot::Set(previous) = previous {
            previous.dispose();
        }
    }

    /// Assigns the first disposable; false when one was assigned already (`TrySetFirst`).
    pub fn try_set_first(&self, value: Rc<dyn IDisposable>) -> bool {
        let mut slot = self.slot.borrow_mut();
        match &*slot {
            Slot::Empty => {
                *slot = Slot::Set(value);
                true
            }
            Slot::Disposed => {
                drop(slot);
                value.dispose();
                true
            }
            Slot::Set(_) => false,
        }
    }

    pub fn dispose(&self) {
        let previous = std::mem::replace(&mut *self.slot.borrow_mut(), Slot::Disposed);
        if let Slot::Set(d) = previous {
            d.dispose();
        }
    }
}

/// A disposable whose underlying disposable can be replaced, which disposes the previous one
/// (`SerialDisposable`).
#[derive(Default)]
pub struct SerialDisposable {
    value: SerialDisposableValue,
}

impl SerialDisposable {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn disposable(&self) -> Option<Rc<dyn IDisposable>> {
        self.value.disposable()
    }

    /// Replaces the underlying disposable and disposes the previous one; a disposable assigned
    /// after this one was disposed is disposed at once.
    pub fn set_disposable(&self, value: Option<Rc<dyn IDisposable>>) {
        self.value.set_disposable(value)
    }
}

impl IDisposable for SerialDisposable {
    fn dispose(&self) {
        self.value.dispose()
    }
}

impl ICancelable for SerialDisposable {
    fn is_disposed(&self) -> bool {
        self.value.is_disposed()
    }
}
