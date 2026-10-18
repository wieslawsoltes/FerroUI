//! `Presentation/TreeDataGridPresentation\`1.cs` and the factory of `Presentation/TreeDataGridPresentation.cs`:
//! the presentation of a core source. It owns only the view columns and the input state; it never
//! owns the source.

use super::core_conversions::{to_core_length, to_view_length};
use super::{
    dispose_column, CellColumnFactory, ITreeDataGridPresentationOptions, PresentationRef, TreeDataGridPresentation,
    TreeDataGridPresentationOptions, TreeDataGridRows,
};
use crate::models::{ColumnRef, ColumnsChangedHandler, IColumnLayoutBatch, IColumns, ITreeDataGridRows};
use crate::selection::{
    as_cell_selection, as_row_selection, register_tree_data_grid_source, source_support,
    ITreeDataGridSelectionInteraction, IUntypedCellSelection, TreeDataGridCellSelectionInteraction,
    TreeDataGridRowSelectionInteraction,
};
use crate::{TreeDataGridRowDropPosition, TreeDataGridSelectionChangedEventArgs, TreeDataGridSelectionMode};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::input::DragDropEffects;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, Rect, Size};
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{IColumn as CoreColumn, ListSortDirection, NotifyingBase};
use ferroui_treedatagrid_core::selection::{
    IReadOnlyList, ITreeDataGridSelection, ITreeSelectionModel, TreeSelectionModelSelectionChangedEventArgs,
};
use ferroui_treedatagrid_core::{CellIndex, ITreeDataGridSource, ITreeDataGridSourceOf, IndexPath, RowMoveEffects};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The presentation of `model` (`TreeDataGridPresentation.Create`): the options' presentation, or
/// the presentation of the source with the default options.
pub fn create_presentation(
    model: &Rc<dyn ITreeDataGridSource>,
    options: Option<&Rc<dyn ITreeDataGridPresentationOptions>>,
) -> PresentationRef {
    match options {
        Some(options) => options.create(model),
        None => TreeDataGridSourcePresentation::new(model.clone(), None) as PresentationRef,
    }
}

fn same_column(a: &Rc<dyn CoreColumn>, b: &Rc<dyn CoreColumn>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

fn same_view(a: &ColumnRef, b: &ColumnRef) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

// The view of a core column and the presentation key it was made for (`ColumnView`), with the
// subscription to the core column's `PropertyChanged`.
struct ColumnEntry {
    model: Rc<dyn CoreColumn>,
    cell: ColumnRef,
    key: Option<String>,
    token: Option<u64>,
}

// A core column whose view could not be made, still followed so that a correction of its key
// presents it.
struct FailedColumn {
    model: Rc<dyn CoreColumn>,
    token: Option<u64>,
}

enum Interaction {
    Row(Rc<TreeDataGridRowSelectionInteraction>),
    Cell(Rc<TreeDataGridCellSelectionInteraction>),
}

impl Interaction {
    fn as_dyn(&self) -> Rc<dyn ITreeDataGridSelectionInteraction> {
        match self {
            Interaction::Row(row) => row.clone() as Rc<dyn ITreeDataGridSelectionInteraction>,
            Interaction::Cell(cell) => cell.clone() as Rc<dyn ITreeDataGridSelectionInteraction>,
        }
    }

    fn dispose(&self) {
        match self {
            Interaction::Row(row) => row.dispose(),
            Interaction::Cell(cell) => cell.dispose(),
        }
    }
}

/// The presentation of a core source (`TreeDataGridPresentation<TModel>`).
pub struct TreeDataGridSourcePresentation {
    weak_self: Weak<Self>,
    model: Rc<dyn ITreeDataGridSource>,
    notifying: NotifyingBase,
    sorted: HandlerList<dyn Fn()>,
    native_selection_changed: HandlerList<dyn Fn(&TreeDataGridSelectionChangedEventArgs)>,
    factory: CellColumnFactory,
    cells: RefCell<Vec<ColumnEntry>>,
    failed_columns: RefCell<Vec<FailedColumn>>,
    columns: Rc<PresentationColumns>,
    rows: RefCell<Option<Rc<TreeDataGridRows>>>,
    interaction: RefCell<Option<Interaction>>,
    selection_identity: RefCell<Option<Rc<dyn ITreeDataGridSelection>>>,
    selection: RefCell<Option<Rc<dyn ITreeSelectionModel>>>,
    selection_token: Cell<Option<u64>>,
    cell_selection: RefCell<Option<Rc<dyn IUntypedCellSelection>>>,
    cell_selection_token: Cell<Option<u64>>,
    selected_cells: RefCell<Vec<CellIndex>>,
    disposed: Cell<bool>,
    suspended: Cell<bool>,
    columns_token: Cell<Option<u64>>,
    model_property_token: Cell<Option<u64>>,
    model_sorted_token: Cell<Option<u64>>,
}

impl TreeDataGridSourcePresentation {
    /// The presentation of `model` with `options` (the default options when `None`).
    ///
    /// # Panics
    /// When a column of the source has a presentation key with no registered presentation, or
    /// the source's selection is not a row or cell selection of the core.
    pub fn new(model: Rc<dyn ITreeDataGridSource>, options: Option<Rc<TreeDataGridPresentationOptions>>) -> Rc<Self> {
        let options = options.unwrap_or_else(TreeDataGridPresentationOptions::new);
        let this = Rc::new_cyclic(|weak_self: &Weak<Self>| Self {
            weak_self: weak_self.clone(),
            model,
            notifying: NotifyingBase::new(),
            sorted: HandlerList::new(),
            native_selection_changed: HandlerList::new(),
            factory: CellColumnFactory::new(options),
            cells: RefCell::new(Vec::new()),
            failed_columns: RefCell::new(Vec::new()),
            columns: PresentationColumns::new(weak_self.clone()),
            rows: RefCell::new(None),
            interaction: RefCell::new(None),
            selection_identity: RefCell::new(None),
            selection: RefCell::new(None),
            selection_token: Cell::new(None),
            cell_selection: RefCell::new(None),
            cell_selection_token: Cell::new(None),
            selected_cells: RefCell::new(Vec::new()),
            disposed: Cell::new(false),
            suspended: Cell::new(false),
            columns_token: Cell::new(None),
            model_property_token: Cell::new(None),
            model_sorted_token: Cell::new(None),
        });

        let result = (|| -> Result<(), String> {
            for column in this.model.columns_untyped() {
                let cell = this.create_column(&column)?;
                if column.is_visible() {
                    this.columns.list().add(cell);
                }
            }
            this.update_selection()?;
            this.subscribe_columns();
            this.subscribe_model();
            Ok(())
        })();
        if let Err(message) = result {
            this.dispose();
            panic!("{message}");
        }
        this
    }

    /// The presentation of a typed source, registering its model and source types
    /// (`new TreeDataGridPresentation<TModel>(model, options)`).
    pub fn of<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static>(
        model: Rc<S>,
        options: Option<Rc<TreeDataGridPresentationOptions>>,
    ) -> Rc<Self> {
        register_tree_data_grid_source::<M, S>();
        Self::new(model as Rc<dyn ITreeDataGridSource>, options)
    }

    fn this(&self) -> Rc<Self> {
        self.weak_self.upgrade().expect("a presentation that is alive")
    }

    /// The source presented (`Model`).
    pub fn model(&self) -> &Rc<dyn ITreeDataGridSource> {
        &self.model
    }

    /// The view columns (`Columns`), as the presentation's own type.
    pub fn presentation_columns(&self) -> &Rc<PresentationColumns> {
        &self.columns
    }

    /// The rows (`Rows`), as the presentation's own type.
    pub fn presentation_rows(&self) -> Rc<TreeDataGridRows> {
        let existing = self.rows.borrow().clone();
        let rows = match existing {
            Some(rows) => rows,
            None => {
                let rows = TreeDataGridRows::new(self.model.rows());
                *self.rows.borrow_mut() = Some(rows.clone());
                rows
            }
        };
        rows.set_rows(self.model.rows());
        rows
    }

    /// Whether the presentation is suspended (detached from its control).
    pub fn is_suspended(&self) -> bool {
        self.suspended.get()
    }

    /// Whether the presentation is disposed.
    pub fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    fn subscribe_model(&self) {
        let weak = self.weak_self.clone();
        let token = self.model.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                this.on_model_property_changed(name);
            }
        }));
        self.model_property_token.set(Some(token));
        let weak = self.weak_self.clone();
        let token = self.model.add_sorted(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_sorted();
            }
        }));
        self.model_sorted_token.set(Some(token));
    }

    fn unsubscribe_model(&self) {
        if let Some(token) = self.model_property_token.take() {
            self.model.property_changed().remove(token);
        }
        if let Some(token) = self.model_sorted_token.take() {
            self.model.remove_sorted(token);
        }
    }

    // `_columnNotifications.CollectionChanged += OnColumnsChanged`: the changes of the source's
    // columns, through the typed operations of the source type.
    fn subscribe_columns(&self) {
        let Some(support) = source_support(&self.model) else {
            return;
        };
        let weak = self.weak_self.clone();
        let token = support.add_columns_changed(
            &self.model,
            Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_columns_changed();
                }
            }),
        );
        self.columns_token.set(token);
    }

    fn unsubscribe_columns(&self) {
        if let Some(token) = self.columns_token.take() {
            if let Some(support) = source_support(&self.model) {
                support.remove_columns_changed(&self.model, token);
            }
        }
    }

    fn subscribe_column(&self, column: &Rc<dyn CoreColumn>) -> u64 {
        let weak = self.weak_self.clone();
        column.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                this.on_column_property_changed(name);
            }
        }))
    }

    /// Stops following the source while the control is detached (`Suspend`).
    pub fn suspend_presentation(&self) {
        if self.disposed.get() || self.suspended.get() {
            return;
        }
        self.suspended.set(true);
        self.unsubscribe_native_selection();
        let interaction = self.interaction.borrow_mut().take();
        if let Some(interaction) = interaction {
            interaction.dispose();
        }
        *self.selection.borrow_mut() = None;
        *self.selection_identity.borrow_mut() = None;
        self.notifying.raise_property_changed("SelectionInteraction");
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.suspend();
        }
        self.unsubscribe_model();
        self.unsubscribe_columns();
        let entries: Vec<(Rc<dyn CoreColumn>, Option<u64>)> =
            self.cells.borrow_mut().iter_mut().map(|e| (e.model.clone(), e.token.take())).collect();
        for (model, token) in entries {
            if let Some(token) = token {
                model.property_changed().remove(token);
            }
        }
        let failed: Vec<(Rc<dyn CoreColumn>, Option<u64>)> =
            self.failed_columns.borrow_mut().iter_mut().map(|e| (e.model.clone(), e.token.take())).collect();
        for (model, token) in failed {
            if let Some(token) = token {
                model.property_changed().remove(token);
            }
        }
    }

    /// Follows the source again after the control is attached (`Resume`).
    ///
    /// # Panics
    /// When a column of the source has a presentation key with no registered presentation.
    pub fn resume_presentation(&self) {
        if self.disposed.get() || !self.suspended.get() {
            return;
        }
        if let Err(message) = self.synchronize_columns() {
            panic!("{message}");
        }
        let entries: Vec<(Rc<dyn CoreColumn>, ColumnRef)> =
            self.cells.borrow().iter().map(|e| (e.model.clone(), e.cell.clone())).collect();
        for (model, cell) in entries {
            let width = to_view_length(model.width());
            match self.columns.list().index_of(&cell) {
                Some(index) => self.columns.set_column_width(index as i32, width),
                None => {
                    if let Some(layout) = cell.as_update_column_layout() {
                        layout.set_width(width);
                    }
                }
            }
            cell.set_sort_direction(model.sort_direction());
            let token = self.subscribe_column(&model);
            if let Some(entry) = self.cells.borrow_mut().iter_mut().find(|e| same_column(&e.model, &model)) {
                entry.token = Some(token);
            }
        }
        self.suspended.set(false);
        self.synchronize_rows();
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.resume();
        }
        if let Err(message) = self.update_selection() {
            panic!("{message}");
        }
        self.subscribe_columns();
        self.subscribe_model();
    }

    /// Releases the view columns and stops following the source (`Dispose`).
    pub fn dispose_presentation(&self) {
        if self.disposed.get() {
            return;
        }
        self.suspend_presentation();
        self.disposed.set(true);
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.dispose();
        }
        let cells: Vec<ColumnRef> = self.cells.borrow_mut().drain(..).map(|e| e.cell).collect();
        for cell in cells {
            dispose_column(&cell);
        }
        self.failed_columns.borrow_mut().clear();
    }

    fn update_selection(&self) -> Result<(), String> {
        let selection = self.model.selection();
        let same = match (&*self.selection_identity.borrow(), &selection) {
            (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            (None, None) => true,
            _ => false,
        };
        if same {
            return Ok(());
        }

        let row = selection.as_ref().and_then(as_row_selection);
        let cell = selection.as_ref().and_then(as_cell_selection);
        // A tree selection whose type is not registered is presented as a row selection.
        let fallback =
            if row.is_none() && cell.is_none() { selection.as_ref().and_then(|s| s.row_selection_model()) } else { None };
        if selection.is_some() && row.is_none() && cell.is_none() && fallback.is_none() {
            return Err("This presentation requires a Core row or cell selection model.".to_string());
        }

        self.unsubscribe_native_selection();
        let previous = self.interaction.borrow_mut().take();
        if let Some(previous) = previous {
            previous.dispose();
        }

        let row_model = row.as_ref().map(|row| row.model()).or_else(|| fallback.clone());
        *self.selection.borrow_mut() = row_model.clone();
        *self.selection_identity.borrow_mut() = selection.clone();
        let interaction = match (&row, &cell, &fallback) {
            (Some(row), _, _) => Some(Interaction::Row(TreeDataGridRowSelectionInteraction::new(self.model.clone(), row.clone()))),
            (None, Some(cell), _) => {
                Some(Interaction::Cell(TreeDataGridCellSelectionInteraction::new(self.model.clone(), cell.clone())))
            }
            (None, None, Some(model)) => {
                Some(Interaction::Row(TreeDataGridRowSelectionInteraction::with_model(self.model.clone(), model.clone())))
            }
            _ => None,
        };
        *self.interaction.borrow_mut() = interaction;

        if let Some(row_model) = &row_model {
            let weak = self.weak_self.clone();
            let token = row_model.add_selection_changed(Rc::new(move |e: &dyn TreeSelectionModelSelectionChangedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.raise_native_selection_changed(&TreeDataGridSelectionChangedEventArgs::from_rows(e));
                }
            }));
            self.selection_token.set(Some(token));
        }

        if let Some(cell) = &cell {
            *self.selected_cells.borrow_mut() = cell.selected_indexes();
            let weak = self.weak_self.clone();
            let token = cell.add_selection_changed(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_native_cell_selection_changed();
                }
            }));
            self.cell_selection_token.set(Some(token));
        }
        *self.cell_selection.borrow_mut() = cell;

        self.notifying.raise_property_changed("SelectionInteraction");
        Ok(())
    }

    fn unsubscribe_native_selection(&self) {
        if let Some(token) = self.selection_token.take() {
            let selection = self.selection.borrow().clone();
            if let Some(selection) = selection {
                selection.remove_selection_changed(token);
            }
        }
        if let Some(token) = self.cell_selection_token.take() {
            let cell = self.cell_selection.borrow().clone();
            if let Some(cell) = cell {
                cell.remove_selection_changed(token);
            }
        }
        *self.cell_selection.borrow_mut() = None;
        self.selected_cells.borrow_mut().clear();
    }

    fn raise_native_selection_changed(&self, e: &TreeDataGridSelectionChangedEventArgs) {
        for (_, handler) in self.native_selection_changed.snapshot().iter() {
            handler(e);
        }
    }

    fn on_native_cell_selection_changed(&self) {
        let Some(cell) = self.cell_selection.borrow().clone() else {
            return;
        };
        let selected = cell.selected_indexes();
        let previous = std::mem::replace(&mut *self.selected_cells.borrow_mut(), selected.clone());
        // `previous.Except(selected)` and `selected.Except(previous)`: distinct, in order.
        let except = |a: &[CellIndex], b: &[CellIndex]| {
            let mut result: Vec<CellIndex> = Vec::new();
            for x in a {
                if !b.contains(x) && !result.contains(x) {
                    result.push(x.clone());
                }
            }
            result
        };
        let deselected = except(&previous, &selected);
        let newly_selected = except(&selected, &previous);
        // The rows of the cells and their models, each once (`GetDistinctRowIndexes`,
        // `GetDistinctModels` of the reference's cell selection of its classic sources).
        let rows = self.model.rows();
        let distinct_rows = |cells: &[CellIndex]| {
            let mut result: Vec<IndexPath> = Vec::new();
            for cell in cells {
                if !result.contains(&cell.row_index) {
                    result.push(cell.row_index.clone());
                }
            }
            result
        };
        let models = |indexes: &[IndexPath]| -> Vec<Option<BoxedValue>> {
            indexes
                .iter()
                .map(|index| {
                    let row_index = rows.model_index_to_row_index(index);
                    if row_index >= 0 && (row_index as usize) < rows.count() {
                        rows.get(row_index as usize).model()
                    } else {
                        None
                    }
                })
                .collect()
        };
        let deselected_rows = distinct_rows(&deselected);
        let selected_rows = distinct_rows(&newly_selected);
        let deselected_items = models(&deselected_rows);
        let selected_items = models(&selected_rows);
        self.raise_native_selection_changed(&TreeDataGridSelectionChangedEventArgs::new(
            Some(deselected_rows),
            Some(selected_rows),
            Some(deselected_items),
            Some(selected_items),
            Some(deselected),
            Some(newly_selected),
        ));
    }

    fn on_model_property_changed(&self, name: &str) {
        if name == "Selection" {
            if let Err(message) = self.update_selection() {
                panic!("{message}");
            }
        } else {
            if name == "Rows" {
                self.synchronize_rows();
            }
            self.notifying.raise_property_changed(name);
        }
    }

    fn on_sorted(&self) {
        for (_, handler) in self.sorted.snapshot().iter() {
            handler();
        }
    }

    fn on_columns_changed(&self) {
        if let Err(message) = self.synchronize_columns() {
            panic!("{message}");
        }
    }

    fn create_column(&self, model: &Rc<dyn CoreColumn>) -> Result<ColumnRef, String> {
        match self.create_cell(model) {
            Ok(cell) => {
                // A column that failed before keeps the subscription it had.
                let failed_index = self.failed_columns.borrow().iter().position(|f| same_column(&f.model, model));
                let was_failed = failed_index.is_some();
                let mut token = failed_index.and_then(|index| self.failed_columns.borrow_mut().remove(index).token);
                if !self.suspended.get() && !was_failed {
                    token = Some(self.subscribe_column(model));
                }
                self.cells.borrow_mut().push(ColumnEntry {
                    model: model.clone(),
                    cell: cell.clone(),
                    key: model.presentation_key(),
                    token,
                });
                Ok(cell)
            }
            Err(message) => {
                let known = self.failed_columns.borrow().iter().any(|f| same_column(&f.model, model));
                if !self.suspended.get() && !known {
                    let token = self.subscribe_column(model);
                    self.failed_columns.borrow_mut().push(FailedColumn { model: model.clone(), token: Some(token) });
                }
                Err(message)
            }
        }
    }

    fn create_cell(&self, model: &Rc<dyn CoreColumn>) -> Result<ColumnRef, String> {
        let cell = self.factory.create(model)?;
        let width = to_view_length(model.width());
        if cell.width() != width {
            if let Some(layout) = cell.as_update_column_layout() {
                layout.set_width(width);
            }
        }
        cell.set_sort_direction(model.sort_direction());
        Ok(cell)
    }

    fn remove_column(&self, model: &Rc<dyn CoreColumn>) {
        let index = self.cells.borrow().iter().position(|e| same_column(&e.model, model));
        let Some(index) = index else {
            return;
        };
        let entry = self.cells.borrow_mut().remove(index);
        if let Some(token) = entry.token {
            model.property_changed().remove(token);
        }
        dispose_column(&entry.cell);
    }

    fn replace_column(&self, model: &Rc<dyn CoreColumn>) -> Result<(), String> {
        let replacement = self.create_cell(model)?;
        let previous = {
            let mut cells = self.cells.borrow_mut();
            let entry = cells.iter_mut().find(|e| same_column(&e.model, model)).expect("a presented column");
            let previous = std::mem::replace(&mut entry.cell, replacement);
            entry.key = model.presentation_key();
            previous
        };
        if let Some(index) = self.columns.list().index_of(&previous) {
            self.columns.list().remove_at(index);
        }
        dispose_column(&previous);
        Ok(())
    }

    fn view_of(&self, model: &Rc<dyn CoreColumn>) -> Option<ColumnRef> {
        self.cells.borrow().iter().find(|e| same_column(&e.model, model)).map(|e| e.cell.clone())
    }

    fn synchronize_columns(&self) -> Result<(), String> {
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.clear_cell_pool();
        }
        let desired = self.model.columns_untyped();
        let contains = |column: &Rc<dyn CoreColumn>| desired.iter().any(|d| same_column(d, column));

        if !self.cells.borrow().is_empty() || !self.failed_columns.borrow().is_empty() {
            let removed: Vec<Rc<dyn CoreColumn>> =
                self.cells.borrow().iter().filter(|e| !contains(&e.model)).map(|e| e.model.clone()).collect();
            for model in removed {
                self.remove_column(&model);
            }

            let removed_failed: Vec<FailedColumn> = {
                let mut failed = self.failed_columns.borrow_mut();
                let (keep, removed): (Vec<FailedColumn>, Vec<FailedColumn>) = failed.drain(..).partition(|f| contains(&f.model));
                *failed = keep;
                removed
            };
            for failed in removed_failed {
                if let Some(token) = failed.token {
                    failed.model.property_changed().remove(token);
                }
            }

            let changed: Vec<Rc<dyn CoreColumn>> = self
                .cells
                .borrow()
                .iter()
                .filter(|e| e.model.presentation_key() != e.key)
                .map(|e| e.model.clone())
                .collect();
            for model in changed {
                self.replace_column(&model)?;
            }
        }

        for model in &desired {
            if self.view_of(model).is_none() {
                self.create_column(model)?;
            }
        }

        let list = self.columns.list();
        let mut target = 0usize;
        for model in &desired {
            if !model.is_visible() {
                continue;
            }
            let cell = self.view_of(model).expect("a presented column");
            if target >= list.count() || !same_view(&list.get(target), &cell) {
                if let Some(old_index) = list.index_of(&cell) {
                    list.remove_at(old_index);
                }
                list.insert(target, cell);
            }
            target += 1;
        }
        while list.count() > target {
            list.remove_at(list.count() - 1);
        }
        Ok(())
    }

    fn synchronize_rows(&self) {
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.set_rows(self.model.rows());
        }
    }

    fn on_column_property_changed(&self, name: &str) {
        if name == "PresentationKey" || name == "IsVisible" {
            if let Err(message) = self.synchronize_columns() {
                panic!("{message}");
            }
        } else if name == "Width" || name == "SortDirection" {
            // Expander models forward the notifications of their inner model.
            let entries: Vec<(Rc<dyn CoreColumn>, ColumnRef)> =
                self.cells.borrow().iter().map(|e| (e.model.clone(), e.cell.clone())).collect();
            for (model, cell) in entries {
                if name == "Width" {
                    let width = to_view_length(model.width());
                    if cell.width() != width {
                        match self.columns.list().index_of(&cell) {
                            Some(index) => self.columns.set_column_width(index as i32, width),
                            None => {
                                if let Some(layout) = cell.as_update_column_layout() {
                                    layout.set_width(width);
                                }
                            }
                        }
                    }
                } else {
                    cell.set_sort_direction(model.sort_direction());
                }
            }
        }
    }

    /// The core column of the view column `column` (`GetModel`).
    pub fn get_model(&self, column: &ColumnRef) -> Option<Rc<dyn CoreColumn>> {
        self.cells.borrow().iter().find(|e| same_view(&e.cell, column)).map(|e| e.model.clone())
    }

    /// The row selection the presentation presents, if it is one (`_selection`).
    pub fn row_selection(&self) -> Option<Rc<dyn ITreeSelectionModel>> {
        self.selection.borrow().clone()
    }

    /// The cell selection the presentation presents, if it is one (`_cellSelection`).
    pub fn cell_selection(&self) -> Option<Rc<dyn IUntypedCellSelection>> {
        self.cell_selection.borrow().clone()
    }

    /// The shared presentation.
    pub fn shared(&self) -> Rc<Self> {
        self.this()
    }
}

impl INotifyPropertyChanged for TreeDataGridSourcePresentation {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl TreeDataGridPresentation for TreeDataGridSourcePresentation {
    fn source_identity(&self) -> Rc<dyn Any> {
        self.model.clone() as Rc<dyn Any>
    }

    fn can_select_multiple(&self) -> bool {
        self.selection.borrow().as_ref().is_some_and(|selection| !selection.single_select())
    }

    fn select(&self, index: &IndexPath, replace: bool) {
        let Some(selection) = self.selection.borrow().clone() else {
            return;
        };
        if replace {
            selection.set_selected_index(index.clone());
        } else {
            selection.select(index);
        }
    }

    fn deselect(&self, index: &IndexPath) {
        let selection = self.selection.borrow().clone();
        if let Some(selection) = selection {
            selection.deselect(index);
        }
    }

    fn is_selected(&self, index: &IndexPath) -> bool {
        let selection = self.selection.borrow().clone();
        selection.is_some_and(|selection| selection.is_selected(index))
    }

    fn columns(&self) -> Rc<dyn IColumns> {
        self.columns.clone() as Rc<dyn IColumns>
    }

    fn rows(&self) -> Rc<dyn ITreeDataGridRows> {
        self.presentation_rows() as Rc<dyn ITreeDataGridRows>
    }

    fn is_hierarchical(&self) -> bool {
        self.model.is_hierarchical()
    }

    fn is_sorted(&self) -> bool {
        self.model.is_sorted()
    }

    fn selection_interaction(&self) -> Option<Rc<dyn ITreeDataGridSelectionInteraction>> {
        self.interaction.borrow().as_ref().map(Interaction::as_dyn)
    }

    fn selected_items(&self) -> Option<Vec<Option<BoxedValue>>> {
        let selection = self.selection.borrow().clone()?;
        Some(selection.selected_items().to_vec())
    }

    fn selected_indexes(&self) -> Option<Vec<IndexPath>> {
        let selection = self.selection.borrow().clone()?;
        Some(selection.selected_indexes().to_vec())
    }

    fn add_sorted(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.sorted.add(handler)
    }

    fn remove_sorted(&self, token: u64) {
        self.sorted.remove(token);
    }

    fn sort_by(&self, column: &ColumnRef, direction: ListSortDirection) -> bool {
        let model = self.get_model(column);
        self.model.sort_by(model.as_deref(), direction)
    }

    fn move_rows(&self, indexes: &[IndexPath], target: &IndexPath, position: TreeDataGridRowDropPosition, effects: DragDropEffects) {
        self.model.move_rows(&*self.model, indexes, target, position.into(), RowMoveEffects(effects.bits() as u32));
    }

    fn dispose(&self) {
        self.dispose_presentation();
    }

    fn suspend(&self) {
        self.suspend_presentation();
    }

    fn resume(&self) {
        self.resume_presentation();
    }

    fn add_native_selection_changed(&self, handler: Rc<dyn Fn(&TreeDataGridSelectionChangedEventArgs)>) -> u64 {
        self.native_selection_changed.add(handler)
    }

    fn remove_native_selection_changed(&self, token: u64) {
        self.native_selection_changed.remove(token);
    }

    fn apply_selection_mode(&self, mode: TreeDataGridSelectionMode) {
        apply_selection_mode_to(&self.model, mode);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Applies a selection mode to a source (`ApplySelectionMode`): a cell selection for a mode of
/// cells, a row selection otherwise, created when the source's selection is not one, with one or
/// many rows or cells.
///
/// # Panics
/// When the source's type is not registered, so that no selection can be created for it
/// (`InvalidOperationException` of the reference's classic sources).
pub fn apply_selection_mode_to(model: &Rc<dyn ITreeDataGridSource>, mode: TreeDataGridSelectionMode) {
    let single = !mode.intersects(TreeDataGridSelectionMode::MULTIPLE);
    let current = model.selection();
    if mode.intersects(TreeDataGridSelectionMode::CELL) {
        let cell = match current.as_ref().and_then(as_cell_selection) {
            Some(cell) => cell,
            None => {
                let selection = source_support(model)
                    .and_then(|support| support.create_cell_selection_model(model))
                    .unwrap_or_else(|| {
                        panic!("Source '{}' does not support automatic cell selection model creation.", source_type_name(model))
                    });
                model.set_selection(Some(selection.clone()));
                as_cell_selection(&selection).expect("a cell selection of the source")
            }
        };
        cell.set_single_select(single);
    } else {
        let is_row = current.as_ref().is_some_and(|s| as_row_selection(s).is_some() || (as_cell_selection(s).is_none() && s.row_selection_model().is_some()));
        let row = match current.as_ref().filter(|_| is_row).and_then(|s| s.row_selection_model()) {
            Some(row) => row,
            None => {
                let selection = source_support(model)
                    .and_then(|support| support.create_row_selection_model(model))
                    .unwrap_or_else(|| {
                        panic!("Source '{}' does not support automatic row selection model creation.", source_type_name(model))
                    });
                model.set_selection(Some(selection.clone()));
                selection.row_selection_model().expect("a row selection of the source")
            }
        };
        row.set_single_select(single);
    }
}

fn source_type_name(source: &Rc<dyn ITreeDataGridSource>) -> String {
    // The concrete type of a trait object has no name at run time; the kind of source does.
    if source.is_hierarchical() {
        "HierarchicalTreeDataGridSource".to_string()
    } else {
        "FlatTreeDataGridSource".to_string()
    }
}

/// The view columns of a presentation (`PresentationColumns`): the column list of the control,
/// whose width changes are written back to the core columns.
pub struct PresentationColumns {
    list: Rc<crate::models::ColumnListBase>,
    owner: Weak<TreeDataGridSourcePresentation>,
}

impl PresentationColumns {
    fn new(owner: Weak<TreeDataGridSourcePresentation>) -> Rc<Self> {
        Rc::new(Self { list: crate::models::ColumnListBase::new(), owner })
    }

    /// The column list.
    pub fn list(&self) -> &Rc<crate::models::ColumnListBase> {
        &self.list
    }

    /// Sets the width of the column at `index` and of its core column (`SetColumnWidth`).
    pub fn set_column_width(&self, index: i32, width: GridLength) {
        self.list.set_column_width(index, width);
        let column = self.list.get(index as usize);
        let model = self.owner.upgrade().and_then(|owner| owner.get_model(&column));
        if let Some(model) = model {
            model.set_width(to_core_length(width));
        }
    }
}

impl IColumns for PresentationColumns {
    fn count(&self) -> usize {
        self.list.count()
    }

    fn get(&self, index: usize) -> ColumnRef {
        self.list.get(index)
    }

    fn cell_measured(&self, column_index: i32, row_index: i32, size: Size) -> Size {
        self.list.cell_measured(column_index, row_index, size)
    }

    fn get_column_at(&self, x: f64) -> (i32, f64) {
        self.list.get_column_at(x)
    }

    fn get_estimated_width(&self, constraint: f64) -> f64 {
        self.list.get_estimated_width(constraint)
    }

    fn commit_actual_widths(&self) {
        self.list.commit_actual_widths();
    }

    fn set_column_width(&self, column_index: i32, width: GridLength) {
        PresentationColumns::set_column_width(self, column_index, width);
    }

    fn viewport_changed(&self, viewport: Rect) {
        self.list.viewport_changed(viewport);
    }

    fn add_layout_invalidated(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.list.add_layout_invalidated(handler)
    }

    fn remove_layout_invalidated(&self, token: u64) {
        self.list.remove_layout_invalidated(token);
    }

    fn add_collection_changed(&self, handler: Rc<ColumnsChangedHandler>) -> u64 {
        self.list.add_collection_changed(handler)
    }

    fn remove_collection_changed(&self, token: u64) {
        self.list.remove_collection_changed(token);
    }

    fn as_layout_batch(&self) -> Option<&dyn IColumnLayoutBatch> {
        Some(&*self.list)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for TreeDataGridSourcePresentation {
    fn drop(&mut self) {
        // A presentation that was not disposed still stops following its source.
        self.unsubscribe_model();
        self.unsubscribe_columns();
        self.unsubscribe_native_selection();
        if let Some(interaction) = self.interaction.borrow_mut().take() {
            interaction.dispose();
        }
        for entry in self.cells.borrow_mut().iter_mut() {
            if let Some(token) = entry.token.take() {
                entry.model.property_changed().remove(token);
            }
        }
        for failed in self.failed_columns.borrow_mut().iter_mut() {
            if let Some(token) = failed.token.take() {
                failed.model.property_changed().remove(token);
            }
        }
        if let Some(rows) = self.rows.borrow().as_ref() {
            rows.suspend();
        }
    }
}
