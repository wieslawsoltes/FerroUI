//! `Selection/TreeSelectionModelSelectionChangedEventArgs.cs`: the arguments of the selection
//! changed event, untyped (a trait) and typed (a struct).

use super::i_tree_selection_model::{box_model, IReadOnlyList, ReadOnlyList};
use crate::accessor::TreeModel;
use crate::IndexPath;
use ferroui_base::BoxedValue;
use std::cell::OnceCell;
use std::rc::Rc;

/// The arguments of the selection changed event of a tree selection model, with untyped items
/// (the reference's abstract base class; its protected `GetUntypedDeselectedItems` and
/// `GetUntypedSelectedItems` are the item methods of the trait).
pub trait TreeSelectionModelSelectionChangedEventArgs {
    /// Gets the indexes of the items that were removed from the selection.
    fn deselected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>>;

    /// Gets the indexes of the items that were added to the selection.
    fn selected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>>;

    /// Gets the items that were removed from the selection.
    fn deselected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>>;

    /// Gets the items that were added to the selection.
    fn selected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>>;
}

/// A typed list of models read as untyped items (the reference's private `Untyped` list).
pub struct UntypedItems<M: TreeModel> {
    source: Rc<dyn IReadOnlyList<Option<Rc<M>>>>,
}

impl<M: TreeModel> UntypedItems<M> {
    pub fn new(source: Rc<dyn IReadOnlyList<Option<Rc<M>>>>) -> Self {
        Self { source }
    }
}

impl<M: TreeModel> IReadOnlyList<Option<BoxedValue>> for UntypedItems<M> {
    fn count(&self) -> usize {
        self.source.count()
    }

    fn get(&self, index: usize) -> Option<BoxedValue> {
        box_model(&self.source.get(index))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Option<BoxedValue>> + '_> {
        Box::new(self.source.iter().map(|item| box_model(&item)))
    }
}

/// The arguments of the selection changed event of a tree selection model of models of type `M`.
pub struct TreeSelectionModelSelectionChangedEventArgsOf<M: TreeModel> {
    deselected_indexes: Rc<dyn IReadOnlyList<IndexPath>>,
    selected_indexes: Rc<dyn IReadOnlyList<IndexPath>>,
    deselected_items: Rc<dyn IReadOnlyList<Option<Rc<M>>>>,
    selected_items: Rc<dyn IReadOnlyList<Option<Rc<M>>>>,
    untyped_deselected_items: OnceCell<Rc<dyn IReadOnlyList<Option<BoxedValue>>>>,
    untyped_selected_items: OnceCell<Rc<dyn IReadOnlyList<Option<BoxedValue>>>>,
}

impl<M: TreeModel> TreeSelectionModelSelectionChangedEventArgsOf<M> {
    /// The arguments; a list that is `None` is empty.
    pub fn new(
        deselected_indexes: Option<Rc<dyn IReadOnlyList<IndexPath>>>,
        selected_indexes: Option<Rc<dyn IReadOnlyList<IndexPath>>>,
        deselected_items: Option<Rc<dyn IReadOnlyList<Option<Rc<M>>>>>,
        selected_items: Option<Rc<dyn IReadOnlyList<Option<Rc<M>>>>>,
    ) -> Self {
        Self {
            deselected_indexes: deselected_indexes.unwrap_or_else(|| Rc::new(ReadOnlyList::<IndexPath>::empty())),
            selected_indexes: selected_indexes.unwrap_or_else(|| Rc::new(ReadOnlyList::<IndexPath>::empty())),
            deselected_items: deselected_items.unwrap_or_else(|| Rc::new(ReadOnlyList::<Option<Rc<M>>>::empty())),
            selected_items: selected_items.unwrap_or_else(|| Rc::new(ReadOnlyList::<Option<Rc<M>>>::empty())),
            untyped_deselected_items: OnceCell::new(),
            untyped_selected_items: OnceCell::new(),
        }
    }

    /// The arguments of a change that only deselected items (`deselectedItems:` alone).
    pub fn with_deselected_items(deselected_items: Rc<dyn IReadOnlyList<Option<Rc<M>>>>) -> Self {
        Self::new(None, None, Some(deselected_items), None)
    }

    /// Gets the indexes of the items that were removed from the selection.
    pub fn deselected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>> {
        self.deselected_indexes.clone()
    }

    /// Gets the indexes of the items that were added to the selection.
    pub fn selected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>> {
        self.selected_indexes.clone()
    }

    /// Gets the items that were removed from the selection.
    pub fn deselected_items(&self) -> Rc<dyn IReadOnlyList<Option<Rc<M>>>> {
        self.deselected_items.clone()
    }

    /// Gets the items that were added to the selection.
    pub fn selected_items(&self) -> Rc<dyn IReadOnlyList<Option<Rc<M>>>> {
        self.selected_items.clone()
    }

    /// The removed items, untyped.
    pub fn get_untyped_deselected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>> {
        self.untyped_deselected_items
            .get_or_init(|| Rc::new(UntypedItems::new(self.deselected_items.clone())))
            .clone()
    }

    /// The added items, untyped.
    pub fn get_untyped_selected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>> {
        self.untyped_selected_items.get_or_init(|| Rc::new(UntypedItems::new(self.selected_items.clone()))).clone()
    }
}

impl<M: TreeModel> TreeSelectionModelSelectionChangedEventArgs for TreeSelectionModelSelectionChangedEventArgsOf<M> {
    fn deselected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>> {
        self.deselected_indexes.clone()
    }

    fn selected_indexes(&self) -> Rc<dyn IReadOnlyList<IndexPath>> {
        self.selected_indexes.clone()
    }

    fn deselected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>> {
        self.get_untyped_deselected_items()
    }

    fn selected_items(&self) -> Rc<dyn IReadOnlyList<Option<BoxedValue>>> {
        self.get_untyped_selected_items()
    }
}
