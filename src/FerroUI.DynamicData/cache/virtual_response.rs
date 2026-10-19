//! `IVirtualResponse` and `VirtualResponse` (`Cache/IVirtualResponse.cs`,
//! `Cache/VirtualResponse.cs`).

use std::fmt;

/// The window of sorted items that a virtual change set holds (`IVirtualResponse`).
pub trait IVirtualResponse {
    /// The number of items requested (`Size`).
    fn size(&self) -> usize;
    /// The index of the first item (`StartIndex`).
    fn start_index(&self) -> usize;
    /// The number of items of the whole collection (`TotalSize`).
    fn total_size(&self) -> usize;
}

/// The window of sorted items that a virtual change set holds (`VirtualResponse`). Two responses
/// are equal when their parts are (`VirtualResponse.DefaultComparer`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct VirtualResponse {
    size: usize,
    start_index: usize,
    total_size: usize,
}

impl VirtualResponse {
    /// The window of `size` items from `start_index`, of `total_size` items in all
    /// (`VirtualResponse(int size, int startIndex, int totalSize)`).
    pub const fn new(size: usize, start_index: usize, total_size: usize) -> Self {
        Self { size, start_index, total_size }
    }

    /// The number of items requested (`Size`).
    pub const fn size(&self) -> usize {
        self.size
    }

    /// The index of the first item (`StartIndex`).
    pub const fn start_index(&self) -> usize {
        self.start_index
    }

    /// The number of items of the whole collection (`TotalSize`).
    pub const fn total_size(&self) -> usize {
        self.total_size
    }
}

impl IVirtualResponse for VirtualResponse {
    fn size(&self) -> usize {
        self.size
    }

    fn start_index(&self) -> usize {
        self.start_index
    }

    fn total_size(&self) -> usize {
        self.total_size
    }
}

impl fmt::Display for VirtualResponse {
    /// `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Size: {}, StartIndex: {}, TotalSize: {}", self.size, self.start_index, self.total_size)
    }
}
