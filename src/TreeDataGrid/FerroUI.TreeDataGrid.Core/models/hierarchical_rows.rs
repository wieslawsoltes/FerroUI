//! `Models/HierarchicalRows.cs`: the rows of a hierarchical source, flattened: each expanded row
//! followed by the rows of its children.

use super::sortable_rows_base::{SortableRowsBase, SortableRowsChangedArgs};
use super::{
    Comparison, HierarchicalRow, IExpanderColumn, IExpanderRow, IExpanderRowController, IRows, RowRef, RowsChangedArgs,
    RowsChangedEvent, RowsChangedHandler,
};
use crate::accessor::TreeModel;
use crate::model_collection::Models;
use crate::{IndexPath, TreeDataGridItemsSourceView};
use ferroui_base::collections::NotifyCollectionChangedAction;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

type RootRows<M> = SortableRowsBase<M, Rc<HierarchicalRow<M>>>;

/// The typed row of an untyped row of a hierarchical source.
pub fn as_hierarchical_row<M: TreeModel>(row: &RowRef) -> Option<Rc<HierarchicalRow<M>>> {
    let any: Rc<dyn Any> = row.clone();
    any.downcast::<HierarchicalRow<M>>().ok()
}

/// The flattened rows of a hierarchical source.
pub struct HierarchicalRows<M: TreeModel> {
    weak_self: Weak<Self>,
    controller: Weak<dyn IExpanderRowController<M>>,
    roots: RefCell<Option<Rc<RootRows<M>>>>,
    expander_column: Rc<dyn IExpanderColumn<M>>,
    // The rows shown, depth first. The list is shared with the arguments of the changes it raises
    // (a snapshot: a clone of the `Rc`), so that the rows of a change are not copied for it; a
    // change while a snapshot is held copies the list first (`Rc::make_mut`).
    flattened_rows: RefCell<Rc<Vec<RowRef>>>,
    // The indent of each flattened row (a row keeps its indent), read by the scans of the
    // descendants of a row without reaching the rows.
    flattened_indents: RefCell<Vec<i32>>,
    comparison: RefCell<Option<Comparison<M>>>,
    ignore_collection_changes: Cell<bool>,
    changed: RowsChangedEvent,
}

impl<M: TreeModel> HierarchicalRows<M> {
    pub fn new(
        controller: Weak<dyn IExpanderRowController<M>>,
        items: Rc<TreeDataGridItemsSourceView<M>>,
        expander_column: Rc<dyn IExpanderColumn<M>>,
        comparison: Option<Comparison<M>>,
    ) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self: &Weak<Self>| Self {
            weak_self: weak_self.clone(),
            controller,
            roots: RefCell::new(None),
            expander_column,
            flattened_rows: RefCell::new(Rc::new(Vec::new())),
            flattened_indents: RefCell::new(Vec::new()),
            comparison: RefCell::new(comparison.clone()),
            ignore_collection_changes: Cell::new(false),
            changed: RowsChangedEvent::default(),
        });
        let owner = Rc::downgrade(&this);
        let roots = RootRows::new(items, comparison, move |model_index, model| {
            let owner = owner.upgrade().expect("the rows of the roots");
            let comparison = owner.comparison.borrow().clone();
            let controller: Weak<dyn IExpanderRowController<M>> = owner.weak_self.clone();
            HierarchicalRow::new(
                controller,
                owner.expander_column.clone(),
                IndexPath::new(model_index),
                model,
                comparison,
            )
        });
        let owner = Rc::downgrade(&this);
        roots.add_collection_changed(Rc::new(move |e: &SortableRowsChangedArgs<'_, Rc<HierarchicalRow<M>>>| {
            if let Some(owner) = owner.upgrade() {
                let new_items: Vec<RowRef> = e.new_items.iter().map(|r| r.clone() as RowRef).collect();
                let old_items: Vec<RowRef> = e.old_items.iter().map(|r| r.clone() as RowRef).collect();
                owner.on_collection_changed(
                    &IndexPath::UNSELECTED,
                    &RowsChangedArgs {
                        action: e.action,
                        new_items: &new_items,
                        old_items: &old_items,
                        new_starting_index: e.new_starting_index,
                        old_starting_index: e.old_starting_index,
                    },
                );
            }
        }));
        *this.roots.borrow_mut() = Some(roots);
        this.initialize_rows();
        this
    }

    fn roots(&self) -> Rc<RootRows<M>> {
        self.roots.borrow().clone().expect("the root rows")
    }

    /// The row at `index`.
    pub fn get_typed(&self, index: usize) -> Rc<HierarchicalRow<M>> {
        Self::typed(&self.flattened_rows.borrow()[index]).this()
    }

    /// The rows.
    pub fn to_vec_typed(&self) -> Vec<Rc<HierarchicalRow<M>>> {
        self.flattened_rows.borrow().iter().map(|row| Self::typed(row).this()).collect()
    }

    /// A flattened row as the hierarchical row it is.
    #[inline]
    fn typed(row: &RowRef) -> &HierarchicalRow<M> {
        row.as_any().downcast_ref::<HierarchicalRow<M>>().expect("a hierarchical row")
    }

    /// The flattened rows, to change (copied first while a snapshot of them is held).
    fn rows_mut(&self) -> std::cell::RefMut<'_, Vec<RowRef>> {
        std::cell::RefMut::map(self.flattened_rows.borrow_mut(), Rc::make_mut)
    }

    /// The root rows, in sorted order.
    pub fn root_rows(&self) -> Vec<Rc<HierarchicalRow<M>>> {
        self.roots().to_vec()
    }

    /// Releases the rows (`Dispose`).
    pub fn dispose(&self) {
        self.ignore_collection_changes.set(true);
        self.roots().dispose();
    }

    /// Expands the rows on the way to the model at `index` (`Expand`).
    pub fn expand(&self, index: &IndexPath) {
        let mut rows: Option<Rc<RootRows<M>>> = Some(self.roots());
        for i in 0..index.count() {
            let Some(current) = rows.take() else {
                break;
            };
            match Self::find_row(&current, index.get(i)) {
                Some(row) => {
                    row.set_is_expanded(true);
                    rows = row.expanded_child_rows();
                }
                None => break,
            }
        }
    }

    /// The row of `rows` (in their order) whose model index is `model_index`; the rows are read
    /// through their list rather than copied.
    fn find_row(rows: &RootRows<M>, model_index: i32) -> Option<Rc<HierarchicalRow<M>>> {
        (0..rows.count()).map(|i| rows.get(i)).find(|row| row.model_index() == model_index)
    }

    /// Expands or collapses the rows below `row` (all rows: `None`) by `predicate`, with one reset
    /// notification (`ExpandCollapseRecursive`).
    pub fn expand_collapse_recursive(&self, predicate: &dyn Fn(&Rc<M>) -> bool, row: Option<&Rc<HierarchicalRow<M>>>) {
        self.ignore_collection_changes.set(true);
        if let Some(row) = row {
            row.set_is_expanded(predicate(row.model()));
        }
        let children = match row {
            None => Some(self.roots().to_vec()),
            Some(row) => row.children(),
        };
        if let Some(children) = children {
            Self::expand_collapse_recursive_core(&children, predicate);
        }
        self.ignore_collection_changes.set(false);
        self.rows_mut().clear();
        self.flattened_indents.borrow_mut().clear();
        self.initialize_rows();
        self.raise_reset();
    }

    /// Collapses the row of the model at `index` (`Collapse`).
    pub fn collapse(&self, index: &IndexPath) {
        let count = index.count();
        let mut rows: Option<Rc<RootRows<M>>> = Some(self.roots());
        for i in 0..count {
            let Some(current) = rows.take() else {
                break;
            };
            match Self::find_row(&current, index.get(i)) {
                Some(row) => {
                    if i == count - 1 {
                        row.set_is_expanded(false);
                    }
                    rows = row.expanded_child_rows();
                }
                None => break,
            }
        }
    }

    /// The model indexes of the expanded rows, those of collapsed parents included.
    pub fn get_expanded_model_indexes(&self) -> Vec<IndexPath> {
        fn add_expanded<M: TreeModel>(rows: &[Rc<HierarchicalRow<M>>], result: &mut Vec<IndexPath>) {
            for row in rows {
                if row.is_expanded() {
                    result.push(row.model_index_path());
                }
                if let Some(children) = row.materialized_children() {
                    add_expanded(&children, result);
                }
            }
        }
        let mut result = Vec::new();
        add_expanded(&self.roots().to_vec(), &mut result);
        result
    }

    /// Shows the models of another view, with one reset notification.
    pub fn set_items(&self, items: Rc<TreeDataGridItemsSourceView<M>>) {
        self.ignore_collection_changes.set(true);
        self.roots().set_items(items);
        self.ignore_collection_changes.set(false);
        self.rows_mut().clear();
        self.flattened_indents.borrow_mut().clear();
        self.initialize_rows();
        self.raise_reset();
    }

    /// Sorts the rows.
    pub fn sort(&self, comparison: Option<Comparison<M>>) {
        *self.comparison.borrow_mut() = comparison.clone();
        self.roots().sort(comparison.clone());
        self.rows_mut().clear();
        self.flattened_indents.borrow_mut().clear();
        self.initialize_rows();
        self.raise_reset();
        for row in self.roots().to_vec() {
            row.sort_children(comparison.clone());
        }
    }

    /// The row index of the parent of the model at `model_index`.
    pub fn get_parent_row_index(&self, model_index: &IndexPath) -> i32 {
        self.model_index_to_row_index_core(&model_index.parent())
    }

    fn model_index_to_row_index_core(&self, model_index: &IndexPath) -> i32 {
        if *model_index == IndexPath::UNSELECTED {
            return -1;
        }
        self.flattened_rows
            .borrow()
            .iter()
            .position(|row| Self::typed(row).has_model_index_path(model_index))
            .map_or(-1, |i| i as i32)
    }

    /// The row index of the model at `model_index`, searched from `from_row_index`; `Some(-1)` for
    /// the empty path (the roots), `None` when the model has no row.
    pub fn try_get_row_index(&self, model_index: &IndexPath, from_row_index: usize) -> Option<i32> {
        if model_index.count() == 0 {
            return Some(-1);
        }
        let rows = self.flattened_rows.borrow();
        (from_row_index..rows.len()).find(|&i| Self::typed(&rows[i]).has_model_index_path(model_index)).map(|i| i as i32)
    }

    fn initialize_rows(&self) {
        let mut rows = Vec::new();
        Self::collect_child_rows(&self.roots(), &mut rows);
        self.flattened_indents.borrow_mut().splice(0..0, rows.iter().map(|row| row.indent()));
        self.rows_mut().splice(0..0, rows.into_iter().map(|row| row as RowRef));
    }

    /// The rows `AddRowsAndDescendants` inserts for `row`: the row and the rows of its expanded
    /// descendants, depth first. The reference inserts them into the flattened list one by one; the
    /// port collects them and inserts them at once (`insert_rows`), which moves the rows after them
    /// once.
    fn collect_rows_and_descendants(row: Rc<HierarchicalRow<M>>, rows: &mut Vec<Rc<HierarchicalRow<M>>>) {
        let children = row.expanded_child_rows();
        rows.push(row);
        if let Some(children) = children {
            Self::collect_child_rows(&children, rows);
        }
    }

    /// The rows of `children` and of their expanded descendants, depth first.
    fn collect_child_rows(children: &RootRows<M>, rows: &mut Vec<Rc<HierarchicalRow<M>>>) {
        let children = children.to_vec();
        rows.reserve(children.len());
        for row in children {
            Self::collect_rows_and_descendants(row, rows);
        }
    }

    fn expand_collapse_recursive_core(rows: &[Rc<HierarchicalRow<M>>], predicate: &dyn Fn(&Rc<M>) -> bool) {
        for row in rows {
            if predicate(row.model()) {
                row.set_is_expanded(true);
                if let Some(children) = row.children() {
                    Self::expand_collapse_recursive_core(&children, predicate);
                }
            } else {
                if let Some(children) = row.children() {
                    Self::expand_collapse_recursive_core(&children, predicate);
                }
                row.set_is_expanded(false);
            }
        }
    }

    fn raise_reset(&self) {
        self.changed.raise(&RowsChangedArgs {
            action: NotifyCollectionChangedAction::Reset,
            new_items: &[],
            old_items: &[],
            new_starting_index: -1,
            old_starting_index: -1,
        });
    }

    fn add(&self, index: usize, items: Vec<Rc<HierarchicalRow<M>>>) {
        let mut rows = Vec::with_capacity(items.len());
        for row in items {
            Self::collect_rows_and_descendants(row, &mut rows);
        }
        self.insert_rows(index, rows);
    }

    /// Inserts `rows` at `index` of the flattened rows and raises the change.
    fn insert_rows(&self, index: usize, rows: Vec<Rc<HierarchicalRow<M>>>) {
        if rows.is_empty() {
            return;
        }
        let count = rows.len();
        self.flattened_indents.borrow_mut().splice(index..index, rows.iter().map(|row| row.indent()));
        self.rows_mut().splice(index..index, rows.into_iter().map(|row| row as RowRef));
        if self.changed.has_handlers() {
            let snapshot = self.flattened_rows.borrow().clone();
            self.changed.raise(&RowsChangedArgs {
                action: NotifyCollectionChangedAction::Add,
                new_items: &snapshot[index..index + count],
                old_items: &[],
                new_starting_index: index as i32,
                old_starting_index: -1,
            });
        }
    }

    fn remove(&self, index: usize, count: usize) {
        if count == 0 {
            return;
        }
        self.flattened_indents.borrow_mut().drain(index..index + count);
        let old_items: Vec<RowRef> = self.rows_mut().drain(index..index + count).collect();
        if self.changed.has_handlers() {
            self.changed.raise(&RowsChangedArgs {
                action: NotifyCollectionChangedAction::Remove,
                new_items: &[],
                old_items: &old_items,
                new_starting_index: -1,
                old_starting_index: index as i32,
            });
        }
    }

    fn advance(&self, row_index: usize, mut count: i32) -> usize {
        let mut i = row_index;
        while count > 0 {
            let row = self.flattened_rows.borrow()[i].clone();
            match Self::typed(&row).children_count() {
                Some(children) if children > 0 => i = self.advance(i + 1, children as i32),
                _ => i += 1,
            }
            count -= 1;
        }
        i
    }

    fn get_descendent_row_count(&self, row_index: i32) -> usize {
        let indents = self.flattened_indents.borrow();
        if row_index == -1 {
            return indents.len();
        }
        let depth = indents[row_index as usize];
        let mut i = row_index as usize + 1;
        while i < indents.len() && indents[i] > depth {
            i += 1;
        }
        i - (row_index as usize + 1)
    }

    fn typed_rows(items: &[RowRef]) -> Vec<Rc<HierarchicalRow<M>>> {
        items.iter().filter_map(as_hierarchical_row::<M>).collect()
    }

    fn on_collection_changed(&self, parent_index: &IndexPath, e: &RowsChangedArgs<'_>) {
        if self.ignore_collection_changes.get() {
            return;
        }
        let Some(parent_row_index) = self.try_get_row_index(parent_index, 0) else {
            return;
        };
        let first = (parent_row_index + 1) as usize;
        match e.action {
            NotifyCollectionChangedAction::Add => {
                let insert = self.advance(first, e.new_starting_index);
                self.add(insert, Self::typed_rows(e.new_items));
            }
            NotifyCollectionChangedAction::Remove => {
                let start = self.advance(first, e.old_starting_index);
                let end = self.advance(start, e.old_items.len() as i32);
                self.remove(start, end - start);
            }
            NotifyCollectionChangedAction::Replace => {
                let start = self.advance(first, e.old_starting_index);
                let end = self.advance(start, e.old_items.len() as i32);
                self.remove(start, end - start);
                self.add(start, Self::typed_rows(e.new_items));
            }
            NotifyCollectionChangedAction::Move => {
                let from_start = self.advance(first, e.old_starting_index);
                let from_end = self.advance(from_start, e.old_items.len() as i32);
                self.remove(from_start, from_end - from_start);
                let to = self.advance(first, e.new_starting_index);
                self.add(to, Self::typed_rows(e.new_items));
            }
            NotifyCollectionChangedAction::Reset => {
                let children = if parent_row_index >= 0 {
                    let parent = Self::typed(&self.flattened_rows.borrow()[parent_row_index as usize]).this();
                    parent.expanded_child_rows()
                } else {
                    Some(self.roots())
                };
                let count = self.get_descendent_row_count(parent_row_index);
                self.remove(first, count);
                if let Some(children) = children {
                    let mut rows = Vec::new();
                    Self::collect_child_rows(&children, &mut rows);
                    self.insert_rows(first, rows);
                }
            }
        }
    }
}

impl<M: TreeModel> IRows for HierarchicalRows<M> {
    fn count(&self) -> usize {
        self.flattened_rows.borrow().len()
    }

    fn get(&self, index: usize) -> RowRef {
        self.flattened_rows.borrow()[index].clone()
    }

    fn model_index_to_row_index(&self, model_index: &IndexPath) -> i32 {
        self.model_index_to_row_index_core(model_index)
    }

    fn row_index_to_model_index(&self, row_index: i32) -> IndexPath {
        let rows = self.flattened_rows.borrow();
        if row_index >= 0 && (row_index as usize) < rows.len() {
            Self::typed(&rows[row_index as usize]).model_index_path()
        } else {
            IndexPath::UNSELECTED
        }
    }

    fn add_collection_changed(&self, handler: Rc<RowsChangedHandler>) -> u64 {
        self.changed.add(handler)
    }

    fn remove_collection_changed(&self, token: u64) {
        self.changed.remove(token)
    }
}

impl<M: TreeModel> IExpanderRowController<M> for HierarchicalRows<M> {
    fn on_begin_expand_collapse(&self, row: &dyn IExpanderRow<M>) {
        if let Some(controller) = self.controller.upgrade() {
            controller.on_begin_expand_collapse(row);
        }
    }

    fn on_end_expand_collapse(&self, row: &dyn IExpanderRow<M>) {
        if let Some(controller) = self.controller.upgrade() {
            controller.on_end_expand_collapse(row);
        }
    }

    /// # Panics
    /// For a row that is not a [`HierarchicalRow`] (`NotSupportedException`).
    fn on_child_collection_changed(&self, row: &dyn IExpanderRow<M>, e: &RowsChangedArgs<'_>) {
        if self.ignore_collection_changes.get() {
            return;
        }
        match row.as_any().downcast_ref::<HierarchicalRow<M>>() {
            Some(row) => self.on_collection_changed(&row.model_index_path(), e),
            None => panic!("Unexpected row type."),
        }
    }

    fn on_child_collection_replaced(&self, row: &dyn IExpanderRow<M>, children: Option<&Models<M>>) {
        if let Some(controller) = self.controller.upgrade() {
            if controller.replaces_child_collections() {
                controller.on_child_collection_replaced(row, children);
            }
        }
    }

    fn replaces_child_collections(&self) -> bool {
        true
    }
}
