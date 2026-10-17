//! `IndexPath.cs`: the path of indexes from the root of a hierarchical source to a row.
//!
//! Adapted, as the reference, from the WinUI project (<https://github.com/microsoft/microsoft-ui-xaml>),
//! MIT License, Copyright (c) Microsoft Corporation.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A path of indexes into a hierarchy: one index for a row of a flat source or a root row, one more
/// per level below. The empty path (`IndexPath::UNSELECTED`, the default) is no row.
///
/// As the reference, a path of one index is stored without an allocation (`_index` is the index
/// plus one, 0 for the empty path); longer paths share an immutable array.
#[derive(Clone, Default)]
pub struct IndexPath {
    index: i32,
    path: Option<Rc<[i32]>>,
}

impl IndexPath {
    /// The empty path.
    pub const UNSELECTED: IndexPath = IndexPath { index: 0, path: None };

    /// The path of one index.
    pub fn new(index: i32) -> Self {
        Self { index: index + 1, path: None }
    }

    /// The path of the given indexes (`params int[]`), stored as an array even when it has one
    /// index or none, as the reference's constructor does.
    pub fn from_indexes(indexes: &[i32]) -> Self {
        Self { index: 0, path: Some(indexes.into()) }
    }

    /// The path of the given indexes, or the empty path for `None` (`IndexPath(IEnumerable<int>?)`).
    pub fn from_iter_opt(indexes: Option<impl IntoIterator<Item = i32>>) -> Self {
        match indexes {
            Some(indexes) => Self { index: 0, path: Some(indexes.into_iter().collect::<Vec<_>>().into()) },
            None => Self::UNSELECTED,
        }
    }

    fn with_base(base_path: &[i32], index: i32) -> Self {
        let mut path = Vec::with_capacity(base_path.len() + 1);
        path.extend_from_slice(base_path);
        path.push(index);
        Self { index: 0, path: Some(path.into()) }
    }

    /// The encoded index (the index plus one; 0 for a path that is an array or empty), shared with
    /// the presentation so that virtualization does not allocate paths.
    pub fn encoded_index(&self) -> i32 {
        self.index
    }

    /// The array of a path of more than one index (or of a path built from an array).
    pub fn path(&self) -> Option<&Rc<[i32]>> {
        self.path.as_ref()
    }

    /// A path from its encoded parts ([`encoded_index`](Self::encoded_index), [`path`](Self::path)).
    pub fn from_encoded(encoded_index: i32, path: Option<Rc<[i32]>>) -> Self {
        Self { index: encoded_index, path }
    }

    /// The number of indexes.
    pub fn count(&self) -> usize {
        match &self.path {
            Some(path) => path.len(),
            None => usize::from(self.index != 0),
        }
    }

    /// Whether the path is empty (no row).
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// The index at `index`.
    ///
    /// # Panics
    /// When `index` is not below [`count`](Self::count) (`IndexOutOfRangeException`).
    pub fn get(&self, index: usize) -> i32 {
        if index >= self.count() {
            panic!("Index was outside the bounds of the array.");
        }
        match &self.path {
            Some(path) => path[index],
            None => self.index - 1,
        }
    }

    /// The last index (`index[^1]`).
    ///
    /// # Panics
    /// For the empty path.
    pub fn last(&self) -> i32 {
        let count = self.count();
        if count == 0 {
            panic!("Index was outside the bounds of the array.");
        }
        self.get(count - 1)
    }

    /// The path without its last index (`index[..^1]`); the empty path for a path of one index.
    ///
    /// # Panics
    /// For the empty path.
    pub fn parent(&self) -> IndexPath {
        let count = self.count();
        if count == 0 {
            panic!("Invalid IndexPath slice.");
        }
        self.slice(0, count - 1)
    }

    /// The path with `child_index` appended.
    ///
    /// # Panics
    /// When `child_index` is negative (`ArgumentException`).
    pub fn append(&self, child_index: i32) -> IndexPath {
        if child_index < 0 {
            panic!("Invalid child index (Parameter 'childIndex')");
        }
        if let Some(path) = &self.path {
            Self::with_base(path, child_index)
        } else if self.index != 0 {
            Self::with_base(&[self.index - 1], child_index)
        } else {
            Self::new(child_index)
        }
    }

    /// Whether this path is an ancestor of `other`: a shorter prefix of it.
    pub fn is_ancestor_of(&self, other: &IndexPath) -> bool {
        if other.count() <= self.count() {
            return false;
        }
        (0..self.count()).all(|i| self.get(i) == other.get(i))
    }

    /// Whether this path is the parent of `other`: its prefix one shorter.
    pub fn is_parent_of(&self, other: &IndexPath) -> bool {
        let size = self.count();
        if other.count() == size + 1 {
            return (0..size).all(|i| self.get(i) == other.get(i));
        }
        false
    }

    /// The `length` indexes from `start`.
    ///
    /// # Panics
    /// When the range is not within the path (`IndexOutOfRangeException`).
    pub fn slice(&self, start: usize, length: usize) -> IndexPath {
        if start + length > self.count() {
            panic!("Invalid IndexPath slice.");
        }
        match length {
            0 => IndexPath::UNSELECTED,
            1 => IndexPath::new(self.get(start)),
            _ => {
                let path = self.path.as_ref().expect("a path of more than one index is an array");
                IndexPath::from_indexes(&path[start..start + length])
            }
        }
    }

    /// The indexes as a vector.
    pub fn to_vec(&self) -> Vec<i32> {
        self.iter().collect()
    }

    /// The indexes.
    pub fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        (0..self.count()).map(move |i| self.get(i))
    }

    /// `CompareTo`: by index, a prefix before the longer path; an empty path before a non-empty
    /// one.
    pub fn compare(&self, other: &IndexPath) -> Ordering {
        let lhs_count = self.count();
        let rhs_count = other.count();
        let mut result = Ordering::Equal;
        if lhs_count != 0 && rhs_count != 0 {
            for i in 0..lhs_count.min(rhs_count) {
                match self.get(i).cmp(&other.get(i)) {
                    Ordering::Equal => continue,
                    order => {
                        result = order;
                        break;
                    }
                }
            }
        }
        if result == Ordering::Equal {
            lhs_count.cmp(&rhs_count)
        } else {
            result
        }
    }
}

impl From<i32> for IndexPath {
    fn from(index: i32) -> Self {
        IndexPath::new(index)
    }
}

impl PartialEq for IndexPath {
    fn eq(&self, other: &Self) -> bool {
        self.compare(other) == Ordering::Equal
    }
}

impl Eq for IndexPath {}

impl PartialOrd for IndexPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexPath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl Hash for IndexPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Equal paths have the same indexes, whichever way they are stored.
        let mut hash_code: i32 = -504981047;
        for index in self.iter() {
            hash_code = hash_code.wrapping_mul(-1521134295).wrapping_add(index);
        }
        state.write_i32(hash_code);
    }
}

impl fmt::Display for IndexPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(path) = &self.path {
            let parts: Vec<String> = path.iter().map(|i| i.to_string()).collect();
            write!(f, "({})", parts.join("."))
        } else if self.index != 0 {
            write!(f, "({})", self.index - 1)
        } else {
            f.write_str("()")
        }
    }
}

impl fmt::Debug for IndexPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// `new IndexPath(a, b, ...)`.
#[macro_export]
macro_rules! index_path {
    () => { $crate::IndexPath::from_indexes(&[]) };
    ($($index:expr),+ $(,)?) => { $crate::IndexPath::from_indexes(&[$($index),+]) };
}
