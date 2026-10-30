// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.

use super::{IndexPath, SelectedItems, SelectionError, SelectionModelChildrenRequestedEventArgs};
use super::selected_items::SelectedItemInfo;
use super::selection_node::{Child, SelectionNode};
use super::selection_tree_helper::{parent_at, traverse, traverse_range};
use ferroui_controls::{ItemsSource, ItemsSourceView};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::{cell::{Cell, RefCell}, rc::{Rc, Weak}};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionModelSelectionChangedEventArgs;

/// The WinUI hierarchical selection model, independent of realized controls.
///
/// It defaults to multiple selection, can resolve arbitrary nested sources, represents group
/// selection as `Some(true)`, `Some(false)` or `None` (partial), and keeps selection synchronized
/// with source collection changes. Its tree is realized only as requested by selection operations.
pub struct SelectionModel {
    this: Weak<Self>,
    root: Rc<SelectionNode>,
    single_select: Cell<bool>,
    selection_invalidated: Cell<bool>,
    tree_revision: Cell<u64>,
    selected_indices: RefCell<Option<SelectedItems<IndexPath>>>,
    selected_items: RefCell<Option<SelectedItems<Option<BoxedValue>>>>,
    selection_changed: HandlerList<dyn Fn(&SelectionModelSelectionChangedEventArgs)>,
    children_requested: HandlerList<dyn Fn(&Rc<SelectionModelChildrenRequestedEventArgs>)>,
    property_changed: Event<str>,
}

impl SelectionModel {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(), root: SelectionNode::new(this.clone(), Weak::new()),
            single_select: Cell::new(false), selection_invalidated: Cell::new(false),
            tree_revision: Cell::new(0),
            selected_indices: RefCell::new(None), selected_items: RefCell::new(None),
            selection_changed: HandlerList::new(), children_requested: HandlerList::new(), property_changed: Event::new(),
        })
    }
    pub fn source(&self) -> Option<ItemsSource> { self.root.source() }
    pub fn set_source(&self, source: Option<ItemsSource>) {
        let removing_source = self.source().is_some() && source.is_none();
        self.clear_selection_core(true, false);
        self.root.set_source(source);
        self.on_selection_changed_with_item_notifications(removing_source);
        self.on_property_changed("Source");
    }
    pub fn single_select(&self) -> bool { self.single_select.get() }
    pub fn set_single_select(&self, value: bool) {
        if self.single_select.replace(value) == value { return; }
        let indices = self.selected_indices();
        if value && indices.count() > 1 {
            let first = indices.get_at(0).expect("the current selection is valid");
            self.clear_selection_core(true, false);
            self.select_path_core(&first, true, true).expect("the first selected path is valid");
        }
        self.on_property_changed("SingleSelect");
    }
    pub fn anchor_index(&self) -> Option<IndexPath> {
        if self.root.anchor() < 0 { return None; }
        let mut node = self.root.clone();
        let mut indexes = Vec::new();
        while node.anchor() >= 0 {
            let index = node.anchor();
            indexes.push(index);
            match node.child(index, false).ok()? {
                Child::Branch(child) => node = child,
                _ => break,
            }
        }
        Some(IndexPath::create_from_indices(indexes))
    }
    pub fn set_anchor_index(&self, value: Option<IndexPath>) -> Result<(), SelectionError> {
        if let Some(path) = value {
            if !path.is_valid() { return Err(SelectionError::InvalidPath); }
            // Validate and realize the ancestors before changing their anchors.
            parent_at(&self.root, &path, true)?;
            let mut node = self.root.clone();
            for (depth, &index) in path.as_slice().iter().enumerate() {
                node.set_anchor(index);
                if depth + 1 < path.as_slice().len() {
                    if let Child::Branch(child) = node.child(index, true)? { node = child; }
                }
            }
        } else { self.root.set_anchor(-1); }
        self.on_property_changed("AnchorIndex");
        Ok(())
    }
    pub fn set_anchor_index_flat(&self, index: i32) -> Result<(), SelectionError> {
        self.set_anchor_index(Some(IndexPath::create_from(index)))
    }
    pub fn set_anchor_index_with_group(&self, group: i32, item: i32) -> Result<(), SelectionError> {
        self.set_anchor_index(Some(IndexPath::create_from_group_and_item_index(group, item)))
    }
    pub fn selected_index(&self) -> Option<IndexPath> {
        let indices = self.selected_indices();
        if indices.is_empty() { None } else { indices.get_at(0).ok() }
    }
    pub fn set_selected_index(&self, index: Option<IndexPath>) -> Result<(), SelectionError> {
        if let Some(index) = index {
            if self.is_selected_at(&index)? != Some(true) {
                self.clear_selection_core(true, false);
                self.select_path_core(&index, true, false)?;
                self.on_selection_changed();
            }
        } else { self.clear_selection(); }
        Ok(())
    }
    pub fn selected_item(&self) -> Option<BoxedValue> {
        let items = self.selected_items();
        if items.is_empty() { None } else { items.get_at(0).ok().flatten() }
    }
    pub fn selected_indices(&self) -> SelectedItems<IndexPath> {
        if let Some(cached) = self.selected_indices.borrow().clone() { return cached; }
        let view = SelectedItems::new(self.selected_infos(false), |info, index| Ok(info.path.clone_with_child_index(index)));
        *self.selected_indices.borrow_mut() = Some(view.clone());
        view
    }
    pub fn selected_items(&self) -> SelectedItems<Option<BoxedValue>> {
        if let Some(cached) = self.selected_items.borrow().clone() { return cached; }
        let view = SelectedItems::new(self.selected_infos(true), |info, index| {
            let node = info.node.upgrade().ok_or(SelectionError::SelectionChanged)?;
            let source = node.source().ok_or(SelectionError::SourceRequired)?;
            if index < 0 || index as usize >= source.count() { return Err(SelectionError::SelectionChanged); }
            Ok(source.get_at(index as usize))
        });
        *self.selected_items.borrow_mut() = Some(view.clone());
        view
    }
    fn selected_infos(&self, require_source: bool) -> Vec<SelectedItemInfo> {
        let mut infos = Vec::new();
        if !require_source || self.source().is_some() {
            traverse(&self.root, false, |node, path| {
                if node.selected_count() > 0 { infos.push(SelectedItemInfo { node: Rc::downgrade(&node), path }); }
            }).expect("traversing realized selection nodes cannot fail");
        }
        infos
    }
    pub fn select(&self, index: i32) -> Result<(), SelectionError> { self.select_flat_core(index, true) }
    pub fn deselect(&self, index: i32) -> Result<(), SelectionError> { self.select_flat_core(index, false) }
    pub fn select_with_group(&self, group: i32, item: i32) -> Result<(), SelectionError> { self.select_group_core(group, item, true) }
    pub fn deselect_with_group(&self, group: i32, item: i32) -> Result<(), SelectionError> { self.select_group_core(group, item, false) }
    pub fn select_at(&self, path: &IndexPath) -> Result<(), SelectionError> { self.select_path_core(path, true, true) }
    pub fn deselect_at(&self, path: &IndexPath) -> Result<(), SelectionError> { self.select_path_core(path, false, true) }
    pub fn is_selected(&self, index: i32) -> Option<bool> { self.root.is_selected_with_partial(index) }
    pub fn is_selected_with_group(&self, group: i32, item: i32) -> Option<bool> {
        match self.root.child(group, false) { Ok(Child::Branch(child)) => child.is_selected_with_partial(item), _ => Some(false) }
    }
    pub fn is_selected_at(&self, path: &IndexPath) -> Result<Option<bool>, SelectionError> {
        let Some(node) = parent_at(&self.root, path, false)? else { return Ok(Some(false)); };
        Ok(if let Some(&index) = path.as_slice().last() { node.is_selected_with_partial(index) } else { node.evaluate() })
    }
    pub fn select_range_from_anchor(&self, index: i32) -> Result<(), SelectionError> { self.select_flat_range_core(index, true) }
    pub fn deselect_range_from_anchor(&self, index: i32) -> Result<(), SelectionError> { self.select_flat_range_core(index, false) }
    pub fn select_range_from_anchor_with_group(&self, group: i32, item: i32) -> Result<(), SelectionError> { self.select_group_range_core(group, item, true) }
    pub fn deselect_range_from_anchor_with_group(&self, group: i32, item: i32) -> Result<(), SelectionError> { self.select_group_range_core(group, item, false) }
    pub fn select_range_from_anchor_to(&self, index: &IndexPath) -> Result<(), SelectionError> {
        let anchor = self.anchor_index().unwrap_or_else(|| IndexPath::create_from_indices(std::iter::repeat_n(0, index.as_slice().len())));
        self.select_range(&anchor, index)
    }
    pub fn deselect_range_from_anchor_to(&self, index: &IndexPath) -> Result<(), SelectionError> {
        let anchor = self.anchor_index().unwrap_or_else(|| IndexPath::create_from_indices(std::iter::repeat_n(0, index.as_slice().len())));
        self.deselect_range(&anchor, index)
    }
    pub fn select_range(&self, start: &IndexPath, end: &IndexPath) -> Result<(), SelectionError> { self.select_range_core(start, end, true) }
    pub fn deselect_range(&self, start: &IndexPath, end: &IndexPath) -> Result<(), SelectionError> { self.select_range_core(start, end, false) }
    pub fn select_all(&self) -> Result<(), SelectionError> {
        self.notifying_on_error(|| traverse(&self.root, true, |node, _| node.select_all()))?;
        self.on_selection_changed();
        Ok(())
    }
    /// Selects a flat source as a single range without reading source items.
    pub fn select_all_flat(&self) {
        self.root.select_all();
        self.on_selection_changed();
    }
    pub fn clear_selection(&self) { self.clear_selection_core(true, true); }
    pub fn selection_changed(&self, handler: impl Fn(&SelectionModelSelectionChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.selection_changed.add(Rc::new(handler));
        let weak = self.this.clone();
        Disposable::create(move || { if let Some(this) = weak.upgrade() { this.selection_changed.remove(token); } })
    }
    pub fn children_requested(&self, handler: impl Fn(&Rc<SelectionModelChildrenRequestedEventArgs>) + 'static) -> Rc<dyn IDisposable> {
        let token = self.children_requested.add(Rc::new(handler));
        let weak = self.this.clone();
        Disposable::create(move || { if let Some(this) = weak.upgrade() { this.children_requested.remove(token); } })
    }
    pub fn on_property_changed(&self, property_name: &str) { self.property_changed.raise(property_name); }
    pub fn is_selection_invalidated_due_to_collection_change(&self) -> bool { self.selection_invalidated.get() }
    pub(crate) fn tree_revision(&self) -> u64 { self.tree_revision.get() }
    pub(crate) fn invalidate_tree(&self) {
        self.tree_revision.set(self.tree_revision.get().wrapping_add(1));
    }
    pub(crate) fn selection_invalidated_due_to_collection_change(&self) {
        self.selection_invalidated.set(true);
        let _reset = ResetFlag(&self.selection_invalidated);
        self.on_selection_changed();
    }
    pub(crate) fn resolve_path(&self, data: Option<BoxedValue>, path: IndexPath) -> Option<ItemsSource> {
        if self.children_requested.is_empty() { return data.as_ref().and_then(Self::source_from_item); }
        let args = SelectionModelChildrenRequestedEventArgs::new(data, path);
        let _expiry = ExpireArgs(args.clone());
        for (_, handler) in self.children_requested.snapshot().iter() { handler(&args); }
        args.children()
    }
    /// Applies FerroUI's registered collection conversions when automatically resolving children.
    pub fn source_from_item(data: &BoxedValue) -> Option<ItemsSource> {
        if let Some(source) = data.downcast_ref::<ItemsSource>() { return Some(source.clone()); }
        if let Some(source) = data.downcast_ref::<Rc<ItemsSourceView>>() { return Some(source.source()); }
        ValueTypes::try_convert(Some(data), ValueType::of::<Option<ItemsSource>>()).flatten()
            .and_then(|value| value.downcast_ref::<Option<ItemsSource>>().cloned().flatten())
    }
    fn clear_cached_views(&self) {
        self.selected_indices.borrow_mut().take();
        self.selected_items.borrow_mut().take();
    }
    fn clear_selection_core(&self, reset_anchor: bool, raise: bool) {
        self.invalidate_tree();
        traverse(&self.root, false, |node, _| node.clear()).expect("traversing realized selection nodes cannot fail");
        self.clear_cached_views();
        if reset_anchor { self.set_anchor_index(None).expect("clearing an anchor cannot fail"); }
        if raise { self.on_selection_changed(); }
    }
    fn on_selection_changed(&self) {
        self.on_selection_changed_with_item_notifications(false);
    }
    fn on_selection_changed_with_item_notifications(&self, removing_source: bool) {
        self.clear_cached_views();
        for (_, handler) in self.selection_changed.snapshot().iter() { handler(&SelectionModelSelectionChangedEventArgs); }
        self.on_property_changed("SelectedIndex");
        self.on_property_changed("SelectedIndices");
        // Removing a source must also replace the old lazy item view in native bindings.
        // Ordinary source-less selection continues to notify only the index properties.
        if removing_source || self.source().is_some() {
            self.on_property_changed("SelectedItem");
            self.on_property_changed("SelectedItems");
        }
    }
    /// Runs a change of the selection; when it fails part-way, what it already changed (a
    /// cleared single selection, part of a range) is announced, as upstream's `SelectImpl`
    /// always ends with `OnSelectionChanged`.
    fn notifying_on_error(
        &self,
        change: impl FnOnce() -> Result<(), SelectionError>,
    ) -> Result<(), SelectionError> {
        let result = change();
        if result.is_err() {
            self.on_selection_changed();
        }
        result
    }
    fn select_flat_core(&self, index: i32, select: bool) -> Result<(), SelectionError> {
        if self.root.is_selected(index) != select {
            self.notifying_on_error(|| {
                if self.single_select() { self.clear_selection_core(true, false); }
                self.root.select(index, select)?;
                self.set_anchor_index_flat(index)
            })?;
            self.on_selection_changed();
        }
        Ok(())
    }
    fn select_group_core(&self, group: i32, item: i32, select: bool) -> Result<(), SelectionError> {
        self.notifying_on_error(|| {
            if self.single_select() { self.clear_selection_core(true, false); }
            let Child::Branch(node) = self.root.child(group, true)? else { return Err(SelectionError::InvalidPath); };
            node.select(item, select)?;
            self.set_anchor_index_with_group(group, item)
        })?;
        self.on_selection_changed();
        Ok(())
    }
    fn select_path_core(&self, path: &IndexPath, select: bool, raise: bool) -> Result<(), SelectionError> {
        if !path.is_valid() || path.is_empty() { return Err(SelectionError::InvalidPath); }
        if self.single_select() {
            if let Some(current) = self.selected_index() {
                if select && current == *path { return Ok(()); }
            } else if !select { return Ok(()); }
        }
        let mut changed = false;
        self.notifying_on_error(|| {
            if self.single_select() && select { self.clear_selection_core(true, false); }
            let node = parent_at(&self.root, path, true)?.ok_or(SelectionError::InvalidPath)?;
            let index = *path.as_slice().last().expect("a nonempty path");
            changed = node.is_selected(index) != select;
            node.select(index, select)?;
            self.set_anchor_index(Some(path.clone()))
        })?;
        self.clear_cached_views();
        if changed && raise { self.on_selection_changed(); }
        Ok(())
    }
    fn select_flat_range_core(&self, end: i32, select: bool) -> Result<(), SelectionError> {
        let anchor = self.anchor_index().map(|p| p.get_at(0)).transpose()?.unwrap_or(0);
        self.notifying_on_error(|| self.root.select_range(anchor, end, select))?;
        self.on_selection_changed();
        Ok(())
    }
    fn select_group_range_core(&self, group: i32, item: i32, select: bool) -> Result<(), SelectionError> {
        let anchor = self.anchor_index().unwrap_or_else(|| IndexPath::create_from_group_and_item_index(0, 0));
        if anchor.get_size() != 2 { return Err(SelectionError::InvalidPath); }
        let target = IndexPath::create_from_group_and_item_index(group, item);
        let (start, end) = if anchor <= target { (&anchor, &target) } else { (&target, &anchor) };
        self.notifying_on_error(|| {
            for group in start.get_at(0)?..=end.get_at(0)? {
                let Child::Branch(node) = self.root.child(group, true)? else { return Err(SelectionError::InvalidPath); };
                let first = if group == start.get_at(0)? { start.get_at(1)? } else { 0 };
                let last = if group == end.get_at(0)? { end.get_at(1)? } else { node.data_count() as i32 - 1 };
                if last >= first { node.select_range(first, last, select)?; }
            }
            Ok(())
        })?;
        self.on_selection_changed();
        Ok(())
    }
    fn select_range_core(&self, start: &IndexPath, end: &IndexPath, select: bool) -> Result<(), SelectionError> {
        if start == end {
            self.select_path_core(start, select, true)?;
            // Preserve the pinned upstream's second notification for a singleton range.
            // Its regression VerifySingleItemRangeRaisesSelectionChangedOnce is ignored upstream.
        } else {
            let (start, end) = if start < end { (start, end) } else { (end, start) };
            self.notifying_on_error(|| {
                traverse_range(&self.root, start, end, |node, index| node.select(index, select))
            })?;
        }
        self.on_selection_changed();
        Ok(())
    }
}

impl INotifyPropertyChanged for SelectionModel {
    fn property_changed(&self) -> &Event<str> { &self.property_changed }
}
impl PartialEq for SelectionModel { fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) } }
impl std::fmt::Display for SelectionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("SelectionModel") }
}
struct ResetFlag<'a>(&'a Cell<bool>);
impl Drop for ResetFlag<'_> { fn drop(&mut self) { self.0.set(false); } }
struct ExpireArgs(Rc<SelectionModelChildrenRequestedEventArgs>);
impl Drop for ExpireArgs { fn drop(&mut self) { self.0.expire(); } }
