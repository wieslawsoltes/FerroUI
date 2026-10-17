//! `FlatTreeDataGridSource.cs`: a source that shows its models as a flat grid.

use crate::accessor::TreeModel;
use crate::i_tree_data_grid_source::{ITreeDataGridSource, ITreeDataGridSourceOf, RowDropPosition, RowMoveEffects};
use crate::model_collection::Models;
use crate::models::{AnonymousSortableRows, ColumnList, ColumnRef, Comparison, IColumn, IRows, ListSortDirection, NotifyingBase};
use crate::selection::{
    downcast_models, erase_models, downcast_selection, ITreeDataGridSelection, TreeDataGridRowSelectionModel, UntypedModels,
};
use crate::{IndexPath, TreeDataGridItemsSourceView};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) fn same_models<M: 'static>(a: &Models<M>, b: &Models<M>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// A source of a flat grid: one row per model.
pub struct FlatTreeDataGridSource<M: TreeModel> {
    weak_self: Weak<Self>,
    notifying: NotifyingBase,
    items: RefCell<Models<M>>,
    items_view: RefCell<Rc<TreeDataGridItemsSourceView<M>>>,
    rows: RefCell<Option<Rc<AnonymousSortableRows<M>>>>,
    comparer: RefCell<Option<Comparison<M>>>,
    selection: RefCell<Option<Rc<dyn ITreeDataGridSelection>>>,
    is_selection_set: Cell<bool>,
    columns: ColumnList<M>,
    sorted: HandlerList<dyn Fn()>,
    // The filter of the classic source of the reference (`Filter`, `RefreshFilter`): rows are the
    // models that pass it.
    filter: RefCell<Option<Rc<dyn Fn(&Rc<M>) -> bool>>>,
    filtered: RefCell<Option<Models<M>>>,
}

impl<M: TreeModel> FlatTreeDataGridSource<M> {
    /// A source of `items`.
    pub fn new(items: Models<M>) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            notifying: NotifyingBase::new(),
            items_view: RefCell::new(TreeDataGridItemsSourceView::get_or_create(Some(items.clone()))),
            items: RefCell::new(items),
            rows: RefCell::new(None),
            comparer: RefCell::new(None),
            selection: RefCell::new(None),
            is_selection_set: Cell::new(false),
            columns: ColumnList::new(),
            sorted: HandlerList::new(),
            filter: RefCell::new(None),
            filtered: RefCell::new(None),
        })
    }

    fn this(&self) -> Rc<Self> {
        self.weak_self.upgrade().expect("a source that is alive")
    }

    /// The columns.
    pub fn columns(&self) -> &ColumnList<M> {
        &self.columns
    }

    /// Adds a column (the reference's collection initializer `Columns = { ... }`).
    pub fn with_column(self: &Rc<Self>, column: ColumnRef<M>) -> Rc<Self> {
        self.columns.add(column);
        self.clone()
    }

    /// The rows, created on first access.
    pub fn typed_rows(&self) -> Rc<AnonymousSortableRows<M>> {
        if let Some(rows) = self.rows.borrow().as_ref() {
            return rows.clone();
        }
        let rows = AnonymousSortableRows::new(self.items_view.borrow().clone(), self.comparer.borrow().clone());
        *self.rows.borrow_mut() = Some(rows.clone());
        rows
    }

    /// The items.
    pub fn items(&self) -> Models<M> {
        self.items.borrow().clone()
    }

    /// Shows other items (the selection follows them).
    pub fn set_items(&self, value: Models<M>) {
        if same_models(&self.items.borrow(), &value) {
            return;
        }
        *self.items.borrow_mut() = value.clone();
        self.refresh_items_view();
        let selection = self.selection.borrow().clone();
        if let Some(selection) = selection {
            selection.set_source(Some(erase_models(value)));
        }
        self.notifying.raise_property_changed("Items");
    }

    fn refresh_items_view(&self) {
        let shown = match self.filter.borrow().clone() {
            Some(filter) => {
                let items = self.items();
                let filtered: Vec<Rc<M>> = (0..items.count()).map(|i| items.get(i)).filter(|m| filter(m)).collect();
                let filtered: Models<M> = Rc::new(filtered);
                *self.filtered.borrow_mut() = Some(filtered.clone());
                filtered
            }
            None => {
                *self.filtered.borrow_mut() = None;
                self.items()
            }
        };
        let view = TreeDataGridItemsSourceView::get_or_create(Some(shown));
        *self.items_view.borrow_mut() = view.clone();
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.set_items(view);
        }
    }

    /// The filter of the rows (the classic source of the reference): only the models it accepts are
    /// shown. A filtered source shows a snapshot of the matching models; call
    /// [`refresh_filter`](Self::refresh_filter) after the models change.
    pub fn filter(&self) -> Option<Rc<dyn Fn(&Rc<M>) -> bool>> {
        self.filter.borrow().clone()
    }

    pub fn set_filter(&self, value: Option<Rc<dyn Fn(&Rc<M>) -> bool>>) {
        *self.filter.borrow_mut() = value;
        self.refresh_items_view();
        self.notifying.raise_property_changed("Filter");
    }

    /// Applies the filter again (`RefreshFilter`).
    pub fn refresh_filter(&self) {
        self.refresh_items_view();
    }

    /// The selection of rows, if the selection is one (`RowSelection`).
    pub fn row_selection(&self) -> Option<Rc<TreeDataGridRowSelectionModel<M>>> {
        self.selection().and_then(|selection| downcast_selection::<TreeDataGridRowSelectionModel<M>>(&selection))
    }

    pub fn is_sorted(&self) -> bool {
        self.comparer.borrow().is_some()
    }

    /// Releases the rows (`Dispose`).
    pub fn dispose(&self) {
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.dispose();
        }
    }

    /// Sorts by `column` (one of the columns).
    pub fn sort_by_column(&self, column: &ColumnRef<M>, direction: ListSortDirection) -> bool {
        if self.columns.index_of(column).is_none() {
            return true;
        }
        if let Some(comparer) = column.get_comparison(direction) {
            *self.comparer.borrow_mut() = Some(comparer.clone());
            let rows = self.rows.borrow().clone();
            if let Some(rows) = rows {
                rows.sort(Some(comparer));
            }
            self.raise_sorted();
            for c in self.columns.to_vec() {
                let same = std::ptr::addr_eq(Rc::as_ptr(&c), Rc::as_ptr(column));
                c.set_sort_direction(if same { Some(direction) } else { None });
            }
        }
        true
    }

    fn raise_sorted(&self) {
        for (_, handler) in self.sorted.snapshot().iter() {
            handler();
        }
    }

    fn as_source_of(&self) -> Rc<dyn ITreeDataGridSourceOf<M>> {
        self.this() as Rc<dyn ITreeDataGridSourceOf<M>>
    }
}

impl<M: TreeModel> INotifyPropertyChanged for FlatTreeDataGridSource<M> {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl<M: TreeModel> ITreeDataGridSource for FlatTreeDataGridSource<M> {
    fn column_count(&self) -> usize {
        self.columns.count()
    }

    fn column(&self, index: usize) -> Rc<dyn IColumn> {
        self.columns.get(index) as Rc<dyn IColumn>
    }

    fn rows(&self) -> Rc<dyn IRows> {
        self.typed_rows()
    }

    fn selection(&self) -> Option<Rc<dyn ITreeDataGridSelection>> {
        if self.selection.borrow().is_none() && !self.is_selection_set.get() {
            let selection = TreeDataGridRowSelectionModel::new(&self.as_source_of());
            *self.selection.borrow_mut() = Some(selection as Rc<dyn ITreeDataGridSelection>);
        }
        self.selection.borrow().clone()
    }

    /// # Panics
    /// When the source of the selection is not the items (`InvalidOperationException`).
    fn set_selection(&self, value: Option<Rc<dyn ITreeDataGridSelection>>) {
        let same = match (&*self.selection.borrow(), &value) {
            (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            _ => false,
        };
        if !same || value.is_none() {
            if let Some(value) = &value {
                let matches = value
                    .source()
                    .and_then(|source| downcast_models::<M>(&source))
                    .is_some_and(|source| same_models(&source, &self.items()));
                if !matches {
                    panic!("Selection source must be set to Items.");
                }
            }
            *self.selection.borrow_mut() = value;
            self.is_selection_set.set(true);
            self.notifying.raise_property_changed("Selection");
        }
    }

    fn is_hierarchical(&self) -> bool {
        false
    }

    fn is_sorted(&self) -> bool {
        FlatTreeDataGridSource::is_sorted(self)
    }

    fn items_untyped(&self) -> UntypedModels {
        erase_models(self.items())
    }

    fn get_model_children_untyped(&self, _model: &BoxedValue) -> Option<Vec<BoxedValue>> {
        Some(Vec::new())
    }

    fn sort_by(&self, column: Option<&dyn IColumn>, direction: ListSortDirection) -> bool {
        let Some(column) = column else {
            return false;
        };
        match self.columns.index_of_untyped(column) {
            Some(index) => self.sort_by_column(&self.columns.get(index), direction),
            // A column of models of this type that is not one of the columns sorts nothing but
            // is handled (`!Columns.Contains`); a column of another type is not.
            None => column.model_type() == std::any::TypeId::of::<M>(),
        }
    }

    fn clear_sort(&self) {
        if self.comparer.borrow().is_none() {
            return;
        }
        *self.comparer.borrow_mut() = None;
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.sort(None);
        }
        for column in self.columns.to_vec() {
            column.set_sort_direction(None);
        }
        self.raise_sorted();
    }

    fn add_sorted(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.sorted.add(handler)
    }

    fn remove_sorted(&self, token: u64) {
        self.sorted.remove(token);
    }

    fn move_rows(
        &self,
        _source: &dyn ITreeDataGridSource,
        indexes: &[IndexPath],
        target_index: &IndexPath,
        position: RowDropPosition,
        effects: RowMoveEffects,
    ) {
        if effects != RowMoveEffects::MOVE {
            panic!("Only move is currently supported for drag/drop.");
        }
        if self.is_sorted() {
            panic!("Drag/drop is not supported on sorted data.");
        }
        if position == RowDropPosition::None {
            return;
        }
        if position == RowDropPosition::Inside {
            panic!("Invalid drop position. (Parameter 'position')");
        }
        let mut ordered: Vec<IndexPath> = indexes.to_vec();
        ordered.sort_by(|a, b| b.cmp(a));
        if ordered.is_empty() {
            return;
        }
        if ordered.iter().any(|x| x.count() != 1) {
            panic!("Invalid source index. (Parameter 'indexes')");
        }
        for i in 1..ordered.len() {
            if ordered[i] == ordered[i - 1] {
                panic!("Duplicate source index. (Parameter 'indexes')");
            }
        }
        if target_index.count() != 1 {
            panic!("Invalid target index. (Parameter 'targetIndex')");
        }
        let items_rc = self.items();
        let Some(items) = items_rc.as_mutable() else {
            panic!("Items does not implement IList<T>.");
        };
        let count = items_rc.count();
        for index in &ordered {
            if index.get(0) < 0 || index.get(0) as usize >= count {
                panic!("Specified argument was out of the range of valid values. (Parameter 'indexes')");
            }
        }
        if target_index.get(0) < 0 || target_index.get(0) as usize >= count {
            panic!("Specified argument was out of the range of valid values. (Parameter 'targetIndex')");
        }
        let mut ti = target_index.get(0);
        if position == RowDropPosition::After {
            ti += 1;
        }

        let selection = self.selection.borrow().clone().and_then(|s| s.row_selection_model());
        let selected_indexes: Option<Vec<i32>> = selection.as_ref().map(|s| {
            let indexes = s.selected_indexes();
            (0..indexes.count()).map(|i| indexes.get(i)).filter(|x| x.count() == 1).map(|x| x.get(0)).collect()
        });
        let single = |path: IndexPath| if path.count() == 1 { path.get(0) } else { -1 };
        let primary_index = selection.as_ref().map_or(-1, |s| single(s.selected_index()));
        let anchor_index = selection.as_ref().map_or(-1, |s| single(s.anchor_index()));
        let range_anchor_index = selection.as_ref().map_or(-1, |s| single(s.range_anchor_index()));

        let mut index_map: Vec<i32> = (0..count as i32).collect();
        let mut source_items: Vec<(Rc<M>, i32)> = Vec::new();
        for src in &ordered {
            let i = src.get(0);
            source_items.push((items_rc.get(i as usize), index_map[i as usize]));
            items.remove_at(i as usize);
            index_map.remove(i as usize);
            if i < ti {
                ti -= 1;
            }
        }
        for (item, original) in source_items.into_iter().rev() {
            items.insert(ti as usize, item);
            index_map.insert(ti as usize, original);
            ti += 1;
        }
        if !items_rc.is_notifying() {
            let rows = self.rows.borrow().clone();
            if let Some(rows) = rows {
                rows.set_items(self.items_view.borrow().clone());
            }
        }
        let map_position = |original: i32| index_map.iter().position(|&x| x == original).map_or(-1, |i| i as i32);
        let map_anchor = |original: i32| {
            let mapped = if original >= 0 { map_position(original) } else { -1 };
            if mapped >= 0 {
                IndexPath::new(mapped)
            } else {
                IndexPath::UNSELECTED
            }
        };
        if let Some(selection) = selection {
            selection.begin_batch_update();
            if let Some(selected) = selected_indexes.filter(|s| !s.is_empty()) {
                selection.clear();
                if primary_index >= 0 {
                    selection.select(&IndexPath::new(map_position(primary_index)));
                }
                for selected_index in selected {
                    if selected_index != primary_index {
                        selection.select(&IndexPath::new(map_position(selected_index)));
                    }
                }
            }
            selection.set_anchor_index(map_anchor(anchor_index));
            selection.set_range_anchor_index(map_anchor(range_anchor_index));
            selection.end_batch_update();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<M: TreeModel> ITreeDataGridSourceOf<M> for FlatTreeDataGridSource<M> {
    fn items(&self) -> Models<M> {
        FlatTreeDataGridSource::items(self)
    }

    fn set_items(&self, value: Models<M>) {
        FlatTreeDataGridSource::set_items(self, value);
    }

    fn columns(&self) -> &ColumnList<M> {
        &self.columns
    }

    fn get_model_children(&self, _model: &Rc<M>) -> Option<Models<M>> {
        None
    }
}
