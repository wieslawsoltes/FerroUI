//! Port of `DragDropContextStore.cs`.

use ferroui_base::{AnyValue, BoxedValue};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

ferroui_base::ferro_thread_local! {
    static ENTRIES: RefCell<HashMap<String, Weak<dyn AnyValue>>> = RefCell::new(HashMap::new());
    static NEXT_KEY: Cell<u64> = const { Cell::new(1) };
}

/// Provides an in-process store for objects attached to drag data: the drag data carries the key
/// of the object, the drop target finds the object by it.
///
/// Deviation (DEVIATIONS.md, Behaviors): the store of the reference is shared by the threads of the
/// process (a concurrent dictionary); the objects of the framework belong to the thread that uses
/// them, so the store is per thread. Its keys are unique in the process (a counter instead of a
/// GUID), as they travel in drag data.
pub(crate) struct DragDropContextStore;

impl DragDropContextStore {
    /// Stores `value` (weakly, as the reference does) and returns its key, or `None` for no value.
    pub(crate) fn add(value: Option<&BoxedValue>) -> Option<String> {
        let value = value?;

        static PROCESS_KEYS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let process_key = PROCESS_KEYS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let thread_key = NEXT_KEY.with(|next| next.replace(next.get() + 1));
        let key = format!("{process_key:016x}-{thread_key:016x}");
        ENTRIES.with(|entries| entries.borrow_mut().insert(key.clone(), Rc::downgrade(value)));
        Some(key)
    }

    /// The value stored under `key`, if it is still alive (an entry whose value is gone is
    /// removed).
    pub(crate) fn get(key: Option<&str>) -> Option<BoxedValue> {
        let key = key?;

        ENTRIES.with(|entries| {
            let reference = entries.borrow().get(key).cloned()?;
            match reference.upgrade() {
                Some(value) => Some(value),
                None => {
                    entries.borrow_mut().remove(key);
                    None
                }
            }
        })
    }

    /// Removes the value stored under `key`.
    pub(crate) fn remove(key: Option<&str>) {
        let Some(key) = key else {
            return;
        };

        // The weak reference is dropped outside of the borrow.
        let removed = ENTRIES.with(|entries| entries.borrow_mut().remove(key));
        drop(removed);
    }
}

/// Not from the reference: removes the value stored under a key when dropped (the `finally` of
/// the drags of the reference, which removes it when the drag ends).
pub(crate) struct RemoveContextOnDrop(pub(crate) Option<String>);

impl Drop for RemoveContextOnDrop {
    fn drop(&mut self) {
        DragDropContextStore::remove(self.0.as_deref());
    }
}
