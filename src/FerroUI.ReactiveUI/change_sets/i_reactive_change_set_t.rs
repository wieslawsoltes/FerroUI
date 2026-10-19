//! `IReactiveChangeSet<T>` (`ReactiveUI.Core/ChangeSets/IReactiveChangeSet{T}.cs`).

use super::{IReactiveChangeSet, ReactiveChange};

/// A batch of changes of a collection of `T` (`IReactiveChangeSet<T>`): a read-only list of
/// [`ReactiveChange`]s (`IReadOnlyList<ReactiveChange<T>>`).
pub trait IReactiveChangeSetOf<T>: IReactiveChangeSet {
    /// The number of changes (`Count`).
    fn count(&self) -> usize;

    /// The change at `index` (the indexer); `None` past the end.
    fn get(&self, index: usize) -> Option<&ReactiveChange<T>>;

    /// The changes, in order (`GetEnumerator`).
    fn changes(&self) -> &[ReactiveChange<T>];
}
