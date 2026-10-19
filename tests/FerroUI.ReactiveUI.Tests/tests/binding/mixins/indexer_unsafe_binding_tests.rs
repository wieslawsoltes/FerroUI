//! `IndexerUnsafeBindingTests` (`ReactiveUI.Binding.Tests/Mixins/IndexerUnsafeBindingTests.cs`).
//!
//! The paths of these tests go through an indexer of a collection (`vm => vm.Items[0]`), which the
//! run-time fallback reads from the expression tree and observes through the change notification
//! of the collection. A property path has no indexer step.

// Not ported: OneWayBindUnsafe_IndexerPath_CarriesTheItem - expression trees (an indexer step of a member chain)

// Not ported: OneWayBindUnsafe_IndexerPath_FollowsTheReplacedItem - expression trees (an indexer step of a member chain)

// Not ported: OneWayBindUnsafe_PropertyOfIndexedItem_CarriesTheProperty - expression trees (an indexer step of a member chain)
