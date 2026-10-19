//! `SortAndPageOptions` and `SortAndVirtualizeOptions` (`Cache/SortAndPageOptions.cs`,
//! `Cache/SortAndVirtualizeOptions.cs`).

/// The options of `SortAndPage` (`SortAndPageOptions`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SortAndPageOptions {
    /// The number of changes from which the sorted items are rebuilt rather than edited; 0 never
    /// rebuilds (`ResetThreshold`, by default `BindingOptions.DefaultResetThreshold`, 25).
    pub reset_threshold: usize,
    /// Find positions with a binary search, which needs the values that the comparer reads never
    /// to change (`UseBinarySearch`).
    pub use_binary_search: bool,
    /// The initial capacity of the sorted list (`InitialCapacity`).
    pub initial_capacity: usize,
}

impl Default for SortAndPageOptions {
    fn default() -> Self {
        Self { reset_threshold: 25, use_binary_search: false, initial_capacity: 0 }
    }
}

/// The options of `SortAndVirtualize` (`SortAndVirtualizeOptions`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SortAndVirtualizeOptions {
    /// The number of changes from which the sorted items are rebuilt rather than edited; 0 never
    /// rebuilds (`ResetThreshold`, by default `BindingOptions.DefaultResetThreshold`, 25).
    pub reset_threshold: usize,
    /// Find positions with a binary search, which needs the values that the comparer reads never
    /// to change (`UseBinarySearch`).
    pub use_binary_search: bool,
    /// The initial capacity of the sorted list (`InitialCapacity`).
    pub initial_capacity: usize,
}

impl Default for SortAndVirtualizeOptions {
    fn default() -> Self {
        Self { reset_threshold: 25, use_binary_search: false, initial_capacity: 0 }
    }
}
