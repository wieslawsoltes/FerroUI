//! `Models/SortableRowsBase.cs`: rows created on demand for the models of a collection, kept in
//! the order of a comparison, updated by the changes of the collection.

use super::collection_extensions::has_known_indexes;
use super::{Comparison, RowRef};
use crate::accessor::TreeModel;
use crate::model_collection::{reset_args, ModelsChangedArgs};
use crate::utils::{sort_helper, stable_sort};
use crate::TreeDataGridItemsSourceView;
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::{Rc, Weak};

/// What [`SortableRowsBase`] needs of its rows (`TRow : IRow<TModel>, IModelIndexableRow, IDisposable`).
pub trait SortableRow: Clone + 'static {
    fn model_index(&self) -> i32;
    fn update_model_index(&self, delta: i32);
    fn dispose(&self);
    /// The row as an untyped row, for the notifications of `IRows`.
    fn row_ref(&self) -> RowRef;
}

/// A change of the rows of a [`SortableRowsBase`].
pub type SortableRowsChangedArgs<'a, R> = NotifyCollectionChangedEventArgs<'a, R>;

/// A handler of [`SortableRowsBase::add_collection_changed`].
pub type SortableRowsChangedHandler<R> = dyn for<'a> Fn(&SortableRowsChangedArgs<'a, R>);

/// The rows of the models of a collection, sorted by an optional comparison.
pub struct SortableRowsBase<M: TreeModel, R: SortableRow> {
    weak_self: Weak<Self>,
    items: RefCell<Rc<TreeDataGridItemsSourceView<M>>>,
    items_token: Cell<Option<u64>>,
    comparison: RefCell<Option<Comparison<M>>>,
    unsorted_rows: RefCell<Option<Vec<R>>>,
    sorted_indexes: RefCell<Option<Vec<i32>>>,
    create_row: Box<dyn Fn(i32, Rc<M>) -> R>,
    handlers: RefCell<Vec<(u64, Rc<SortableRowsChangedHandler<R>>)>>,
    next_token: Cell<u64>,
}

impl<M: TreeModel, R: SortableRow> SortableRowsBase<M, R> {
    /// Rows of `items`; `create_row` creates the row of a model (`CreateRow`).
    pub fn new(
        items: Rc<TreeDataGridItemsSourceView<M>>,
        comparison: Option<Comparison<M>>,
        create_row: impl Fn(i32, Rc<M>) -> R + 'static,
    ) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            items: RefCell::new(items.clone()),
            items_token: Cell::new(None),
            comparison: RefCell::new(comparison),
            unsorted_rows: RefCell::new(None),
            sorted_indexes: RefCell::new(None),
            create_row: Box::new(create_row),
            handlers: RefCell::new(Vec::new()),
            next_token: Cell::new(1),
        });
        this.subscribe(&items);
        this
    }

    fn subscribe(&self, items: &Rc<TreeDataGridItemsSourceView<M>>) {
        let weak = self.weak_self.clone();
        let token = items.add_collection_changed(Rc::new(move |e: &ModelsChangedArgs<'_, M>| {
            if let Some(this) = weak.upgrade() {
                this.on_items_collection_changed(e);
            }
        }));
        self.items_token.set(Some(token));
    }

    fn unsubscribe(&self) {
        if let Some(token) = self.items_token.take() {
            let items = self.items.borrow().clone();
            items.remove_collection_changed(token);
        }
    }

    pub fn add_collection_changed(&self, handler: Rc<SortableRowsChangedHandler<R>>) -> u64 {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.handlers.borrow_mut().push((token, handler));
        token
    }

    pub fn remove_collection_changed(&self, token: u64) {
        self.handlers.borrow_mut().retain(|(t, _)| *t != token);
    }

    fn has_handlers(&self) -> bool {
        !self.handlers.borrow().is_empty()
    }

    fn raise(&self, args: &SortableRowsChangedArgs<'_, R>) {
        let handlers: Vec<Rc<SortableRowsChangedHandler<R>>> = self.handlers.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler(args);
        }
    }

    fn raise_reset(&self) {
        self.raise(&NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Reset,
            new_items: &[],
            old_items: &[],
            new_starting_index: -1,
            old_starting_index: -1,
        });
    }

    /// The items view.
    pub fn items(&self) -> Rc<TreeDataGridItemsSourceView<M>> {
        self.items.borrow().clone()
    }

    /// The number of rows.
    pub fn count(&self) -> usize {
        if let Some(rows) = self.unsorted_rows.borrow().as_ref() {
            return rows.len();
        }
        self.items.borrow().count()
    }

    /// The row at `index`, in sorted order; the rows are created on first access.
    pub fn get(&self, index: usize) -> R {
        self.get_or_create_rows();
        let rows = self.unsorted_rows.borrow();
        let rows = rows.as_ref().expect("rows created");
        match self.sorted_indexes.borrow().as_ref() {
            None => rows[index].clone(),
            Some(sorted) => rows[sorted[index] as usize].clone(),
        }
    }

    /// The rows, in sorted order.
    pub fn to_vec(&self) -> Vec<R> {
        self.get_or_create_rows();
        let rows = self.unsorted_rows.borrow();
        let rows = rows.as_ref().expect("rows created");
        match self.sorted_indexes.borrow().as_ref() {
            None => rows.clone(),
            Some(sorted) => sorted.iter().map(|&i| rows[i as usize].clone()).collect(),
        }
    }

    /// The rows created so far, unsorted (none before the first access).
    pub fn unsorted_rows_if_created(&self) -> Option<Vec<R>> {
        self.unsorted_rows.borrow().clone()
    }

    /// Releases the rows and stops listening (`Dispose`).
    pub fn dispose(&self) {
        self.set_items(TreeDataGridItemsSourceView::empty());
    }

    /// Shows the models of another view.
    pub fn set_items(&self, items: Rc<TreeDataGridItemsSourceView<M>>) {
        self.unsubscribe();
        *self.items.borrow_mut() = items.clone();
        self.subscribe(&items);
        self.on_items_collection_changed(&reset_args::<M>());
    }

    /// Sorts the rows by `comparison` (`None`: the order of the models).
    pub fn sort(&self, comparison: Option<Comparison<M>>) {
        let has_comparison = comparison.is_some();
        *self.comparison.borrow_mut() = comparison;
        if self.unsorted_rows.borrow().is_some() {
            let sorted = if has_comparison { Some(self.create_sorted_indexes()) } else { None };
            *self.sorted_indexes.borrow_mut() = sorted;
            self.raise_reset();
        }
    }

    /// Whether `items` are the models of the rows, in order, compared by identity.
    pub fn has_same_items(&self, items: Option<&crate::Models<M>>) -> bool {
        let view = self.items();
        match items {
            None => view.count() == 0,
            Some(items) => {
                let count = items.count();
                count == view.count() && (0..count).all(|i| Rc::ptr_eq(&view.get_at(i), &items.get(i)))
            }
        }
    }

    /// The row index of the model at `model_index`, or -1 (a negative value for a model that is not
    /// in the sorted indexes).
    pub fn model_index_to_row_index(&self, model_index: i32) -> i32 {
        let has_sorted = self.sorted_indexes.borrow().is_some();
        if !has_sorted {
            let count = self.items.borrow().count() as i32;
            return if model_index >= 0 && model_index < count { model_index } else { -1 };
        }
        let sorted = self.sorted_indexes.borrow().clone().expect("sorted indexes");
        let mut compare = self.compare_items_by_index();
        sort_helper::binary_search(&sorted, &model_index, &mut compare)
    }

    /// The model index of the row at `row_index`.
    pub fn row_index_to_model_index(&self, row_index: i32) -> i32 {
        match self.sorted_indexes.borrow().as_ref() {
            Some(sorted) => sorted[row_index as usize],
            None => row_index,
        }
    }

    fn compare_items_by_index(&self) -> impl FnMut(&i32, &i32) -> Ordering {
        let items = self.items();
        let comparison = self.comparison.borrow().clone().expect("a comparison");
        move |&index1: &i32, &index2: &i32| {
            match comparison(&items.get_at(index1 as usize), &items.get_at(index2 as usize)) {
                // The order of the models keeps the sort stable.
                Ordering::Equal => index1.cmp(&index2),
                order => order,
            }
        }
    }

    fn create_sorted_indexes(&self) -> Vec<i32> {
        // The models are read once for the sort, which compares each many times: the comparisons
        // borrow them rather than reading them from the items for every comparison.
        let items = self.items.borrow().clone();
        let models: Vec<Rc<M>> = (0..items.count()).map(|i| items.get_at(i)).collect();
        let comparer = self.comparison.borrow().clone().expect("a comparison");
        let mut compare = |&index1: &i32, &index2: &i32| match comparer(&models[index1 as usize], &models[index2 as usize]) {
            // The order of the models keeps the sort stable.
            Ordering::Equal => index1.cmp(&index2),
            order => order,
        };
        stable_sort::sorted_map(models.len(), &mut compare)
    }

    fn get_or_create_rows(&self) {
        if self.unsorted_rows.borrow().is_some() {
            return;
        }
        let items = self.items();
        let count = items.count();
        let mut rows = Vec::with_capacity(count);
        for i in 0..count {
            rows.push((self.create_row)(i as i32, items.get_at(i)));
        }
        *self.unsorted_rows.borrow_mut() = Some(rows);
        if self.comparison.borrow().is_some() {
            let sorted = self.create_sorted_indexes();
            *self.sorted_indexes.borrow_mut() = Some(sorted);
        }
    }

    fn reset_rows(&self) {
        let rows = self.unsorted_rows.borrow_mut().take();
        if let Some(rows) = rows {
            for row in &rows {
                row.dispose();
            }
        }
        *self.sorted_indexes.borrow_mut() = None;
    }

    fn move_rows(&self, old_index: usize, new_index: usize, count: usize) -> Vec<R> {
        let (moved, to_update) = {
            let mut rows = self.unsorted_rows.borrow_mut();
            let rows = rows.as_mut().expect("rows created");
            let moved: Vec<R> = rows.drain(old_index..old_index + count).collect();
            for (i, row) in moved.iter().enumerate() {
                rows.insert(new_index + i, row.clone());
            }
            let start = old_index.min(new_index);
            let end = (old_index + count).max(new_index + count);
            let to_update: Vec<(R, i32)> =
                (start..end).map(|i| (rows[i].clone(), i as i32 - rows[i].model_index())).filter(|(_, d)| *d != 0).collect();
            (moved, to_update)
        };
        for (row, delta) in to_update {
            row.update_model_index(delta);
        }
        moved
    }

    fn on_items_collection_changed(&self, e: &ModelsChangedArgs<'_, M>) {
        let reset = reset_args::<M>();
        let e = if has_known_indexes(e) { e } else { &reset };
        if self.comparison.borrow().is_none() {
            self.on_items_collection_changed_unsorted(e);
        } else {
            self.on_items_collection_changed_sorted(e);
        }
    }

    fn insert_unsorted(&self, index: usize, models: &[Rc<M>]) {
        let mut created = Vec::with_capacity(models.len());
        for (i, model) in models.iter().enumerate() {
            created.push((self.create_row)((index + i) as i32, model.clone()));
        }
        let following: Vec<R> = {
            let mut rows = self.unsorted_rows.borrow_mut();
            let rows = rows.as_mut().expect("rows created");
            rows.splice(index..index, created);
            rows[index + models.len()..].to_vec()
        };
        for row in following {
            row.update_model_index(models.len() as i32);
        }
    }

    fn remove_unsorted(&self, index: usize, count: usize) {
        let (removed, following) = {
            let mut rows = self.unsorted_rows.borrow_mut();
            let rows = rows.as_mut().expect("rows created");
            let removed: Vec<R> = rows.drain(index..index + count).collect();
            (removed, rows[index..].to_vec())
        };
        for row in &removed {
            row.dispose();
        }
        for row in following {
            row.update_model_index(-(count as i32));
        }
    }

    fn rows_slice(&self, index: usize, count: usize) -> Vec<R> {
        self.unsorted_rows.borrow().as_ref().expect("rows created")[index..index + count].to_vec()
    }

    fn on_items_collection_changed_unsorted(&self, e: &ModelsChangedArgs<'_, M>) {
        if self.unsorted_rows.borrow().is_none() {
            return;
        }
        match e.action {
            NotifyCollectionChangedAction::Add => {
                let index = e.new_starting_index as usize;
                self.insert_unsorted(index, e.new_items);
                let added = self.rows_slice(index, e.new_items.len());
                self.raise(&NotifyCollectionChangedEventArgs {
                    action: NotifyCollectionChangedAction::Add,
                    new_items: &added,
                    old_items: &[],
                    new_starting_index: e.new_starting_index,
                    old_starting_index: -1,
                });
            }
            NotifyCollectionChangedAction::Remove => {
                let index = e.old_starting_index as usize;
                let count = e.old_items.len();
                let old_items = if self.has_handlers() { self.rows_slice(index, count) } else { Vec::new() };
                self.remove_unsorted(index, count);
                self.raise(&NotifyCollectionChangedEventArgs {
                    action: NotifyCollectionChangedAction::Remove,
                    new_items: &[],
                    old_items: &old_items,
                    new_starting_index: -1,
                    old_starting_index: e.old_starting_index,
                });
            }
            NotifyCollectionChangedAction::Replace => {
                let index = e.old_starting_index as usize;
                let count = e.old_items.len();
                if count != e.new_items.len() {
                    self.remove_unsorted(index, count);
                    let at = if e.new_starting_index >= 0 { e.new_starting_index as usize } else { index };
                    self.insert_unsorted(at, e.new_items);
                    self.raise_reset();
                    return;
                }
                let old_items = if self.has_handlers() { self.rows_slice(index, count) } else { Vec::new() };
                for i in 0..count {
                    let old = self.unsorted_rows.borrow().as_ref().expect("rows created")[index + i].clone();
                    old.dispose();
                    let row = (self.create_row)((index + i) as i32, e.new_items[i].clone());
                    self.unsorted_rows.borrow_mut().as_mut().expect("rows created")[index + i] = row;
                }
                let new_items = self.rows_slice(index, count);
                self.raise(&NotifyCollectionChangedEventArgs {
                    action: NotifyCollectionChangedAction::Replace,
                    new_items: &new_items,
                    old_items: &old_items,
                    new_starting_index: index as i32,
                    old_starting_index: index as i32,
                });
            }
            NotifyCollectionChangedAction::Move => {
                let moved = self.move_rows(e.old_starting_index as usize, e.new_starting_index as usize, e.old_items.len());
                self.raise(&NotifyCollectionChangedEventArgs {
                    action: NotifyCollectionChangedAction::Move,
                    new_items: &moved,
                    old_items: &moved,
                    new_starting_index: e.new_starting_index,
                    old_starting_index: e.old_starting_index,
                });
            }
            NotifyCollectionChangedAction::Reset => {
                self.reset_rows();
                self.raise_reset();
            }
        }
    }

    fn add_sorted(&self, start_index: usize, count: usize) {
        let items = self.items();
        // The new rows among the unsorted rows.
        for i in start_index..start_index + count {
            let row = (self.create_row)(i as i32, items.get_at(i));
            self.unsorted_rows.borrow_mut().as_mut().expect("rows created").insert(i, row);
        }
        // The indexes of the rows after them.
        let following: Vec<R> = self.unsorted_rows.borrow().as_ref().expect("rows created")[start_index + count..].to_vec();
        for row in following {
            row.update_model_index(count as i32);
        }
        // The indexes of the sorted indexes after them.
        {
            let mut sorted = self.sorted_indexes.borrow_mut();
            for ix in sorted.as_mut().expect("sorted indexes").iter_mut() {
                if *ix >= start_index as i32 {
                    *ix += count as i32;
                }
            }
        }
        // Each new row at its place in the sorted indexes.
        for i in 0..count {
            let model_index = (start_index + i) as i32;
            let sorted = self.sorted_indexes.borrow().clone().expect("sorted indexes");
            let mut compare = self.compare_items_by_index();
            let mut index = sort_helper::binary_search(&sorted, &model_index, &mut compare);
            if index < 0 {
                index = !index;
            }
            self.sorted_indexes.borrow_mut().as_mut().expect("sorted indexes").insert(index as usize, model_index);
            let row = self.unsorted_rows.borrow().as_ref().expect("rows created")[model_index as usize].clone();
            self.raise(&NotifyCollectionChangedEventArgs {
                action: NotifyCollectionChangedAction::Add,
                new_items: std::slice::from_ref(&row),
                old_items: &[],
                new_starting_index: index,
                old_starting_index: -1,
            });
        }
    }

    fn remove_sorted(&self, start_index: usize, count: usize) {
        let end_index = start_index + count;
        let removed_rows: Vec<R> = {
            let mut rows = self.unsorted_rows.borrow_mut();
            rows.as_mut().expect("rows created").drain(start_index..end_index).collect()
        };
        for row in &removed_rows {
            row.dispose();
        }
        let mut i = 0usize;
        loop {
            let ix = match self.sorted_indexes.borrow().as_ref().expect("sorted indexes").get(i) {
                Some(&ix) => ix,
                None => break,
            };
            if ix >= start_index as i32 && ix < end_index as i32 {
                self.sorted_indexes.borrow_mut().as_mut().expect("sorted indexes").remove(i);
                let row = removed_rows[(ix as usize) - start_index].clone();
                self.raise(&NotifyCollectionChangedEventArgs {
                    action: NotifyCollectionChangedAction::Remove,
                    new_items: &[],
                    old_items: std::slice::from_ref(&row),
                    new_starting_index: -1,
                    old_starting_index: i as i32,
                });
                continue;
            } else if ix >= end_index as i32 {
                let new_ix = ix - count as i32;
                self.sorted_indexes.borrow_mut().as_mut().expect("sorted indexes")[i] = new_ix;
                let row = self.unsorted_rows.borrow().as_ref().expect("rows created")[new_ix as usize].clone();
                row.update_model_index(-(count as i32));
            }
            i += 1;
        }
    }

    fn on_items_collection_changed_sorted(&self, e: &ModelsChangedArgs<'_, M>) {
        if self.unsorted_rows.borrow().is_none() {
            return;
        }
        match e.action {
            NotifyCollectionChangedAction::Add => self.add_sorted(e.new_starting_index as usize, e.new_items.len()),
            NotifyCollectionChangedAction::Remove => self.remove_sorted(e.old_starting_index as usize, e.old_items.len()),
            NotifyCollectionChangedAction::Replace => {
                self.remove_sorted(e.old_starting_index as usize, e.old_items.len());
                self.add_sorted(e.new_starting_index as usize, e.new_items.len());
            }
            NotifyCollectionChangedAction::Move => {
                self.move_rows(e.old_starting_index as usize, e.new_starting_index as usize, e.old_items.len());
                let sorted = self.create_sorted_indexes();
                *self.sorted_indexes.borrow_mut() = Some(sorted);
                self.raise_reset();
            }
            NotifyCollectionChangedAction::Reset => {
                self.reset_rows();
                self.raise_reset();
            }
        }
    }
}
