//! `SortOptions` (`List/SortOptions.cs`).

/// The options of the sort of a list (`SortOptions`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SortOptions {
    /// No options (`None`).
    #[default]
    None,
    /// Find positions with a binary search, which is faster but needs the values that the comparer
    /// reads never to change (`UseBinarySearch`).
    UseBinarySearch,
}
