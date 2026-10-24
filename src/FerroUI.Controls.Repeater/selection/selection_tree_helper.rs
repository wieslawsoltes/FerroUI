// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.

use super::{IndexPath, SelectionError};
use super::selection_node::{Child, SelectionNode};
use std::rc::Rc;

/// Resolves only the ancestors of a path. The terminal component addresses an item of the node.
pub(crate) fn parent_at(root: &Rc<SelectionNode>, path: &IndexPath, realize: bool) -> Result<Option<Rc<SelectionNode>>, SelectionError> {
    if !path.is_valid() { return Err(SelectionError::InvalidPath); }
    let mut node = root.clone();
    for &index in path.as_slice().iter().take(path.as_slice().len().saturating_sub(1)) {
        match node.child(index, realize)? {
            Child::Branch(child) => node = child,
            Child::Unrealized if !realize => return Ok(None),
            _ => return Err(SelectionError::InvalidPath),
        }
    }
    Ok(Some(node))
}

/// Preorder traversal queues children before invoking the action, allowing the action to clear.
pub(crate) fn traverse(root: &Rc<SelectionNode>, realize: bool, mut action: impl FnMut(Rc<SelectionNode>, IndexPath)) -> Result<(), SelectionError> {
    let mut pending = vec![(root.clone(), IndexPath::default())];
    while let Some((node, path)) = pending.pop() {
        let count = if realize { node.data_count() } else { node.children().len() };
        for i in (0..count).rev() {
            if let Child::Branch(child) = node.child(i as i32, realize)? {
                pending.push((child, path.clone_with_child_index(i as i32)));
            }
        }
        action(node, path);
    }
    Ok(())
}

/// Visits the leaves of an inclusive lexical interval. Unrelated branches stay unrealized.
pub(crate) fn traverse_range(root: &Rc<SelectionNode>, start: &IndexPath, end: &IndexPath, mut action: impl FnMut(&SelectionNode, i32) -> Result<(), SelectionError>) -> Result<(), SelectionError> {
    if !start.is_valid() || !end.is_valid() || start.is_empty() || end.is_empty() { return Err(SelectionError::InvalidPath); }
    let mut pending = vec![(root.clone(), IndexPath::default())];
    while let Some((node, prefix)) = pending.pop() {
        let depth = prefix.as_slice().len();
        let first = if start.starts_with(&prefix) { start.as_slice().get(depth).copied().unwrap_or(0) } else { 0 };
        let last = if end.starts_with(&prefix) { end.as_slice().get(depth).copied().unwrap_or(node.data_count() as i32 - 1) } else { node.data_count() as i32 - 1 };
        for index in (first..=last).rev() {
            let path = prefix.clone_with_child_index(index);
            if (path < *start && !start.starts_with(&path)) || (path > *end && !path.starts_with(end)) { continue; }
            match node.child(index, true)? {
                Child::Branch(child) if child.data_count() > 0 => pending.push((child, path)),
                _ => action(&node, index)?,
            }
        }
    }
    Ok(())
}
