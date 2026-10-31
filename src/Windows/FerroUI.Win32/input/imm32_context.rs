//! Ownership and borrowing of native IMM contexts. A context returned by
//! ImmGetContext is released; a context created by this backend is destroyed
//! only after its window association and all outstanding borrows are gone.

use super::imm32_input_method::{Imm32Parent, ImmSystem};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(super) struct OwnedImmContext {
    system: Rc<dyn ImmSystem>,
    pub handle: isize,
}

impl OwnedImmContext {
    pub fn create(system: Rc<dyn ImmSystem>) -> Option<Rc<Self>> {
        let handle = system.imm_create_context();
        (handle != 0).then(|| Rc::new(Self { system, handle }))
    }
}

impl Drop for OwnedImmContext {
    fn drop(&mut self) {
        self.system.imm_destroy_context(self.handle);
    }
}

/// One successful native acquisition, including the original HWND/HIMC pair.
/// Keeping an owned handle alive also protects a borrow across reentrant window
/// destruction: the release happens before the final owned handle is destroyed.
pub(super) struct ImmContext {
    system: Rc<dyn ImmSystem>,
    hwnd: isize,
    pub handle: isize,
    _owned: Option<Rc<OwnedImmContext>>,
}

impl ImmContext {
    pub fn acquire(
        system: Rc<dyn ImmSystem>,
        hwnd: isize,
        owned: Option<Rc<OwnedImmContext>>,
    ) -> Option<Self> {
        if hwnd == 0 {
            return None;
        }
        let handle = system.imm_get_context(hwnd);
        (handle != 0).then(|| Self {
            system,
            hwnd,
            handle,
            _owned: owned,
        })
    }
}

impl Drop for ImmContext {
    fn drop(&mut self) {
        self.system.imm_release_context(self.hwnd, self.handle);
    }
}

/// A physical window's context state. Borrowed default contexts are never
/// destroyed. Disabling saves the association so enabling can restore that exact
/// context instead of manufacturing a new one every time focus changes.
pub(super) struct ImmWindowContext {
    system: Rc<dyn ImmSystem>,
    pub hwnd: isize,
    pub parent: Option<Weak<dyn Imm32Parent>>,
    pub suspended: Cell<Option<isize>>,
    pub original: Cell<isize>,
    pub owned: RefCell<Option<Rc<OwnedImmContext>>>,
    pub retired: Cell<bool>,
    disposed: Cell<bool>,
}

impl ImmWindowContext {
    pub fn new(
        system: Rc<dyn ImmSystem>,
        hwnd: isize,
        parent: Option<Weak<dyn Imm32Parent>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            system,
            hwnd,
            parent,
            suspended: Cell::new(None),
            original: Cell::new(0),
            owned: RefCell::new(None),
            retired: Cell::new(false),
            disposed: Cell::new(false),
        })
    }

    pub fn same_parent(&self, parent: Option<&Weak<dyn Imm32Parent>>) -> bool {
        match (self.parent.as_ref(), parent) {
            (Some(a), Some(b)) => Weak::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }

    pub fn acquire(&self) -> Option<ImmContext> {
        let owned = self.owned.borrow().clone();
        ImmContext::acquire(self.system.clone(), self.hwnd, owned)
    }

    pub fn dispose(&self) {
        self.retired.set(true);
        if self.disposed.replace(true) {
            return;
        }
        let owned = self.owned.take();
        let suspended = self.suspended.take();
        if owned.is_none() && suspended.is_none() {
            // An untouched default association remains owned by the OS.
            return;
        }
        let context = ImmContext::acquire(self.system.clone(), self.hwnd, owned.clone());
        let current = context.as_ref().map_or(0, |context| context.handle);
        // No borrowed native handle may outlive its final owned handle.
        drop(context);
        if let Some(owned) = &owned {
            if current == owned.handle {
                self.system
                    .imm_associate_context(self.hwnd, self.original.get());
            } else if current == 0 {
                let restore = suspended
                    .map(|context| {
                        if context == owned.handle {
                            self.original.get()
                        } else {
                            context
                        }
                    })
                    .unwrap_or(0);
                if restore != 0 {
                    self.system.imm_associate_context(self.hwnd, restore);
                }
            }
        } else if current == 0 {
            if let Some(suspended) = suspended.filter(|context| *context != 0) {
                self.system.imm_associate_context(self.hwnd, suspended);
            }
        }
        drop(owned);
    }
}

impl Drop for ImmWindowContext {
    fn drop(&mut self) {
        self.dispose();
    }
}
