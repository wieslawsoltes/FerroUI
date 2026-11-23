//! Sorting of a [`TableView`](crate::TableView) (WinUI's `TableView_Sort.cpp`, the part the
//! gallery page needs): the sort cycle of a column, the order of its values, and the view of the
//! items source in that order. The items source of the application is never changed.

use crate::items_source::{IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource};
use crate::{TableViewCell, TableViewColumn};
use ferroui_base::{BoxedValue, Ref};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::{Rc, Weak};

/// The direction a column of a table is sorted in (WinUI `SortDirection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SortDirection {
    /// The rows are in the order of the items source.
    #[default]
    None = 0,
    /// Smallest first.
    Ascending = 1,
    /// Largest first.
    Descending = 2,
}

/// The directions a click on a column header goes through (WinUI `TableViewSortCycle`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TableViewSortCycle {
    /// Ascending, descending, ascending.
    #[default]
    AscendingDescending = 0,
    /// Ascending, descending, the order of the source.
    AscendingDescendingNone = 1,
    /// Descending, ascending, descending.
    DescendingAscending = 2,
    /// Descending, ascending, the order of the source.
    DescendingAscendingNone = 3,
}

impl TableViewSortCycle {
    /// The direction after `current` (WinUI's `ToggleSortDirection`).
    pub(crate) fn next(self, current: SortDirection) -> SortDirection {
        use SortDirection::{Ascending, Descending, None};
        let (opening, second) = match self {
            Self::AscendingDescending | Self::AscendingDescendingNone => (Ascending, Descending),
            Self::DescendingAscending | Self::DescendingAscendingNone => (Descending, Ascending),
        };
        let to_unsorted = matches!(self, Self::AscendingDescendingNone | Self::DescendingAscendingNone);
        if current == opening {
            second
        } else if current == second {
            if to_unsorted { None } else { opening }
        } else {
            opening
        }
    }
}

/// Orders two row items, for a column whose cells have no value to compare (WinUI's
/// `ITableViewSortComparer`).
pub type TableViewSortComparer = Rc<dyn Fn(&Option<BoxedValue>, &Option<BoxedValue>) -> Ordering>;

/// Orders two cell values: no value first, numbers by value, text ignoring case and then by its
/// exact characters. Values of different or unknown types compare equal, so the sort, which is
/// stable, leaves them in the order of the source.
pub(crate) fn compare_values(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> Ordering {
    let (a, b) = match (a, b) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Less,
        (Some(_), None) => return Ordering::Greater,
        (Some(a), Some(b)) => (a, b),
    };
    if let (Some(a), Some(b)) = (number(a), number(b)) {
        return a.total_cmp(&b);
    }
    if let (Some(a), Some(b)) = (a.downcast_ref::<String>(), b.downcast_ref::<String>()) {
        return a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b));
    }
    if let (Some(a), Some(b)) = (a.downcast_ref::<bool>(), b.downcast_ref::<bool>()) {
        return a.cmp(b);
    }
    Ordering::Equal
}

fn number(value: &BoxedValue) -> Option<f64> {
    macro_rules! read {
        ($($type_:ty),*) => {
            $(if let Some(value) = value.downcast_ref::<$type_>() {
                return Some(*value as f64);
            })*
        };
    }
    read!(f64, f32, i32, i64, u32, u64, i16, u16, i8, u8, usize, isize);
    None
}

/// The indices of the items of `source` in the order of `column`, or `None` when the column has
/// neither a comparer nor a binding to read values with.
pub(crate) fn sorted_order(
    source: &ItemsSource,
    column: &Ref<TableViewColumn>,
    direction: SortDirection,
) -> Option<Vec<usize>> {
    let count = source.count();
    let mut order: Vec<usize> = (0..count).collect();
    let directed = |ordering: Ordering| if direction == SortDirection::Descending { ordering.reverse() } else { ordering };

    if let Some(comparer) = column.custom_sort_comparer() {
        let items: Vec<Option<BoxedValue>> = source.to_vec();
        order.sort_by(|&a, &b| directed(comparer(&items[a], &items[b])));
        return Some(order);
    }

    column.binding()?;
    // The values are what the cells of the column show: one cell, not in the tree, reads them.
    let reader = TableViewCell::new();
    reader.set_column(Some(column.clone()));
    let keys: Vec<Option<BoxedValue>> = (0..count)
        .map(|index| {
            reader.set_data_context(source.get_at(index));
            reader.content()
        })
        .collect();
    reader.set_column(None);
    order.sort_by(|&a, &b| directed(compare_values(&keys[a], &keys[b])));
    Some(order)
}

/// The items of a source in a sorted order. Follows the changes of the source: its order is made
/// again, and its listeners get a reset.
pub(crate) struct SortedItems {
    source: ItemsSource,
    column: Ref<TableViewColumn>,
    direction: SortDirection,
    order: RefCell<Vec<usize>>,
    handlers: RefCell<Vec<(u64, Rc<ItemsChangedHandler>)>>,
    next_token: Cell<u64>,
    source_token: Cell<Option<u64>>,
}

impl SortedItems {
    /// The view of `source` sorted by `column`, or `None` when the column cannot be sorted.
    pub(crate) fn new(
        source: ItemsSource,
        column: Ref<TableViewColumn>,
        direction: SortDirection,
    ) -> Option<Rc<Self>> {
        let order = sorted_order(&source, &column, direction)?;
        let items = Rc::new(Self {
            source,
            column,
            direction,
            order: RefCell::new(order),
            handlers: RefCell::new(Vec::new()),
            next_token: Cell::new(1),
            source_token: Cell::new(None),
        });
        let weak: Weak<Self> = Rc::downgrade(&items);
        let handler: Rc<ItemsChangedHandler> = Rc::new(move |_: &ItemsChangedEventArgs<'_>| {
            if let Some(items) = weak.upgrade() {
                items.source_changed();
            }
        });
        items.source_token.set(items.source.list().add_collection_changed(handler));
        Some(items)
    }

    fn source_changed(&self) {
        let order = sorted_order(&self.source, &self.column, self.direction)
            .unwrap_or_else(|| (0..self.source.count()).collect());
        *self.order.borrow_mut() = order;
        let handlers: Vec<Rc<ItemsChangedHandler>> =
            self.handlers.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler(&ItemsChangedEventArgs::RESET);
        }
    }
}

impl Drop for SortedItems {
    fn drop(&mut self) {
        if let Some(token) = self.source_token.take() {
            self.source.list().remove_collection_changed(token);
        }
    }
}

impl IItemsList for SortedItems {
    fn count(&self) -> usize {
        self.order.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        let source_index = self.order.borrow()[index];
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
        self.handlers.borrow_mut().retain(|(existing, _)| *existing != token);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
