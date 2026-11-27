//! `TableViewSource` (WinUI's tabular `TableViewSource`): an items list over another items
//! source, which a [`TableView`](crate::TableView) presents in an order (the sort of a column).
//!
//! The view holds the indexes of the items of its source in the order it presents them. It is
//! built again when the order or the source changes, and then raises a reset: a sort reorders
//! every row anyway. A source that does not notify is read when the view is built.

use crate::{IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::{Rc, Weak};

/// Selects the value an item is sorted by.
pub(crate) type TableViewSortKey = Rc<dyn Fn(&Option<BoxedValue>) -> Option<BoxedValue>>;

/// The order of a [`TableViewSource`]: the value of each item it sorts by, and the direction.
#[derive(Clone)]
pub(crate) struct TableViewSortDescription {
    pub(crate) key: TableViewSortKey,
    pub(crate) descending: bool,
}

/// An items list over another items source, presented in the order of a sort.
pub struct TableViewSource {
    source: ItemsSource,
    sort: RefCell<Option<TableViewSortDescription>>,
    /// The indexes in `source` of the items of the view, in the order of the view.
    view: RefCell<Vec<usize>>,
    handlers: RefCell<Vec<(u64, Rc<ItemsChangedHandler>)>>,
    next_token: Cell<u64>,
    /// The subscription to the changes of `source`, if it notifies.
    source_token: Cell<Option<u64>>,
}

impl TableViewSource {
    /// A view over `items` (WinUI's `TableViewSource.From`), in their order.
    pub fn from(items: ItemsSource) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak: &Weak<Self>| {
            let source_token = if items.is_notifying() {
                let weak = weak.clone();
                items.list().add_collection_changed(Rc::new(move |_: &ItemsChangedEventArgs<'_>| {
                    if let Some(this) = weak.upgrade() {
                        this.refresh();
                    }
                }))
            } else {
                None
            };
            Self {
                source: items,
                sort: RefCell::new(None),
                view: RefCell::new(Vec::new()),
                handlers: RefCell::new(Vec::new()),
                next_token: Cell::new(1),
                source_token: Cell::new(source_token),
            }
        });
        this.rebuild();
        this
    }

    /// The items source the view presents.
    pub fn source(&self) -> &ItemsSource {
        &self.source
    }

    /// The index in [`source`](Self::source) of the item at `index` of the view.
    pub fn source_index(&self, index: usize) -> Option<usize> {
        self.view.borrow().get(index).copied()
    }

    pub(crate) fn sort(&self) -> Option<TableViewSortDescription> {
        self.sort.borrow().clone()
    }

    /// Sets the order of the view, or the order of the source with `None`, and raises a reset.
    pub(crate) fn set_sort(&self, value: Option<TableViewSortDescription>) {
        *self.sort.borrow_mut() = value;
        self.refresh();
    }

    /// Builds the view again and raises a reset.
    pub fn refresh(&self) {
        self.rebuild();
        let handlers: Vec<Rc<ItemsChangedHandler>> =
            self.handlers.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler(&ItemsChangedEventArgs::RESET);
        }
    }

    fn rebuild(&self) {
        let count = self.source.count();
        let mut indexes: Vec<usize> = (0..count).collect();

        if let Some(sort) = self.sort.borrow().as_ref() {
            // One key per item, read once: the key may evaluate a binding.
            let keys: Vec<Option<BoxedValue>> = (0..count).map(|index| (sort.key)(&self.source.get_at(index))).collect();
            // A stable sort: equal keys keep the order of the source, in both directions.
            indexes.sort_by(|&a, &b| {
                let ordering = compare_keys(&keys[a], &keys[b]);
                if sort.descending {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
        }

        *self.view.borrow_mut() = indexes;
    }
}

/// The order of two sort keys: a missing value before every value, values in the order of
/// [`ValueTypes::compare`], and values without an order between them as equal.
pub(crate) fn compare_keys(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => ValueTypes::compare(a, b).unwrap_or(Ordering::Equal),
    }
}

impl Drop for TableViewSource {
    fn drop(&mut self) {
        if let Some(token) = self.source_token.take() {
            self.source.list().remove_collection_changed(token);
        }
    }
}

impl IItemsList for TableViewSource {
    fn count(&self) -> usize {
        self.view.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        let source_index = self.view.borrow()[index];
        self.source.get_at(source_index)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.handlers.borrow_mut().push((token, handler));
        Some(token)
    }

    fn remove_collection_changed(&self, token: u64) {
        self.handlers.borrow_mut().retain(|(t, _)| *t != token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl From<Rc<TableViewSource>> for ItemsSource {
    fn from(value: Rc<TableViewSource>) -> Self {
        ItemsSource::new(value)
    }
}
