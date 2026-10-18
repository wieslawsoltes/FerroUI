//! `Presentation/TreeDataGridRows.cs`: the rows of a presentation, which realize its cells.
//!
//! The items and collection notifications are the core's rows themselves. This view object only
//! adds cell realization; it maintains no second row collection or per-row wrappers.

use super::CellModelPool;
use crate::models::{CellRef, ColumnRef, ITreeDataGridRows};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_treedatagrid_core::models::{IRows, RowRef, RowsChangedArgs, RowsChangedEvent, RowsChangedHandler};
use ferroui_treedatagrid_core::IndexPath;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Rows that can retarget a realized cell to another row without returning it to a pool
/// (`IReusableCellRows`).
pub trait IReusableCellRows {
    /// Retargets `cell` of `column` to the row at `row_index`; false when the column cannot.
    fn try_reuse_cell(&self, column: &ColumnRef, cell: &CellRef, row_index: i32) -> bool;
}

/// The rows of a presentation as [`IReusableCellRows`], when they are the rows of a
/// [`TreeDataGridPresentation`](super::TreeDataGridSourcePresentation) (`rows as IReusableCellRows`).
pub fn as_reusable_cell_rows(rows: &dyn ITreeDataGridRows) -> Option<&dyn IReusableCellRows> {
    rows.as_any().downcast_ref::<TreeDataGridRows>().map(|rows| rows as &dyn IReusableCellRows)
}

/// The rows of a presentation (`TreeDataGridRows<TModel>`).
pub struct TreeDataGridRows {
    weak_self: Weak<Self>,
    rows: RefCell<Rc<dyn IRows>>,
    subscription: Cell<Option<u64>>,
    cell_pool: RefCell<Option<CellModelPool>>,
    collection_changed: RowsChangedEvent,
}

impl TreeDataGridRows {
    pub fn new(rows: Rc<dyn IRows>) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            rows: RefCell::new(rows),
            subscription: Cell::new(None),
            cell_pool: RefCell::new(None),
            collection_changed: RowsChangedEvent::default(),
        });
        this.resume();
        this
    }

    fn subscribe(&self, rows: &Rc<dyn IRows>) -> u64 {
        let weak = self.weak_self.clone();
        rows.add_collection_changed(Rc::new(move |e: &RowsChangedArgs<'_>| {
            if let Some(this) = weak.upgrade() {
                this.collection_changed.raise(e);
            }
        }))
    }

    fn inner(&self) -> Rc<dyn IRows> {
        self.rows.borrow().clone()
    }

    /// Whether the rows follow the changes of the core's rows.
    pub fn is_subscribed(&self) -> bool {
        self.subscription.get().is_some()
    }

    /// Follows the changes of the core's rows again (`Resume`).
    pub fn resume(&self) {
        if self.subscription.get().is_none() {
            let rows = self.inner();
            self.subscription.set(Some(self.subscribe(&rows)));
        }
    }

    /// Stops following the core's rows and empties the cell pool (`Suspend`).
    pub fn suspend(&self) {
        if let Some(token) = self.subscription.take() {
            self.inner().remove_collection_changed(token);
        }
        self.clear_cell_pool();
    }

    /// Disposes the pooled cells (`ClearCellPool`).
    pub fn clear_cell_pool(&self) {
        let cells = self.cell_pool.borrow_mut().as_mut().map(CellModelPool::clear).unwrap_or_default();
        for cell in cells {
            cell.dispose();
        }
    }

    /// The number of cells in the pool.
    pub fn pooled_cell_count(&self) -> usize {
        self.cell_pool.borrow().as_ref().map_or(0, CellModelPool::count)
    }

    /// Shows other core rows (`SetRows`): a reset when they are not the rows shown.
    pub fn set_rows(&self, rows: Rc<dyn IRows>) {
        if std::ptr::addr_eq(Rc::as_ptr(&self.inner()), Rc::as_ptr(&rows)) {
            return;
        }
        self.clear_cell_pool();
        let subscribed = self.subscription.take();
        if let Some(token) = subscribed {
            self.inner().remove_collection_changed(token);
        }
        *self.rows.borrow_mut() = rows.clone();
        if subscribed.is_some() {
            self.subscription.set(Some(self.subscribe(&rows)));
        }
        let args = NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Reset,
            new_items: &[],
            old_items: &[],
            new_starting_index: -1,
            old_starting_index: -1,
        };
        self.collection_changed.raise(&args);
    }

    /// The core's rows.
    pub fn core_rows(&self) -> Rc<dyn IRows> {
        self.inner()
    }

    /// Stops following the core's rows (`Dispose`).
    pub fn dispose(&self) {
        self.suspend();
    }

    fn row(&self, row_index: i32) -> RowRef {
        let rows = self.inner();
        if row_index < 0 || row_index as usize >= rows.count() {
            panic!("Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')");
        }
        rows.get(row_index as usize)
    }

    // `CellModelPool.Take`: the pooled cells of the column are popped one by one until one can be
    // retargeted to the row; the others are disposed.
    fn take_pooled(&self, column: &ColumnRef, row: &RowRef) -> Option<CellRef> {
        let cell_column = column.as_cell_column()?;
        let hit = self.cell_pool.borrow_mut().as_mut().is_some_and(|pool| pool.touch_column(column));
        if !hit {
            return None;
        }
        loop {
            let cell = self.cell_pool.borrow_mut().as_mut().and_then(|pool| pool.pop(column))?;
            if cell_column.try_reuse_cell(&cell, row) {
                return Some(cell);
            }
            cell.dispose();
        }
    }
}

impl ITreeDataGridRows for TreeDataGridRows {
    fn count(&self) -> usize {
        self.inner().count()
    }

    fn get(&self, index: usize) -> RowRef {
        self.inner().get(index)
    }

    fn get_row_at(&self, y: f64) -> (i32, f64) {
        if y.abs() < f64::EPSILON {
            (0, 0.0)
        } else {
            (-1, -1.0)
        }
    }

    fn model_index_to_row_index(&self, model_index: &IndexPath) -> i32 {
        self.inner().model_index_to_row_index(model_index)
    }

    fn row_index_to_model_index(&self, row_index: i32) -> IndexPath {
        self.inner().row_index_to_model_index(row_index)
    }

    /// # Panics
    /// When the column is not a column that creates cells (the reference's cast to `ICellColumn`).
    fn realize_cell(&self, column: &ColumnRef, _column_index: i32, row_index: i32) -> CellRef {
        let row = self.row(row_index);
        if let Some(cell) = self.take_pooled(column, &row) {
            return cell;
        }
        let cell_column = column.as_cell_column().expect("Unable to cast the column to a column that creates cells.");
        cell_column.create_cell(&row)
    }

    fn unrealize_cell(&self, cell: &CellRef, _column_index: i32, _row_index: i32) {
        cell.dispose();
    }

    fn add_collection_changed(&self, handler: Rc<RowsChangedHandler>) -> u64 {
        self.collection_changed.add(handler)
    }

    fn remove_collection_changed(&self, token: u64) {
        self.collection_changed.remove(token);
    }

    /// `IRecyclingCellRows.TryRecycleCell`: only while the rows follow their source.
    fn try_recycle_cell(&self, column: &ColumnRef, cell: &CellRef) -> bool {
        if self.subscription.get().is_none() {
            return false;
        }
        let suspended = cell.as_recyclable().is_some_and(|recyclable| recyclable.try_suspend());
        if !suspended {
            return false;
        }
        let mut evicted = Vec::new();
        {
            let mut pool = self.cell_pool.borrow_mut();
            pool.get_or_insert_with(CellModelPool::new).add(column, cell.clone(), &mut evicted);
        }
        for cell in evicted {
            cell.dispose();
        }
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IReusableCellRows for TreeDataGridRows {
    fn try_reuse_cell(&self, column: &ColumnRef, cell: &CellRef, row_index: i32) -> bool {
        let Some(cell_column) = column.as_cell_column() else {
            return false;
        };
        let row = self.row(row_index);
        cell_column.try_reuse_cell(cell, &row)
    }
}
