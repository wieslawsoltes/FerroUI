use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A resource or subscription that is released explicitly.
///
/// Unlike `Drop`, disposal is an explicit, idempotent operation: dropping a
/// handle without calling [`dispose`](IDisposable::dispose) keeps the
/// subscription alive, which is what callers that ignore the returned handle
/// expect.
pub trait IDisposable {
    fn dispose(&self);
}

/// Handles compare by identity (reference equality), so that the handle a
/// subscription or binding returns can be held in untyped values.
impl PartialEq for dyn IDisposable {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// Factory for common [`IDisposable`] implementations.
pub struct Disposable;

struct EmptyDisposable;

impl IDisposable for EmptyDisposable {
    fn dispose(&self) {}
}

struct AnonymousDisposable<F: FnOnce()> {
    action: Cell<Option<F>>,
}

impl<F: FnOnce()> IDisposable for AnonymousDisposable<F> {
    fn dispose(&self) {
        if let Some(action) = self.action.take() {
            action();
        }
    }
}

impl Disposable {
    /// A disposable that does nothing.
    pub fn empty() -> Rc<dyn IDisposable> {
        thread_local! {
            static EMPTY: Rc<dyn IDisposable> = Rc::new(EmptyDisposable);
        }
        EMPTY.with(Rc::clone)
    }

    /// A disposable that runs `action` the first time it is disposed.
    pub fn create(action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        Rc::new(AnonymousDisposable { action: Cell::new(Some(action)) })
    }

    /// A disposable that gives `state` to `dispose` the first time it is
    /// disposed.
    pub fn create_with_state<TState: 'static>(
        state: TState,
        dispose: impl FnOnce(TState) + 'static,
    ) -> Rc<dyn IDisposable> {
        Rc::new(AnonymousDisposable { action: Cell::new(Some(move || dispose(state))) })
    }
}

/// Holds a replaceable inner disposable; assigning a new one disposes the
/// previous one.
#[derive(Default)]
pub struct SerialDisposable {
    current: RefCell<Option<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl SerialDisposable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, value: Option<Rc<dyn IDisposable>>) {
        if self.disposed.get() {
            if let Some(v) = value {
                v.dispose();
            }
            return;
        }
        let old = self.current.replace(value);
        if let Some(old) = old {
            old.dispose();
        }
    }
}

impl IDisposable for SerialDisposable {
    fn dispose(&self) {
        if !self.disposed.replace(true) {
            if let Some(old) = self.current.take() {
                old.dispose();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    #[test]
    fn a_disposable_with_state_gives_it_to_the_action_once() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let s = seen.clone();
        let target = Disposable::create_with_state(5, move |state: i32| s.borrow_mut().push(state));

        assert!(seen.borrow().is_empty());
        target.dispose();
        target.dispose();
        assert_eq!(vec![5], *seen.borrow());
    }
}
