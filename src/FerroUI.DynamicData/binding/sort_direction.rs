//! `SortDirection` (`Binding/SortDirection.cs`).

/// The direction of a sort (`SortDirection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// From the least to the greatest (`Ascending`).
    #[default]
    Ascending,
    /// From the greatest to the least (`Descending`).
    Descending,
}
