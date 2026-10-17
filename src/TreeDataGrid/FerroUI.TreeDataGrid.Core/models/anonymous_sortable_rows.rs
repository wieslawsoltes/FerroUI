//! `Models/AnonymousSortableRows.cs` (with `AnonymousRowItems.cs`): the rows of a flat source,
//! one reused row object, sorted through a map of indexes.

use super::collection_extensions::has_known_indexes;
use super::{AnonymousRow, Comparison, IRows, RowRef, RowsChangedArgs, RowsChangedEvent, RowsChangedHandler};
use crate::accessor::TreeModel;
use crate::model_collection::{reset_args, ModelsChangedArgs};
use crate::utils::{sort_helper, stable_sort};
use crate::{IndexPath, TreeDataGridItemsSourceView};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::{Rc, Weak};

/// The rows of the models of a flat source.
pub struct AnonymousSortableRows<M: TreeModel> {
    weak_self: Weak<Self>,
    row: Rc<AnonymousRow<M>>,
    items: RefCell<Rc<TreeDataGridItemsSourceView<M>>>,
    items_token: Cell<Option<u64>>,
    comparer: RefCell<Option<Comparison<M>>>,
    sorted_indexes: RefCell<Option<Vec<i32>>>,
    changed: RowsChangedEvent,
}

impl<M: TreeModel> AnonymousSortableRows<M> {
    pub fn new(items: Rc<TreeDataGridItemsSourceView<M>>, comparer: Option<Comparison<M>>) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            row: AnonymousRow::new(),
            items: RefCell::new(items.clone()),
            items_token: Cell::new(None),
            comparer: RefCell::new(comparer),
            sorted_indexes: RefCell::new(None),
            changed: RowsChangedEvent::default(),
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

    /// The reused row, pointed at the model at `index`.
    pub fn get_typed(&self, index: usize) -> Rc<AnonymousRow<M>> {
        let items = self.items.borrow().clone();
        if self.comparer.borrow().is_none() {
            self.row.update(index as i32, items.get_at(index));
            return self.row.clone();
        }
        self.ensure_sorted_indexes();
        let model_index = self.sorted_indexes.borrow().as_ref().expect("sorted indexes")[index];
        self.row.update(model_index, items.get_at(model_index as usize));
        self.row.clone()
    }

    fn ensure_sorted_indexes(&self) {
        if self.sorted_indexes.borrow().is_none() {
            let sorted = self.create_sorted_indexes();
            *self.sorted_indexes.borrow_mut() = Some(sorted);
        }
    }

    /// Releases the models and stops listening (`Dispose`).
    pub fn dispose(&self) {
        self.set_items(TreeDataGridItemsSourceView::empty());
    }

    /// Shows the models of another view.
    pub fn set_items(&self, items: Rc<TreeDataGridItemsSourceView<M>>) {
        if let Some(token) = self.items_token.take() {
            let old = self.items.borrow().clone();
            old.remove_collection_changed(token);
        }
        *self.items.borrow_mut() = items.clone();
        self.subscribe(&items);
        self.on_items_collection_changed(&reset_args::<M>());
    }

    /// Sorts the rows (`None`: the order of the models). Raises no notification: the source does.
    pub fn sort(&self, comparer: Option<Comparison<M>>) {
        let has = comparer.is_some();
        *self.comparer.borrow_mut() = comparer;
        let sorted = if has { Some(self.create_sorted_indexes()) } else { None };
        *self.sorted_indexes.borrow_mut() = sorted;
    }

    fn compare_items_by_index(&self) -> impl FnMut(&i32, &i32) -> Ordering {
        let items = self.items.borrow().clone();
        let comparer = self.comparer.borrow().clone().expect("a comparer");
        move |&index1: &i32, &index2: &i32| match comparer(&items.get_at(index1 as usize), &items.get_at(index2 as usize)) {
            Ordering::Equal => index1.cmp(&index2),
            order => order,
        }
    }

    fn create_sorted_indexes(&self) -> Vec<i32> {
        let count = self.items.borrow().count();
        let mut compare = self.compare_items_by_index();
        stable_sort::sorted_map(count, &mut compare)
    }

    fn rows_of(index: i32, models: &[Rc<M>]) -> Vec<RowRef> {
        models.iter().enumerate().map(|(i, m)| AnonymousRow::of(index.max(0) + i as i32, m.clone()) as RowRef).collect()
    }

    fn on_items_collection_changed(&self, e: &ModelsChangedArgs<'_, M>) {
        let reset = reset_args::<M>();
        let e = if has_known_indexes(e) { e } else { &reset };
        if self.comparer.borrow().is_none() {
            self.on_items_collection_changed_unsorted(e);
        } else {
            self.on_items_collection_changed_sorted(e);
        }
    }

    fn on_items_collection_changed_unsorted(&self, e: &ModelsChangedArgs<'_, M>) {
        if !self.changed.has_handlers() {
            return;
        }
        let new_items = Self::rows_of(e.new_starting_index, e.new_items);
        let old_items = Self::rows_of(e.old_starting_index, e.old_items);
        let args = match e.action {
            NotifyCollectionChangedAction::Add => RowsChangedArgs {
                action: e.action,
                new_items: &new_items,
                old_items: &[],
                new_starting_index: e.new_starting_index,
                old_starting_index: -1,
            },
            NotifyCollectionChangedAction::Remove => RowsChangedArgs {
                action: e.action,
                new_items: &[],
                old_items: &old_items,
                new_starting_index: -1,
                old_starting_index: e.old_starting_index,
            },
            // The reference's arguments of a replace have the old index as the index of both.
            NotifyCollectionChangedAction::Replace => RowsChangedArgs {
                action: e.action,
                new_items: &new_items,
                old_items: &old_items,
                new_starting_index: e.old_starting_index,
                old_starting_index: e.old_starting_index,
            },
            NotifyCollectionChangedAction::Move => RowsChangedArgs {
                action: e.action,
                new_items: &new_items,
                old_items: &new_items,
                new_starting_index: e.new_starting_index,
                old_starting_index: e.old_starting_index,
            },
            NotifyCollectionChangedAction::Reset => RowsChangedArgs {
                action: e.action,
                new_items: &[],
                old_items: &[],
                new_starting_index: -1,
                old_starting_index: -1,
            },
        };
        self.changed.raise(&args);
    }

    fn raise_reset(&self) {
        self.changed.raise(&NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Reset,
            new_items: &[],
            old_items: &[],
            new_starting_index: -1,
            old_starting_index: -1,
        });
    }

    fn add_sorted(&self, start_index: usize, count: usize) {
        {
            let mut sorted = self.sorted_indexes.borrow_mut();
            for ix in sorted.as_mut().expect("sorted indexes").iter_mut() {
                if *ix >= start_index as i32 {
                    *ix += count as i32;
                }
            }
        }
        let items = self.items.borrow().clone();
        for i in 0..count {
            let model_index = (start_index + i) as i32;
            let sorted = self.sorted_indexes.borrow().clone().expect("sorted indexes");
            let mut compare = self.compare_items_by_index();
            let mut index = sort_helper::binary_search(&sorted, &model_index, &mut compare);
            if index < 0 {
                index = !index;
            }
            self.sorted_indexes.borrow_mut().as_mut().expect("sorted indexes").insert(index as usize, model_index);
            self.row.update(model_index, items.get_at(model_index as usize));
            let row = self.row.clone() as RowRef;
            self.changed.raise(&RowsChangedArgs {
                action: NotifyCollectionChangedAction::Add,
                new_items: std::slice::from_ref(&row),
                old_items: &[],
                new_starting_index: index,
                old_starting_index: -1,
            });
        }
    }

    fn remove_sorted(&self, start_index: usize, removed: &[Rc<M>]) {
        let count = removed.len();
        let end_index = start_index + count;
        let mut i = 0usize;
        loop {
            let ix = match self.sorted_indexes.borrow().as_ref().expect("sorted indexes").get(i) {
                Some(&ix) => ix,
                None => break,
            };
            if ix >= start_index as i32 && ix < end_index as i32 {
                self.sorted_indexes.borrow_mut().as_mut().expect("sorted indexes").remove(i);
                self.row.update(ix, removed[ix as usize - start_index].clone());
                let row = self.row.clone() as RowRef;
                self.changed.raise(&RowsChangedArgs {
                    action: NotifyCollectionChangedAction::Remove,
                    new_items: &[],
                    old_items: std::slice::from_ref(&row),
                    new_starting_index: -1,
                    old_starting_index: i as i32,
                });
                continue;
            } else if ix >= end_index as i32 {
                self.sorted_indexes.borrow_mut().as_mut().expect("sorted indexes")[i] = ix - count as i32;
            }
            i += 1;
        }
    }

    fn on_items_collection_changed_sorted(&self, e: &ModelsChangedArgs<'_, M>) {
        // Rows that were never read: the type of the change does not matter, the presenter only
        // has to know that the collection changed.
        if self.sorted_indexes.borrow().is_none() {
            self.raise_reset();
            return;
        }
        match e.action {
            NotifyCollectionChangedAction::Add => self.add_sorted(e.new_starting_index as usize, e.new_items.len()),
            NotifyCollectionChangedAction::Remove => self.remove_sorted(e.old_starting_index as usize, e.old_items),
            NotifyCollectionChangedAction::Replace | NotifyCollectionChangedAction::Move => {
                self.remove_sorted(e.old_starting_index as usize, e.old_items);
                self.add_sorted(e.new_starting_index as usize, e.new_items.len());
            }
            NotifyCollectionChangedAction::Reset => {
                let sorted = self.create_sorted_indexes();
                *self.sorted_indexes.borrow_mut() = Some(sorted);
                self.raise_reset();
            }
        }
    }
}

impl<M: TreeModel> IRows for AnonymousSortableRows<M> {
    fn count(&self) -> usize {
        match self.sorted_indexes.borrow().as_ref() {
            Some(sorted) => sorted.len(),
            None => self.items.borrow().count(),
        }
    }

    fn get(&self, index: usize) -> RowRef {
        self.get_typed(index)
    }

    fn model_index_to_row_index(&self, model_index: &IndexPath) -> i32 {
        let count = self.items.borrow().count();
        if model_index.count() != 1 || model_index.get(0) < 0 || model_index.get(0) as usize >= count {
            return -1;
        }
        if self.comparer.borrow().is_some() {
            self.ensure_sorted_indexes();
        }
        let i = model_index.get(0);
        let sorted = self.sorted_indexes.borrow().clone();
        match sorted {
            None => i,
            Some(sorted) => {
                let mut compare = self.compare_items_by_index();
                sort_helper::binary_search(&sorted, &i, &mut compare)
            }
        }
    }

    fn row_index_to_model_index(&self, row_index: i32) -> IndexPath {
        if row_index < 0 || row_index as usize >= self.items.borrow().count() {
            return IndexPath::UNSELECTED;
        }
        if self.comparer.borrow().is_some() {
            self.ensure_sorted_indexes();
        }
        match self.sorted_indexes.borrow().as_ref() {
            Some(sorted) => IndexPath::new(sorted[row_index as usize]),
            None => IndexPath::new(row_index),
        }
    }

    fn add_collection_changed(&self, handler: Rc<RowsChangedHandler>) -> u64 {
        self.changed.add(handler)
    }

    fn remove_collection_changed(&self, token: u64) {
        self.changed.remove(token)
    }
}
