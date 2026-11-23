use std::cell::Cell;
use smallvec::SmallVec;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A list of event handlers that tolerates being modified while it is being
/// invoked: invocation works on a snapshot, and the snapshot is only copied
/// when the list actually changes.
///
/// A list is one pointer wide. An object with a dozen events, almost all of
/// which nobody subscribes to, pays that for each of them and nothing else
/// (desktop-performance.md, section 9.40).
///
/// The tokens of every list come from one counter of the process: within a
/// list, and across lists, a later subscription has a greater token, and a
/// token is never given out twice. The typed and the untyped form of one
/// event, which run their handlers in subscription order as the single
/// event of the original, need nothing more than that.
pub struct HandlerList<F: ?Sized> {
    /// `None` until the first handler is added: most lists of an object never get one, and an
    /// empty list then costs no allocation.
    ///
    /// A `Cell`: the handlers are taken out for the length of one operation of the list and
    /// put back, and no code of a handler runs in between (see [`HandlerList::with`]).
    handlers: Cell<Option<Rc<Handlers<F>>>>,
}

/// The handlers of a list: most lists hold one or two, which are stored with the list's shared
/// allocation.
type Handlers<F> = SmallVec<[(u64, Rc<F>); 2]>;

/// The handlers of a [`HandlerList`] at the time of [`HandlerList::snapshot`]: later
/// modifications of the list do not affect it.
pub struct HandlerSnapshot<F: ?Sized>(Option<Rc<Handlers<F>>>);

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

/// The counter the tokens of every list come from.
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

impl<F: ?Sized> Default for HandlerList<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: ?Sized> HandlerList<F> {
    pub const fn new() -> Self {
        Self { handlers: Cell::new(None) }
    }

    /// Runs `operation` with the handlers, which are out of the list
    /// meanwhile. `operation` runs no code of a handler and drops no
    /// handler: either could come back to this list and find it empty.
    #[inline]
    fn with<R>(&self, operation: impl FnOnce(&mut Option<Rc<Handlers<F>>>) -> R) -> R {
        let mut handlers = self.handlers.take();
        let result = operation(&mut handlers);
        self.handlers.set(handlers);
        result
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.with(|handlers| handlers.as_ref().is_none_or(|handlers| handlers.is_empty()))
    }

    pub fn len(&self) -> usize {
        self.with(|handlers| handlers.as_ref().map_or(0, |handlers| handlers.len()))
    }

    /// Adds a handler and returns a token that removes it again.
    pub fn add(&self, handler: Rc<F>) -> u64 {
        let id = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
        self.with(|handlers| {
            let handlers = handlers.get_or_insert_with(|| Rc::new(SmallVec::new()));
            Rc::make_mut_or_clone(handlers).push((id, handler));
        });
        id
    }

    /// Removes the handler registered under `token`.
    pub fn remove(&self, token: u64) -> bool {
        let removed = self.with(|handlers| {
            let handlers = handlers.as_mut()?;
            // Tokens only grow and a handler is added at the end: the list is in their order.
            let index = handlers.binary_search_by_key(&token, |(id, _)| *id).ok()?;
            Some(Rc::make_mut_or_clone(handlers).remove(index))
        });
        // The handler is released with the list whole again: what it owns
        // may unsubscribe from this list as it goes.
        removed.is_some()
    }

    /// Returns the current handlers; later modifications do not affect it.
    #[inline]
    pub fn snapshot(&self) -> HandlerSnapshot<F> {
        HandlerSnapshot(self.with(|handlers| handlers.clone()))
    }
}

trait MakeMutOrClone<T> {
    fn make_mut_or_clone(this: &mut Self) -> &mut T;
}

impl<F: ?Sized> MakeMutOrClone<Handlers<F>> for Rc<Handlers<F>> {
    fn make_mut_or_clone(this: &mut Self) -> &mut Handlers<F> {
        if Rc::get_mut(this).is_none() {
            let copy: Handlers<F> = this.iter().map(|(id, f)| (*id, f.clone())).collect();
            *this = Rc::new(copy);
        }
        Rc::get_mut(this).expect("uniquely owned after copy")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(list: &HandlerList<dyn Fn()>) -> Vec<u64> {
        list.snapshot().iter().map(|(token, _)| *token).collect()
    }

    /// A handler is found by its token wherever it is in the list, and the handlers left keep their order: the
    /// list is searched as one in the order of its tokens.
    #[test]
    fn a_handler_is_removed_by_its_token_wherever_it_is_in_the_list() {
        let list: HandlerList<dyn Fn()> = HandlerList::new();
        let other: HandlerList<dyn Fn()> = HandlerList::new();
        let mut added = Vec::new();
        for index in 0..9 {
            added.push(list.add(Rc::new(|| {})));
            // The tokens of a list need not follow each other: other lists take from the same counter.
            if index % 2 == 0 {
                other.add(Rc::new(|| {}));
            }
        }
        assert_eq!(tokens(&list), added);
        assert!(added.windows(2).all(|pair| pair[0] < pair[1]));

        // The first, the last, one between them, and one of a list that shares its storage with a snapshot.
        for index in [0, 8, 4] {
            assert!(list.remove(added[index]));
        }
        let snapshot = list.snapshot();
        assert!(list.remove(added[5]));
        assert_eq!(snapshot.len(), 6);
        let left: Vec<u64> = [1, 2, 3, 6, 7].iter().map(|index| added[*index]).collect();
        assert_eq!(tokens(&list), left);

        // A token that was removed, one of another list and one never given out remove nothing.
        assert!(!list.remove(added[4]));
        assert!(!list.remove(tokens(&other)[0]));
        assert!(!list.remove(0));
        assert!(!list.remove(u64::MAX));
        assert_eq!(tokens(&list), left);

        // A handler added after removals is the last one.
        let last = list.add(Rc::new(|| {}));
        assert_eq!(tokens(&list).last(), Some(&last));
        for token in left {
            assert!(list.remove(token));
        }
        assert!(list.remove(last));
        assert!(list.is_empty());
        assert!(!list.remove(last));
    }
}
