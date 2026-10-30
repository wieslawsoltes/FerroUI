// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.

use super::{IndexPath, SelectionError};
use super::selection_node::SelectionNode;
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub(crate) struct SelectedItemInfo { pub node: Weak<SelectionNode>, pub path: IndexPath }

struct View<T> {
    infos: Vec<SelectedItemInfo>,
    count: usize,
    get: fn(&SelectedItemInfo, i32) -> Result<T, SelectionError>,
}

/// A lazy, shared, read-only view over selected nodes. No array of selected items is copied.
/// Nodes are weakly held, so a view does not retain an old source tree after a reset.
pub struct SelectedItems<T>(Rc<View<T>>);

impl<T> Clone for SelectedItems<T> { fn clone(&self) -> Self { Self(self.0.clone()) } }
impl<T> PartialEq for SelectedItems<T> { fn eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.0, &other.0) } }

impl<T> SelectedItems<T> {
    pub(crate) fn new(infos: Vec<SelectedItemInfo>, get: fn(&SelectedItemInfo, i32) -> Result<T, SelectionError>) -> Self {
        let count = infos.iter().filter_map(|info| info.node.upgrade()).map(|node| node.selected_count()).sum();
        Self(Rc::new(View { infos, count, get }))
    }
    pub fn count(&self) -> usize { self.0.count }
    pub fn is_empty(&self) -> bool { self.count() == 0 }
    pub fn get_at(&self, index: usize) -> Result<T, SelectionError> {
        if index >= self.count() { return Err(SelectionError::IndexOutOfRange); }
        let mut offset = 0;
        for info in &self.0.infos {
            let node = info.node.upgrade().ok_or(SelectionError::SelectionChanged)?;
            let count = node.selected_count();
            if index < offset + count {
                return (self.0.get)(info, node.selected_index_at(index - offset)?);
            }
            offset += count;
        }
        Err(SelectionError::SelectionChanged)
    }
    pub fn iter(&self) -> SelectedItemsIter<T> { SelectedItemsIter { view: self.clone(), next: 0 } }
    pub fn to_vec(&self) -> Result<Vec<T>, SelectionError> { self.iter().collect() }
}

/// Enumerates a view without materializing its items.
pub struct SelectedItemsIter<T> { view: SelectedItems<T>, next: usize }
impl<T> Iterator for SelectedItemsIter<T> {
    type Item = Result<T, SelectionError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.view.count() { return None; }
        let result = self.view.get_at(self.next);
        self.next += 1;
        Some(result)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.view.count() - self.next;
        (remaining, Some(remaining))
    }
}
impl<T> ExactSizeIterator for SelectedItemsIter<T> {}

