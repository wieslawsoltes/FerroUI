//! `Selection/ITreeDataGridCellSelectionModel.cs`, `Selection/TreeDataGridCellSelectionModel.cs` and
//! `Selection/TreeDataGridCellSelectionChangedEventArgs.cs`: the selection of cells of a source, the
//! product of a selection of columns and a selection of rows.

use super::{
    ITreeDataGridSelection, ITreeSelectionModel, TreeDataGridRowSelectionModel, TreeSelectionModelBase,
    TreeSelectionModelHooks, UntypedModels,
};
use crate::accessor::TreeModel;
use crate::i_tree_data_grid_source::ITreeDataGridSourceOf;
use crate::model_collection::{ModelCollection, Models, ModelsChangedHandler};
use crate::models::{ColumnRef, IColumn};
use crate::{CellIndex, IndexPath};
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::utilities::HandlerList;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The arguments of the change of a cell selection (`TreeDataGridCellSelectionChangedEventArgs`).
#[derive(Clone, Copy, Debug, Default)]
pub struct TreeDataGridCellSelectionChangedEventArgs;

/// A column as an item of the selection of columns (`TreeSelectionModelBase<IColumn>`): compared
/// by identity.
pub struct ColumnSelectionItem(pub Rc<dyn IColumn>);

impl PartialEq for ColumnSelectionItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

/// The columns of a source as a collection of [`ColumnSelectionItem`]s that follows the changes of
/// the column list.
struct ColumnItems<M: TreeModel> {
    source: Weak<dyn ITreeDataGridSourceOf<M>>,
    items: RefCell<Vec<(usize, Rc<ColumnSelectionItem>)>>,
    handlers: RefCell<Vec<(u64, Rc<ModelsChangedHandler<ColumnSelectionItem>>)>>,
    next_token: Cell<u64>,
    list_token: Cell<Option<u64>>,
}

fn column_key<M: 'static>(column: &ColumnRef<M>) -> usize {
    Rc::as_ptr(column) as *const () as usize
}

impl<M: TreeModel> ColumnItems<M> {
    fn new(source: &Rc<dyn ITreeDataGridSourceOf<M>>) -> Rc<Self> {
        let this = Rc::new(Self {
            source: Rc::downgrade(source),
            items: RefCell::new(Vec::new()),
            handlers: RefCell::new(Vec::new()),
            next_token: Cell::new(1),
            list_token: Cell::new(None),
        });
        this.sync();
        let weak = Rc::downgrade(&this);
        let token = source.columns().list().add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, ColumnRef<M>>| {
            if let Some(this) = weak.upgrade() {
                this.on_columns_changed(e);
            }
        }));
        this.list_token.set(Some(token));
        this
    }

    fn item_of(&self, column: &ColumnRef<M>) -> Rc<ColumnSelectionItem> {
        let key = column_key(column);
        if let Some((_, item)) = self.items.borrow().iter().find(|(k, _)| *k == key) {
            return item.clone();
        }
        Rc::new(ColumnSelectionItem(column.clone() as Rc<dyn IColumn>))
    }

    fn sync(&self) {
        let Some(source) = self.source.upgrade() else {
            self.items.borrow_mut().clear();
            return;
        };
        let columns = source.columns().to_vec();
        let items: Vec<(usize, Rc<ColumnSelectionItem>)> = columns.iter().map(|c| (column_key(c), self.item_of(c))).collect();
        *self.items.borrow_mut() = items;
    }

    fn on_columns_changed(&self, e: &NotifyCollectionChangedEventArgs<'_, ColumnRef<M>>) {
        let old_items: Vec<Rc<ColumnSelectionItem>> = e.old_items.iter().map(|c| self.item_of(c)).collect();
        self.sync();
        let new_items: Vec<Rc<ColumnSelectionItem>> = e.new_items.iter().map(|c| self.item_of(c)).collect();
        let args = NotifyCollectionChangedEventArgs {
            action: e.action,
            new_items: &new_items,
            old_items: &old_items,
            new_starting_index: e.new_starting_index,
            old_starting_index: e.old_starting_index,
        };
        let handlers: Vec<Rc<ModelsChangedHandler<ColumnSelectionItem>>> = self.handlers.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler(&args);
        }
    }

    fn detach(&self) {
        if let (Some(token), Some(source)) = (self.list_token.take(), self.source.upgrade()) {
            source.columns().list().remove_collection_changed(token);
        }
    }
}

impl<M: TreeModel> ModelCollection<ColumnSelectionItem> for ColumnItems<M> {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get(&self, index: usize) -> Rc<ColumnSelectionItem> {
        self.items.borrow()[index].1.clone()
    }

    fn add_collection_changed(&self, handler: Rc<ModelsChangedHandler<ColumnSelectionItem>>) -> Option<u64> {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.handlers.borrow_mut().push((token, handler));
        Some(token)
    }

    fn remove_collection_changed(&self, token: u64) {
        self.handlers.borrow_mut().retain(|(t, _)| *t != token);
    }

    fn is_notifying(&self) -> bool {
        true
    }
}

/// The selection of columns of a cell selection (`ColumnSelection`).
struct ColumnSelection {
    base: Rc<TreeSelectionModelBase<ColumnSelectionItem>>,
    changed_pending: Cell<bool>,
    state_changed: HandlerList<dyn Fn()>,
}

impl ColumnSelection {
    fn new(source: Models<ColumnSelectionItem>) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self: &Weak<Self>| {
            let base = TreeSelectionModelBase::with_source(Some(source));
            base.set_hooks(weak_self.clone() as Weak<dyn TreeSelectionModelHooks<ColumnSelectionItem>>);
            Self { base, changed_pending: Cell::new(false), state_changed: HandlerList::new() }
        });
        let weak = Rc::downgrade(&this);
        let changed = move || {
            if let Some(this) = weak.upgrade() {
                this.changed();
            }
        };
        let c = changed.clone();
        this.base.add_selection_changed(Rc::new(move |_| c()));
        let c = changed.clone();
        this.base.add_indexes_changed(Rc::new(move |_| c()));
        this.base.add_source_reset(Rc::new(move |_| changed()));
        this
    }

    fn changed(&self) {
        if self.base.is_source_collection_changing() {
            self.changed_pending.set(true);
        } else {
            self.raise();
        }
    }

    fn raise(&self) {
        for (_, handler) in self.state_changed.snapshot().iter() {
            handler();
        }
    }

    fn detach(&self) {
        self.base.clear();
        self.base.set_source(None);
    }
}

impl TreeSelectionModelHooks<ColumnSelectionItem> for ColumnSelection {
    fn get_children(&self, _model: &TreeSelectionModelBase<ColumnSelectionItem>, _node: &Rc<ColumnSelectionItem>) -> Option<Models<ColumnSelectionItem>> {
        None
    }

    fn on_source_collection_change_finished(&self, _model: &TreeSelectionModelBase<ColumnSelectionItem>) {
        if !self.changed_pending.replace(false) {
            return;
        }
        self.raise();
    }
}

/// The selection of cells of a source (`TreeDataGridCellSelectionModel<TModel>`).
pub struct TreeDataGridCellSelectionModel<M: TreeModel> {
    source: Weak<dyn ITreeDataGridSourceOf<M>>,
    column_items: Rc<ColumnItems<M>>,
    columns: Rc<ColumnSelection>,
    rows: Rc<TreeDataGridRowSelectionModel<M>>,
    batch_depth: Cell<i32>,
    changed: Cell<bool>,
    disposed: Cell<bool>,
    selection_changed: HandlerList<dyn Fn(&TreeDataGridCellSelectionChangedEventArgs)>,
}

impl<M: TreeModel> TreeDataGridCellSelectionModel<M> {
    /// The selection of cells of `source`.
    pub fn new(source: &Rc<dyn ITreeDataGridSourceOf<M>>) -> Rc<Self> {
        let column_items = ColumnItems::new(source);
        let columns = ColumnSelection::new(column_items.clone() as Models<ColumnSelectionItem>);
        let rows = TreeDataGridRowSelectionModel::new(source);
        let this = Rc::new(Self {
            source: Rc::downgrade(source),
            column_items,
            columns,
            rows,
            batch_depth: Cell::new(0),
            changed: Cell::new(false),
            disposed: Cell::new(false),
            selection_changed: HandlerList::new(),
        });
        let weak = Rc::downgrade(&this);
        let on_state = move || {
            if let Some(this) = weak.upgrade() {
                this.on_state_changed();
            }
        };
        let s = on_state.clone();
        this.columns.state_changed.add(Rc::new(s));
        this.rows.add_state_changed(Rc::new(on_state));
        this
    }

    fn source(&self) -> Rc<dyn ITreeDataGridSourceOf<M>> {
        self.source.upgrade().expect("the source of the selection")
    }

    /// The selection of rows of the cell selection (`RowSelection`).
    pub fn row_selection(&self) -> &Rc<TreeDataGridRowSelectionModel<M>> {
        &self.rows
    }

    /// The number of selected cells.
    pub fn count(&self) -> i32 {
        self.columns.base.count() * self.rows.base().count()
    }

    pub fn single_select(&self) -> bool {
        self.rows.base().single_select()
    }

    pub fn set_single_select(&self, value: bool) {
        self.begin_update();
        self.columns.base.set_single_select(value);
        self.rows.base().set_single_select(value);
        self.end_update();
    }

    pub fn selected_index(&self) -> CellIndex {
        CellIndex::new(column_index(&self.columns.base.selected_index()), self.rows.base().selected_index())
    }

    pub fn set_selected_index(&self, value: CellIndex) {
        self.set_selected_range(&value, 1, 1);
    }

    pub fn anchor_index(&self) -> CellIndex {
        CellIndex::new(column_index(&self.columns.base.anchor_index()), self.rows.base().anchor_index())
    }

    pub fn range_anchor_index(&self) -> CellIndex {
        CellIndex::new(column_index(&self.columns.base.range_anchor_index()), self.rows.base().range_anchor_index())
    }

    /// The selected cells, row by row (`SelectedIndexes`).
    pub fn selected_indexes(&self) -> Vec<CellIndex> {
        let columns = self.columns.base.selected_indexes();
        let rows = self.rows.base().selected_indexes();
        let column_count = super::IReadOnlyList::count(&*columns);
        let count = self.count().max(0) as usize;
        (0..count)
            .map(|index| {
                CellIndex::new(
                    column_index(&super::IReadOnlyList::get(&*columns, index % column_count)),
                    super::IReadOnlyList::get(&*rows, index / column_count),
                )
            })
            .collect()
    }

    /// The selected cell at `index`.
    ///
    /// # Panics
    /// When `index` is out of range (`ArgumentOutOfRangeException`).
    pub fn selected_index_at(&self, index: usize) -> CellIndex {
        if index >= self.count().max(0) as usize {
            panic!("Specified argument was out of the range of valid values. (Parameter 'index')");
        }
        self.selected_indexes()[index].clone()
    }

    pub fn add_selection_changed(&self, handler: Rc<dyn Fn(&TreeDataGridCellSelectionChangedEventArgs)>) -> u64 {
        self.selection_changed.add(handler)
    }

    pub fn remove_selection_changed(&self, token: u64) {
        self.selection_changed.remove(token);
    }

    pub fn is_selected(&self, index: &CellIndex) -> bool {
        self.is_selected_at(index.column_index, &index.row_index)
    }

    pub fn is_selected_at(&self, column_index: i32, row_index: &IndexPath) -> bool {
        column_index >= 0 && self.columns.base.is_selected(&IndexPath::new(column_index)) && self.rows.base().is_selected(row_index)
    }

    pub fn clear(&self) {
        self.begin_update();
        self.columns.base.clear();
        self.rows.base().clear();
        self.end_update();
    }

    /// Selects `column_count` columns and `row_count` rows from `start` (negative counts go back).
    pub fn set_selected_range(&self, start: &CellIndex, column_count: i32, row_count: i32) {
        let source = self.source();
        let rows = source.rows();
        let row = rows.model_index_to_row_index(&start.row_index);
        let columns_count = source.columns().count() as i32;
        if column_count == 0 || row_count == 0 || start.column_index < 0 || start.column_index >= columns_count || row < 0 {
            self.clear();
            return;
        }
        let single = self.single_select();
        let end_column = end(start.column_index, if single { 1 } else { column_count }, columns_count);
        let end_row = end(row, if single { 1 } else { row_count }, rows.count() as i32);
        self.begin_update();
        self.columns.base.set_selected_index(IndexPath::new(start.column_index));
        self.rows.base().set_selected_index(start.row_index.clone());
        for i in start.column_index.min(end_column)..=start.column_index.max(end_column) {
            self.columns.base.select(&IndexPath::new(i));
        }
        for i in row.min(end_row)..=row.max(end_row) {
            self.rows.base().select(&rows.row_index_to_model_index(i));
        }
        self.columns.base.set_anchor_index(IndexPath::new(start.column_index));
        self.rows.base().set_anchor_index(start.row_index.clone());
        self.columns.base.set_range_anchor_index(IndexPath::new(end_column));
        self.rows.base().set_range_anchor_index(rows.row_index_to_model_index(end_row));
        self.end_update();
    }

    /// Detaches the selection from its source (`Dispose`).
    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.columns.detach();
        self.column_items.detach();
        self.rows.base().clear();
        ITreeDataGridSelection::set_source(&*self.rows, None);
    }

    fn begin_update(&self) {
        if self.disposed.get() {
            panic!("Cannot access a disposed object. Object name: 'TreeDataGridCellSelectionModel`1'.");
        }
        self.batch_depth.set(self.batch_depth.get() + 1);
        self.columns.base.begin_batch_update();
        self.rows.base().begin_batch_update();
    }

    fn end_update(&self) {
        self.columns.base.end_batch_update();
        self.rows.base().end_batch_update();
        self.batch_depth.set(self.batch_depth.get() - 1);
        if self.batch_depth.get() == 0 && self.changed.get() {
            self.publish_change();
        }
    }

    fn on_state_changed(&self) {
        self.changed.set(true);
        if self.batch_depth.get() == 0 {
            self.publish_change();
        }
    }

    fn publish_change(&self) {
        self.changed.set(false);
        let e = TreeDataGridCellSelectionChangedEventArgs;
        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler(&e);
        }
    }
}

fn end(start: i32, count: i32, size: i32) -> i32 {
    (start as i64 + count as i64 - count.signum() as i64).clamp(0, (size - 1).max(0) as i64) as i32
}

fn column_index(index: &IndexPath) -> i32 {
    if index.count() == 1 {
        index.get(0)
    } else {
        -1
    }
}

impl<M: TreeModel> ITreeDataGridSelection for TreeDataGridCellSelectionModel<M> {
    fn source(&self) -> Option<UntypedModels> {
        ITreeDataGridSelection::source(&*self.rows)
    }

    fn set_source(&self, value: Option<UntypedModels>) {
        self.begin_update();
        ITreeDataGridSelection::set_source(&*self.rows, value);
        self.end_update();
    }

    fn row_selection_model(&self) -> Option<Rc<dyn ITreeSelectionModel>> {
        Some(self.rows.base().clone() as Rc<dyn ITreeSelectionModel>)
    }
}

