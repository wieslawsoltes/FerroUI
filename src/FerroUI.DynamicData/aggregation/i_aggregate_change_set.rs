//! `IAggregateChangeSet<T>` (`Aggregation/IAggregateChangeSet.cs`).

use super::AggregateItem;

/// A change set shaped for fast continuous aggregations: the items added and removed, in order
/// (`IAggregateChangeSet<T>`, an `IEnumerable<AggregateItem<T>>`).
///
/// The sequences of aggregate change sets are `Rc<dyn IObservable<Rc<dyn IAggregateChangeSet<T>>>>`
/// ([`for_aggregation`](super::AggregationCacheEx::for_aggregation)), the operators of which are
/// those of [`AggregateChangeSetEx`](super::AggregateChangeSetEx).
pub trait IAggregateChangeSet<T> {
    /// The items added and removed, in the order of the changes (`GetEnumerator`).
    fn items(&self) -> Box<dyn Iterator<Item = AggregateItem<T>> + '_>;
}

/// A sequence of aggregate change sets (`IObservable<IAggregateChangeSet<T>>`).
pub type AggregateChangeSets<T> = std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn IAggregateChangeSet<T>>>>;
