// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root.

use std::{cmp::Ordering, fmt, rc::Rc};

/// Failures of path traversal, expired event arguments and stale selection views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    IndexOutOfRange,
    InvalidPath,
    SourceRequired,
    EventArgsExpired,
    SelectionChanged,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::IndexOutOfRange => "The index is outside the source or path.",
            Self::InvalidPath => "The selection path must contain nonnegative indexes.",
            Self::SourceRequired => "A source is required to resolve a nested selection path.",
            Self::EventArgsExpired => "Source and SourceIndex are only accessible in the ChildrenRequested handler.",
            Self::SelectionChanged => "Selection changed after the SelectedIndices or SelectedItems view was read.",
        })
    }
}

impl std::error::Error for SelectionError {}

/// An immutable path from the source root. Display uses the upstream `R.0.1` notation.
///
/// A path may store a negative component, as upstream constructors do; selection operations
/// validate it. An empty path denotes the root, not an unselected item (`None` denotes that).
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexPath(Rc<[i32]>);

impl IndexPath {
    pub fn create_from(index: i32) -> Self { Self(Rc::from([index])) }
    pub fn create_from_group_and_item_index(group_index: i32, item_index: i32) -> Self {
        Self(Rc::from([group_index, item_index]))
    }
    pub fn create_from_indices(indices: impl IntoIterator<Item = i32>) -> Self {
        Self(indices.into_iter().collect::<Vec<_>>().into())
    }
    pub fn get_size(&self) -> i32 { self.0.len() as i32 }
    pub fn get_at(&self, index: i32) -> Result<i32, SelectionError> {
        usize::try_from(index).ok().and_then(|i| self.0.get(i).copied()).ok_or(SelectionError::IndexOutOfRange)
    }
    pub fn compare_to(&self, other: &Self) -> i32 {
        match self.cmp(other) { Ordering::Less => -1, Ordering::Equal => 0, Ordering::Greater => 1 }
    }
    pub fn is_valid(&self) -> bool { self.0.iter().all(|index| *index >= 0) }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn as_slice(&self) -> &[i32] { &self.0 }
    pub fn clone_with_child_index(&self, child_index: i32) -> Self {
        Self::create_from_indices(self.0.iter().copied().chain([child_index]))
    }
    pub(crate) fn starts_with(&self, prefix: &Self) -> bool { self.0.starts_with(&prefix.0) }
}

impl From<i32> for IndexPath { fn from(index: i32) -> Self { Self::create_from(index) } }
impl From<&[i32]> for IndexPath { fn from(indices: &[i32]) -> Self { Self(Rc::from(indices)) } }
impl fmt::Display for IndexPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("R")?;
        for index in self.0.iter() { write!(f, ".{index}")?; }
        Ok(())
    }
}
impl fmt::Debug for IndexPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self, f) }
}

