//! `HierarchicalTreeDataGridSource.cs`: a source that shows its models as a tree with columns.

use crate::accessor::TreeModel;
use crate::flat_tree_data_grid_source::same_models;
use crate::i_tree_data_grid_source::{ITreeDataGridSource, ITreeDataGridSourceOf, RowDropPosition, RowMoveEffects};
use crate::model_collection::{Models, MutableModelCollection};
use crate::models::{
    ColumnList, ColumnRef, Comparison, HierarchicalRow, HierarchicalRows, IColumn, IExpanderColumn, IExpanderRow,
    IExpanderRowController, IRows, ListSortDirection, NotifyingBase, RowEventArgs, RowsChangedArgs,
};
use crate::selection::{
    downcast_models, downcast_selection, erase_models, ITreeDataGridSelection, TreeDataGridRowSelectionModel, UntypedModels,
};
use crate::{IndexPath, TreeDataGridItemsSourceView};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A handler of the row events of a hierarchical source.
pub type RowEventHandler<M> = dyn Fn(&RowEventArgs<Rc<HierarchicalRow<M>>>);

/// A source of a tree with columns: the rows of the root models, each expandable to the rows of
/// its children through the expander column.
pub struct HierarchicalTreeDataGridSource<M: TreeModel> {
    weak_self: Weak<Self>,
    notifying: NotifyingBase,
    items: RefCell<Models<M>>,
    items_view: RefCell<Rc<TreeDataGridItemsSourceView<M>>>,
    expander_column: RefCell<Option<Rc<dyn IExpanderColumn<M>>>>,
    rows: RefCell<Option<Rc<HierarchicalRows<M>>>>,
    comparison: RefCell<Option<Comparison<M>>>,
    selection: RefCell<Option<Rc<dyn ITreeDataGridSelection>>>,
    is_selection_set: Cell<bool>,
    columns: ColumnList<M>,
    sorted: HandlerList<dyn Fn()>,
    row_expanding: HandlerList<RowEventHandler<M>>,
    row_expanded: HandlerList<RowEventHandler<M>>,
    row_collapsing: HandlerList<RowEventHandler<M>>,
    row_collapsed: HandlerList<RowEventHandler<M>>,
}

impl<M: TreeModel> HierarchicalTreeDataGridSource<M> {
    /// A source of `items`.
    pub fn new(items: Models<M>) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            notifying: NotifyingBase::new(),
            items_view: RefCell::new(TreeDataGridItemsSourceView::get_or_create(Some(items.clone()))),
            items: RefCell::new(items),
            expander_column: RefCell::new(None),
            rows: RefCell::new(None),
            comparison: RefCell::new(None),
            selection: RefCell::new(None),
            is_selection_set: Cell::new(false),
            columns: ColumnList::new_hierarchical(),
            sorted: HandlerList::new(),
            row_expanding: HandlerList::new(),
            row_expanded: HandlerList::new(),
            row_collapsing: HandlerList::new(),
            row_collapsed: HandlerList::new(),
        });
        let weak = Rc::downgrade(&this);
        this.columns.list().add_collection_changed(Rc::new(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_columns_collection_changed(e);
            }
        }));
        this
    }

    /// A source of one root model.
    pub fn from_item(item: Rc<M>) -> Rc<Self> {
        Self::new(Rc::new(vec![item]))
    }

    fn this(&self) -> Rc<Self> {
        self.weak_self.upgrade().expect("a source that is alive")
    }

    pub fn columns(&self) -> &ColumnList<M> {
        &self.columns
    }

    /// Adds a column (the reference's collection initializer `Columns = { ... }`).
    pub fn with_column(self: &Rc<Self>, column: ColumnRef<M>) -> Rc<Self> {
        self.columns.add(column);
        self.clone()
    }

    pub fn items(&self) -> Models<M> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Models<M>) {
        if same_models(&self.items.borrow(), &value) {
            return;
        }
        *self.items.borrow_mut() = value.clone();
        let view = TreeDataGridItemsSourceView::get_or_create(Some(value.clone()));
        *self.items_view.borrow_mut() = view.clone();
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.set_items(view);
        }
        let selection = self.selection.borrow().clone();
        if let Some(selection) = selection {
            selection.set_source(Some(erase_models(value)));
        }
        self.notifying.raise_property_changed("Items");
    }

    /// The rows, created on first access.
    ///
    /// # Panics
    /// When the source has no columns or no expander column (`InvalidOperationException`).
    pub fn typed_rows(&self) -> Rc<HierarchicalRows<M>> {
        self.get_or_create_rows()
    }

    /// The selection of rows, if the selection is one (`RowSelection`).
    pub fn row_selection(&self) -> Option<Rc<TreeDataGridRowSelectionModel<M>>> {
        self.selection().and_then(|selection| downcast_selection::<TreeDataGridRowSelectionModel<M>>(&selection))
    }

    pub fn is_sorted(&self) -> bool {
        self.comparison.borrow().is_some()
    }

    pub fn add_row_expanding(&self, handler: Rc<RowEventHandler<M>>) -> u64 {
        self.row_expanding.add(handler)
    }
    pub fn remove_row_expanding(&self, token: u64) {
        self.row_expanding.remove(token);
    }
    pub fn add_row_expanded(&self, handler: Rc<RowEventHandler<M>>) -> u64 {
        self.row_expanded.add(handler)
    }
    pub fn remove_row_expanded(&self, token: u64) {
        self.row_expanded.remove(token);
    }
    pub fn add_row_collapsing(&self, handler: Rc<RowEventHandler<M>>) -> u64 {
        self.row_collapsing.add(handler)
    }
    pub fn remove_row_collapsing(&self, token: u64) {
        self.row_collapsing.remove(token);
    }
    pub fn add_row_collapsed(&self, handler: Rc<RowEventHandler<M>>) -> u64 {
        self.row_collapsed.add(handler)
    }
    pub fn remove_row_collapsed(&self, token: u64) {
        self.row_collapsed.remove(token);
    }

    /// Releases the rows (`Dispose`).
    pub fn dispose(&self) {
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.dispose();
        }
    }

    pub fn collapse(&self, index: &IndexPath) {
        self.get_or_create_rows().collapse(index);
    }

    pub fn collapse_all(&self) {
        self.get_or_create_rows().expand_collapse_recursive(&|_| false, None);
    }

    pub fn expand(&self, index: &IndexPath) {
        self.get_or_create_rows().expand(index);
    }

    pub fn expand_all(&self) {
        self.get_or_create_rows().expand_collapse_recursive(&|_| true, None);
    }

    /// Expands the models for which `predicate` holds and collapses the others.
    pub fn expand_collapse_recursive(&self, predicate: &dyn Fn(&Rc<M>) -> bool) {
        self.get_or_create_rows().expand_collapse_recursive(predicate, None);
    }

    /// [`expand_collapse_recursive`](Self::expand_collapse_recursive) below `row`.
    pub fn expand_collapse_recursive_from(&self, row: &Rc<HierarchicalRow<M>>, predicate: &dyn Fn(&Rc<M>) -> bool) {
        self.get_or_create_rows().expand_collapse_recursive(predicate, Some(row));
    }

    /// The model at `index`.
    ///
    /// # Panics
    /// When the source has no expander column (`InvalidOperationException`).
    pub fn try_get_model_at(&self, index: &IndexPath) -> Option<Rc<M>> {
        let expander = self.expander_column.borrow().clone().unwrap_or_else(|| panic!("No expander column defined."));
        let mut items: Option<Models<M>> = Some(self.items());
        let count = index.count();
        for depth in 0..count {
            let i = index.get(depth);
            match &items {
                Some(list) if i >= 0 && (i as usize) < list.count() => {
                    let e = list.get(i as usize);
                    if depth < count - 1 {
                        items = expander.get_child_models(&e);
                    } else {
                        return Some(e);
                    }
                }
                _ => break,
            }
        }
        None
    }

    /// Sorts the rows by `comparison`.
    pub fn sort(&self, comparison: Option<Comparison<M>>) {
        *self.comparison.borrow_mut() = comparison.clone();
        let rows = self.rows.borrow().clone();
        if let Some(rows) = rows {
            rows.sort(comparison);
        }
    }

    /// Sorts by `column` (one of the columns).
    pub fn sort_by_column(&self, column: &ColumnRef<M>, direction: ListSortDirection) -> bool {
        if self.columns.index_of(column).is_none() {
            return false;
        }
        let Some(comparison) = column.get_comparison(direction) else {
            return false;
        };
        self.sort(Some(comparison));
        self.raise_sorted();
        for c in self.columns.to_vec() {
            let same = std::ptr::addr_eq(Rc::as_ptr(&c), Rc::as_ptr(column));
            c.set_sort_direction(if same { Some(direction) } else { None });
        }
        true
    }

    fn raise_sorted(&self) {
        for (_, handler) in self.sorted.snapshot().iter() {
            handler();
        }
    }

    /// The child models of `model`.
    ///
    /// # Panics
    /// When the source has no expander column (`InvalidOperationException`).
    pub fn get_model_children(&self, model: &Rc<M>) -> Option<Models<M>> {
        let expander = self.expander_column.borrow().clone().unwrap_or_else(|| panic!("No expander column defined."));
        expander.get_child_models(model)
    }

    /// The row index of the model at `index`, searched from `from_row_index` (-1: no row).
    pub fn get_row_index(&self, index: &IndexPath, from_row_index: usize) -> i32 {
        self.rows.borrow().as_ref().and_then(|rows| rows.try_get_row_index(index, from_row_index)).unwrap_or(-1)
    }

    fn get_or_create_rows(&self) -> Rc<HierarchicalRows<M>> {
        if let Some(rows) = self.rows.borrow().as_ref() {
            return rows.clone();
        }
        if self.columns.count() == 0 {
            panic!("No columns defined.");
        }
        let Some(expander) = self.expander_column.borrow().clone() else {
            panic!("No expander column defined.");
        };
        let controller: Weak<dyn IExpanderRowController<M>> = self.weak_self.clone();
        let rows = HierarchicalRows::new(controller, self.items_view.borrow().clone(), expander, self.comparison.borrow().clone());
        *self.rows.borrow_mut() = Some(rows.clone());
        rows
    }

    fn on_columns_collection_changed(&self, e: &ferroui_base::collections::NotifyCollectionChangedEventArgs<'_, ColumnRef<M>>) {
        match e.action {
            NotifyCollectionChangedAction::Add => {
                for column in e.new_items {
                    if column.as_expander_column_any() {
                        if self.expander_column.borrow().is_some() {
                            panic!("Only one expander column is allowed.");
                        }
                        *self.expander_column.borrow_mut() = Some(Self::expander_of(column));
                        break;
                    }
                }
            }
            NotifyCollectionChangedAction::Remove => self.handle_remove_replace_or_move(e.old_items, "removed"),
            NotifyCollectionChangedAction::Replace => self.handle_remove_replace_or_move(e.old_items, "replaced"),
            // A reordering does not replace the expander of the rows.
            NotifyCollectionChangedAction::Move => {}
            NotifyCollectionChangedAction::Reset => {
                let expanders: Vec<ColumnRef<M>> =
                    self.columns.to_vec().into_iter().filter(|c| c.as_expander_column_any()).collect();
                if expanders.len() > 1 {
                    panic!("Sequence contains more than one element");
                }
                *self.expander_column.borrow_mut() = expanders.first().map(Self::expander_of);
            }
        }
    }

    fn expander_of(column: &ColumnRef<M>) -> Rc<dyn IExpanderColumn<M>> {
        let any: Rc<dyn Any> = column.clone();
        match any.downcast::<crate::models::HierarchicalExpanderColumn<M>>() {
            Ok(expander) => expander as Rc<dyn IExpanderColumn<M>>,
            Err(_) => Rc::new(ExpanderColumnRef(column.clone())) as Rc<dyn IExpanderColumn<M>>,
        }
    }

    fn handle_remove_replace_or_move(&self, items: &[ColumnRef<M>], action: &str) {
        for column in items {
            if column.as_expander_column_any() && self.expander_column.borrow().is_some() {
                panic!("The expander column cannot be {action}.");
            }
        }
    }

    fn get_items(&self, path: &IndexPath) -> Models<M> {
        let children = if path.count() == 0 {
            Some(self.items())
        } else {
            match self.try_get_model_at(path) {
                Some(parent) => self.get_model_children(&parent),
                None => panic!("Index was outside the bounds of the array."),
            }
        };
        let Some(children) = children else {
            panic!("The requested drop target has no children.");
        };
        if children.as_mutable().is_none() {
            panic!("Items does not implement IList<T>.");
        }
        children
    }

    fn try_get_items(&self, path: &IndexPath) -> Option<Models<M>> {
        let children = if path.count() == 0 {
            Some(self.items())
        } else {
            self.try_get_model_at(path).and_then(|parent| self.get_model_children(&parent))
        };
        children.filter(|c| c.as_mutable().is_some())
    }

    fn move_rows_core(&self, indexes: &[IndexPath], target_index: &IndexPath, position: RowDropPosition, effects: RowMoveEffects) {
        if effects != RowMoveEffects::MOVE {
            panic!("Only move is currently supported for drag/drop.");
        }
        if self.is_sorted() {
            panic!("Drag/drop is not supported on sorted data.");
        }
        if position == RowDropPosition::None {
            return;
        }
        let mut ordered: Vec<IndexPath> = indexes.to_vec();
        ordered.sort();
        if ordered.is_empty() {
            return;
        }
        for i in 1..ordered.len() {
            if ordered[i] == ordered[i - 1] {
                panic!("Duplicate source index. (Parameter 'indexes')");
            }
        }
        for source_index in &ordered {
            if source_index.is_ancestor_of(target_index) || (source_index == target_index && position == RowDropPosition::Inside) {
                panic!("A row cannot be moved into itself or one of its descendants.");
            }
        }

        let (target_items, original_target_parent_path, mut ti) = if position == RowDropPosition::Inside {
            let items = self.get_items(target_index);
            let count = items.count() as i32;
            (items, target_index.clone(), count)
        } else {
            if target_index.count() == 0 {
                panic!("Invalid target index. (Parameter 'targetIndex')");
            }
            let parent = target_index.parent();
            let items = self.get_items(&parent);
            let ti = target_index.last();
            if ti < 0 || ti as usize >= items.count() {
                panic!("Specified argument was out of the range of valid values. (Parameter 'targetIndex')");
            }
            (items, parent, ti)
        };
        if position == RowDropPosition::After {
            ti += 1;
        }

        struct SourceItem<M: 'static> {
            items: Models<M>,
            index: i32,
            item: Rc<M>,
        }
        let source_items: Vec<SourceItem<M>> = ordered
            .iter()
            .map(|x| {
                if x.count() == 0 {
                    panic!("Invalid source index. (Parameter 'indexes')");
                }
                let parent = x.parent();
                let items = self.get_items(&parent);
                let index = x.last();
                if index < 0 || index as usize >= items.count() {
                    panic!("Specified argument was out of the range of valid values. (Parameter 'indexes')");
                }
                let item = items.get(index as usize);
                SourceItem { items, index, item }
            })
            .collect();

        // The source items grouped by collection, in order of first appearance.
        let mut groups: Vec<Vec<usize>> = Vec::new();
        for (i, source) in source_items.iter().enumerate() {
            match groups.iter_mut().find(|g| same_models(&source_items[g[0]].items, &source.items)) {
                Some(group) => group.push(i),
                None => groups.push(vec![i]),
            }
        }
        for group in &groups {
            let mut seen: Vec<i32> = group.iter().map(|&i| source_items[i].index).collect();
            seen.sort();
            seen.dedup();
            if seen.len() != group.len() {
                panic!("Duplicate physical source index. (Parameter 'indexes')");
            }
        }

        let contains_target_collection = |root: &Rc<M>| -> bool {
            let Some(expander) = self.expander_column.borrow().clone() else {
                return false;
            };
            let mut pending = vec![root.clone()];
            let mut visited: Vec<*const M> = Vec::new();
            while let Some(model) = pending.pop() {
                let key = Rc::as_ptr(&model);
                if visited.contains(&key) {
                    continue;
                }
                visited.push(key);
                let children = expander.get_child_models(&model);
                if let Some(children) = &children {
                    if same_models(children, &target_items) {
                        return true;
                    }
                    for i in 0..children.count() {
                        pending.push(children.get(i));
                    }
                }
            }
            false
        };
        if source_items.iter().any(|x| !same_models(&x.items, &target_items) && contains_target_collection(&x.item)) {
            panic!("A row cannot be moved into a collection in its own subtree.");
        }

        let original_target_offset = ti;
        ti -= source_items.iter().filter(|x| same_models(&x.items, &target_items) && x.index < original_target_offset).count() as i32;

        let map_after_removals = |original: &IndexPath| -> IndexPath {
            let mut result = original.to_vec();
            for (depth, slot) in result.iter_mut().enumerate() {
                let parent = original.slice(0, depth);
                let Some(parent_items) = self.try_get_items(&parent) else {
                    continue;
                };
                *slot -= source_items.iter().filter(|x| same_models(&x.items, &parent_items) && x.index < original.get(depth)).count() as i32;
            }
            IndexPath::from_indexes(&result)
        };
        let insert_index = ti;
        let target_parent_path = map_after_removals(&original_target_parent_path);

        let is_valid_model_path = |path: &IndexPath| -> bool {
            if path.count() == 0 {
                return false;
            }
            match self.try_get_items(&path.parent()) {
                Some(items) => path.last() >= 0 && (path.last() as usize) < items.count(),
                None => false,
            }
        };

        let map_final_path = |original: &IndexPath| -> (IndexPath, bool) {
            let occurrences: Vec<(Option<Models<M>>, i32)> =
                (0..original.count()).map(|depth| (self.try_get_items(&original.slice(0, depth)), original.get(depth))).collect();
            let after_removal = map_after_removals(original);
            let mut source_offset: i32 = -1;
            let mut source_depth: i32 = -1;
            for (i, source) in source_items.iter().enumerate() {
                for (depth, (items, index)) in occurrences.iter().enumerate() {
                    if items.as_ref().is_some_and(|items| same_models(items, &source.items))
                        && *index == source.index
                        && depth as i32 + 1 > source_depth
                    {
                        source_offset = i as i32;
                        source_depth = depth as i32 + 1;
                    }
                }
            }
            if source_offset >= 0 {
                let mut path = target_parent_path.append(insert_index + source_offset);
                let rest = after_removal.slice(source_depth as usize, after_removal.count() - source_depth as usize);
                for relative in rest.iter() {
                    path = path.append(relative);
                }
                return (path, true);
            }
            let mut indexes = after_removal.to_vec();
            for (depth, (items, _)) in occurrences.iter().enumerate() {
                if items.as_ref().is_some_and(|items| same_models(items, &target_items)) && indexes[depth] >= insert_index {
                    indexes[depth] += source_items.len() as i32;
                }
            }
            (IndexPath::from_indexes(&indexes), false)
        };

        let selection = self.selection.borrow().clone().and_then(|s| s.row_selection_model());
        let original_primary_selection = selection.as_ref().map(|s| s.selected_index()).unwrap_or_default();
        let original_selections: Option<Vec<IndexPath>> = selection.as_ref().map(|s| {
            let indexes = s.selected_indexes();
            (0..indexes.count()).map(|i| indexes.get(i)).collect()
        });
        let mapped_anchor = selection
            .as_ref()
            .map(|s| s.anchor_index())
            .filter(|a| a.count() > 0 && is_valid_model_path(a))
            .map(|a| map_final_path(&a).0)
            .unwrap_or_default();
        let mapped_range_anchor = selection
            .as_ref()
            .map(|s| s.range_anchor_index())
            .filter(|a| a.count() > 0 && is_valid_model_path(a))
            .map(|a| map_final_path(&a).0)
            .unwrap_or_default();
        let mapped_selections: Option<Vec<(IndexPath, (IndexPath, bool))>> =
            original_selections.as_ref().map(|all| all.iter().map(|original| (original.clone(), map_final_path(original))).collect());
        let rows = self.rows.borrow().clone();
        let mapped_expansions: Option<Vec<IndexPath>> = rows.as_ref().map(|rows| {
            let mut mapped: Vec<IndexPath> = Vec::new();
            for expanded in rows.get_expanded_model_indexes() {
                let path = map_final_path(&expanded).0;
                if !mapped.contains(&path) {
                    mapped.push(path);
                }
            }
            mapped.sort_by_key(|x| x.count());
            mapped
        });
        let refresh_rows =
            rows.is_some() && (!target_items.is_notifying() || source_items.iter().any(|x| !x.items.is_notifying()));

        if let (Some(selection), Some(mapped)) = (&selection, &mapped_selections) {
            for (original, (_, moved)) in mapped {
                if *moved {
                    selection.deselect(original);
                }
            }
        }
        for group in &groups {
            let mut members: Vec<usize> = group.clone();
            members.sort_by(|a, b| source_items[*b].index.cmp(&source_items[*a].index));
            for i in members {
                let source = &source_items[i];
                source.items.as_mutable().expect("a mutable collection").remove_at(source.index as usize);
            }
        }
        {
            let target: &dyn MutableModelCollection<M> = target_items.as_mutable().expect("a mutable collection");
            for source in &source_items {
                target.insert(ti as usize, source.item.clone());
                ti += 1;
            }
        }
        if refresh_rows {
            if let Some(rows) = &rows {
                rows.set_items(self.items_view.borrow().clone());
            }
        }
        if let (Some(rows), Some(expansions)) = (&rows, &mapped_expansions) {
            for expanded in expansions {
                rows.expand(expanded);
            }
        }
        if let Some(selection) = selection {
            selection.begin_batch_update();
            if original_selections.as_ref().is_some_and(|s| !s.is_empty()) {
                let restored: Vec<(IndexPath, IndexPath)> = mapped_selections
                    .expect("mapped selections")
                    .into_iter()
                    .map(|(original, (path, _))| (original, path))
                    .collect();
                let primary_moved_index = restored.iter().position(|(original, _)| *original == original_primary_selection);
                selection.clear();
                if let Some(primary) = primary_moved_index {
                    selection.select(&restored[primary].1);
                }
                for (i, (_, path)) in restored.iter().enumerate() {
                    if Some(i) != primary_moved_index {
                        selection.select(path);
                    }
                }
            }
            selection.set_anchor_index(mapped_anchor);
            selection.set_range_anchor_index(mapped_range_anchor);
            selection.end_batch_update();
        }
    }
}

/// An expander column that is not a [`HierarchicalExpanderColumn`](crate::models::HierarchicalExpanderColumn):
/// reached through the column's own expander interface.
struct ExpanderColumnRef<M: TreeModel>(ColumnRef<M>);

impl<M: TreeModel> ExpanderColumnRef<M> {
    fn expander(&self) -> &dyn IExpanderColumn<M> {
        self.0.as_expander_column().expect("an expander column")
    }
}

impl<M: TreeModel> INotifyPropertyChanged for ExpanderColumnRef<M> {
    fn property_changed(&self) -> &Event<str> {
        self.0.property_changed()
    }
}

impl<M: TreeModel> IColumn for ExpanderColumnRef<M> {
    fn id(&self) -> String {
        self.0.id()
    }
    fn header(&self) -> Option<BoxedValue> {
        self.0.header()
    }
    fn width(&self) -> crate::GridLength {
        self.0.width()
    }
    fn set_width(&self, value: crate::GridLength) {
        self.0.set_width(value)
    }
    fn is_visible(&self) -> bool {
        self.0.is_visible()
    }
    fn set_is_visible(&self, value: bool) {
        self.0.set_is_visible(value)
    }
    fn presentation_key(&self) -> Option<String> {
        self.0.presentation_key()
    }
    fn set_presentation_key(&self, value: Option<String>) {
        self.0.set_presentation_key(value)
    }
    fn sort_direction(&self) -> Option<ListSortDirection> {
        self.0.sort_direction()
    }
    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.0.set_sort_direction(value)
    }
    fn tag(&self) -> Option<BoxedValue> {
        self.0.tag()
    }
    fn set_tag(&self, value: Option<BoxedValue>) {
        self.0.set_tag(value)
    }
    fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }
    fn model_type(&self) -> std::any::TypeId {
        self.0.model_type()
    }
}

impl<M: TreeModel> crate::models::IColumnOf<M> for ExpanderColumnRef<M> {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        self.0.get_comparison(direction)
    }
    fn kind(&self) -> crate::models::ColumnKind<'_, M> {
        self.0.kind()
    }
    fn as_expander_column(&self) -> Option<&dyn IExpanderColumn<M>> {
        Some(self)
    }
    fn as_expander_column_any(&self) -> bool {
        true
    }
}

impl<M: TreeModel> IExpanderColumn<M> for ExpanderColumnRef<M> {
    fn has_children(&self, model: &Rc<M>) -> bool {
        self.expander().has_children(model)
    }
    fn get_model_is_expanded(&self, model: &Rc<M>) -> Option<bool> {
        self.expander().get_model_is_expanded(model)
    }
    fn get_child_models(&self, model: &Rc<M>) -> Option<Models<M>> {
        self.expander().get_child_models(model)
    }
    fn set_model_is_expanded(&self, row: &dyn IExpanderRow<M>) {
        self.expander().set_model_is_expanded(row)
    }
    fn expansion_observer(&self) -> Option<Rc<crate::PropertyPathObserver<M>>> {
        self.expander().expansion_observer()
    }
}

impl<M: TreeModel> INotifyPropertyChanged for HierarchicalTreeDataGridSource<M> {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl<M: TreeModel> ITreeDataGridSource for HierarchicalTreeDataGridSource<M> {
    fn column_count(&self) -> usize {
        self.columns.count()
    }

    fn column(&self, index: usize) -> Rc<dyn IColumn> {
        self.columns.get(index) as Rc<dyn IColumn>
    }

    fn rows(&self) -> Rc<dyn IRows> {
        self.get_or_create_rows()
    }

    fn selection(&self) -> Option<Rc<dyn ITreeDataGridSelection>> {
        if self.selection.borrow().is_none() && !self.is_selection_set.get() {
            let source: Rc<dyn ITreeDataGridSourceOf<M>> = self.this();
            let selection = TreeDataGridRowSelectionModel::new(&source);
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
        true
    }

    fn is_sorted(&self) -> bool {
        HierarchicalTreeDataGridSource::is_sorted(self)
    }

    fn items_untyped(&self) -> UntypedModels {
        erase_models(self.items())
    }

    fn get_model_children_untyped(&self, model: &BoxedValue) -> Option<Vec<BoxedValue>> {
        let any: Rc<dyn Any> = model.clone();
        let model = any.downcast::<M>().ok()?;
        self.get_model_children(&model).map(|children| (0..children.count()).map(|i| children.get(i) as BoxedValue).collect())
    }

    fn sort_by(&self, column: Option<&dyn IColumn>, direction: ListSortDirection) -> bool {
        let Some(column) = column else {
            return false;
        };
        match self.columns.index_of_untyped(column) {
            Some(index) => self.sort_by_column(&self.columns.get(index), direction),
            None => false,
        }
    }

    fn clear_sort(&self) {
        if self.comparison.borrow().is_none() {
            return;
        }
        self.sort(None);
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
        self.move_rows_core(indexes, target_index, position, effects);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<M: TreeModel> ITreeDataGridSourceOf<M> for HierarchicalTreeDataGridSource<M> {
    fn items(&self) -> Models<M> {
        HierarchicalTreeDataGridSource::items(self)
    }

    fn set_items(&self, value: Models<M>) {
        HierarchicalTreeDataGridSource::set_items(self, value);
    }

    fn columns(&self) -> &ColumnList<M> {
        &self.columns
    }

    fn get_model_children(&self, model: &Rc<M>) -> Option<Models<M>> {
        HierarchicalTreeDataGridSource::get_model_children(self, model)
    }
}

impl<M: TreeModel> IExpanderRowController<M> for HierarchicalTreeDataGridSource<M> {
    fn on_begin_expand_collapse(&self, row: &dyn IExpanderRow<M>) {
        if let Some(r) = typed_row(row) {
            let args = RowEventArgs::new(r);
            let handlers = if !row.is_expanded() { &self.row_expanding } else { &self.row_collapsing };
            for (_, handler) in handlers.snapshot().iter() {
                handler(&args);
            }
        }
    }

    fn on_end_expand_collapse(&self, row: &dyn IExpanderRow<M>) {
        if let Some(r) = typed_row(row) {
            let args = RowEventArgs::new(r);
            let handlers = if row.is_expanded() { &self.row_expanded } else { &self.row_collapsed };
            for (_, handler) in handlers.snapshot().iter() {
                handler(&args);
            }
        }
    }

    fn on_child_collection_changed(&self, _row: &dyn IExpanderRow<M>, _e: &RowsChangedArgs<'_>) {}

    fn on_child_collection_replaced(&self, row: &dyn IExpanderRow<M>, children: Option<&Models<M>>) {
        let Some(selection) = self.row_selection() else {
            return;
        };
        if let Some(indexable) = row.as_model_indexable() {
            selection.base().reset_children_source(&indexable.model_index_path(), children.cloned());
        }
    }

    fn replaces_child_collections(&self) -> bool {
        true
    }
}

/// The shared row of a row the controller is given.
fn typed_row<M: TreeModel>(row: &dyn IExpanderRow<M>) -> Option<Rc<HierarchicalRow<M>>> {
    row.as_any().downcast_ref::<HierarchicalRow<M>>().map(|r| r.shared())
}

