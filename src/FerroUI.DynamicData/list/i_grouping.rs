//! `IGrouping<TObject, TGroupKey>` (`List/IGrouping.cs`).

/// The state of a group of a list at one moment (`IGrouping<TObject, TGroupKey>`): the items of
/// the change sets of `GroupWithImmutableState` and `GroupOnPropertyWithImmutableState`.
pub trait IGrouping<T, G> {
    /// The number of items (`Count`).
    fn count(&self) -> usize;

    /// The items (`Items`).
    fn items(&self) -> Vec<T>;

    /// The group key (`Key`).
    fn key(&self) -> &G;
}
