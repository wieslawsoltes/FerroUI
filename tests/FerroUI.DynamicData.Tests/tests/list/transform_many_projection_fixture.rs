//! `TransformManyProjectionFixture` (`DynamicData.Tests/List/TransformManyProjectionFixture.cs`): the
//! children of parents projected with their parent, as one list.
//!
//! The fixture refreshes each parent on the changes of its `ObservableCollection<NestedChild>`,
//! through `AutoRefreshOnObservable(self => self.Children.ToObservableChangeSet())`.

// Not ported: AddRange - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
// Not ported: RemoveChild - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
// Not ported: RemoveParent - needs ObservableCollectionEx.ToObservableChangeSet (wave 2)
