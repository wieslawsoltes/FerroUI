use super::read_only_selection_list_base::{IReadOnlySelectionList, ReadOnlySelectionList};
use super::selected_items::SelectedItemsUntyped;
use ferroui_base::{BoxedValue, PropertyValue};
use std::cell::OnceCell;
use std::rc::Rc;

thread_local! {
    static EMPTY_INDEXES: Rc<dyn IReadOnlySelectionList<i32>> = Rc::new(ReadOnlySelectionList::new(Vec::new()));
}

/// The arguments of the selection changed event of a selection model, with
/// untyped items.
pub trait SelectionModelSelectionChangedEventArgs {
    /// Gets the indexes of the items that were removed from the selection.
    fn deselected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>>;

    /// Gets the indexes of the items that were added to the selection.
    fn selected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>>;

    /// Gets the items that were removed from the selection.
    fn deselected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>>;

    /// Gets the items that were added to the selection.
    fn selected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>>;
}

/// The arguments of the selection changed event of a selection model of
/// items of type `T`.
pub struct SelectionModelSelectionChangedEventArgsOf<T: PropertyValue> {
    untyped_deselected_items: OnceCell<Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>>>,
    untyped_selected_items: OnceCell<Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>>>,
    deselected_indexes: Rc<dyn IReadOnlySelectionList<i32>>,
    selected_indexes: Rc<dyn IReadOnlySelectionList<i32>>,
    deselected_items: Rc<dyn IReadOnlySelectionList<Option<T>>>,
    selected_items: Rc<dyn IReadOnlySelectionList<Option<T>>>,
}

impl<T: PropertyValue> SelectionModelSelectionChangedEventArgsOf<T> {
    pub fn new(
        deselected_indices: Option<Rc<dyn IReadOnlySelectionList<i32>>>,
        selected_indices: Option<Rc<dyn IReadOnlySelectionList<i32>>>,
        deselected_items: Option<Rc<dyn IReadOnlySelectionList<Option<T>>>>,
        selected_items: Option<Rc<dyn IReadOnlySelectionList<Option<T>>>>,
    ) -> Self {
        Self {
            untyped_deselected_items: OnceCell::new(),
            untyped_selected_items: OnceCell::new(),
            deselected_indexes: deselected_indices.unwrap_or_else(|| EMPTY_INDEXES.with(Rc::clone)),
            selected_indexes: selected_indices.unwrap_or_else(|| EMPTY_INDEXES.with(Rc::clone)),
            deselected_items: deselected_items.unwrap_or_else(|| Rc::new(ReadOnlySelectionList::new(Vec::new()))),
            selected_items: selected_items.unwrap_or_else(|| Rc::new(ReadOnlySelectionList::new(Vec::new()))),
        }
    }

    /// Gets the indexes of the items that were removed from the selection.
    pub fn deselected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>> {
        self.deselected_indexes.clone()
    }

    /// Gets the indexes of the items that were added to the selection.
    pub fn selected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>> {
        self.selected_indexes.clone()
    }

    /// Gets the items that were removed from the selection.
    pub fn deselected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<T>>> {
        self.deselected_items.clone()
    }

    /// Gets the items that were added to the selection.
    pub fn selected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<T>>> {
        self.selected_items.clone()
    }
}

impl<T: PropertyValue> SelectionModelSelectionChangedEventArgs for SelectionModelSelectionChangedEventArgsOf<T> {
    fn deselected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>> {
        self.deselected_indexes.clone()
    }

    fn selected_indexes(&self) -> Rc<dyn IReadOnlySelectionList<i32>> {
        self.selected_indexes.clone()
    }

    fn deselected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>> {
        self.untyped_deselected_items
            .get_or_init(|| Rc::new(SelectedItemsUntyped::new(self.deselected_items.clone())))
            .clone()
    }

    fn selected_items(&self) -> Rc<dyn IReadOnlySelectionList<Option<BoxedValue>>> {
        self.untyped_selected_items
            .get_or_init(|| Rc::new(SelectedItemsUntyped::new(self.selected_items.clone())))
            .clone()
    }
}
