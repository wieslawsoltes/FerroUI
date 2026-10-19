//! `SortReason` (`Cache/SortReason.cs`).

/// Why a sorted change set was produced (`SortReason`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SortReason {
    /// The first batch of items (`InitialLoad`).
    #[default]
    InitialLoad,
    /// The comparer changed (`ComparerChanged`).
    ComparerChanged,
    /// The items changed (`DataChanged`).
    DataChanged,
    /// The items were sorted again, as asked (`Reorder`).
    Reorder,
    /// The sorted collection was rebuilt, because of the size of the changes (`Reset`).
    Reset,
}
