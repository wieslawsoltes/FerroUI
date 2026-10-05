use crate::items_source::ItemsChangedHandler;
use crate::utils::CollectionUtils;
use ferroui_base::utilities::HandlerList;
use std::cell::OnceCell;
use std::rc::Rc;

/// A read-only, indexable list exposed by a selection model: the selected
/// indexes (`T` = `i32`) or the selected items (`T` = `Option<_>`, a null
/// item being `None`).
///
/// A list is a view: it reads the selection when it is asked, it does not
/// copy it. Handles are `Rc<dyn IReadOnlySelectionList<T>>`.
pub trait IReadOnlySelectionList<T: Clone + PartialEq + 'static> {
    /// The number of elements.
    fn count(&self) -> usize;

    /// The element at `index`. Panics if the index is out of range.
    fn get(&self, index: usize) -> T;

    /// Enumerates the elements.
    fn iter(&self) -> Box<dyn Iterator<Item = T> + '_>;

    /// Subscribes to changes of the list. Returns the token for
    /// [`remove_collection_changed`](Self::remove_collection_changed), or
    /// `None` if the list does not notify.
    fn add_collection_changed(&self, _handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        None
    }

    /// Unsubscribes a handler.
    fn remove_collection_changed(&self, _token: u64) {}

    /// Whether the list has no elements.
    fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Whether the list contains an element equal to `value`.
    fn contains(&self, value: &T) -> bool {
        self.count() != 0 && self.index_of(value) != -1
    }

    /// The index of the first element equal to `value`, or `-1`.
    fn index_of(&self, value: &T) -> i32 {
        for i in 0..self.count() {
            if self.get(i) == *value {
                return i as i32;
            }
        }

        -1
    }

    /// Copies the elements.
    fn to_vec(&self) -> Vec<T> {
        self.iter().collect()
    }
}

/// A selection list holding its own elements.
pub struct ReadOnlySelectionList<T>(Vec<T>);

impl<T> ReadOnlySelectionList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self(items)
    }
}

impl<T: Clone + PartialEq + 'static> IReadOnlySelectionList<T> for ReadOnlySelectionList<T> {
    fn count(&self) -> usize {
        self.0.len()
    }

    fn get(&self, index: usize) -> T {
        if index >= self.0.len() {
            panic!("The index was out of range.");
        }

        self.0[index].clone()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = T> + '_> {
        Box::new(self.0.iter().cloned())
    }

    fn to_vec(&self) -> Vec<T> {
        self.0.clone()
    }
}

/// The state shared by the selection lists: the change notification. A list
/// raises a reset whenever the selection changes, whatever the change.
pub struct ReadOnlySelectionListBase {
    collection_changed: OnceCell<HandlerList<ItemsChangedHandler>>,
}

impl Default for ReadOnlySelectionListBase {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadOnlySelectionListBase {
    pub fn new() -> Self {
        Self { collection_changed: OnceCell::new() }
    }

    pub fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> u64 {
        self.collection_changed.get_or_init(HandlerList::new).add(handler)
    }

    pub fn remove_collection_changed(&self, token: u64) {
        if let Some(handlers) = self.collection_changed.get() {
            handlers.remove(token);
        }
    }

    pub fn raise_collection_reset(&self) {
        let Some(handlers) = self.collection_changed.get() else {
            return;
        };

        if handlers.is_empty() {
            return;
        }

        for (_, handler) in handlers.snapshot().iter() {
            handler(&CollectionUtils::RESET_EVENT_ARGS);
        }
    }
}
