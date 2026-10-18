use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A list of event handlers that tolerates being modified while it is being
/// invoked: invocation works on a snapshot, and the snapshot is only copied
/// when the list actually changes.
pub struct HandlerList<F: ?Sized> {
    /// `None` until the first handler is added: most lists of an object never get one, and an
    /// empty list then costs no allocation.
    handlers: RefCell<Option<Rc<Vec<(u64, Rc<F>)>>>>,
    next_id: HandlerIds,
}

/// The handlers of a [`HandlerList`] at the time of [`HandlerList::snapshot`]: later
/// modifications of the list do not affect it.
pub struct HandlerSnapshot<F: ?Sized>(Option<Rc<Vec<(u64, Rc<F>)>>>);

impl<F: ?Sized> Clone for HandlerSnapshot<F> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<F: ?Sized> std::ops::Deref for HandlerSnapshot<F> {
    type Target = [(u64, Rc<F>)];

    #[inline]
    fn deref(&self) -> &[(u64, Rc<F>)] {
        match &self.0 {
            Some(handlers) => handlers,
            None => &[],
        }
    }
}

/// Where the tokens of a handler list come from.
enum HandlerIds {
    Own(Cell<u64>),
    /// A counter shared with other lists, so that tokens across them are in
    /// subscription order (the typed and untyped forms of one event).
    Shared(Rc<Cell<u64>>),
    /// The counter of the process, which every list made with
    /// [`HandlerList::with_ordered_ids`] shares: tokens across all of them are
    /// in subscription order, and a list needs no counter of its own.
    Ordered,
}

/// The counter of [`HandlerIds::Ordered`].
static ORDERED_IDS: AtomicU64 = AtomicU64::new(1);

impl HandlerIds {
    fn next(&self) -> u64 {
        let cell = match self {
            Self::Own(cell) => cell,
            Self::Shared(cell) => cell,
            Self::Ordered => return ORDERED_IDS.fetch_add(1, Ordering::Relaxed),
        };
        let id = cell.get();
        cell.set(id + 1);
        id
    }
}

impl<F: ?Sized> Default for HandlerList<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: ?Sized> HandlerList<F> {
    pub fn new() -> Self {
        Self { handlers: RefCell::new(None), next_id: HandlerIds::Own(Cell::new(1)) }
    }

    /// A list whose tokens come from `ids`, a counter shared with other
    /// lists: tokens across all of them increase in subscription order.
    pub fn with_shared_ids(ids: Rc<Cell<u64>>) -> Self {
        Self { handlers: RefCell::new(None), next_id: HandlerIds::Shared(ids) }
    }

    /// A list whose tokens come from the counter of the process, which every
    /// list made this way shares: tokens across all of them increase in
    /// subscription order, as with [`with_shared_ids`](Self::with_shared_ids),
    /// without a counter to allocate.
    pub fn with_ordered_ids() -> Self {
        Self { handlers: RefCell::new(None), next_id: HandlerIds::Ordered }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.handlers.borrow().as_ref().is_none_or(|handlers| handlers.is_empty())
    }

    pub fn len(&self) -> usize {
        self.handlers.borrow().as_ref().map_or(0, |handlers| handlers.len())
    }

    /// Adds a handler and returns a token that removes it again.
    pub fn add(&self, handler: Rc<F>) -> u64 {
        let id = self.next_id.next();
        let mut handlers = self.handlers.borrow_mut();
        let handlers = handlers.get_or_insert_with(|| Rc::new(Vec::new()));
        Rc::make_mut_or_clone(handlers).push((id, handler));
        id
    }

    /// Removes the handler registered under `token`.
    pub fn remove(&self, token: u64) -> bool {
        let mut handlers = self.handlers.borrow_mut();
        let Some(handlers) = handlers.as_mut() else {
            return false;
        };
        match handlers.iter().position(|(id, _)| *id == token) {
            Some(index) => {
                Rc::make_mut_or_clone(handlers).remove(index);
                true
            }
            None => false,
        }
    }

    /// Returns the current handlers; later modifications do not affect it.
    #[inline]
    pub fn snapshot(&self) -> HandlerSnapshot<F> {
        HandlerSnapshot(self.handlers.borrow().clone())
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
