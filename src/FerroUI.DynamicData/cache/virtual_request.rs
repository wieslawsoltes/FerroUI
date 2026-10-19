//! `IVirtualRequest` and `VirtualRequest` (`Cache/IVirtualRequest.cs`, `Cache/VirtualRequest.cs`).

use std::fmt;

/// A request of a window of sorted items (`IVirtualRequest`).
pub trait IVirtualRequest {
    /// The number of items (`Size`).
    fn size(&self) -> usize;
    /// The index of the first item (`StartIndex`).
    fn start_index(&self) -> usize;
}

/// A request of a window of sorted items (`VirtualRequest`): the index of the first item and the
/// number of items.
///
/// Deviation (DEVIATIONS.md, DynamicData): the index and the size are `usize`, so a negative
/// request, which the operators ignore upstream, cannot be made. Two requests are equal when their
/// parts are (`VirtualRequest.StartIndexSizeComparer`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VirtualRequest {
    start_index: usize,
    size: usize,
}

impl VirtualRequest {
    /// The first 25 items (`VirtualRequest.Default`, `new VirtualRequest()`).
    pub const DEFAULT: Self = Self { start_index: 0, size: 25 };

    /// The `size` items from `start_index` (`VirtualRequest(int startIndex, int size)`).
    pub const fn new(start_index: usize, size: usize) -> Self {
        Self { start_index, size }
    }

    /// The number of items (`Size`).
    pub const fn size(&self) -> usize {
        self.size
    }

    /// The index of the first item (`StartIndex`).
    pub const fn start_index(&self) -> usize {
        self.start_index
    }
}

impl Default for VirtualRequest {
    /// The first 25 items (`new VirtualRequest()`).
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl IVirtualRequest for VirtualRequest {
    fn size(&self) -> usize {
        self.size
    }

    fn start_index(&self) -> usize {
        self.start_index
    }
}

impl fmt::Display for VirtualRequest {
    /// `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StartIndex: {}, Size: {}", self.start_index, self.size)
    }
}
