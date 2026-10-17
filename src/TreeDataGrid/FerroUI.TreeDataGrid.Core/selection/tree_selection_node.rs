//! `Selection/TreeSelectionNode.cs`: a node of the selection of a hierarchical source: the
//! selected ranges of one collection of the hierarchy and the nodes of its children.
//!
//! This source file is adapted from the WinUI project (<https://github.com/microsoft/microsoft-ui-xaml>),
//! MIT License, Copyright (c) Microsoft Corporation.
//!
//! Ownership: the selection model owns its root node, and each node owns its child nodes
//! (`Rc`); a node refers back to the model weakly. The node is the weak listener of the changes of
//! its source (`ICollectionChangedListener`).

use super::index_range::IndexRange;
use super::index_ranges::IndexRanges;
use super::selection_node_base::{same_source, SelectionNodeBase, SelectionNodeBaseImpl};
use super::tree_selection_model_base::{Operation, TreeSelectionModelBase};
use crate::accessor::TreeModel;
use crate::items_source_view::TreeDataGridItemsSourceView;
use crate::model_collection::{Models, ModelsChangedArgs};
use crate::models::collection_extensions::{has_known_indexes, insert_many};
use crate::utils::ICollectionChangedListener;
use crate::IndexPath;
use ferroui_base::collections::NotifyCollectionChangedAction;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Children<M> = Vec<Option<Rc<TreeSelectionNode<M>>>>;

/// A node of a tree selection: the selection of the children of the model at [`path`](Self::path)
/// (the root node: of the source).
pub struct TreeSelectionNode<M: TreeModel> {
    base: SelectionNodeBase<M>,
    owner: Weak<TreeSelectionModelBase<M>>,
    path: RefCell<IndexPath>,
    children: RefCell<Option<Children<M>>>,
}

impl<M: TreeModel> TreeSelectionNode<M> {
    /// A root node of the selection model `owner`.
    pub fn new(owner: Weak<TreeSelectionModelBase<M>>) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| {
            let derived: Weak<dyn SelectionNodeBaseImpl<M>> = this.clone();
            let listener: Weak<dyn ICollectionChangedListener<M>> = this.clone();
            let base = SelectionNodeBase::new(derived, listener);
            base.set_ranges_enabled(true);
            Self { base, owner, path: RefCell::new(IndexPath::UNSELECTED), children: RefCell::new(None) }
        })
    }

    /// The node of the children of the model at `index` of `parent`'s collection.
    pub fn new_child(owner: Weak<TreeSelectionModelBase<M>>, parent: &TreeSelectionNode<M>, index: i32) -> Rc<Self> {
        let node = Self::new(owner.clone());
        *node.path.borrow_mut() = parent.path().append(index);

        if let Some(items_view) = parent.items_view() {
            let item = items_view.get_at(index as usize);
            let children = owner.upgrade().and_then(|owner| owner.get_children(&item));
            node.set_source(children);
        }

        node
    }

    /// The base state of the node.
    pub fn node_base(&self) -> &SelectionNodeBase<M> {
        &self.base
    }

    /// The path of the model whose children the node selects.
    pub fn path(&self) -> IndexPath {
        self.path.borrow().clone()
    }

    fn set_path(&self, path: IndexPath) {
        *self.path.borrow_mut() = path;
    }

    /// Gets the source collection.
    pub fn source(&self) -> Option<Models<M>> {
        self.base.source()
    }

    /// Sets the source collection.
    pub fn set_source(&self, value: Option<Models<M>>) {
        self.base.set_source(value);
    }

    /// The view of the source collection.
    pub fn items_view(&self) -> Option<Rc<TreeDataGridItemsSourceView<M>>> {
        self.base.items_view()
    }

    /// The selected ranges.
    pub fn ranges(&self) -> Rc<Vec<IndexRange>> {
        self.base.ranges()
    }

    /// Replaces the source collection and handles the replacement as a reset.
    pub fn reset_source(&self, source: Option<Models<M>>) {
        if same_source(self.source().as_ref(), source.as_ref()) {
            return;
        }
        self.set_source(source);
        SelectionNodeBaseImpl::on_source_reset(self);
    }

    /// Whether the node has a child node.
    pub fn has_children(&self) -> bool {
        match self.children.borrow().as_ref() {
            Some(children) => children.iter().any(Option::is_some),
            None => false,
        }
    }

    /// The child nodes, by index in the source collection (a snapshot).
    pub fn children(&self) -> Option<Children<M>> {
        self.children.borrow().clone()
    }

    /// Adds the selected ranges of the node and its descendants to the deselected ranges of
    /// `operation`.
    pub fn clear(&self, operation: &Operation<M>) {
        let ranges = self.base.ranges();

        if !ranges.is_empty() {
            let path = self.path();
            let mut deselected = operation.deselected_ranges.borrow_mut();
            let deselected = deselected.get_or_insert_with(IndexRanges::new);
            for range in ranges.iter() {
                deselected.add_range(&path, *range);
            }
        }

        if let Some(children) = self.children() {
            for child in children.iter().flatten() {
                child.clear(operation);
            }
        }
    }

    /// Adds `range` to the selection (`CommitSelect(IndexRange)`).
    pub fn commit_select_range(&self, range: IndexRange) -> i32 {
        self.base.commit_select(range.begin(), range.end())
    }

    /// Removes `range` from the selection (`CommitDeselect(IndexRange)`).
    pub fn commit_deselect_range(&self, range: IndexRange) -> i32 {
        self.base.commit_deselect(range.begin(), range.end())
    }

    /// The child node at `index`, if it exists.
    pub fn get_child(&self, index: i32) -> Option<Rc<Self>> {
        if index < 0 {
            return None;
        }
        let children = self.children.borrow();
        let children = children.as_ref()?;
        children.get(index as usize).cloned().flatten()
    }

    /// The child node at `index`, created if the collection has an item at `index`.
    pub fn get_or_create_child(&self, index: i32) -> Option<Rc<Self>> {
        if let Some(result) = self.get_child(index) {
            return Some(result);
        }

        let child_count = match self.items_view() {
            Some(items_view) => items_view.count() as i32,
            None => self.children.borrow().as_ref().map_or(0, |children| children.len() as i32).max(index),
        };

        if index >= 0 && index < child_count {
            {
                let mut children = self.children.borrow_mut();
                let children = children.get_or_insert_with(Vec::new);
                Self::resize(children, child_count as usize);
                if let Some(existing) = &children[index as usize] {
                    return Some(existing.clone());
                }
            }

            // Created without the children borrowed: the creation reads the children of the
            // model from the selection model.
            let child = Self::new_child(self.owner.clone(), self, index);

            let mut children = self.children.borrow_mut();
            if let Some(children) = children.as_mut() {
                if let Some(slot) = children.get_mut(index as usize) {
                    if let Some(existing) = slot.as_ref() {
                        return Some(existing.clone());
                    }
                    *slot = Some(child.clone());
                }
            }

            return Some(child);
        }

        None
    }

    /// Removes the child nodes that select nothing and have no child nodes.
    pub fn prune_empty_children(&self) {
        let Some(children) = self.children() else {
            return;
        };

        for (i, child) in children.iter().enumerate() {
            if let Some(node) = child {
                node.prune_empty_children();

                if node.base.ranges().is_empty() && !node.has_children() {
                    node.set_source(None);
                    if let Some(children) = self.children.borrow_mut().as_mut() {
                        if let Some(slot) = children.get_mut(i) {
                            *slot = None;
                        }
                    }
                }
            }
        }
    }

    fn owner(&self) -> Option<Rc<TreeSelectionModelBase<M>>> {
        self.owner.upgrade()
    }

    fn ancestor_index_changed(&self, parent_index: &IndexPath, shift_index: i32, shift_delta: i32) {
        let mut path = self.path();

        if Self::shift_index(parent_index, shift_index, shift_delta, &mut path) {
            self.set_path(path);
        }

        if let Some(children) = self.children() {
            for child in children.iter().flatten() {
                child.ancestor_index_changed(parent_index, shift_index, shift_delta);
            }
        }
    }

    fn ancestor_removed(&self, removed: &mut Option<Vec<Option<Rc<M>>>>) {
        let ranges = self.base.ranges();

        if !ranges.is_empty() {
            let items = removed.get_or_insert_with(Vec::new);

            if let Some(items_view) = self.items_view() {
                for range in ranges.iter() {
                    for i in range.begin()..=range.end() {
                        items.push(Some(items_view.get_at(i as usize)));
                    }
                }
            }
        }

        if let Some(children) = self.children() {
            for child in children.iter().flatten() {
                child.ancestor_removed(removed);
            }
        }

        self.set_source(None);
    }

    fn ancestor_reset(&self, removed_count: &mut i32) {
        if !self.base.ranges().is_empty() {
            *removed_count += self.base.commit_deselect(0, i32::MAX);
        }

        if let Some(children) = self.children() {
            for child in children.iter().flatten() {
                child.ancestor_reset(removed_count);
            }
        }

        self.set_source(None);
    }

    fn adjust_children_for_move(&self, old_index: i32, new_index: i32, count: i32) {
        let rebase: Vec<(usize, Rc<Self>)> = {
            let mut children = self.children.borrow_mut();
            let Some(children) = children.as_mut() else {
                return;
            };

            if count <= 0 {
                return;
            }

            if old_index < 0 || old_index as usize >= children.len() {
                return;
            }

            let old_index = old_index as usize;
            let available = children.len() - old_index;
            let count = (count as usize).min(available);

            if count == 0 {
                return;
            }

            let moving: Children<M> = children.drain(old_index..old_index + count).collect();

            let insert_index = (new_index.max(0) as usize).min(children.len());
            let tail = children.split_off(insert_index);
            children.extend(moving);
            children.extend(tail);

            let start = old_index.min(insert_index);
            let end = children.len().min(old_index.max(insert_index) + count);

            (start..end).filter_map(|i| children[i].clone().map(|child| (i, child))).collect()
        };

        let path = self.path();
        for (i, child) in rebase {
            child.rebase_path(path.append(i as i32));
        }
    }

    fn adjust_children_for_replace(
        &self,
        old_index: i32,
        old_count: i32,
        new_index: i32,
        new_count: i32,
        removed: &mut Option<Vec<Option<Rc<M>>>>,
    ) {
        if old_index < 0 {
            return;
        }

        let Some(snapshot) = self.children() else {
            return;
        };

        let old = old_index as usize;
        let remove_count = (old_count.max(0) as usize).min(snapshot.len().saturating_sub(old));

        for i in 0..remove_count {
            if let Some(child) = &snapshot[old + i] {
                child.ancestor_removed(removed);
            }
        }

        let rebase: Vec<(usize, Rc<Self>)> = {
            let mut children = self.children.borrow_mut();
            let Some(children) = children.as_mut() else {
                return;
            };

            let start = old.min(children.len());
            let end = (start + remove_count).min(children.len());
            children.drain(start..end);

            let insert_index = (new_index.max(0) as usize).min(children.len());
            insert_many(children, insert_index, None, new_count.max(0) as usize);

            (old.min(insert_index)..children.len())
                .filter_map(|i| children[i].clone().map(|child| (i, child)))
                .collect()
        };

        let path = self.path();
        for (i, child) in rebase {
            child.rebase_path(path.append(i as i32));
        }
    }

    fn rebase_path(&self, path: IndexPath) {
        self.set_path(path.clone());

        let Some(children) = self.children() else {
            return;
        };

        for (i, child) in children.iter().enumerate() {
            if let Some(child) = child {
                child.rebase_path(path.append(i as i32));
            }
        }
    }

    fn resize(list: &mut Children<M>, count: usize) {
        let current = list.len();

        if count < current {
            list.truncate(count);
        } else if count > current {
            if count > list.capacity() {
                list.reserve(count - list.len());
            }

            // As the reference, the new slots are inserted at the start.
            insert_many(list, 0, None, count - current);
        }
    }

    /// Shifts the index of `path` at the depth of the children of `parent_index` by
    /// `shift_delta` if it is at or after `shift_index`; returns whether the path changed.
    pub fn shift_index(parent_index: &IndexPath, shift_index: i32, shift_delta: i32, path: &mut IndexPath) -> bool {
        let depth = parent_index.count();

        if path.get(depth) >= shift_index {
            let mut indexes = path.to_vec();
            indexes[depth] += shift_delta;
            *path = IndexPath::from_indexes(&indexes);
            return true;
        }

        false
    }
}

impl<M: TreeModel> SelectionNodeBaseImpl<M> for TreeSelectionNode<M> {
    fn node_base(&self) -> &SelectionNodeBase<M> {
        &self.base
    }

    fn on_source_collection_change_started(&self) {
        if let Some(owner) = self.owner() {
            owner.on_node_collection_change_started();
        }
    }

    fn on_source_collection_changed(&self, e: &ModelsChangedArgs<'_, M>) {
        if !has_known_indexes(e) {
            self.on_source_reset();
            return;
        }

        // Assigned by every action that does not return.
        let shift_start_index: i32;
        let shift_delta: i32;
        let mut indexes_changed: bool;
        let mut shift_end_index = -1;
        let mut removed: Option<Vec<Option<Rc<M>>>> = None;
        let mut move_old_index = -1;
        let mut move_insert_index = -1;
        let mut move_count = 0;
        let mut replace_old_index = -1;
        let mut replace_new_index = -1;
        let mut replace_old_count = 0;
        let mut replace_new_count = 0;

        // Adjust the selection in this node according to the collection change.
        match e.action {
            NotifyCollectionChangedAction::Add => {
                shift_start_index = e.new_starting_index;
                shift_delta = e.new_items.len() as i32;
                indexes_changed = self.base.on_items_added(shift_start_index, e.new_items).shift_delta > 0;
            }
            NotifyCollectionChangedAction::Remove => {
                shift_start_index = e.old_starting_index;
                shift_delta = -(e.old_items.len() as i32);
                let change = self.base.on_items_removed(shift_start_index, e.old_items);
                indexes_changed = change.shift_delta != 0;
                removed = change.removed_items;
            }
            NotifyCollectionChangedAction::Replace => {
                replace_old_index = e.old_starting_index;
                replace_new_index = if e.new_starting_index >= 0 { e.new_starting_index } else { replace_old_index };
                replace_old_count = e.old_items.len() as i32;
                replace_new_count = e.new_items.len() as i32;
                let remove_change = self.base.on_items_removed(e.old_starting_index, e.old_items);
                let add_change = self.base.on_items_added(replace_new_index, e.new_items);
                shift_start_index = replace_old_index.min(replace_new_index);
                shift_delta = replace_new_count - replace_old_count;
                indexes_changed = shift_delta != 0 || remove_change.shift_delta != 0 || add_change.shift_delta != 0;
                removed = remove_change.removed_items;
            }
            NotifyCollectionChangedAction::Move => {
                if e.old_starting_index < 0 {
                    self.on_source_reset();
                    return;
                }

                move_old_index = e.old_starting_index;
                move_count = e.old_items.len() as i32;
                if move_old_index == e.new_starting_index || move_count == 0 {
                    return;
                }
                shift_start_index = e.old_starting_index.min(e.new_starting_index);
                shift_end_index =
                    (e.old_starting_index + move_count - 1).max(e.new_starting_index + move_count - 1);
                shift_delta = if e.old_starting_index < e.new_starting_index { -move_count } else { move_count };
                indexes_changed = true;

                let remove_move_change = self.base.on_items_removed(e.old_starting_index, e.old_items);
                let insert_index = e.new_starting_index;

                self.base.on_items_added(insert_index, e.new_items);
                move_insert_index = insert_index;

                match remove_move_change.deselected_ranges.filter(|ranges| !ranges.is_empty()) {
                    Some(deselected_ranges) => {
                        let mut moved_ranges = Vec::with_capacity(deselected_ranges.len());

                        for range in &deselected_ranges {
                            let relative_begin = range.begin() - e.old_starting_index;
                            let begin = insert_index + relative_begin;
                            let end = begin + (range.end() - range.begin());
                            moved_ranges.push(IndexRange::new(begin, end));

                            if self.base.ranges_enabled() {
                                self.base.commit_select(begin, end);
                            }
                        }

                        let moved_items = match remove_move_change.removed_items {
                            Some(items) if !items.is_empty() => items,
                            _ => Vec::new(),
                        };

                        self.on_selection_moved(
                            e.old_starting_index,
                            insert_index,
                            &deselected_ranges,
                            &moved_ranges,
                            &moved_items,
                        );

                        removed = None;
                    }
                    None => {
                        removed = remove_move_change.removed_items;
                    }
                }
            }
            NotifyCollectionChangedAction::Reset => {
                self.on_source_reset();
                return;
            }
        }

        // Adjust the paths of any child nodes.
        if self.children.borrow().is_some() {
            if e.action == NotifyCollectionChangedAction::Replace {
                self.adjust_children_for_replace(
                    replace_old_index,
                    replace_old_count,
                    replace_new_index,
                    replace_new_count,
                    &mut removed,
                );
            } else if e.action == NotifyCollectionChangedAction::Move && move_count > 0 && move_insert_index >= 0 {
                self.adjust_children_for_move(move_old_index, move_insert_index, move_count);
            } else {
                let children = self.children().unwrap_or_default();

                if shift_delta != 0 && !children.is_empty() {
                    let path = self.path();
                    let start = shift_start_index.max(0) as usize;

                    for (i, child) in children.iter().enumerate().skip(start) {
                        let i = i as i32;

                        if shift_delta < 1 && i >= shift_start_index && i < shift_start_index - shift_delta {
                            if let Some(child) = child {
                                child.ancestor_removed(&mut removed);
                            }
                        } else {
                            if let Some(child) = child {
                                child.ancestor_index_changed(&path, shift_start_index, shift_delta);
                            }
                            indexes_changed = true;
                        }
                    }

                    let mut children = self.children.borrow_mut();
                    if let Some(children) = children.as_mut() {
                        let start = start.min(children.len());

                        if shift_delta > 0 {
                            insert_many(children, start, None, shift_delta as usize);
                        } else {
                            let end = (start + (-shift_delta) as usize).min(children.len());
                            children.drain(start..end);
                        }
                    }
                }
            }
        }

        if shift_delta != 0 || removed.as_ref().is_some_and(|removed| !removed.is_empty()) || replace_old_index >= 0 {
            if shift_end_index == -1 {
                shift_end_index = self.items_view().map_or(0, |items_view| items_view.count() as i32);
            }

            if let Some(owner) = self.owner() {
                owner.on_node_collection_changed(
                    &self.path(),
                    shift_start_index,
                    shift_end_index,
                    shift_delta,
                    indexes_changed,
                    removed.as_deref(),
                    move_old_index,
                    move_insert_index,
                    move_count,
                    replace_old_index,
                    replace_old_count,
                    replace_new_count,
                );
            }
        }
    }

    fn on_source_collection_change_finished(&self) {
        if let Some(owner) = self.owner() {
            owner.on_node_collection_change_finished();
        }
    }

    fn on_source_reset(&self) {
        let mut removed = self.base.commit_deselect(0, i32::MAX);

        let children = self.children.borrow_mut().take();
        if let Some(children) = children {
            for child in children.iter().flatten() {
                child.ancestor_reset(&mut removed);
            }
        }

        if let Some(owner) = self.owner() {
            owner.on_node_collection_reset(&self.path(), removed);
        }
    }

    fn on_selection_moved(
        &self,
        _old_start_index: i32,
        _new_start_index: i32,
        old_selected_ranges: &[IndexRange],
        new_selected_ranges: &[IndexRange],
        moved_items: &[Option<Rc<M>>],
    ) {
        if let Some(owner) = self.owner() {
            owner.on_node_selection_moved(&self.path(), old_selected_ranges, new_selected_ranges, moved_items);
        }
    }
}

impl<M: TreeModel> ICollectionChangedListener<M> for TreeSelectionNode<M> {
    fn pre_changed(&self, collection: &Models<M>, e: &ModelsChangedArgs<'_, M>) {
        self.base.pre_changed(collection, e);
    }

    fn changed(&self, collection: &Models<M>, e: &ModelsChangedArgs<'_, M>) {
        self.base.changed(collection, e);
    }

    fn post_changed(&self, collection: &Models<M>, e: &ModelsChangedArgs<'_, M>) {
        self.base.post_changed(collection, e);
    }
}
