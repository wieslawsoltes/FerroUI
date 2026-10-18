//! `Models/TreeDataGrid/ColumnListBase.cs`: the columns of a presentation in a notifying list, and
//! their layout: the widths of auto, pixel and star columns committed at the end of a measure (in
//! batches during a layout), the geometry that finds the column at a position, and the estimate of
//! the column at a viewport position before the columns are measured.
//!
//! The list is the reference's `NotifyingListBase<TColumn>` with the overrides of
//! `ColumnListBase`: the bookkeeping of the layout (the constraints of the last commit, the
//! subscriptions to the columns' `ActualWidth`) is updated before a change is notified, so that a
//! listener sees a consistent list.

use super::double_utils::{are_close, greater_than, is_zero, widths_equal};
use super::{
    ColumnRef, ColumnsChangedArgs, ColumnsChangedHandler, IColumnLayoutBatch, IColumns, IUpdateColumnLayout,
};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Rect, Size};
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::models::NotifyingBase;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

const INDEXER_NAME: &str = "Item[]";

/// The estimate of the column at a viewport position (`IColumnViewportEstimator`), which the
/// virtualizing presenter of the columns uses before the columns in view are realized.
pub trait IColumnViewportEstimator {
    /// The index and start of the column at `viewport_start`, from the widths of the columns from
    /// `first_realized_index` (whose start is `realized_start`), or estimated from the average
    /// width; `estimated_element_size` is the estimate of the last layout, updated.
    #[allow(clippy::too_many_arguments)]
    fn get_or_estimate_column_at(
        &self,
        viewport_start: f64,
        viewport_end: f64,
        item_count: i32,
        realized_start: f64,
        first_realized_index: i32,
        estimated_element_size: &mut f64,
    ) -> (i32, f64);

    /// The average width of the measured columns, or -1 when no column is measured.
    fn estimate_element_size(&self) -> f64;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BatchUpdateType {
    None,
    Insert,
    Remove,
    Reset,
}

fn same_column(a: &ColumnRef, b: &ColumnRef) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `ColumnListBase<TColumn>`: an implementation of [`IColumns`] that stores its columns in a list.
pub struct ColumnListBase {
    weak_self: Weak<Self>,
    notifying: NotifyingBase,
    items: RefCell<Vec<ColumnRef>>,
    // The token of the subscription to the `PropertyChanged` of each column, in the order of the
    // columns.
    column_tokens: RefCell<Vec<u64>>,
    collection_changed: HandlerList<ColumnsChangedHandler>,
    busy: Cell<u32>,
    batch_update: Cell<BatchUpdateType>,
    layout_invalidated: HandlerList<dyn Fn()>,
    actual_width_batch_depth: Cell<i32>,
    actual_width_batch_needs_final_measure: Cell<bool>,
    initialized: Cell<bool>,
    column_widths_dirty: Cell<bool>,
    committed_constraints: RefCell<Vec<(f64, f64)>>,
    viewport_width: Cell<f64>,
    column_ends: RefCell<Vec<f64>>,
    geometry_dirty: Cell<bool>,
    estimated_element_size: Cell<f64>,
}

impl ColumnListBase {
    /// An empty list.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            notifying: NotifyingBase::new(),
            items: RefCell::new(Vec::new()),
            column_tokens: RefCell::new(Vec::new()),
            collection_changed: HandlerList::new(),
            busy: Cell::new(0),
            batch_update: Cell::new(BatchUpdateType::None),
            layout_invalidated: HandlerList::new(),
            actual_width_batch_depth: Cell::new(0),
            actual_width_batch_needs_final_measure: Cell::new(false),
            initialized: Cell::new(false),
            column_widths_dirty: Cell::new(true),
            committed_constraints: RefCell::new(Vec::new()),
            viewport_width: Cell::new(0.0),
            column_ends: RefCell::new(Vec::new()),
            geometry_dirty: Cell::new(true),
            estimated_element_size: Cell::new(-1.0),
        })
    }

    /// A list of `columns`.
    pub fn from_columns(columns: impl IntoIterator<Item = ColumnRef>) -> Rc<Self> {
        let this = Self::new();
        this.add_range(columns);
        this
    }

    /// The shared list.
    pub fn this(&self) -> Rc<Self> {
        self.weak_self.upgrade().expect("a column list that is alive")
    }

    /// The `PropertyChanged` of the list (`Count`, `Item[]`).
    pub fn notifying(&self) -> &NotifyingBase {
        &self.notifying
    }

    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// The column at `index`.
    ///
    /// # Panics
    /// When `index` is out of range.
    pub fn get(&self, index: usize) -> ColumnRef {
        self.items.borrow()[index].clone()
    }

    pub fn try_get(&self, index: usize) -> Option<ColumnRef> {
        self.items.borrow().get(index).cloned()
    }

    /// The columns, in order.
    pub fn to_vec(&self) -> Vec<ColumnRef> {
        self.items.borrow().clone()
    }

    /// The index of `column`, compared by identity.
    pub fn index_of(&self, column: &ColumnRef) -> Option<usize> {
        self.items.borrow().iter().position(|c| same_column(c, column))
    }

    pub fn contains(&self, column: &ColumnRef) -> bool {
        self.index_of(column).is_some()
    }

    /// `CollectionChanged`.
    pub fn add_collection_changed(&self, handler: Rc<ColumnsChangedHandler>) -> u64 {
        self.collection_changed.add(handler)
    }

    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.collection_changed.remove(token)
    }

    /// `LayoutInvalidated`: the widths of the columns changed and the layout must run again.
    pub fn add_layout_invalidated(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.layout_invalidated.add(handler)
    }

    pub fn remove_layout_invalidated(&self, token: u64) -> bool {
        self.layout_invalidated.remove(token)
    }

    pub fn add(&self, column: ColumnRef) {
        let index = self.count();
        self.insert(index, column);
    }

    /// Adds the columns one by one (`AddRange`).
    pub fn add_range(&self, columns: impl IntoIterator<Item = ColumnRef>) {
        for column in columns {
            self.add(column);
        }
    }

    /// Inserts a column.
    ///
    /// # Panics
    /// When `index` is greater than the count, or during a batch of another kind.
    pub fn insert(&self, index: usize, column: ColumnRef) {
        self.check_reentrancy();
        self.insert_item_core(index, column.clone());

        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.raise_count_and_indexer();
                self.raise(NotifyCollectionChangedAction::Add, std::slice::from_ref(&column), &[], index as i32, -1);
            }
            BatchUpdateType::Reset | BatchUpdateType::Insert => {}
            BatchUpdateType::Remove => panic!("Operation not permitted during a batch update."),
        }
    }

    /// Removes the column at `index`.
    ///
    /// # Panics
    /// When `index` is out of range, or during a batch of another kind.
    pub fn remove_at(&self, index: usize) {
        self.check_reentrancy();
        let removed = self.remove_item_core(index);

        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.raise_count_and_indexer();
                self.raise(NotifyCollectionChangedAction::Remove, &[], std::slice::from_ref(&removed), -1, index as i32);
            }
            BatchUpdateType::Reset | BatchUpdateType::Remove => {}
            BatchUpdateType::Insert => panic!("Operation not permitted during a batch update."),
        }
    }

    /// Removes `column`; false when it is not in the list.
    pub fn remove(&self, column: &ColumnRef) -> bool {
        match self.index_of(column) {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// Replaces the column at `index`.
    ///
    /// # Panics
    /// When `index` is out of range, or during a batch other than a reset.
    pub fn set(&self, index: usize, column: ColumnRef) {
        self.check_reentrancy();
        self.column_widths_dirty.set(true);
        self.geometry_dirty.set(true);
        let old_token = self.column_tokens.borrow()[index];
        let original = self.items.borrow()[index].clone();
        Self::unsubscribe_column(&original, old_token);
        let token = self.subscribe_column(&column);
        self.column_tokens.borrow_mut()[index] = token;
        self.committed_constraints.borrow_mut()[index] = (f64::NAN, f64::NAN);
        self.items.borrow_mut()[index] = column.clone();

        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.notifying.raise_property_changed(INDEXER_NAME);
                self.raise(
                    NotifyCollectionChangedAction::Replace,
                    std::slice::from_ref(&column),
                    std::slice::from_ref(&original),
                    index as i32,
                    index as i32,
                );
            }
            BatchUpdateType::Reset => {}
            _ => panic!("Operation not permitted during a batch update."),
        }
    }

    /// Moves the column at `old_index` to `new_index` (`MoveItem`).
    ///
    /// # Panics
    /// When an index is out of range (`ArgumentOutOfRangeException`), or during a batch other
    /// than a reset.
    pub fn move_item(&self, old_index: usize, new_index: usize) {
        if old_index >= self.count() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'oldIndex')");
        }
        if new_index >= self.count() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'newIndex')");
        }

        // Keep the constraint snapshots aligned before the collection-changed event is raised,
        // so synchronous listeners always observe a consistent column list.
        self.check_reentrancy();
        self.geometry_dirty.set(true);
        {
            let mut constraints = self.committed_constraints.borrow_mut();
            let moved = constraints.remove(old_index);
            constraints.insert(new_index, moved);
        }
        self.column_widths_dirty.set(true);

        let removed = {
            let mut items = self.items.borrow_mut();
            let removed = items.remove(old_index);
            items.insert(new_index, removed.clone());
            removed
        };
        {
            let mut tokens = self.column_tokens.borrow_mut();
            let token = tokens.remove(old_index);
            tokens.insert(new_index, token);
        }

        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.notifying.raise_property_changed(INDEXER_NAME);
                self.raise(
                    NotifyCollectionChangedAction::Move,
                    std::slice::from_ref(&removed),
                    std::slice::from_ref(&removed),
                    new_index as i32,
                    old_index as i32,
                );
            }
            BatchUpdateType::Reset => {}
            _ => panic!("Operation not permitted during a batch update."),
        }
    }

    /// Removes every column.
    ///
    /// # Panics
    /// During a batch other than a reset.
    pub fn clear(&self) {
        self.check_reentrancy();
        self.column_widths_dirty.set(true);
        self.geometry_dirty.set(true);
        let items = std::mem::take(&mut *self.items.borrow_mut());
        let tokens = std::mem::take(&mut *self.column_tokens.borrow_mut());
        for (column, token) in items.iter().zip(tokens) {
            Self::unsubscribe_column(column, token);
        }
        self.committed_constraints.borrow_mut().clear();

        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.raise_count_and_indexer();
                self.raise(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1);
            }
            BatchUpdateType::Reset => {}
            _ => panic!("Operation not permitted during a batch update."),
        }
    }

    /// Inserts the columns `action` hands over, from `index`, with one notification
    /// (`InsertRange`).
    pub fn insert_range(&self, index: usize, action: impl FnOnce(&mut dyn FnMut(ColumnRef))) {
        self.check_reentrancy();
        self.batch_update.set(BatchUpdateType::Insert);

        let mut i = index;
        {
            let mut insert = |column: ColumnRef| {
                self.insert(i, column);
                i += 1;
            };
            action(&mut insert);
        }
        self.batch_update.set(BatchUpdateType::None);

        if i > index {
            self.raise_count_and_indexer();
            let added: Vec<ColumnRef> = self.items.borrow()[index..i].to_vec();
            self.raise(NotifyCollectionChangedAction::Add, &added, &[], index as i32, -1);
        }
    }

    /// Removes `count` columns from `index` with one notification (`RemoveRange`).
    pub fn remove_range(&self, index: usize, count: usize) {
        if count == 0 {
            return;
        }

        self.check_reentrancy();
        self.batch_update.set(BatchUpdateType::Remove);
        let removed: Vec<ColumnRef> = self.items.borrow()[index..index + count].to_vec();
        for _ in 0..count {
            self.remove_at(index);
        }
        self.batch_update.set(BatchUpdateType::None);

        self.raise_count_and_indexer();
        self.raise(NotifyCollectionChangedAction::Remove, &[], &removed, -1, index as i32);
    }

    /// Changes the list in `action` and raises one reset notification (`Reset`).
    pub fn reset(&self, action: impl FnOnce(&Self)) {
        self.check_reentrancy();
        self.batch_update.set(BatchUpdateType::Reset);
        action(self);
        self.batch_update.set(BatchUpdateType::None);

        self.raise_count_and_indexer();
        self.raise(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1);
    }

    // `ColumnListBase.InsertItem` and the insertion of `Collection<T>.InsertItem`.
    fn insert_item_core(&self, index: usize, column: ColumnRef) {
        if index > self.count() {
            panic!("Index must be within the bounds of the List. (Parameter 'index')");
        }
        self.column_widths_dirty.set(true);
        self.geometry_dirty.set(true);
        let token = self.subscribe_column(&column);
        self.column_tokens.borrow_mut().insert(index, token);
        self.committed_constraints.borrow_mut().insert(index, (f64::NAN, f64::NAN));
        self.items.borrow_mut().insert(index, column);
    }

    // `ColumnListBase.RemoveItem` and the removal of `Collection<T>.RemoveItem`.
    fn remove_item_core(&self, index: usize) -> ColumnRef {
        self.column_widths_dirty.set(true);
        self.geometry_dirty.set(true);
        let removed = self.items.borrow_mut().remove(index);
        let token = self.column_tokens.borrow_mut().remove(index);
        Self::unsubscribe_column(&removed, token);
        self.committed_constraints.borrow_mut().remove(index);
        removed
    }

    // The subscription holds the list weakly, so that a column does not keep an old list alive
    // (the reference's weak event handler).
    fn subscribe_column(&self, column: &ColumnRef) -> u64 {
        let weak = self.weak_self.clone();
        column.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                this.on_column_property_changed(name);
            }
        }))
    }

    fn unsubscribe_column(column: &ColumnRef, token: u64) {
        column.property_changed().remove(token);
    }

    fn on_column_property_changed(&self, name: &str) {
        if name.is_empty() || name == "ActualWidth" {
            self.geometry_dirty.set(true);
        }
    }

    fn raise_count_and_indexer(&self) {
        self.notifying.raise_property_changed("Count");
        self.notifying.raise_property_changed(INDEXER_NAME);
    }

    fn raise(
        &self,
        action: NotifyCollectionChangedAction,
        new_items: &[ColumnRef],
        old_items: &[ColumnRef],
        new_index: i32,
        old_index: i32,
    ) {
        let handlers = self.collection_changed.snapshot();
        if handlers.is_empty() {
            return;
        }
        let args = ColumnsChangedArgs {
            action,
            new_items,
            old_items,
            new_starting_index: new_index,
            old_starting_index: old_index,
        };

        // `BlockReentrancy`.
        self.busy.set(self.busy.get() + 1);
        struct Guard<'a>(&'a Cell<u32>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() - 1);
            }
        }
        let _guard = Guard(&self.busy);
        for (_, handler) in handlers.iter() {
            handler(&args);
        }
    }

    /// Changes are allowed while a notification is raised only when one handler listens: a
    /// re-entrant change would make the arguments of later handlers invalid.
    ///
    /// # Panics
    /// On a re-entrant change with more than one listener (`InvalidOperationException`).
    pub fn check_reentrancy(&self) {
        if self.busy.get() > 0 && self.collection_changed.len() > 1 {
            panic!("NotifyingList reentrancy not allowed.");
        }
    }

    fn layout_of(column: &ColumnRef) -> &dyn IUpdateColumnLayout {
        column.as_update_column_layout().expect("a column that lays itself out (IUpdateColumnLayout)")
    }

    fn raise_layout_invalidated(&self) {
        for (_, handler) in self.layout_invalidated.snapshot().iter() {
            handler();
        }
    }

    /// The size a cell gets after it was measured at `size` (`CellMeasured`).
    pub fn cell_measured(&self, column_index: i32, row_index: i32, size: Size) -> Size {
        let column = self.get(column_index as usize);
        let layout = Self::layout_of(&column);
        self.initialized.set(true);
        let measured_width = layout.cell_measured(size.width, row_index);
        let committed = self.committed_constraints.borrow()[column_index as usize];

        if !widths_equal(measured_width, column.actual_width())
            || !widths_equal(committed.0, layout.min_actual_width())
            || !widths_equal(committed.1, layout.max_actual_width())
        {
            self.column_widths_dirty.set(true);
        }

        Size::new(measured_width, size.height)
    }

    /// The index and left edge of the column at `x`, or (-1, -1) (`GetColumnAt`).
    pub fn get_column_at(&self, x: f64) -> (i32, f64) {
        self.ensure_geometry();

        let ends = self.column_ends.borrow();

        // Upper bound skips zero-width columns and preserves exclusive right edges (the reference's
        // binary search, as the standard library's partition point).
        let low = ends.partition_point(|&end| end <= x);

        if x >= 0.0 && low < ends.len() && x < ends[low] {
            return (low as i32, if low == 0 { 0.0 } else { ends[low - 1] });
        }

        (-1, -1.0)
    }

    #[inline]
    fn ensure_geometry(&self) {
        if self.geometry_dirty.get() {
            self.rebuild_geometry();
        }
    }

    /// Computes the right edges of the columns of the known prefix and the estimated column width.
    #[cold]
    fn rebuild_geometry(&self) {
        let columns = self.to_vec();
        let mut column_ends = Vec::with_capacity(columns.len());
        let mut end = 0.0;
        let mut total = 0.0;
        let mut measured_count = 0;
        let mut known_prefix = true;
        for column in &columns {
            let width = column.actual_width();
            // Positions beyond an unmeasured column are unknown. The average still includes
            // measured columns beyond that point, as in the uncached estimator.
            known_prefix &= !width.is_nan() && width >= 0.0;
            if known_prefix {
                end += width;
                column_ends.push(end);
            }
            if !width.is_nan() && width > 0.0 {
                total += width;
                measured_count += 1;
            }
        }

        *self.column_ends.borrow_mut() = column_ends;
        self.estimated_element_size.set(if measured_count > 0 { total / measured_count as f64 } else { -1.0 });
        self.geometry_dirty.set(false);
    }

    /// The estimated total width of the columns in the viewport width `constraint`
    /// (`GetEstimatedWidth`).
    pub fn get_estimated_width(&self, constraint: f64) -> f64 {
        let mut has_star = false;
        let mut total_measured = 0.0;
        let mut measured_count = 0;
        let mut unmeasured_count = 0;

        for column in self.to_vec() {
            let layout = Self::layout_of(&column);

            if column.width().is_star() {
                has_star = true;
                total_measured += layout.min_actual_width();
            } else if !column.actual_width().is_nan() {
                total_measured += column.actual_width();
                measured_count += 1;
            } else {
                unmeasured_count += 1;
            }
        }

        // If there are star columns, and all measured columns fit within the available space
        // then we will fill the available space.
        if has_star && !constraint.is_infinite() && total_measured < constraint {
            return constraint;
        }

        // If there are a mix of measured and unmeasured columns then use the measured columns to
        // estimate the size of the unmeasured columns.
        if measured_count > 0 && unmeasured_count > 0 {
            let estimated = (total_measured / measured_count as f64) * unmeasured_count as f64;
            return total_measured + estimated;
        }

        total_measured
    }

    /// Commits the widths of the columns, unless a batch defers them (`CommitActualWidths`).
    pub fn commit_actual_widths(&self) {
        if self.actual_width_batch_depth.get() == 0 {
            self.update_column_sizes();
        }
    }

    /// Sets the width of the column at `column_index` (`SetColumnWidth`).
    pub fn set_column_width(&self, column_index: i32, width: GridLength) {
        let column = self.get(column_index as usize);

        if width != column.width() {
            Self::layout_of(&column).set_width(width);
            self.column_widths_dirty.set(true);
            self.raise_layout_invalidated();
            self.update_column_sizes();
        }
    }

    /// The viewport changed: star columns share its new width (`ViewportChanged`).
    pub fn viewport_changed(&self, viewport: Rect) {
        if !are_close(self.viewport_width.get(), viewport.width) {
            self.viewport_width.set(viewport.width);
            self.column_widths_dirty.set(true);
            if self.initialized.get() {
                self.update_column_sizes();
            }
        }
    }

    fn update_column_sizes(&self) {
        if !self.column_widths_dirty.get() {
            return;
        }

        self.column_widths_dirty.set(false);
        // Custom columns may commit widths without raising PropertyChanged.
        self.geometry_dirty.set(true);
        let columns = self.to_vec();
        let mut total_stars = 0.0;
        let mut available_space = self.viewport_width.get();
        let mut invalidated = false;

        // First commit the actual width for all non-star width columns and get a total of the
        // number of stars for star width columns.
        for column in &columns {
            let layout = Self::layout_of(column);

            if !column.width().is_star() {
                invalidated |= layout.commit_actual_width();
                available_space -= not_nan(column.actual_width());
            } else {
                total_stars += column.width().value();
            }
        }

        if total_stars > 0.0 {
            // Size the star columns.
            let mut star_width_was_constrained = false;

            available_space = available_space.max(0.0);

            // Do a first pass to calculate star column widths.
            for column in &columns {
                let layout = Self::layout_of(column);

                if column.width().is_star() {
                    layout.calculate_star_width(available_space, total_stars);
                    star_width_was_constrained |= layout.star_width_was_constrained();
                }
            }

            // If the width of any star columns was constrained by their min/max size, and we
            // actually had any space to distribute between star columns, then we need to update
            // the star width for the non-constrained columns.
            if star_width_was_constrained && greater_than(available_space, 0.0) {
                let initial_available_space = available_space;
                let initial_total_stars = total_stars;

                for column in &columns {
                    let layout = Self::layout_of(column);

                    if layout.star_width_was_constrained() {
                        available_space -=
                            Self::get_constrained_star_width(column, layout, initial_available_space, initial_total_stars);
                        total_stars -= column.width().value();
                    }
                }

                for column in &columns {
                    let layout = Self::layout_of(column);
                    if column.width().is_star() && !layout.star_width_was_constrained() {
                        layout.calculate_star_width(available_space, total_stars);
                    }
                }
            }

            // Finally commit the star column widths.
            for column in &columns {
                let layout = Self::layout_of(column);

                if column.width().is_star() {
                    invalidated |= layout.commit_actual_width();
                }
            }
        }

        {
            let mut constraints = self.committed_constraints.borrow_mut();
            for (i, column) in columns.iter().enumerate() {
                let layout = Self::layout_of(column);
                if i < constraints.len() {
                    constraints[i] = (layout.min_actual_width(), layout.max_actual_width());
                }
            }
        }

        self.geometry_dirty.set(true);
        if invalidated {
            self.raise_layout_invalidated();
        }
    }

    fn get_constrained_star_width(
        column: &ColumnRef,
        layout: &dyn IUpdateColumnLayout,
        available_space: f64,
        total_stars: f64,
    ) -> f64 {
        let width = (available_space / total_stars) * column.width().value();
        math_min(math_max(width, layout.min_actual_width()), layout.max_actual_width())
    }
}

fn not_nan(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v
    }
}

// `Math.Max` and `Math.Min`: a NaN operand gives NaN.
fn math_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn math_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

impl Drop for ColumnListBase {
    fn drop(&mut self) {
        let items = std::mem::take(&mut *self.items.borrow_mut());
        let tokens = std::mem::take(&mut *self.column_tokens.borrow_mut());
        for (column, token) in items.iter().zip(tokens) {
            Self::unsubscribe_column(column, token);
        }
    }
}

impl INotifyPropertyChanged for ColumnListBase {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl IColumns for ColumnListBase {
    fn count(&self) -> usize {
        ColumnListBase::count(self)
    }

    fn get(&self, index: usize) -> ColumnRef {
        ColumnListBase::get(self, index)
    }

    fn cell_measured(&self, column_index: i32, row_index: i32, size: Size) -> Size {
        ColumnListBase::cell_measured(self, column_index, row_index, size)
    }

    fn get_column_at(&self, x: f64) -> (i32, f64) {
        ColumnListBase::get_column_at(self, x)
    }

    fn get_estimated_width(&self, constraint: f64) -> f64 {
        ColumnListBase::get_estimated_width(self, constraint)
    }

    fn commit_actual_widths(&self) {
        ColumnListBase::commit_actual_widths(self)
    }

    fn set_column_width(&self, column_index: i32, width: GridLength) {
        ColumnListBase::set_column_width(self, column_index, width)
    }

    fn viewport_changed(&self, viewport: Rect) {
        ColumnListBase::viewport_changed(self, viewport)
    }

    fn add_layout_invalidated(&self, handler: Rc<dyn Fn()>) -> u64 {
        ColumnListBase::add_layout_invalidated(self, handler)
    }

    fn remove_layout_invalidated(&self, token: u64) {
        ColumnListBase::remove_layout_invalidated(self, token);
    }

    fn add_collection_changed(&self, handler: Rc<ColumnsChangedHandler>) -> u64 {
        ColumnListBase::add_collection_changed(self, handler)
    }

    fn remove_collection_changed(&self, token: u64) {
        ColumnListBase::remove_collection_changed(self, token);
    }

    fn as_layout_batch(&self) -> Option<&dyn IColumnLayoutBatch> {
        Some(self)
    }

    fn as_viewport_estimator(&self) -> Option<&dyn IColumnViewportEstimator> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IColumnLayoutBatch for ColumnListBase {
    fn is_actual_width_commit_deferred(&self) -> bool {
        self.actual_width_batch_depth.get() > 0
    }

    fn begin_actual_width_batch(&self) {
        let depth = self.actual_width_batch_depth.get();
        self.actual_width_batch_depth.set(depth + 1);
        if depth == 0 {
            self.actual_width_batch_needs_final_measure.set(false);
        }
    }

    /// # Panics
    /// When no batch is active (`InvalidOperationException`).
    fn end_actual_width_batch(&self) -> bool {
        if self.actual_width_batch_depth.get() <= 0 {
            panic!("No column width batch is active.");
        }

        let depth = self.actual_width_batch_depth.get() - 1;
        self.actual_width_batch_depth.set(depth);
        if depth > 0 {
            return false;
        }

        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.actual_width_batch_needs_final_measure);
        self.update_column_sizes();
        self.actual_width_batch_needs_final_measure.get()
    }

    fn request_final_measure(&self) {
        self.actual_width_batch_needs_final_measure.set(true);
    }
}

impl IColumnViewportEstimator for ColumnListBase {
    fn get_or_estimate_column_at(
        &self,
        viewport_start: f64,
        viewport_end: f64,
        item_count: i32,
        realized_start: f64,
        first_realized_index: i32,
        estimated_element_size: &mut f64,
    ) -> (i32, f64) {
        // We have no elements, nothing to do here.
        if item_count <= 0 {
            return (-1, 0.0);
        }

        // If we're at 0 then display the first item.
        if is_zero(viewport_start) {
            return (0, 0.0);
        }

        let mut u = realized_start;

        for (i, column) in self.to_vec().iter().enumerate() {
            let size = column.actual_width();

            // A zero-width Auto column has not provided a useful viewport anchor yet. Falling
            // back to the measured-width estimate keeps large horizontal jumps accurate while
            // still allowing zero to reserve its configured minimum width.
            if size.is_nan() || size <= 0.0 {
                break;
            }

            let end = u + size;

            if end > viewport_start && u < viewport_end {
                return (first_realized_index + i as i32, u);
            }

            u = end;
        }

        // We don't have any realized elements in the requested viewport, or can't rely on the
        // realized start being valid. Estimate the index using only the estimated size. First,
        // estimate the element size, using the given estimate if we don't have any measured
        // elements.
        let estimated_size = match self.estimate_element_size() {
            v if v == -1.0 => *estimated_element_size,
            v => v,
        };

        // Store the estimated size for the next layout pass.
        *estimated_element_size = estimated_size;

        // Estimate the element at the start of the viewport.
        let index = ((viewport_start / estimated_size) as i32).min(item_count - 1);
        (index, index as f64 * estimated_size)
    }

    fn estimate_element_size(&self) -> f64 {
        self.ensure_geometry();
        self.estimated_element_size.get()
    }
}
