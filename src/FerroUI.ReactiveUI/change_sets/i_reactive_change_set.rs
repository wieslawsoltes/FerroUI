//! `IReactiveChangeSet` (`ReactiveUI.Core/ChangeSets/IReactiveChangeSet.cs`).

/// A batch of changes of a collection, seen without the type of its items
/// (`IReactiveChangeSet`): the counts of additions and removals that tell whether the count of the
/// collection changed.
pub trait IReactiveChangeSet {
    /// The number of [`Add`](super::ReactiveChangeReason::Add) changes (`Adds`).
    fn adds(&self) -> usize;

    /// The number of [`Remove`](super::ReactiveChangeReason::Remove) changes (`Removes`).
    fn removes(&self) -> usize;
}
