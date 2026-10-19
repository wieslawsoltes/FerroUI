//! `AggregateType` (`Aggregation/AggregateType.cs`).

/// Whether an item enters or leaves an aggregation (`AggregateType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AggregateType {
    /// The item is added (`Add`).
    Add,
    /// The item is removed (`Remove`).
    Remove,
}
