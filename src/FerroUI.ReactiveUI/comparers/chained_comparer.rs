//! `ChainedComparer<T>` (`ReactiveUI.Core/Comparers/ChainedComparer.cs`).

use super::Comparer;
use std::cmp::Ordering;
use std::rc::Rc;

/// A comparer that applies a parent comparer, then a comparison to break its ties
/// (`ChainedComparer<T>`): the chain that `then_by` and `order_by` build.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's class is internal; it is public here so that
/// its tests, in their own crate, reach it. Upstream also returns 0 for two null values before it
/// asks the comparison; a value here cannot be null, and an optional value compares through the
/// comparison it is given (`None` equals `None`).
pub struct ChainedComparer<T> {
    parent: Option<Comparer<T>>,
    comparison: Comparer<T>,
}

impl<T: 'static> ChainedComparer<T> {
    /// A comparer that applies `parent`, if any, then `comparison`
    /// (`ChainedComparer(IComparer<T>?, Comparison<T>)`).
    pub fn new(parent: Option<Comparer<T>>, comparison: impl Fn(&T, &T) -> Ordering + 'static) -> Self {
        Self { parent, comparison: Rc::new(comparison) }
    }

    /// The order of `x` and `y`: the parent's, unless it finds them equal (`Compare`).
    pub fn compare(&self, x: &T, y: &T) -> Ordering {
        let parent_result = self.parent.as_ref().map_or(Ordering::Equal, |parent| parent(x, y));
        if parent_result != Ordering::Equal {
            return parent_result;
        }
        (self.comparison)(x, y)
    }

    /// The comparer as the shared comparison function that sorting takes.
    pub fn into_comparer(self) -> Comparer<T> {
        Rc::new(move |x: &T, y: &T| self.compare(x, y))
    }
}
