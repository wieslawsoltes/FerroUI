//! `AggregateItem<TObject>` (`Aggregation/AggregateItem.cs`).

use super::AggregateType;

/// An item that is added to or removed from a continuous aggregation (`AggregateItem<TObject>`).
///
/// Equality and hashing are those of the type and the item (upstream's `IEquatable`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AggregateItem<T> {
    aggregate_type: AggregateType,
    item: T,
}

impl<T> AggregateItem<T> {
    /// An item of `aggregate_type` (`AggregateItem(AggregateType, TObject)`).
    pub fn new(aggregate_type: AggregateType, item: T) -> Self {
        Self { aggregate_type, item }
    }

    /// Whether the item is added or removed (`Type`).
    pub fn aggregate_type(&self) -> AggregateType {
        self.aggregate_type
    }

    /// The item (`Item`).
    pub fn item(&self) -> &T {
        &self.item
    }

    /// The item, taken out.
    pub fn into_item(self) -> T {
        self.item
    }
}
