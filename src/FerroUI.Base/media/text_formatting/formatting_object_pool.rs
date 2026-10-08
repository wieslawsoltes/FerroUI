use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::media::text_formatting::formatting_buffer_helper::FormattingBufferHelper;
use crate::media::text_formatting::{TextLine, TextRun, UnshapedTextRun};

/// Contains various list pools that are commonly used during text formatting.
///
/// This class provides an instance per thread.
///
/// In most applications, there'll be only one instance: on the UI thread,
/// which is responsible for layout.
// Internal upstream; public so the Skia unit tests reach it.
pub struct FormattingObjectPool {
    pub text_run_lists: ListPool<Rc<dyn TextRun>>,
    pub unshaped_text_run_lists: ListPool<Rc<UnshapedTextRun>>,
    pub text_lines: ListPool<Rc<dyn TextLine>>,
}

/// A list rented from a [`ListPool`]. Give it back with [`ListPool::return_list`].
pub type RentedList<T> = Vec<T>;

impl FormattingObjectPool {
    /// The pool of the current thread.
    pub fn instance() -> &'static FormattingObjectPool {
        thread_local! {
            static INSTANCE: &'static FormattingObjectPool = Box::leak(Box::new(FormattingObjectPool {
                text_run_lists: ListPool::new(),
                unshaped_text_run_lists: ListPool::new(),
                text_lines: ListPool::new(),
            }));
        }
        INSTANCE.with(|instance| *instance)
    }

    /// Checks (in debug builds) that every rented list was returned.
    #[cfg_attr(not(test), allow(dead_code))] // called by tests, as upstream's test base does
    pub fn verify_all_returned(&self) {
        self.text_run_lists.verify_all_returned();
        self.unshaped_text_run_lists.verify_all_returned();
        self.text_lines.verify_all_returned();
    }
}

/// A pool of reusable lists.
pub struct ListPool<T> {
    lists: RefCell<Vec<Vec<T>>>,
    pending_return_count: Cell<i32>,
}

impl<T> ListPool<T> {
    /// We don't need a big number here, these are for temporary usages only
    /// which should quickly be returned.
    const MAX_SIZE: usize = 16;

    fn new() -> Self {
        Self { lists: RefCell::new(Vec::with_capacity(Self::MAX_SIZE)), pending_return_count: Cell::new(0) }
    }

    /// Takes an empty list out of the pool.
    pub fn rent(&self) -> RentedList<T> {
        let list = self.lists.borrow_mut().pop().unwrap_or_default();

        debug_assert!(list.is_empty(), "A RentedList has been used after being returned!");

        self.pending_return_count.set(self.pending_return_count.get() + 1);

        list
    }

    /// Gives a list back to the pool. The items are dropped; the storage is kept.
    pub fn return_list(&self, mut rented_list: RentedList<T>) {
        self.pending_return_count.set(self.pending_return_count.get() - 1);

        FormattingBufferHelper::clear_then_reset_if_too_large(&mut rented_list);

        let mut lists = self.lists.borrow_mut();

        if lists.len() < Self::MAX_SIZE {
            lists.push(rented_list);
        }
    }

    /// Gives an optional list back to the pool (upstream accepts null).
    pub fn return_optional(&self, rented_list: Option<RentedList<T>>) {
        if let Some(rented_list) = rented_list {
            self.return_list(rented_list);
        }
    }

    /// Checks (in debug builds) that every rented list was returned.
    #[cfg_attr(not(test), allow(dead_code))] // see FormattingObjectPool::verify_all_returned
    pub fn verify_all_returned(&self) {
        if cfg!(debug_assertions) {
            let pending_return_count = self.pending_return_count.replace(0);

            if pending_return_count > 0 {
                panic!(
                    "{pending_return_count} RentedList<{}> haven't been returned to the pool!",
                    std::any::type_name::<T>()
                );
            }

            if pending_return_count < 0 {
                panic!(
                    "{} RentedList<{}> extra lists have been returned to the pool!",
                    -pending_return_count,
                    std::any::type_name::<T>()
                );
            }
        }
    }
}
