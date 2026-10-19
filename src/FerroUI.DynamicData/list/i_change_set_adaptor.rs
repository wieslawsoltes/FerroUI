//! `IChangeSetAdaptor<T>` (`List/IChangeSetAdaptor.cs`).

use super::ChangeSet;

/// Applies the change sets of a list to something else, such as a collection (`IChangeSetAdaptor<T>`).
pub trait IChangeSetAdaptor<T> {
    /// Applies `changes` (`Adapt`).
    fn adapt(&self, changes: &ChangeSet<T>);
}
