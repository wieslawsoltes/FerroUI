// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.

use super::{IndexPath, SelectionError, SelectionModel};
use ferroui_controls::{ItemsSource, ItemsChangedEventArgs};
use ferroui_controls::selection::IndexRange;
use ferroui_base::collections::NotifyCollectionChangedAction;
use std::{cell::{Cell, RefCell}, rc::{Rc, Weak}};

/// The shared leaf sentinel of the upstream tree is an enum variant: it has no owned state.
#[derive(Clone, Default)]
pub(crate) enum Child { #[default] Unrealized, Leaf, Branch(Rc<SelectionNode>) }

pub(crate) struct SelectionNode {
    manager: Weak<SelectionModel>,
    parent: Weak<Self>,
    this: Weak<Self>,
    source: RefCell<Option<ItemsSource>>,
    subscription: Cell<Option<u64>>,
    children: RefCell<Vec<Child>>,
    ranges: RefCell<Vec<IndexRange>>,
    anchor: Cell<i32>,
    data_count: Cell<usize>,
}

impl SelectionNode {
    pub fn new(manager: Weak<SelectionModel>, parent: Weak<Self>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            manager, parent, this: this.clone(), source: RefCell::new(None), subscription: Cell::new(None),
            children: RefCell::new(Vec::new()), ranges: RefCell::new(Vec::new()), anchor: Cell::new(-1), data_count: Cell::new(0),
        })
    }
    pub fn source(&self) -> Option<ItemsSource> { self.source.borrow().clone() }
    pub fn set_source(&self, source: Option<ItemsSource>) {
        if self.source() == source { return; }
        self.clear();
        if let Some(token) = self.subscription.take() {
            if let Some(old) = self.source() { old.list().remove_collection_changed(token); }
        }
        *self.source.borrow_mut() = source.clone();
        self.data_count.set(source.as_ref().map_or(0, ItemsSource::count));
        if let Some(source) = source {
            let weak = self.this.clone();
            self.subscription.set(source.list().add_collection_changed(Rc::new(move |args| {
                if let Some(node) = weak.upgrade() { node.source_changed(args); }
            })));
        }
    }
    pub fn data_count(&self) -> usize { self.data_count.get() }
    pub fn anchor(&self) -> i32 { self.anchor.get() }
    pub fn set_anchor(&self, index: i32) { self.anchor.set(index); }
    pub fn children(&self) -> Vec<Child> { self.children.borrow().clone() }
    pub fn child(&self, index: i32, realize: bool) -> Result<Child, SelectionError> {
        let index = usize::try_from(index).map_err(|_| SelectionError::InvalidPath)?;
        if let Some(child) = self.children.borrow().get(index).filter(|child| !matches!(child, Child::Unrealized)) {
            return Ok(child.clone());
        }
        if !realize { return Ok(Child::Unrealized); }
        let manager = self.manager.upgrade().ok_or(SelectionError::SelectionChanged)?;
        let revision = manager.tree_revision();
        let source = self.source().ok_or(SelectionError::SourceRequired)?;
        let count = source.count();
        if manager.tree_revision() != revision { return Err(SelectionError::SelectionChanged); }
        if index >= count { return Err(SelectionError::IndexOutOfRange); }
        let data = source.get_at(index);
        if manager.tree_revision() != revision { return Err(SelectionError::SelectionChanged); }
        let resolved = if data.is_some() { manager.resolve_path(data, self.index_path().clone_with_child_index(index as i32)) } else { None };
        // User callbacks can mutate the same source or discard an ancestor without changing this
        // node's source identity. A shared revision prevents installing a child at a stale path.
        if manager.tree_revision() != revision || self.source().as_ref() != Some(&source) {
            return Err(SelectionError::SelectionChanged);
        }
        if let Some(child) = self.children.borrow().get(index).filter(|child| !matches!(child, Child::Unrealized)) {
            // A nested request can realize this slot and select its descendants. Keep that node.
            return Ok(child.clone());
        }
        let child = if let Some(source) = resolved {
            let child = Self::new(self.manager.clone(), self.this.clone());
            child.set_source(Some(source));
            Child::Branch(child)
        } else { Child::Leaf };
        // Reading a custom child source's count and subscribing to it are callbacks too.
        if manager.tree_revision() != revision { return Err(SelectionError::SelectionChanged); }
        let installed = {
            let mut children = self.children.borrow_mut();
            if children.is_empty() { children.resize(count, Child::Unrealized); }
            if index >= children.len() { return Err(SelectionError::SelectionChanged); }
            if matches!(children[index], Child::Unrealized) { children[index] = child.clone(); }
            children[index].clone()
        };
        Ok(installed)
    }
    pub fn index_path(&self) -> IndexPath {
        let mut path = Vec::new();
        let mut child = self.this.upgrade();
        let mut parent = self.parent.upgrade();
        while let (Some(current), Some(owner)) = (child, parent) {
            let index = owner.children.borrow().iter().position(|node| matches!(node, Child::Branch(node) if Rc::ptr_eq(node, &current)));
            if let Some(index) = index { path.push(index as i32); }
            child = Some(owner.clone());
            parent = owner.parent.upgrade();
        }
        path.reverse();
        IndexPath::create_from_indices(path)
    }
    pub fn selected_count(&self) -> usize {
        self.ranges.borrow().iter().map(|r| (i64::from(r.end()) - i64::from(r.begin()) + 1) as usize).sum()
    }
    pub fn selected_index_at(&self, mut index: usize) -> Result<i32, SelectionError> {
        for range in self.ranges.borrow().iter() {
            let count = (i64::from(range.end()) - i64::from(range.begin()) + 1) as usize;
            if index < count { return Ok((i64::from(range.begin()) + index as i64) as i32); }
            index -= count;
        }
        Err(SelectionError::IndexOutOfRange)
    }
    pub fn is_selected(&self, index: i32) -> bool { self.ranges.borrow().iter().any(|range| range.contains(index)) }
    pub fn is_selected_with_partial(&self, index: i32) -> Option<bool> {
        let child = self.children.borrow().get(index as usize).cloned();
        match child { Some(Child::Branch(node)) => node.evaluate(), _ => Some(self.is_selected(index)) }
    }
    pub fn evaluate(&self) -> Option<bool> {
        if self.children.borrow().is_empty() {
            let count = self.selected_count();
            return if count == 0 { Some(false) } else if count == self.data_count() { Some(true) } else { None };
        }
        let mut any_selected = false;
        let mut any_unselected = false;
        for index in 0..self.data_count() {
            match self.is_selected_with_partial(index as i32) {
                Some(true) => any_selected = true,
                Some(false) => any_unselected = true,
                None => return None,
            }
            if any_selected && any_unselected { return None; }
        }
        Some(any_selected)
    }
    fn valid_index(&self, index: i32) -> bool { index >= 0 && (self.source().is_none() || index < self.data_count() as i32) }
    pub fn select(&self, index: i32, select: bool) -> Result<(), SelectionError> {
        self.select_range(index, index, select)
    }
    pub fn select_range(&self, first: i32, last: i32, select: bool) -> Result<(), SelectionError> {
        if !self.valid_index(first) || !self.valid_index(last) { return Err(SelectionError::IndexOutOfRange); }
        let range = IndexRange::new(first, last);
        // Reuse the normalized inclusive range engine already used by FerroUI's flat model.
        // This preserves the selected set while avoiding upstream's overlapping range scans.
        if select { IndexRange::add(&mut self.ranges.borrow_mut(), range, None); }
        else { IndexRange::remove(Some(&mut self.ranges.borrow_mut()), range, None); }
        Ok(())
    }
    pub fn select_all(&self) {
        if self.data_count() > 0 { let _ = self.select_range(0, self.data_count() as i32 - 1, true); }
    }
    pub fn clear(&self) {
        self.ranges.borrow_mut().clear();
        self.anchor.set(-1);
        let children = std::mem::take(&mut *self.children.borrow_mut());
        drop(children);
    }
    fn source_changed(&self, args: &ItemsChangedEventArgs<'_>) {
        if let Some(manager) = self.manager.upgrade() { manager.invalidate_tree(); }
        let previous_state = self.evaluate();
        let mut changed = false;
        match args.action {
            NotifyCollectionChangedAction::Add => changed = self.items_added(args.new_starting_index, args.new_items.len()),
            NotifyCollectionChangedAction::Remove => changed = self.items_removed(args.old_starting_index, args.old_items.len()),
            NotifyCollectionChangedAction::Replace => {
                changed = self.items_removed(args.old_starting_index, args.old_items.len());
                changed |= self.items_added(args.new_starting_index, args.new_items.len());
            }
            NotifyCollectionChangedAction::Reset => { self.clear(); changed = true; }
            // SelectionNode::OnSourceListChanged does not handle Move at the pinned revision.
            NotifyCollectionChangedAction::Move => {}
        }
        self.data_count.set(self.source().as_ref().map_or(0, ItemsSource::count));
        changed |= previous_state != self.evaluate();
        if changed { if let Some(manager) = self.manager.upgrade() { manager.selection_invalidated_due_to_collection_change(); } }
    }
    fn items_added(&self, index: i32, count: usize) -> bool {
        if index < 0 || count == 0 { return false; }
        let count = count as i32;
        let mut shifted = Vec::new();
        let mut changed = false;
        for range in self.ranges.borrow().iter() {
            if range.end() < index { shifted.push(*range); }
            else {
                if range.begin() < index { shifted.push(IndexRange::new(range.begin(), index - 1)); }
                shifted.push(IndexRange::new(range.begin().max(index) + count, range.end() + count));
                changed = true;
            }
        }
        *self.ranges.borrow_mut() = shifted;
        let mut children = self.children.borrow_mut();
        if !children.is_empty() && index as usize <= children.len() {
            children.splice(index as usize..index as usize, std::iter::repeat_n(Child::Unrealized, count as usize));
            changed = true;
        }
        if self.anchor() >= index { self.set_anchor(self.anchor() + count); }
        changed
    }
    fn items_removed(&self, index: i32, count: usize) -> bool {
        if index < 0 || count == 0 { return false; }
        let count = count as i32;
        let end = index + count - 1;
        let old = self.ranges.borrow().clone();
        IndexRange::remove(Some(&mut self.ranges.borrow_mut()), IndexRange::new(index, end), None);
        for range in self.ranges.borrow_mut().iter_mut() {
            if range.begin() > end { *range = IndexRange::new(range.begin() - count, range.end() - count); }
        }
        let mut changed = old != *self.ranges.borrow();
        let removed = {
            let mut children = self.children.borrow_mut();
            if !children.is_empty() && (index + count) as usize <= children.len() {
                changed = true;
                children.drain(index as usize..(index + count) as usize).collect::<Vec<_>>()
            } else { Vec::new() }
        };
        drop(removed);
        if self.anchor() >= index { self.set_anchor(self.anchor() - count); }
        changed
    }
}

impl Drop for SelectionNode {
    fn drop(&mut self) {
        if let Some(token) = self.subscription.take() {
            if let Some(source) = self.source.get_mut().as_ref() { source.list().remove_collection_changed(token); }
        }
    }
}

