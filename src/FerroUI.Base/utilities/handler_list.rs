use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A list of event handlers that tolerates being modified while it is being
/// invoked: invocation works on a snapshot, and the snapshot is only copied
/// when the list actually changes.
pub struct HandlerList<F: ?Sized> {
    handlers: RefCell<Rc<Vec<(u64, Rc<F>)>>>,
    next_id: Cell<u64>,
}

impl<F: ?Sized> Default for HandlerList<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: ?Sized> HandlerList<F> {
    pub fn new() -> Self {
        Self { handlers: RefCell::new(Rc::new(Vec::new())), next_id: Cell::new(1) }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.handlers.borrow().is_empty()
    }

    pub fn len(&self) -> usize {
        self.handlers.borrow().len()
    }

    /// Adds a handler and returns a token that removes it again.
    pub fn add(&self, handler: Rc<F>) -> u64 {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let mut handlers = self.handlers.borrow_mut();
        Rc::make_mut_or_clone(&mut handlers).push((id, handler));
        id
    }

    /// Removes the handler registered under `token`.
    pub fn remove(&self, token: u64) -> bool {
        let mut handlers = self.handlers.borrow_mut();
        match handlers.iter().position(|(id, _)| *id == token) {
            Some(index) => {
                Rc::make_mut_or_clone(&mut handlers).remove(index);
                true
            }
            None => false,
        }
    }

    /// Returns the current handlers; later modifications do not affect it.
    #[inline]
    pub fn snapshot(&self) -> Rc<Vec<(u64, Rc<F>)>> {
        self.handlers.borrow().clone()
    }
}

trait MakeMutOrClone<T> {
    fn make_mut_or_clone(this: &mut Self) -> &mut T;
}

impl<F: ?Sized> MakeMutOrClone<Vec<(u64, Rc<F>)>> for Rc<Vec<(u64, Rc<F>)>> {
    fn make_mut_or_clone(this: &mut Self) -> &mut Vec<(u64, Rc<F>)> {
        if Rc::get_mut(this).is_none() {
            let copy: Vec<(u64, Rc<F>)> = this.iter().map(|(id, f)| (*id, f.clone())).collect();
            *this = Rc::new(copy);
        }
        Rc::get_mut(this).expect("uniquely owned after copy")
    }
}
