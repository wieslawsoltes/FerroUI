//! `ListFilterPolicy` (`List/ListFilterPolicy.cs`).

/// How a dynamic filter of a list applies a new predicate (`ListFilterPolicy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListFilterPolicy {
    /// Clears the filtered list and adds the items that match again: the order of the source is
    /// kept, at the cost of a reset of the targets (`ClearAndReplace`).
    ClearAndReplace,
    /// Removes the items that no longer match and adds those that now do (`CalculateDiff`), the
    /// default of the operators.
    CalculateDiff,
}
